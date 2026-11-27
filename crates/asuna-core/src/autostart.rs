//! "Open AsuNa when I sign in". Windows: Run registry key, macOS: LaunchAgent, Linux: XDG autostart.

use auto_launch::AutoLaunchBuilder;

pub const HIDDEN_ARG: &str = "--hidden";

pub fn set(enabled: bool) -> Result<(), String> {
    let exe = std::env::var("APPIMAGE")
        .ok()
        .map(std::path::PathBuf::from)
        .or_else(|| std::env::current_exe().ok())
        .ok_or("Could not find the AsuNa executable")?;
    let launcher = AutoLaunchBuilder::new()
        .set_app_name("AsuNa Launcher")
        .set_app_path(&exe.to_string_lossy())
        .set_args(&[HIDDEN_ARG])
        .set_use_launch_agent(true)
        .build()
        .map_err(|e| e.to_string())?;
    let result = if enabled { launcher.enable() } else { launcher.disable() };
    result.map_err(|e| e.to_string())
}
