//! Settings and user state, kept in one JSON file in the data folder.

use crate::types::{LaunchOverride, LauncherId, Settings};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet, HashMap};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CustomGame {
    pub id: String,
    pub title: String,
    pub executable: PathBuf,
    pub args: Option<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct StoreData {
    pub favorites: BTreeSet<String>,
    pub hidden: BTreeSet<String>,
    /// Last launch through AsuNa (unix ms), by game key.
    pub last_played: BTreeMap<String, i64>,
    pub disabled_launchers: BTreeSet<LauncherId>,
    pub custom_games: Vec<CustomGame>,
    pub launch_overrides: BTreeMap<String, LaunchOverride>,
    pub settings: Settings,
    pub cache: HashMap<String, Value>,
}

impl StoreData {
    pub fn load(path: &Path) -> Self {
        std::fs::read_to_string(path).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
    }

    /// Atomic save: write a temp file, then rename over the old one.
    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        write_json_atomic(path, self)
    }
}

pub fn write_json_atomic<T: Serialize>(path: &Path, value: &T) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("tmp");
    std::fs::write(&tmp, serde_json::to_vec(value)?)?;
    std::fs::rename(tmp, path)
}

/// Where AsuNa keeps its files (e.g. %APPDATA%\AsuNa on Windows).
pub fn data_dir() -> PathBuf {
    dirs::data_dir().unwrap_or_else(std::env::temp_dir).join("AsuNa")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_and_tolerates_missing_fields() {
        let dir = tempfile::tempdir().unwrap();
        let p = dir.path().join("s.json");
        let mut d = StoreData::default();
        d.favorites.insert("steam:570".into());
        d.settings.open_at_login = true;
        d.settings.path_overrides.insert(LauncherId::Steam, PathBuf::from("/x"));
        d.save(&p).unwrap();
        assert_eq!(StoreData::load(&p), d);
        std::fs::write(&p, r#"{"favorites":["a"],"settings":{"show_art":false}}"#).unwrap();
        let l = StoreData::load(&p);
        assert!(l.favorites.contains("a"));
        assert!(!l.settings.show_art);
        assert!(l.settings.scan_on_startup);
    }
}
