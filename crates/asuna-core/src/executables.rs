use crate::types::Platform;
use std::path::{Path, PathBuf};

const SKIP_NAME: &[&str] = &[
    "unins", "uninstall", "setup", "installer", "crash", "report", "redist", "dxsetup", "directx", "dotnet",
    "easyanticheat", "eac_", "battleye", "be_service", "prereq", "helper", "updater", "update", "cefprocess",
    "subprocess", "7z", "vcredist", "touchup", "cleanup",
];
const SKIP_DIR: &[&str] = &[
    "_commonredist", "redist", "redistributables", "directx", "dotnet", "vcredist", "__installer", "support",
    "_support", "installer", "installers", "easyanticheat", "battleye", "tools",
];

struct Candidate {
    path: PathBuf,
    depth: usize,
    size: u64,
    stem: String,
}

/// Most likely game executables in an install folder, best guess first.
pub fn find_executables(root: &Path, title: &str, platform: Platform) -> Vec<PathBuf> {
    let mut found = Vec::new();
    walk(root, 0, platform, &mut found);
    let words: Vec<String> = title
        .to_ascii_lowercase()
        .split(|c: char| !c.is_ascii_alphanumeric())
        .filter(|w| w.len() > 1)
        .map(str::to_string)
        .collect();
    let squashed = words.concat();
    let score = |c: &Candidate| -> i64 {
        let n: String = c.stem.to_ascii_lowercase().chars().filter(|c| c.is_ascii_alphanumeric()).collect();
        let mut s = 0i64;
        if !squashed.is_empty() && (n == squashed || squashed.starts_with(&n) && n.len() > 2 || n.starts_with(&squashed)) {
            s += 100;
        }
        s += words.iter().filter(|w| n.contains(w.as_str())).count() as i64 * 20;
        if n.contains("shipping") || n.contains("win64") || n.contains("x64") {
            s += 10;
        }
        if n.contains("launcher") {
            s -= 15;
        }
        s - c.depth as i64 * 5
    };
    found.sort_by(|a, b| score(b).cmp(&score(a)).then(b.size.cmp(&a.size)));
    found.into_iter().take(8).map(|c| c.path).collect()
}

fn walk(dir: &Path, depth: usize, platform: Platform, out: &mut Vec<Candidate>) {
    let Ok(rd) = std::fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        if out.len() > 200 {
            return;
        }
        let name = e.file_name().to_string_lossy().into_owned();
        let lower = name.to_ascii_lowercase();
        let Ok(ft) = e.file_type() else { continue };
        if ft.is_dir() {
            if depth < 3 && !SKIP_DIR.contains(&lower.as_str()) {
                walk(&e.path(), depth + 1, platform, out);
            }
            continue;
        }
        if !ft.is_file() || SKIP_NAME.iter().any(|s| lower.contains(s)) {
            continue;
        }
        let Ok(meta) = e.metadata() else { continue };
        let ok = match platform {
            Platform::Windows => lower.ends_with(".exe"),
            _ => is_executable(&meta) && !lower.ends_with(".so") && !lower.ends_with(".dylib") && !lower.ends_with(".dll"),
        };
        if ok {
            let stem = name.strip_suffix(".exe").or_else(|| name.strip_suffix(".EXE")).unwrap_or(&name).to_string();
            out.push(Candidate { path: e.path(), depth, size: meta.len(), stem });
        }
    }
}

#[cfg(unix)]
fn is_executable(m: &std::fs::Metadata) -> bool {
    use std::os::unix::fs::PermissionsExt;
    m.permissions().mode() & 0o111 != 0
}
#[cfg(not(unix))]
fn is_executable(_m: &std::fs::Metadata) -> bool {
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::testing::tree;

    #[test]
    fn ranks_the_game_exe_first() {
        let t = tree(&[
            ("unins000.exe", "x"),
            ("Launcher.exe", "xx"),
            ("Bin/Win64/HollowKnight.exe", "xxx"),
            ("_CommonRedist/vcredist_x64.exe", "xxxxxxxx"),
            ("UnityCrashHandler64.exe", "xxxx"),
            ("readme.txt", "x"),
        ]);
        let found = find_executables(t.path(), "Hollow Knight", Platform::Windows);
        assert_eq!(found[0], t.path().join("Bin/Win64/HollowKnight.exe"));
        assert_eq!(found.len(), 2);
    }
}
