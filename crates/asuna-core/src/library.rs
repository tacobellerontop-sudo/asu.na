//! Runs every provider and merges the results with the user's own state.

use crate::context::Ctx;
use crate::providers::{self, Detection, Provider};
use crate::registry::{Registry, SystemRegistry};
use crate::store::{write_json_atomic, CustomGame, StoreData};
use crate::types::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// Raw scan results; user state is merged on top by [`build`].
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ScanResult {
    pub games: Vec<DetectedGame>,
    pub launchers: Vec<LauncherStatus>,
    /// Unix ms.
    pub scanned_at: i64,
}

pub fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

pub fn powershell(script: &str) -> String {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        std::process::Command::new("powershell.exe")
            .args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", script])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).into_owned())
            .unwrap_or_default()
    }
    #[cfg(not(windows))]
    {
        let _ = script;
        String::new()
    }
}

/// Scans every enabled launcher in parallel. `cache` is updated in place (Steam names).
pub fn scan(data: &StoreData, cache: &Mutex<HashMap<String, Value>>) -> ScanResult {
    let platform = Platform::current();
    let registry = SystemRegistry;
    let online = data.settings.online_lookups;
    let fetch = move |url: &str| -> Result<Value, String> {
        if online {
            crate::net::get_json(url)
        } else {
            Err("online lookups are off".into())
        }
    };
    let env: HashMap<String, String> = std::env::vars().collect();
    let home = dirs::home_dir().unwrap_or_default();
    let list = providers::all();

    let results: Vec<(LauncherStatus, Vec<DetectedGame>)> = std::thread::scope(|s| {
        let handles: Vec<_> = list
            .iter()
            .map(|p| {
                let ctx = Ctx {
                    platform,
                    env: env.clone(),
                    home: home.clone(),
                    registry: &registry as &dyn Registry,
                    powershell: &powershell,
                    fetch_json: &fetch,
                    cache,
                    path_override: data.settings.path_overrides.get(&p.id()).cloned(),
                };
                let enabled = !data.disabled_launchers.contains(&p.id());
                s.spawn(move || scan_one(p.as_ref(), &ctx, platform, enabled))
            })
            .collect();
        handles.into_iter().map(|h| h.join().expect("provider panicked")).collect()
    });

    let mut games = Vec::new();
    let mut launchers = Vec::new();
    for (status, g) in results {
        launchers.push(status);
        games.extend(g);
    }
    ScanResult { games, launchers, scanned_at: now_ms() }
}

fn scan_one(p: &dyn Provider, ctx: &Ctx, platform: Platform, enabled: bool) -> (LauncherStatus, Vec<DetectedGame>) {
    let mut status = LauncherStatus {
        id: p.id(),
        name: p.id().label().to_string(),
        supported: p.platforms().contains(&platform),
        enabled,
        detected: false,
        path: None,
        installed_count: 0,
        owned_count: 0,
        download_url: Some(p.download_url().to_string()),
        error: None,
    };
    if !status.supported || !enabled {
        return (status, vec![]);
    }
    let d = p.detect(ctx);
    status.detected = d.detected;
    status.path = d.path.clone();
    if !d.detected {
        return (status, vec![]);
    }
    let games = p.scan(ctx, &d);
    status.installed_count = games.iter().filter(|g| g.installed).count();
    status.owned_count = games.iter().filter(|g| !g.catalog_only).count();
    (status, games)
}

pub fn open_launcher_action(id: LauncherId, status: Option<&LauncherStatus>) -> Option<LaunchAction> {
    let p = providers::all().into_iter().find(|p| p.id() == id)?;
    Some(p.open(&Detection { detected: status.is_some_and(|s| s.detected), path: status.and_then(|s| s.path.clone()) }))
}

pub fn custom_to_detected(c: &CustomGame) -> DetectedGame {
    let mut g = DetectedGame::new(LauncherId::Custom, &c.id, &c.title);
    g.installed = true;
    g.install_path = c.executable.parent().map(Path::to_path_buf);
    g.executable = Some(c.executable.clone());
    g.actions.launch = Some(LaunchAction::Exec {
        file: c.executable.clone(),
        args: split_args(c.args.as_deref()),
        cwd: g.install_path.clone(),
        env: vec![],
    });
    g
}

pub fn split_args(args: Option<&str>) -> Vec<String> {
    let Some(a) = args else { return vec![] };
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut quoted = false;
    for ch in a.chars() {
        match ch {
            '"' => quoted = !quoted,
            c if c.is_whitespace() && !quoted => {
                if !cur.is_empty() {
                    out.push(std::mem::take(&mut cur));
                }
            }
            c => cur.push(c),
        }
    }
    if !cur.is_empty() {
        out.push(cur);
    }
    out
}

/// Runs the game's own executable, skipping its launcher.
pub fn direct_launch(g: &DetectedGame, exe: &Path, args: Option<&str>) -> LaunchAction {
    LaunchAction::Exec {
        file: exe.to_path_buf(),
        args: split_args(args),
        cwd: exe.parent().map(Path::to_path_buf),
        // Lets DRM-free Steam games skip "restart through Steam" (same as steam_appid.txt).
        env: if g.launcher == LauncherId::Steam {
            vec![("SteamAppId".into(), g.id.clone()), ("SteamGameId".into(), g.id.clone())]
        } else {
            vec![]
        },
    }
}

/// Merges scan results with favorites, hidden games, launch overrides and custom games.
pub fn build(scan: &ScanResult, data: &StoreData) -> Vec<Game> {
    let mut games: Vec<Game> = scan
        .games
        .iter()
        .cloned()
        .chain(data.custom_games.iter().map(custom_to_detected))
        .map(|mut info| {
            let key = info.key();
            if let Some(ours) = data.last_played.get(&key) {
                info.last_played = Some(info.last_played.unwrap_or(0).max(*ours));
            }
            let o = (info.launcher != LauncherId::Custom).then(|| data.launch_overrides.get(&key)).flatten();
            let direct_executable: Option<PathBuf> = o.and_then(|o| o.executable.clone()).or_else(|| info.executable.clone());
            let launch_mode = match (o.map(|o| o.mode), &direct_executable) {
                (Some(LaunchMode::Direct), Some(_)) if info.installed => LaunchMode::Direct,
                _ => LaunchMode::Launcher,
            };
            let mut actions = info.actions.clone();
            if launch_mode == LaunchMode::Direct {
                actions.launch = Some(direct_launch(&info, direct_executable.as_ref().unwrap(), o.and_then(|o| o.args.as_deref())));
            }
            Game {
                favorite: data.favorites.contains(&key),
                hidden: data.hidden.contains(&key),
                direct_args: o.and_then(|o| o.args.clone()),
                key,
                info,
                launch_mode,
                direct_executable,
                actions,
            }
        })
        .collect();
    games.sort_by_key(|g| g.info.title.to_lowercase());
    games
}

pub fn save_scan_cache(path: &Path, scan: &ScanResult) {
    let _ = write_json_atomic(path, scan);
}

pub fn load_scan_cache(path: &Path) -> Option<ScanResult> {
    serde_json::from_str(&std::fs::read_to_string(path).ok()?).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merges_user_state_and_direct_launch() {
        let mut g = DetectedGame::new(LauncherId::Steam, "570", "Dota 2");
        g.installed = true;
        g.actions.launch = Some(LaunchAction::uri("steam://rungameid/570"));
        let scan = ScanResult { games: vec![g], launchers: vec![], scanned_at: 0 };
        let mut data = StoreData::default();
        data.favorites.insert("steam:570".into());
        data.last_played.insert("steam:570".into(), 5);
        data.custom_games.push(CustomGame { id: "x".into(), title: "Alpha".into(), executable: "/g/a.exe".into(), args: Some("-w \"a b\"".into()) });
        let games = build(&scan, &data);
        assert_eq!(games[0].info.title, "Alpha");
        assert_eq!(
            games[0].actions.launch,
            Some(LaunchAction::Exec { file: "/g/a.exe".into(), args: vec!["-w".into(), "a b".into()], cwd: Some("/g".into()), env: vec![] })
        );
        let dota = &games[1];
        assert!(dota.favorite);
        assert_eq!(dota.info.last_played, Some(5));
        assert_eq!(dota.launch_mode, LaunchMode::Launcher);

        // Direct mode without an executable stays on the launcher.
        data.launch_overrides.insert("steam:570".into(), LaunchOverride { mode: LaunchMode::Direct, executable: None, args: None });
        assert_eq!(build(&scan, &data)[1].launch_mode, LaunchMode::Launcher);

        data.launch_overrides.insert(
            "steam:570".into(),
            LaunchOverride { mode: LaunchMode::Direct, executable: Some("/s/dota2.exe".into()), args: Some("-novid".into()) },
        );
        let dota = build(&scan, &data).remove(1);
        assert_eq!(dota.launch_mode, LaunchMode::Direct);
        assert_eq!(
            dota.actions.launch,
            Some(LaunchAction::Exec {
                file: "/s/dota2.exe".into(),
                args: vec!["-novid".into()],
                cwd: Some("/s".into()),
                env: vec![("SteamAppId".into(), "570".into()), ("SteamGameId".into(), "570".into())]
            })
        );
    }
}
