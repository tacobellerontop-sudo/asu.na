use crate::types::LaunchAction;
use std::process::{Command, Stdio};

/// Only launcher protocols (plus web pages) may be opened.
const ALLOWED_SCHEMES: &[&str] = &[
    "steam:",
    "com.epicgames.launcher:",
    "goggalaxy:",
    "battlenet:",
    "origin2:",
    "link2ea:",
    "uplay:",
    "xbox:",
    "ms-windows-store:",
    "ms-settings:",
    "https:",
];

pub fn is_allowed_uri(uri: &str) -> bool {
    let lower = uri.to_ascii_lowercase();
    ALLOWED_SCHEMES.iter().any(|s| lower.starts_with(s))
}

pub fn run(action: &LaunchAction) -> Result<(), String> {
    match action {
        LaunchAction::Uri { uri } => {
            if !is_allowed_uri(uri) {
                return Err(format!("Blocked link: {uri}"));
            }
            open::that_detached(uri).map_err(|e| format!("Could not open {uri}: {e}"))
        }
        LaunchAction::Exec { file, args, cwd, env } => {
            let mut cmd = Command::new(file);
            cmd.args(args).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
            if let Some(dir) = cwd {
                cmd.current_dir(dir);
            }
            for (k, v) in env {
                cmd.env(k, v);
            }
            #[cfg(windows)]
            {
                use std::os::windows::process::CommandExt;
                const DETACHED_PROCESS: u32 = 0x0000_0008;
                const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;
                cmd.creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP);
            }
            cmd.spawn().map(|_| ()).map_err(|e| format!("Could not start {}: {e}", file.display()))
        }
    }
}

pub fn open_folder(path: &std::path::Path) -> Result<(), String> {
    open::that_detached(path).map_err(|e| e.to_string())
}

pub fn open_url(url: &str) -> Result<(), String> {
    if !url.starts_with("https://") {
        return Err("Only https links can be opened".into());
    }
    open::that_detached(url).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    #[test]
    fn only_launcher_protocols() {
        assert!(super::is_allowed_uri("steam://rungameid/570"));
        assert!(super::is_allowed_uri("com.epicgames.launcher://apps/x?action=launch"));
        assert!(!super::is_allowed_uri("file:///C:/Windows/system32/cmd.exe"));
        assert!(!super::is_allowed_uri("javascript:alert(1)"));
    }
}
