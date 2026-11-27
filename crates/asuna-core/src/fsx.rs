use std::path::{Path, PathBuf};

pub fn read_text(p: impl AsRef<Path>) -> Option<String> {
    std::fs::read_to_string(p).ok()
}

pub fn list_dir(p: impl AsRef<Path>) -> Vec<String> {
    std::fs::read_dir(p)
        .map(|rd| rd.flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect())
        .unwrap_or_default()
}

pub fn first_existing<I: IntoIterator<Item = Option<PathBuf>>>(paths: I) -> Option<PathBuf> {
    paths.into_iter().flatten().find(|p| p.exists())
}

/// Turns "C:/Games/X/" into "C:\Games\X" on Windows-style paths.
pub fn win_path(s: &str) -> PathBuf {
    PathBuf::from(s.replace('/', "\\").trim_end_matches('\\'))
}

/// Last segment of a path written with either separator.
pub fn any_basename(s: &str) -> &str {
    s.trim_end_matches(['/', '\\']).rsplit(['/', '\\']).next().unwrap_or(s)
}
