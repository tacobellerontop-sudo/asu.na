use super::{Detection, Provider, WINDOWS_MAC};
use crate::context::Ctx;
use crate::fsx::{first_existing, list_dir, read_text};
use crate::types::*;
use std::path::PathBuf;

pub struct Riot;

/// Riot product ids as used by `RiotClientServices --launch-product`. All are free to play.
pub const CATALOG: &[(&str, &str)] = &[
    ("valorant", "VALORANT"),
    ("league_of_legends", "League of Legends"),
    ("bacon", "Legends of Runeterra"),
    ("lion", "2XKO"),
];

const PATCHLINE: &str = "live";

fn data_dir(ctx: &Ctx) -> PathBuf {
    match ctx.platform {
        Platform::Windows => ctx.env_or("ProgramData", r"C:\ProgramData").join("Riot Games"),
        _ => PathBuf::from("/Users/Shared/Riot Games"),
    }
}

/// RiotClientInstalls.json points at RiotClientServices.exe.
pub fn parse_client_installs(json: &str) -> Option<String> {
    let v: serde_json::Value = serde_json::from_str(json).ok()?;
    v["rc_default"].as_str().or(v["rc_live"].as_str()).map(str::to_string)
}

/// One top-level scalar from Riot's small product_settings.yaml files.
pub fn yaml_value(yaml: &str, key: &str) -> Option<String> {
    yaml.lines().find_map(|l| {
        let rest = l.strip_prefix(key)?.strip_prefix(':')?;
        let v = rest.trim().trim_matches('"').trim();
        (!v.is_empty()).then(|| v.to_string())
    })
}

impl Provider for Riot {
    fn id(&self) -> LauncherId {
        LauncherId::Riot
    }
    fn platforms(&self) -> &'static [Platform] {
        WINDOWS_MAC
    }
    fn download_url(&self) -> &'static str {
        "https://www.riotgames.com/en/download"
    }

    fn detect(&self, ctx: &Ctx) -> Detection {
        let from_json = read_text(data_dir(ctx).join("RiotClientInstalls.json")).and_then(|j| parse_client_installs(&j));
        Detection::found(first_existing([
            ctx.path_override.as_ref().map(|p| p.join("RiotClientServices.exe")),
            from_json.map(PathBuf::from),
            Some(PathBuf::from(r"C:\Riot Games\Riot Client\RiotClientServices.exe")),
        ]))
    }

    fn open(&self, d: &Detection) -> LaunchAction {
        match &d.path {
            Some(p) => LaunchAction::exec(p, &[]),
            None => LaunchAction::uri("https://www.riotgames.com/en/download"),
        }
    }

    fn scan(&self, ctx: &Ctx, d: &Detection) -> Vec<DetectedGame> {
        let Some(client) = &d.path else { return vec![] };
        let meta = data_dir(ctx).join("Metadata");
        let folders = list_dir(&meta);
        CATALOG
            .iter()
            .map(|(product, title)| {
                let id = format!("{product}.{PATCHLINE}");
                let install = folders
                    .contains(&id)
                    .then(|| read_text(meta.join(&id).join(format!("{id}.product_settings.yaml"))))
                    .flatten()
                    .and_then(|y| yaml_value(&y, "product_install_full_path"))
                    .map(PathBuf::from)
                    .filter(|p| p.exists());
                let run = |flag: &str| LaunchAction::Exec {
                    file: client.clone(),
                    args: vec![format!("--{flag}-product={product}"), format!("--{flag}-patchline={PATCHLINE}")],
                    cwd: None,
                    env: vec![],
                };
                let mut g = DetectedGame::new(LauncherId::Riot, *product, *title);
                g.installed = install.is_some();
                g.catalog_only = install.is_none();
                g.actions = Actions {
                    launch: Some(run("launch")),
                    // Launching a product that isn't installed opens its install page in the Riot Client.
                    install: Some(run("launch")),
                    uninstall: install.as_ref().map(|_| run("uninstall")),
                    store: Some(run("launch")),
                };
                g.install_path = install;
                g
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::testing::{tree, Fakes};

    #[test]
    fn yaml_and_installs_json() {
        assert_eq!(yaml_value("a: 1\nproduct_install_full_path: \"C:/Riot Games/VALORANT/live\"\n", "product_install_full_path"), Some("C:/Riot Games/VALORANT/live".into()));
        assert_eq!(parse_client_installs(r#"{"rc_default":"C:/Riot Games/Riot Client/RiotClientServices.exe"}"#), Some("C:/Riot Games/Riot Client/RiotClientServices.exe".into()));
    }

    #[test]
    fn detects_client_and_installed_products() {
        let games_dir = tree(&[("VALORANT/live/x", "")]);
        let val = games_dir.path().join("VALORANT/live");
        let client_dir = tree(&[("RiotClientServices.exe", "")]);
        let client = client_dir.path().join("RiotClientServices.exe");
        let pd = tree(&[
            ("Riot Games/RiotClientInstalls.json", &serde_json::json!({ "rc_default": client }).to_string()),
            ("Riot Games/Metadata/valorant.live/valorant.live.product_settings.yaml", &format!("product_install_full_path: \"{}\"\n", val.display())),
        ]);
        let f = Fakes::new();
        let ctx = f.ctx(Platform::Windows, &[("ProgramData", pd.path())]);
        let d = Riot.detect(&ctx);
        assert_eq!(d.path, Some(client.clone()));
        let games = Riot.scan(&ctx, &d);
        let v = games.iter().find(|g| g.id == "valorant").unwrap();
        assert!(v.installed);
        assert_eq!(v.install_path, Some(val));
        assert_eq!(v.actions.launch, Some(LaunchAction::exec(&client, &["--launch-product=valorant", "--launch-patchline=live"])));
        let lol = games.iter().find(|g| g.id == "league_of_legends").unwrap();
        assert!(!lol.installed && lol.catalog_only && lol.actions.uninstall.is_none());
    }
}
