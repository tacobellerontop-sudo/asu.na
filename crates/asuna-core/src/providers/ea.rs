use super::{Detection, Provider, WINDOWS};
use crate::context::Ctx;
use crate::fsx::{first_existing, read_text};
use crate::registry::{children, split_command_line, UNINSTALL_KEYS};
use crate::types::*;
use std::collections::HashSet;
use std::path::PathBuf;

pub struct Ea;

/// Content ids from <game>\__Installer\installerdata.xml, used by EA's launch links.
pub fn parse_content_ids(xml: &str) -> Vec<String> {
    let lower = xml.to_ascii_lowercase();
    let mut out = Vec::new();
    let mut from = 0;
    while let Some(start) = lower[from..].find("<contentid>") {
        let s = from + start + "<contentid>".len();
        let Some(end) = lower[s..].find("</contentid>") else { break };
        let id = xml[s..s + end].trim();
        if !id.is_empty() {
            out.push(id.to_string());
        }
        from = s + end;
    }
    out
}

fn is_ea_publisher(p: &str) -> bool {
    let p = p.to_ascii_lowercase();
    ["electronic arts", "ea games", "ea swiss", "respawn", "bioware", "codemasters"].iter().any(|x| p.contains(x))
}

impl Provider for Ea {
    fn id(&self) -> LauncherId {
        LauncherId::Ea
    }
    fn platforms(&self) -> &'static [Platform] {
        WINDOWS
    }
    fn download_url(&self) -> &'static str {
        "https://www.ea.com/ea-app"
    }
    fn detect(&self, ctx: &Ctx) -> Detection {
        let key = ctx.registry.query(r"HKLM\SOFTWARE\Electronic Arts\EA Desktop");
        Detection::found(first_existing([
            ctx.path_override.clone(),
            key.first().and_then(|k| k.get("ClientPath")).map(PathBuf::from),
            Some(ctx.env_or("ProgramFiles", r"C:\Program Files").join("Electronic Arts").join("EA Desktop").join("EA Desktop").join("EADesktop.exe")),
        ]))
    }
    fn open(&self, d: &Detection) -> LaunchAction {
        match &d.path {
            Some(p) => LaunchAction::exec(p, &[]),
            None => LaunchAction::uri("origin2://library/open"),
        }
    }

    fn scan(&self, ctx: &Ctx, _d: &Detection) -> Vec<DetectedGame> {
        let mut out = Vec::new();
        let mut seen = HashSet::new();
        for root in UNINSTALL_KEYS {
            let keys = ctx.registry.query(root);
            for k in children(&keys, root) {
                let (Some(title), Some(dir)) = (k.get("DisplayName"), k.get("InstallLocation")) else { continue };
                if !is_ea_publisher(k.get("Publisher").unwrap_or("")) {
                    continue;
                }
                let tl = title.to_ascii_lowercase();
                if tl.starts_with("ea app") || tl.starts_with("ea desktop") || tl.starts_with("origin") {
                    continue;
                }
                let dir = PathBuf::from(dir.trim_end_matches('\\'));
                // Not managed by the EA app without installer data.
                let Some(xml) = read_text(dir.join("__Installer").join("installerdata.xml")) else { continue };
                let ids = parse_content_ids(&xml);
                let Some(first) = ids.first().cloned() else { continue };
                if !seen.insert(first.clone()) {
                    continue;
                }
                let mut g = DetectedGame::new(LauncherId::Ea, &first, title);
                g.installed = true;
                g.install_path = Some(dir);
                g.size_bytes = k.get("EstimatedSize").and_then(|s| s.parse::<u64>().ok()).map(|kb| kb * 1024);
                g.actions.launch = Some(LaunchAction::uri(format!("origin2://game/launch?offerIds={}&autoDownload=1", ids.join("%2C"))));
                g.actions.install = Some(LaunchAction::uri(format!("origin2://game/download?offerId={first}")));
                g.actions.store = Some(LaunchAction::uri("origin2://store/open"));
                g.actions.uninstall = k
                    .get("UninstallString")
                    .and_then(split_command_line)
                    .map(|(file, args)| LaunchAction::Exec { file: file.into(), args, cwd: None, env: vec![] });
                out.push(g);
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::testing::{tree, Fakes};
    use crate::registry::RegKey;

    #[test]
    fn content_ids() {
        assert_eq!(parse_content_ids("<contentIDs><contentID>1234</contentID><ContentID> 5678 </ContentID></contentIDs>"), vec!["1234", "5678"]);
    }

    #[test]
    fn only_games_with_installer_data() {
        let with = tree(&[("__Installer/installerdata.xml", "<contentIDs><contentID>Origin.OFR.50.0002694</contentID></contentIDs>")]);
        let w = with.path().to_string_lossy().to_string();
        let u = r"HKEY_LOCAL_MACHINE\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall";
        let f = Fakes::new().reg(vec![
            RegKey::new(&format!(r"{u}\apex"), &[("DisplayName", "Apex Legends"), ("Publisher", "Electronic Arts"), ("InstallLocation", &w)]),
            RegKey::new(&format!(r"{u}\other"), &[("DisplayName", "Not a game"), ("Publisher", "Electronic Arts"), ("InstallLocation", "/nope")]),
            RegKey::new(&format!(r"{u}\eaapp"), &[("DisplayName", "EA app"), ("Publisher", "Electronic Arts"), ("InstallLocation", &w)]),
        ]);
        let games = Ea.scan(&f.ctx(Platform::Windows, &[]), &Detection::default());
        assert_eq!(games.len(), 1);
        assert_eq!(games[0].title, "Apex Legends");
        assert_eq!(games[0].actions.launch, Some(LaunchAction::uri("origin2://game/launch?offerIds=Origin.OFR.50.0002694&autoDownload=1")));
    }
}
