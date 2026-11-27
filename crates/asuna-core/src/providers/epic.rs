use super::{Detection, Provider, WINDOWS_MAC};
use crate::context::Ctx;
use crate::fsx::{first_existing, list_dir, read_text};
use crate::types::*;
use base64::Engine;
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;

const URI: &str = "com.epicgames.launcher://apps";

pub struct Epic;

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "PascalCase", default)]
pub struct Manifest {
    pub app_name: String,
    pub display_name: String,
    pub catalog_namespace: Option<String>,
    pub catalog_item_id: Option<String>,
    pub install_location: Option<String>,
    pub install_size: Option<u64>,
    pub launch_executable: Option<String>,
    pub main_game_app_name: Option<String>,
    pub app_categories: Option<Vec<String>>,
    #[serde(rename = "bIsIncompleteInstall")]
    pub incomplete: bool,
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct CatalogItem {
    pub id: String,
    pub namespace: String,
    pub title: String,
    pub categories: Vec<Category>,
    pub main_game_item: Option<serde_json::Value>,
    pub release_info: Vec<ReleaseInfo>,
    pub key_images: Vec<KeyImage>,
}
#[derive(Debug, Default, Deserialize)]
pub struct Category {
    pub path: String,
}
#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct ReleaseInfo {
    pub app_id: String,
    pub platform: Option<Vec<String>>,
}
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
pub struct KeyImage {
    #[serde(rename = "type")]
    pub kind: String,
    pub url: String,
}

/// Epic deep links use `namespace:catalogItemId:appName`, URL-encoded.
pub fn app_ref(ns: Option<&str>, catalog: Option<&str>, app: &str) -> String {
    match (ns, catalog) {
        (Some(n), Some(c)) => format!("{n}%3A{c}%3A{app}"),
        _ => app.to_string(),
    }
}

pub fn actions(r: &str) -> Actions {
    Actions {
        launch: Some(LaunchAction::uri(format!("{URI}/{r}?action=launch&silent=true"))),
        install: Some(LaunchAction::uri(format!("{URI}/{r}?action=install"))),
        // Epic has no uninstall link; the library is where its menu lives.
        uninstall: Some(LaunchAction::uri("com.epicgames.launcher://library")),
        store: Some(LaunchAction::uri("com.epicgames.launcher://store")),
    }
}

/// Base games only: no DLC, no Unreal Engine and other apps.
pub fn is_base_game(m: &Manifest) -> bool {
    if m.main_game_app_name.as_deref().is_some_and(|main| main != m.app_name) {
        return false;
    }
    !matches!(&m.app_categories, Some(c) if !c.is_empty() && !c.iter().any(|x| x == "games"))
}

/// The launcher caches the owned catalog as base64 JSON. Undocumented, so failures mean "nothing extra".
pub fn parse_catalog_cache(raw: &str) -> Vec<CatalogItem> {
    base64::engine::general_purpose::STANDARD
        .decode(raw.trim())
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}

fn art_from(images: &[KeyImage]) -> Art {
    let pick = |types: &[&str]| types.iter().find_map(|t| images.iter().find(|i| i.kind == *t).map(|i| i.url.clone()));
    Art {
        cover: pick(&["DieselGameBoxTall", "OfferImageTall", "Thumbnail"]),
        header: pick(&["DieselGameBox", "OfferImageWide", "DieselStoreFrontWide"]),
        hero: pick(&["DieselStoreFrontWide", "OfferImageWide", "DieselGameBox"]),
        logo: pick(&["DieselGameBoxLogo"]),
    }
}

fn dirs(ctx: &Ctx) -> (PathBuf, PathBuf) {
    if ctx.platform == Platform::Windows {
        let manifests = ctx.env_or("ProgramData", r"C:\ProgramData").join("Epic").join("EpicGamesLauncher").join("Data").join("Manifests");
        let local = ctx.env("LOCALAPPDATA").map(PathBuf::from).unwrap_or_else(|| ctx.home.join("AppData").join("Local"));
        (manifests, local.join("EpicGamesLauncher").join("Saved").join("Data").join("Catalog").join("catcache.bin"))
    } else {
        let base = ctx.home.join("Library/Application Support/Epic/EpicGamesLauncher");
        (base.join("Data/Manifests"), base.join("Saved/Data/Catalog/catcache.bin"))
    }
}

impl Provider for Epic {
    fn id(&self) -> LauncherId {
        LauncherId::Epic
    }
    fn platforms(&self) -> &'static [Platform] {
        WINDOWS_MAC
    }
    fn download_url(&self) -> &'static str {
        "https://store.epicgames.com/download"
    }
    fn detect(&self, ctx: &Ctx) -> Detection {
        Detection::found(first_existing([ctx.path_override.clone(), Some(dirs(ctx).0)]))
    }
    fn open(&self, _d: &Detection) -> LaunchAction {
        LaunchAction::uri("com.epicgames.launcher://library")
    }

    fn scan(&self, ctx: &Ctx, d: &Detection) -> Vec<DetectedGame> {
        let (default_manifests, catcache) = dirs(ctx);
        let manifest_dir = d.path.clone().unwrap_or(default_manifests);
        let catalog = read_text(&catcache).map(|r| parse_catalog_cache(&r)).unwrap_or_default();
        let by_id: HashMap<&str, &CatalogItem> = catalog.iter().map(|c| (c.id.as_str(), c)).collect();

        let mut games: BTreeMap<String, DetectedGame> = BTreeMap::new();
        for file in list_dir(&manifest_dir) {
            if !file.ends_with(".item") {
                continue;
            }
            let Some(m) = read_text(manifest_dir.join(&file)).and_then(|t| serde_json::from_str::<Manifest>(&t).ok()) else {
                continue;
            };
            if m.app_name.is_empty() || m.display_name.is_empty() || !is_base_game(&m) {
                continue;
            }
            let mut g = DetectedGame::new(LauncherId::Epic, &m.app_name, &m.display_name);
            g.installed = !m.incomplete;
            g.install_path = m.install_location.as_ref().map(PathBuf::from);
            g.size_bytes = m.install_size.filter(|s| *s > 0);
            g.executable = match (&m.install_location, &m.launch_executable) {
                (Some(dir), Some(exe)) if !exe.is_empty() => Some(PathBuf::from(dir).join(exe)),
                _ => None,
            };
            if let Some(item) = m.catalog_item_id.as_deref().and_then(|id| by_id.get(id)) {
                g.art = art_from(&item.key_images);
            }
            g.actions = actions(&app_ref(m.catalog_namespace.as_deref(), m.catalog_item_id.as_deref(), &m.app_name));
            games.insert(m.app_name.clone(), g);
        }

        // Owned but not installed, from the catalog cache.
        let want = if ctx.platform == Platform::Mac { "Mac" } else { "Windows" };
        for item in &catalog {
            if item.main_game_item.is_some() || !item.categories.iter().any(|c| c.path == "games") {
                continue;
            }
            let Some(rel) = item
                .release_info
                .iter()
                .find(|r| r.platform.as_ref().is_none_or(|p| p.iter().any(|x| x == want)))
            else {
                continue;
            };
            if rel.app_id.is_empty() || games.contains_key(&rel.app_id) {
                continue;
            }
            let mut g = DetectedGame::new(LauncherId::Epic, &rel.app_id, &item.title);
            g.art = art_from(&item.key_images);
            g.actions = actions(&app_ref(Some(&item.namespace), Some(&item.id), &rel.app_id));
            games.insert(rel.app_id.clone(), g);
        }
        // Installed first in key order is not meaningful; keep stable order by title later.
        games.into_values().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::testing::{tree, Fakes};
    use base64::Engine;

    #[test]
    fn refs_and_dlc_filter() {
        assert_eq!(app_ref(Some("fn"), Some("abc"), "Fortnite"), "fn%3Aabc%3AFortnite");
        assert_eq!(app_ref(None, None, "Fortnite"), "Fortnite");
        let m = |main: Option<&str>, cats: Option<Vec<&str>>| Manifest {
            app_name: "a".into(),
            display_name: "A".into(),
            main_game_app_name: main.map(Into::into),
            app_categories: cats.map(|c| c.into_iter().map(Into::into).collect()),
            ..Default::default()
        };
        assert!(!is_base_game(&m(Some("b"), None)));
        assert!(!is_base_game(&m(None, Some(vec!["engines"]))));
        assert!(is_base_game(&m(Some("a"), Some(vec!["public", "games"]))));
        assert!(parse_catalog_cache("junk").is_empty());
    }

    #[test]
    fn reads_manifests_and_owned_catalog() {
        let catalog = serde_json::json!([
            { "id": "cat1", "namespace": "ns1", "title": "Installed Game", "categories": [{ "path": "games" }], "releaseInfo": [{ "appId": "App1", "platform": ["Windows"] }], "keyImages": [{ "type": "DieselGameBoxTall", "url": "https://img/tall.jpg" }] },
            { "id": "cat2", "namespace": "ns2", "title": "Owned Game", "categories": [{ "path": "games" }], "releaseInfo": [{ "appId": "App2", "platform": ["Windows"] }] },
            { "id": "cat3", "namespace": "ns2", "title": "Some DLC", "categories": [{ "path": "games" }], "mainGameItem": { "id": "cat2" }, "releaseInfo": [{ "appId": "App3" }] },
            { "id": "cat4", "namespace": "ns4", "title": "Mac only", "categories": [{ "path": "games" }], "releaseInfo": [{ "appId": "App4", "platform": ["Mac"] }] }
        ]);
        let pd = tree(&[
            ("Epic/EpicGamesLauncher/Data/Manifests/a.item", r#"{ "AppName": "App1", "DisplayName": "Installed Game", "CatalogNamespace": "ns1", "CatalogItemId": "cat1", "InstallLocation": "C:\\Epic\\Game", "LaunchExecutable": "Bin/Game.exe", "InstallSize": 10, "AppCategories": ["public", "games"] }"#),
            ("Epic/EpicGamesLauncher/Data/Manifests/b.item", r#"{ "AppName": "UE_5", "DisplayName": "Unreal Engine", "AppCategories": ["engines"] }"#),
            ("Epic/EpicGamesLauncher/Data/Manifests/c.item", "{broken"),
        ]);
        let encoded = base64::engine::general_purpose::STANDARD.encode(catalog.to_string());
        let la = tree(&[("EpicGamesLauncher/Saved/Data/Catalog/catcache.bin", &encoded)]);
        let f = Fakes::new();
        let ctx = f.ctx(Platform::Windows, &[("ProgramData", pd.path()), ("LOCALAPPDATA", la.path())]);
        let d = Epic.detect(&ctx);
        assert!(d.detected);
        let games = Epic.scan(&ctx, &d);
        let ids: Vec<_> = games.iter().map(|g| (g.id.as_str(), g.installed)).collect();
        assert_eq!(ids, vec![("App1", true), ("App2", false)]);
        assert_eq!(games[0].art.cover.as_deref(), Some("https://img/tall.jpg"));
        assert_eq!(games[0].executable, Some(PathBuf::from("C:\\Epic\\Game").join("Bin/Game.exe")));
        assert_eq!(
            games[1].actions.install,
            Some(LaunchAction::uri("com.epicgames.launcher://apps/ns2%3Acat2%3AApp2?action=install"))
        );
    }
}
