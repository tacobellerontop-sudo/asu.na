use super::{Detection, Provider, ALL_DESKTOP};
use crate::context::Ctx;
use crate::fsx::{first_existing, list_dir, read_text};
use crate::types::*;
use crate::vdf::{self, Vdf};
use serde_json::json;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

const CDN: &str = "https://cdn.cloudflare.steamstatic.com/steam/apps";
/// Runtimes and redistributables that show up as apps but are not games.
const NON_GAME_IDS: &[&str] = &["228980", "1070560", "1391110", "1628350", "1493710", "2180100", "1826330"];
/// Name lookups per scan for owned-but-not-installed games. Results are cached for good.
const MAX_NAME_LOOKUPS: usize = 80;

pub struct Steam;

pub fn art(appid: &str) -> Art {
    Art {
        cover: Some(format!("{CDN}/{appid}/library_600x900.jpg")),
        header: Some(format!("{CDN}/{appid}/header.jpg")),
    }
}

pub fn actions(appid: &str) -> Actions {
    Actions {
        launch: Some(LaunchAction::uri(format!("steam://rungameid/{appid}"))),
        install: Some(LaunchAction::uri(format!("steam://install/{appid}"))),
        uninstall: Some(LaunchAction::uri(format!("steam://uninstall/{appid}"))),
        store: Some(LaunchAction::uri(format!("steam://store/{appid}"))),
    }
}

fn is_non_game(id: &str, name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    NON_GAME_IDS.contains(&id)
        || ["proton", "steam linux runtime", "steamworks", "steamvr"].iter().any(|p| n.starts_with(p))
}

/// Every library folder listed in libraryfolders.vdf, plus the main install.
pub fn parse_library_folders(text: &str, root: &Path) -> Vec<PathBuf> {
    let mut out = vec![root.to_path_buf()];
    if let Some(lf) = vdf::parse(text).get("libraryfolders") {
        for (_, entry) in lf.entries() {
            // New format: "0" { "path" "D:\\SteamLibrary" }   Old format: "1" "D:\\SteamLibrary"
            let p = entry.str().or_else(|| entry.get_str("path"));
            if let Some(p) = p.filter(|p| p.contains(['/', '\\'])) {
                let pb = PathBuf::from(p);
                if !out.contains(&pb) {
                    out.push(pb);
                }
            }
        }
    }
    out
}

#[derive(Debug, PartialEq)]
pub struct AppManifest {
    pub appid: String,
    pub name: String,
    pub installdir: Option<String>,
    pub size_on_disk: Option<u64>,
    pub last_played: Option<i64>,
    pub fully_installed: bool,
}

pub fn parse_app_manifest(text: &str) -> Option<AppManifest> {
    let v = vdf::parse(text);
    let st = v.get("AppState")?;
    let appid = st.get_str("appid")?.to_string();
    let name = st.get_str("name").filter(|n| !n.is_empty())?.to_string();
    let flags: u64 = st.get_str("StateFlags").and_then(|s| s.parse().ok()).unwrap_or(4);
    Some(AppManifest {
        appid,
        name,
        installdir: st.get_str("installdir").map(str::to_string),
        size_on_disk: st.get_str("SizeOnDisk").and_then(|s| s.parse().ok()).filter(|n| *n > 0),
        last_played: st.get_str("LastPlayed").and_then(|s| s.parse::<i64>().ok()).filter(|n| *n > 0).map(|s| s * 1000),
        // StateFlags bit 4 = fully installed; other states are updating or partial downloads.
        fully_installed: flags & 4 == 4,
    })
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PlayStats {
    pub last_played: Option<i64>,
    pub playtime: Option<u64>,
}

/// Per-app play stats from userdata/<id>/config/localconfig.vdf (covers uninstalled games too).
pub fn parse_local_config(text: &str) -> BTreeMap<String, PlayStats> {
    let v = vdf::parse(text);
    let mut out = BTreeMap::new();
    if let Some(apps) = v.path(&["UserLocalConfigStore", "Software", "Valve", "Steam", "apps"]) {
        for (id, e) in apps.entries() {
            if !id.chars().all(|c| c.is_ascii_digit()) || matches!(e, Vdf::Str(_)) {
                continue;
            }
            out.insert(
                id.clone(),
                PlayStats {
                    last_played: e.get_str("LastPlayed").and_then(|s| s.parse::<i64>().ok()).filter(|n| *n > 0).map(|s| s * 1000),
                    playtime: e.get_str("Playtime").and_then(|s| s.parse().ok()).filter(|n| *n > 0),
                },
            );
        }
    }
    out
}

fn find_root(ctx: &Ctx) -> Option<PathBuf> {
    if let Some(p) = &ctx.path_override {
        return Some(p.clone());
    }
    let h = &ctx.home;
    match ctx.platform {
        Platform::Windows => {
            let reg = ctx.registry.query(r"HKCU\Software\Valve\Steam");
            let from_reg = reg.first().and_then(|k| k.get("SteamPath")).map(|s| PathBuf::from(s.replace('/', "\\")));
            first_existing([
                from_reg,
                Some(ctx.env_or("ProgramFiles(x86)", r"C:\Program Files (x86)").join("Steam")),
                Some(ctx.env_or("ProgramFiles", r"C:\Program Files").join("Steam")),
            ])
        }
        Platform::Mac => first_existing([Some(h.join("Library/Application Support/Steam"))]),
        Platform::Linux => first_existing([
            Some(h.join(".steam/steam")),
            Some(h.join(".local/share/Steam")),
            Some(h.join(".var/app/com.valvesoftware.Steam/.local/share/Steam")),
        ]),
    }
}

/// (name, is_game) for an appid, from cache or the Steam store API.
fn resolve_name(ctx: &Ctx, appid: &str, allow_network: bool) -> Option<(String, bool)> {
    let key = format!("steam.appname.{appid}");
    if let Some(v) = ctx.cache_get(&key) {
        return Some((v["name"].as_str()?.to_string(), v["isGame"].as_bool()?));
    }
    if !allow_network {
        return None;
    }
    let data = (ctx.fetch_json)(&format!("https://store.steampowered.com/api/appdetails?appids={appid}&filters=basic")).ok()?;
    let entry = &data[appid];
    let (name, is_game) = if entry["success"].as_bool() == Some(true) {
        (entry["data"]["name"].as_str().unwrap_or("").to_string(), entry["data"]["type"].as_str() == Some("game"))
    } else {
        // Delisted or region locked: remember so we stop asking.
        (String::new(), false)
    };
    ctx.cache_set(&key, json!({ "name": name, "isGame": is_game }));
    Some((name, is_game))
}

impl Provider for Steam {
    fn id(&self) -> LauncherId {
        LauncherId::Steam
    }
    fn platforms(&self) -> &'static [Platform] {
        ALL_DESKTOP
    }
    fn download_url(&self) -> &'static str {
        "https://store.steampowered.com/about/"
    }

    fn detect(&self, ctx: &Ctx) -> Detection {
        let path = find_root(ctx);
        Detection { detected: path.as_ref().is_some_and(|p| p.join("steamapps").exists()), path }
    }

    fn open(&self, _d: &Detection) -> LaunchAction {
        LaunchAction::uri("steam://open/games")
    }

    fn scan(&self, ctx: &Ctx, d: &Detection) -> Vec<DetectedGame> {
        let Some(root) = &d.path else { return vec![] };
        let libraries = read_text(root.join("steamapps").join("libraryfolders.vdf"))
            .map(|t| parse_library_folders(&t, root))
            .unwrap_or_else(|| vec![root.clone()]);

        // Play stats for every Steam user that has signed in on this machine.
        let mut stats: BTreeMap<String, PlayStats> = BTreeMap::new();
        for user in list_dir(root.join("userdata")) {
            let Some(text) = read_text(root.join("userdata").join(&user).join("config").join("localconfig.vdf")) else {
                continue;
            };
            for (id, s) in parse_local_config(&text) {
                let e = stats.entry(id).or_default();
                e.last_played = e.last_played.max(s.last_played);
                e.playtime = e.playtime.max(s.playtime);
            }
        }

        let mut games: BTreeMap<String, DetectedGame> = BTreeMap::new();
        for lib in libraries {
            let apps_dir = lib.join("steamapps");
            for file in list_dir(&apps_dir) {
                if !(file.starts_with("appmanifest_") && file.ends_with(".acf")) {
                    continue;
                }
                let Some(m) = read_text(apps_dir.join(&file)).and_then(|t| parse_app_manifest(&t)) else { continue };
                if is_non_game(&m.appid, &m.name) {
                    continue;
                }
                let s = stats.get(&m.appid).copied().unwrap_or_default();
                let mut g = DetectedGame::new(LauncherId::Steam, &m.appid, &m.name);
                g.installed = m.fully_installed;
                g.install_path = m.installdir.as_ref().map(|d| apps_dir.join("common").join(d));
                g.size_bytes = m.size_on_disk;
                g.last_played = s.last_played.or(m.last_played);
                g.playtime_minutes = s.playtime;
                g.art = art(&m.appid);
                g.actions = actions(&m.appid);
                games.insert(m.appid.clone(), g);
            }
        }

        // Games this account has played before but which are not installed here.
        let mut lookups = 0;
        for (id, s) in &stats {
            if games.contains_key(id) || NON_GAME_IDS.contains(&id.as_str()) {
                continue;
            }
            let cached = ctx.cache_get(&format!("steam.appname.{id}")).is_some();
            if !cached {
                if lookups >= MAX_NAME_LOOKUPS {
                    continue;
                }
                lookups += 1;
            }
            let Some((name, is_game)) = resolve_name(ctx, id, true) else { continue };
            if !is_game || name.is_empty() {
                continue;
            }
            let mut g = DetectedGame::new(LauncherId::Steam, id, name);
            g.last_played = s.last_played;
            g.playtime_minutes = s.playtime;
            g.art = art(id);
            g.actions = actions(id);
            games.insert(id.clone(), g);
        }
        games.into_values().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::testing::{tree, Fakes};
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    #[test]
    fn library_folders_new_and_old() {
        let modern = r#""libraryfolders" { "0" { "path" "C:\\Steam" "apps" { "10" "1" } } "1" { "path" "D:\\SteamLibrary" } }"#;
        assert_eq!(
            parse_library_folders(modern, Path::new(r"C:\Steam")),
            vec![PathBuf::from(r"C:\Steam"), PathBuf::from(r"D:\SteamLibrary")]
        );
        let legacy = r#""LibraryFolders" { "TimeNextStatsReport" "123" "ContentStatsID" "-1" "1" "E:\\Games" }"#;
        assert_eq!(parse_library_folders(legacy, Path::new("/root")), vec![PathBuf::from("/root"), PathBuf::from(r"E:\Games")]);
    }

    #[test]
    fn app_manifest() {
        let m = parse_app_manifest(r#""AppState" { "appid" "570" "name" "Dota 2" "StateFlags" "4" "installdir" "dota 2 beta" "SizeOnDisk" "1000" "LastPlayed" "1700000000" }"#).unwrap();
        assert_eq!(m.appid, "570");
        assert_eq!(m.size_on_disk, Some(1000));
        assert_eq!(m.last_played, Some(1_700_000_000_000));
        assert!(m.fully_installed);
        assert!(!parse_app_manifest(r#""AppState" { "appid" "1" "name" "x" "StateFlags" "1026" }"#).unwrap().fully_installed);
        assert!(parse_app_manifest("garbage").is_none());
    }

    #[test]
    fn scans_libraries_skips_runtimes_and_adds_owned_games() {
        let lib2 = tree(&[("steamapps/appmanifest_620.acf", r#""AppState" { "appid" "620" "name" "Portal 2" "StateFlags" "6" "installdir" "Portal 2" }"#)]);
        let lib2_path = lib2.path().to_string_lossy().replace('\\', "\\\\");
        let root = tree(&[
            ("steamapps/appmanifest_570.acf", r#""AppState" { "appid" "570" "name" "Dota 2" "StateFlags" "4" "installdir" "dota 2 beta" "SizeOnDisk" "42" }"#),
            ("steamapps/appmanifest_228980.acf", r#""AppState" { "appid" "228980" "name" "Steamworks Common Redistributables" "StateFlags" "4" }"#),
            ("steamapps/appmanifest_1493710.acf", r#""AppState" { "appid" "1493710" "name" "Proton Experimental" "StateFlags" "4" }"#),
            (
                "userdata/123/config/localconfig.vdf",
                r#""UserLocalConfigStore" { "Software" { "Valve" { "Steam" { "apps" {
                    "570" { "LastPlayed" "1700000000" "Playtime" "600" }
                    "999" { "Playtime" "1" }
                    "998" { "Playtime" "5" }
                } } } } }"#,
            ),
        ]);
        std::fs::write(
            root.path().join("steamapps/libraryfolders.vdf"),
            format!(r#""libraryfolders" {{ "1" {{ "path" "{lib2_path}" }} }}"#),
        )
        .unwrap();

        let calls = Arc::new(AtomicUsize::new(0));
        let c2 = calls.clone();
        let mut f = Fakes::new();
        f.fetch = Box::new(move |url| {
            c2.fetch_add(1, Ordering::SeqCst);
            let id = url.split("appids=").nth(1).unwrap().split('&').next().unwrap().to_string();
            let (name, ty) = if id == "999" { ("Hidden Gem", "game") } else { ("Soundtrack", "music") };
            Ok(serde_json::json!({ id: { "success": true, "data": { "name": name, "type": ty } } }))
        });
        let mut ctx = f.ctx(Platform::Linux, &[]);
        ctx.path_override = Some(root.path().to_path_buf());
        let d = Steam.detect(&ctx);
        assert!(d.detected);
        let games = Steam.scan(&ctx, &d);
        let ids: Vec<_> = games.iter().map(|g| (g.id.as_str(), g.installed)).collect();
        assert_eq!(ids, vec![("570", true), ("620", true), ("999", false)]);
        let dota = &games[0];
        assert_eq!(dota.playtime_minutes, Some(600));
        assert_eq!(dota.size_bytes, Some(42));
        assert_eq!(dota.install_path, Some(root.path().join("steamapps/common/dota 2 beta")));
        assert_eq!(dota.actions.launch, Some(LaunchAction::uri("steam://rungameid/570")));
        assert_eq!(calls.load(Ordering::SeqCst), 2);
        // Cached: no more network on the second scan.
        Steam.scan(&ctx, &d);
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn not_detected_without_steam() {
        let f = Fakes::new();
        assert!(!Steam.detect(&f.ctx(Platform::Linux, &[])).detected);
    }
}
