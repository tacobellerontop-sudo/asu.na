use super::{Detection, Provider, WINDOWS};
use crate::context::Ctx;
use crate::fsx::{any_basename, first_existing, win_path};
use crate::registry::{children, UNINSTALL_KEYS};
use crate::types::*;
use std::collections::HashMap;
use std::path::PathBuf;

const LAUNCHER_KEY: &str = r"HKLM\SOFTWARE\WOW6432Node\Ubisoft\Launcher";
const INSTALLS_KEY: &str = r"HKLM\SOFTWARE\WOW6432Node\Ubisoft\Launcher\Installs";

pub struct Ubisoft;

pub fn actions(id: &str) -> Actions {
    Actions {
        launch: Some(LaunchAction::uri(format!("uplay://launch/{id}/0"))),
        install: Some(LaunchAction::uri(format!("uplay://install/{id}"))),
        uninstall: Some(LaunchAction::uri(format!("uplay://uninstall/{id}"))),
        store: Some(LaunchAction::uri(format!("uplay://open/game/{id}"))),
    }
}

impl Provider for Ubisoft {
    fn id(&self) -> LauncherId {
        LauncherId::Ubisoft
    }
    fn platforms(&self) -> &'static [Platform] {
        WINDOWS
    }
    fn download_url(&self) -> &'static str {
        "https://ubisoftconnect.com/"
    }
    fn detect(&self, ctx: &Ctx) -> Detection {
        let all = ctx.registry.query(LAUNCHER_KEY);
        let root = all.iter().find(|k| k.name.eq_ignore_ascii_case("Launcher"));
        Detection::found(first_existing([
            ctx.path_override.clone(),
            root.and_then(|k| k.get("InstallDir")).map(PathBuf::from),
            Some(ctx.env_or("ProgramFiles(x86)", r"C:\Program Files (x86)").join("Ubisoft").join("Ubisoft Game Launcher")),
        ]))
    }
    fn open(&self, _d: &Detection) -> LaunchAction {
        LaunchAction::uri("uplay://")
    }

    fn scan(&self, ctx: &Ctx, _d: &Detection) -> Vec<DetectedGame> {
        let all = ctx.registry.query(INSTALLS_KEY);
        let installs = children(&all, INSTALLS_KEY);
        if installs.is_empty() {
            return vec![];
        }
        // Friendly names live in the uninstall entries: "Uplay Install <id>".
        let mut names = HashMap::new();
        for root in UNINSTALL_KEYS {
            let keys = ctx.registry.query(root);
            for k in children(&keys, root) {
                if let (Some(id), Some(dn)) = (k.name.strip_prefix("Uplay Install "), k.get("DisplayName")) {
                    names.insert(id.to_string(), dn.to_string());
                }
            }
        }
        installs
            .into_iter()
            .map(|k| {
                let dir = k.get("InstallDir").map(win_path);
                let title = names.get(&k.name).cloned().or_else(|| k.get("InstallDir").map(|d| any_basename(d).to_string())).unwrap_or_else(|| format!("Ubisoft game {}", k.name));
                let mut g = DetectedGame::new(LauncherId::Ubisoft, &k.name, title);
                g.installed = true;
                g.install_path = dir;
                g.actions = actions(&k.name);
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
    fn names_from_uninstall_entries() {
        let f = Fakes::new().reg(vec![
            RegKey::new(r"HKEY_LOCAL_MACHINE\SOFTWARE\WOW6432Node\Ubisoft\Launcher\Installs\5595", &[("InstallDir", "C:/Ubisoft/AC Valhalla/")]),
            RegKey::new(r"HKEY_LOCAL_MACHINE\SOFTWARE\WOW6432Node\Ubisoft\Launcher\Installs\46", &[("InstallDir", "C:/Ubisoft/Far Cry 3/")]),
            RegKey::new(r"HKEY_LOCAL_MACHINE\SOFTWARE\WOW6432Node\Microsoft\Windows\CurrentVersion\Uninstall\Uplay Install 5595", &[("DisplayName", "Assassin's Creed Valhalla")]),
        ]);
        let mut games = Ubisoft.scan(&f.ctx(Platform::Windows, &[]), &Detection::default());
        games.sort_by(|a, b| a.title.cmp(&b.title));
        assert_eq!(games[0].title, "Assassin's Creed Valhalla");
        assert_eq!(games[1].title, "Far Cry 3");
        assert_eq!(games[0].actions.launch, Some(LaunchAction::uri("uplay://launch/5595/0")));
    }
}
