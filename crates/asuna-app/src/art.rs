//! Box art is downloaded once and kept on disk, so the library works offline and GPUI
//! only ever decodes local files.

use asuna_core::providers::steam;
use std::collections::hash_map::DefaultHasher;
use std::collections::{BTreeMap, HashMap};
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

#[derive(Clone, Debug, PartialEq)]
pub enum ArtState {
    Loading,
    Ready(PathBuf),
    Missing,
}

/// Marks a URL that has no image. Versioned so misses from older builds get retried.
pub const MISS_EXT: &str = "miss3";

pub fn cache_path(dir: &Path, url: &str) -> PathBuf {
    let mut h = DefaultHasher::new();
    url.hash(&mut h);
    let ext = if url.to_ascii_lowercase().contains(".png") {
        "png"
    } else {
        "jpg"
    };
    dir.join(format!("{:016x}.{ext}", h.finish()))
}

pub fn miss_path(dir: &Path, url: &str) -> PathBuf {
    cache_path(dir, url).with_extension(MISS_EXT)
}

enum Got {
    Image(Vec<u8>),
    NotFound,
    Failed,
}

fn download(url: &str) -> Got {
    match asuna_core::net::get_bytes(url) {
        Ok(bytes) if bytes.len() > 512 => Got::Image(bytes),
        Ok(_) => Got::NotFound,
        Err(e) if e.contains("404") || e.contains("403") => Got::NotFound,
        Err(_) => Got::Failed,
    }
}

/// Hashed asset URLs per Steam app id, looked up once per run.
static STEAM_ASSETS: Mutex<Option<HashMap<String, BTreeMap<String, String>>>> = Mutex::new(None);

fn steam_assets(appid: &str) -> BTreeMap<String, String> {
    if let Some(found) = STEAM_ASSETS
        .lock()
        .unwrap()
        .as_ref()
        .and_then(|m| m.get(appid))
    {
        return found.clone();
    }
    let assets = asuna_core::net::get_json(&steam::store_assets_url(appid))
        .map(|v| steam::parse_store_assets(&v))
        .unwrap_or_default();
    STEAM_ASSETS
        .lock()
        .unwrap()
        .get_or_insert_with(HashMap::new)
        .insert(appid.to_string(), assets.clone());
    assets
}

/// Downloads `url` into the cache. Returns the file path, or None when the image does not exist.
/// Steam URLs that 404 are retried through the store API, which knows the hashed paths newer games use.
pub fn fetch(dir: &Path, url: &str) -> Option<PathBuf> {
    let path = cache_path(dir, url);
    if path.exists() {
        return Some(path);
    }
    let miss = miss_path(dir, url);
    if miss.exists() {
        return None;
    }
    let mut got = download(url);
    if matches!(got, Got::NotFound) {
        if let Some((appid, key)) = steam::asset_key_for(url) {
            let assets = steam_assets(appid);
            let candidates: &[&str] = if key == "library_capsule" {
                &["library_capsule_2x", "library_capsule"]
            } else {
                &[key]
            };
            if let Some(alt) = candidates.iter().find_map(|k| assets.get(*k)) {
                got = download(alt);
            }
        }
    }
    match got {
        Got::Image(bytes) => {
            std::fs::create_dir_all(dir).ok()?;
            std::fs::write(&path, bytes).ok()?;
            Some(path)
        }
        Got::NotFound => {
            // Remember so we don't ask again.
            let _ = std::fs::create_dir_all(dir);
            let _ = std::fs::write(&miss, b"");
            None
        }
        Got::Failed => None,
    }
}
