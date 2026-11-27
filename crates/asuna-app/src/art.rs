//! Box art is downloaded once and kept on disk, so the library works offline and GPUI
//! only ever decodes local files.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq)]
pub enum ArtState {
    Loading,
    Ready(PathBuf),
    Missing,
}

pub fn cache_path(dir: &Path, url: &str) -> PathBuf {
    let mut h = DefaultHasher::new();
    url.hash(&mut h);
    let ext = if url.to_ascii_lowercase().contains(".png") { "png" } else { "jpg" };
    dir.join(format!("{:016x}.{ext}", h.finish()))
}

/// Downloads `url` into the cache. Returns the file path, or None when the image does not exist.
pub fn fetch(dir: &Path, url: &str) -> Option<PathBuf> {
    let path = cache_path(dir, url);
    if path.exists() {
        return Some(path);
    }
    let miss = path.with_extension("miss");
    if miss.exists() {
        return None;
    }
    match asuna_core::net::get_bytes(url) {
        Ok(bytes) if bytes.len() > 512 => {
            std::fs::create_dir_all(dir).ok()?;
            std::fs::write(&path, bytes).ok()?;
            Some(path)
        }
        Ok(_) => {
            // Tiny body = placeholder / not found. Remember so we don't ask again.
            let _ = std::fs::write(&miss, b"");
            None
        }
        Err(e) => {
            if e.contains("404") || e.contains("403") {
                let _ = std::fs::create_dir_all(dir);
                let _ = std::fs::write(&miss, b"");
            }
            None
        }
    }
}
