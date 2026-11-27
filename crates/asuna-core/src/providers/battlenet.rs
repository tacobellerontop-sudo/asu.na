use super::{Detection, Provider, WINDOWS};
use crate::context::Ctx;
use crate::fsx::first_existing;
use crate::registry::{children, split_command_line, UNINSTALL_KEYS};
use crate::types::*;
use std::collections::HashMap;
use std::path::PathBuf;

pub struct BattleNet;

/// Battle.net has no readable "owned games" list, so this catalog doubles as the install list.
/// `matches` holds lower-case prefixes of the Windows uninstall DisplayName.
pub const CATALOG: &[(&str, &str, &[&str])] = &[
    ("WoW", "World of Warcraft", &["world of warcraft"]),
    ("WoWC", "World of Warcraft Classic", &["world of warcraft classic"]),
    ("Pro", "Overwatch 2", &["overwatch"]),
    ("WTCG", "Hearthstone", &["hearthstone"]),
    ("Fen", "Diablo IV", &["diablo iv"]),
    ("OSI", "Diablo II: Resurrected", &["diablo ii"]),
    ("D3", "Diablo III", &["diablo iii"]),
    ("ANBS", "Diablo Immortal", &["diablo immortal"]),
    ("Hero", "Heroes of the Storm", &["heroes of the storm"]),
    ("S2", "StarCraft II", &["starcraft ii"]),
    ("S1", "StarCraft: Remastered", &["starcraft remastered", "starcraft"]),
    ("W3", "Warcraft III: Reforged", &["warcraft iii"]),
    ("GRY", "Warcraft Rumble", &["warcraft rumble"]),
    ("RTRO", "Blizzard Arcade Collection", &["blizzard arcade"]),
    ("Auks", "Call of Duty", &["call of duty"]),
];

/// Finds the catalog code for an uninstall DisplayName, preferring the longest match.
pub fn match_code(display_name: &str) -> Option<&'static str> {
    let n = display_name.trim().to_ascii_lowercase();
    CATALOG
        .iter()
        .flat_map(|(code, _, prefixes)| prefixes.iter().map(move |p| (*code, *p)))
        .filter(|(_, p)| n == *p || n.starts_with(&format!("{p} ")) || n.starts_with(&format!("{p}:")))
        .max_by_key(|(_, p)| p.len())
        .map(|(c, _)| c)
}

impl Provider for BattleNet {
    fn id(&self) -> LauncherId {
        LauncherId::Battlenet
    }
    fn platforms(&self) -> &'static [Platform] {
        WINDOWS
    }
    fn download_url(&self) -> &'static str {
        "https://download.battle.net/"
    }
    fn detect(&self, ctx: &Ctx) -> Detection {
        let dir = first_existing([ctx.path_override.clone(), Some(ctx.env_or("ProgramFiles(x86)", r"C:\Program Files (x86)").join("Battle.net"))]);
        Detection::found(dir.map(|d| d.join("Battle.net.exe")).filter(|e| e.exists()))
    }
    fn open(&self, d: &Detection) -> LaunchAction {
        match &d.path {
            Some(p) => LaunchAction::exec(p, &[]),
            None => LaunchAction::uri("battlenet://"),
        }
    }

    fn scan(&self, ctx: &Ctx, d: &Detection) -> Vec<DetectedGame> {
        let mut installed: HashMap<&str, (Option<PathBuf>, Option<LaunchAction>)> = HashMap::new();
        for root in UNINSTALL_KEYS {
            let keys = ctx.registry.query(root);
            for k in children(&keys, root) {
                if !k.get("Publisher").unwrap_or("").to_ascii_lowercase().contains("blizzard") {
                    continue;
                }
                let Some(code) = k.get("DisplayName").and_then(match_code) else { continue };
                let uninstall = k
                    .get("UninstallString")
                    .and_then(split_command_line)
                    .map(|(file, args)| LaunchAction::Exec { file: file.into(), args, cwd: None, env: vec![] });
                installed.insert(code, (k.get("InstallLocation").map(PathBuf::from), uninstall));
            }
        }
        CATALOG
            .iter()
            .map(|(code, title, _)| {
                let page = LaunchAction::uri(format!("battlenet://{code}"));
                let mut g = DetectedGame::new(LauncherId::Battlenet, *code, *title);
                if let Some((dir, uninstall)) = installed.remove(code) {
                    g.installed = true;
                    g.install_path = dir;
                    g.actions.uninstall = uninstall;
                } else {
                    g.catalog_only = true;
                }
                g.actions.launch = Some(match &d.path {
                    Some(client) => LaunchAction::Exec { file: client.clone(), args: vec![format!("--exec=launch {code}")], cwd: None, env: vec![] },
                    None => page.clone(),
                });
                g.actions.install = Some(page.clone());
                g.actions.store = Some(page);
                g
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::testing::Fakes;
    use crate::registry::RegKey;

    #[test]
    fn matching() {
        assert_eq!(match_code("Overwatch"), Some("Pro"));
        assert_eq!(match_code("World of Warcraft Classic"), Some("WoWC"));
        assert_eq!(match_code("World of Warcraft"), Some("WoW"));
        assert_eq!(match_code("Diablo III"), Some("D3"));
        assert_eq!(match_code("Diablo II: Resurrected"), Some("OSI"));
        assert_eq!(match_code("StarCraft II"), Some("S2"));
        assert_eq!(match_code("Something else"), None);
    }

    #[test]
    fn installed_vs_catalog() {
        let u = r"HKEY_LOCAL_MACHINE\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall";
        let f = Fakes::new().reg(vec![
            RegKey::new(&format!(r"{u}\Overwatch"), &[("DisplayName", "Overwatch"), ("Publisher", "Blizzard Entertainment"), ("InstallLocation", r"C:\Games\Overwatch"), ("UninstallString", r#""C:\ProgramData\Battle.net\Agent\Blizzard Uninstaller.exe" --uid=prometheus"#)]),
            RegKey::new(&format!(r"{u}\Diablo IV"), &[("DisplayName", "Diablo IV"), ("Publisher", "Someone Else")]),
        ]);
        let d = Detection { detected: true, path: Some(PathBuf::from(r"C:\BNet\Battle.net.exe")) };
        let games = BattleNet.scan(&f.ctx(Platform::Windows, &[]), &d);
        let ow = games.iter().find(|g| g.id == "Pro").unwrap();
        assert!(ow.installed && !ow.catalog_only);
        assert_eq!(ow.actions.launch, Some(LaunchAction::exec(r"C:\BNet\Battle.net.exe", &["--exec=launch Pro"])));
        let d4 = games.iter().find(|g| g.id == "Fen").unwrap();
        assert!(!d4.installed && d4.catalog_only);
    }
}
