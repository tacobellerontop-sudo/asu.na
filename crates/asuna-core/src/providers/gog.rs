use super::{Detection, Provider, WINDOWS};
use crate::context::Ctx;
use crate::fsx::first_existing;
use crate::registry::children;
use crate::types::*;
use std::path::PathBuf;

const GAMES_KEY: &str = r"HKLM\SOFTWARE\WOW6432Node\GOG.com\Games";
const CLIENT_KEY: &str = r"HKLM\SOFTWARE\WOW6432Node\GOG.com\GalaxyClient\paths";

pub struct Gog;

impl Provider for Gog {
    fn id(&self) -> LauncherId {
        LauncherId::Gog
    }
    fn platforms(&self) -> &'static [Platform] {
        WINDOWS
    }
    fn download_url(&self) -> &'static str {
        "https://www.gog.com/galaxy"
    }

    fn detect(&self, ctx: &Ctx) -> Detection {
        let paths = ctx.registry.query(CLIENT_KEY);
        let dir = first_existing([
            ctx.path_override.clone(),
            paths.first().and_then(|k| k.get("client")).map(PathBuf::from),
            Some(ctx.env_or("ProgramFiles(x86)", r"C:\Program Files (x86)").join("GOG Galaxy")),
        ]);
        if let Some(exe) = dir.map(|d| d.join("GalaxyClient.exe")).filter(|e| e.exists()) {
            return Detection { detected: true, path: Some(exe) };
        }
        // Games from offline installers still register under GOG.com\Games.
        Detection { detected: !ctx.registry.query(GAMES_KEY).is_empty(), path: None }
    }

    fn open(&self, d: &Detection) -> LaunchAction {
        match &d.path {
            Some(p) => LaunchAction::exec(p, &[]),
            None => LaunchAction::uri("goggalaxy://openStoreUrl/www.gog.com"),
        }
    }

    fn scan(&self, ctx: &Ctx, d: &Detection) -> Vec<DetectedGame> {
        let all = ctx.registry.query(GAMES_KEY);
        let mut out = Vec::new();
        for key in children(&all, GAMES_KEY) {
            let id = key.get("gameID").unwrap_or(&key.name).to_string();
            let Some(title) = key.get("gameName") else { continue };
            // DLCs point at their base game.
            if key.get("dependsOn").is_some() {
                continue;
            }
            let path = key.get("path").map(PathBuf::from);
            let exe = key.get("exe").map(PathBuf::from);
            let mut g = DetectedGame::new(LauncherId::Gog, &id, title);
            g.installed = true;
            g.executable = exe.clone();
            g.actions.launch = match (&d.path, &path, &exe) {
                (Some(galaxy), Some(p), _) => Some(LaunchAction::Exec {
                    file: galaxy.clone(),
                    args: vec!["/command=runGame".into(), format!("/gameId={id}"), format!("/path={}", p.display())],
                    cwd: None,
                    env: vec![],
                }),
                (_, _, Some(e)) => Some(LaunchAction::Exec {
                    file: e.clone(),
                    args: vec![],
                    cwd: key.get("workingDir").map(PathBuf::from).or(path.clone()),
                    env: vec![],
                }),
                _ => None,
            };
            let page = LaunchAction::uri(format!("goggalaxy://openGameView/{id}"));
            g.actions.install = Some(page.clone());
            g.actions.store = Some(page);
            g.actions.uninstall = path.as_ref().map(|p| p.join("unins000.exe")).filter(|u| u.exists()).map(|u| LaunchAction::exec(u, &[]));
            g.install_path = path;
            out.push(g);
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
    fn lists_games_skips_dlc_launches_through_galaxy() {
        let galaxy = tree(&[("GalaxyClient.exe", "")]);
        let game = tree(&[("unins000.exe", "")]);
        let gd = game.path().to_string_lossy().to_string();
        let f = Fakes::new().reg(vec![
            RegKey::new(r"HKEY_LOCAL_MACHINE\SOFTWARE\WOW6432Node\GOG.com\GalaxyClient\paths", &[("client", &galaxy.path().to_string_lossy())]),
            RegKey::new(r"HKEY_LOCAL_MACHINE\SOFTWARE\WOW6432Node\GOG.com\Games\1", &[("gameID", "1"), ("gameName", "The Witcher"), ("path", &gd), ("exe", "witcher.exe")]),
            RegKey::new(r"HKEY_LOCAL_MACHINE\SOFTWARE\WOW6432Node\GOG.com\Games\2", &[("gameID", "2"), ("gameName", "DLC"), ("dependsOn", "1")]),
        ]);
        let ctx = f.ctx(Platform::Windows, &[]);
        let d = Gog.detect(&ctx);
        assert_eq!(d.path, Some(galaxy.path().join("GalaxyClient.exe")));
        let games = Gog.scan(&ctx, &d);
        assert_eq!(games.len(), 1);
        assert_eq!(
            games[0].actions.launch,
            Some(LaunchAction::Exec {
                file: galaxy.path().join("GalaxyClient.exe"),
                args: vec!["/command=runGame".into(), "/gameId=1".into(), format!("/path={gd}")],
                cwd: None,
                env: vec![]
            })
        );
        assert_eq!(games[0].actions.uninstall, Some(LaunchAction::exec(game.path().join("unins000.exe"), &[])));
    }
}
