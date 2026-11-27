use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LauncherId {
    Steam,
    Epic,
    Gog,
    Battlenet,
    Ea,
    Ubisoft,
    Xbox,
    Riot,
    Custom,
}

impl LauncherId {
    pub const ALL: [LauncherId; 9] = [
        Self::Steam,
        Self::Epic,
        Self::Gog,
        Self::Battlenet,
        Self::Ea,
        Self::Ubisoft,
        Self::Xbox,
        Self::Riot,
        Self::Custom,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Self::Steam => "Steam",
            Self::Epic => "Epic Games",
            Self::Gog => "GOG Galaxy",
            Self::Battlenet => "Battle.net",
            Self::Ea => "EA app",
            Self::Ubisoft => "Ubisoft Connect",
            Self::Xbox => "Xbox",
            Self::Riot => "Riot Client",
            Self::Custom => "Added by you",
        }
    }

    pub fn short(self) -> &'static str {
        match self {
            Self::Epic => "Epic",
            Self::Gog => "GOG",
            Self::Ea => "EA",
            Self::Ubisoft => "Ubisoft",
            Self::Riot => "Riot",
            Self::Custom => "Custom",
            other => other.label(),
        }
    }

    /// Brand-ish accent as 0xRRGGBB.
    pub fn color(self) -> u32 {
        match self {
            Self::Steam => 0x66c0f4,
            Self::Epic => 0xe6e6e6,
            Self::Gog => 0xb36bff,
            Self::Battlenet => 0x148eff,
            Self::Ea => 0xff4747,
            Self::Ubisoft => 0x3d8bff,
            Self::Xbox => 0x3fbf3f,
            Self::Riot => 0xeb0029,
            Self::Custom => 0xf5a524,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Steam => "steam",
            Self::Epic => "epic",
            Self::Gog => "gog",
            Self::Battlenet => "battlenet",
            Self::Ea => "ea",
            Self::Ubisoft => "ubisoft",
            Self::Xbox => "xbox",
            Self::Riot => "riot",
            Self::Custom => "custom",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Platform {
    Windows,
    Mac,
    Linux,
}

impl Platform {
    pub fn current() -> Self {
        if cfg!(windows) {
            Self::Windows
        } else if cfg!(target_os = "macos") {
            Self::Mac
        } else {
            Self::Linux
        }
    }
}

/// Something AsuNa can do on the user's behalf.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum LaunchAction {
    Uri {
        uri: String,
    },
    Exec {
        file: PathBuf,
        #[serde(default)]
        args: Vec<String>,
        #[serde(default)]
        cwd: Option<PathBuf>,
        #[serde(default)]
        env: Vec<(String, String)>,
    },
}

impl LaunchAction {
    pub fn uri(s: impl Into<String>) -> Self {
        Self::Uri { uri: s.into() }
    }
    pub fn exec(file: impl Into<PathBuf>, args: &[&str]) -> Self {
        Self::Exec {
            file: file.into(),
            args: args.iter().map(|s| s.to_string()).collect(),
            cwd: None,
            env: vec![],
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Actions {
    pub launch: Option<LaunchAction>,
    pub install: Option<LaunchAction>,
    pub uninstall: Option<LaunchAction>,
    pub store: Option<LaunchAction>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameAction {
    Launch,
    Install,
    Uninstall,
    Store,
    OpenFolder,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Art {
    /// Portrait box art (roughly 2:3).
    pub cover: Option<String>,
    /// Landscape header.
    pub header: Option<String>,
}

/// A game as a provider reports it, before user state is merged in.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct DetectedGame {
    pub launcher: LauncherId,
    /// Launcher-specific id (Steam appid, Epic AppName, ...).
    pub id: String,
    pub title: String,
    pub installed: bool,
    pub install_path: Option<PathBuf>,
    pub size_bytes: Option<u64>,
    /// Unix milliseconds.
    pub last_played: Option<i64>,
    pub playtime_minutes: Option<u64>,
    #[serde(default)]
    pub art: Art,
    /// Listed from a catalog without proof of ownership (Battle.net, Riot titles).
    #[serde(default)]
    pub catalog_only: bool,
    /// The game's own executable when the launcher records it.
    pub executable: Option<PathBuf>,
    pub actions: Actions,
}

impl DetectedGame {
    pub fn new(launcher: LauncherId, id: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            launcher,
            id: id.into(),
            title: title.into(),
            installed: false,
            install_path: None,
            size_bytes: None,
            last_played: None,
            playtime_minutes: None,
            art: Art::default(),
            catalog_only: false,
            executable: None,
            actions: Actions::default(),
        }
    }

    pub fn key(&self) -> String {
        format!("{}:{}", self.launcher.as_str(), self.id)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LaunchMode {
    #[default]
    Launcher,
    Direct,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct LaunchOverride {
    pub mode: LaunchMode,
    pub executable: Option<PathBuf>,
    pub args: Option<String>,
}

/// A game in the library: detection plus the user's own state.
#[derive(Clone, Debug, PartialEq)]
pub struct Game {
    pub key: String,
    pub info: DetectedGame,
    pub favorite: bool,
    pub hidden: bool,
    pub launch_mode: LaunchMode,
    pub direct_executable: Option<PathBuf>,
    pub direct_args: Option<String>,
    /// Actions after the launch override is applied.
    pub actions: Actions,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct LauncherStatus {
    pub id: LauncherId,
    pub name: String,
    pub supported: bool,
    pub enabled: bool,
    pub detected: bool,
    pub path: Option<PathBuf>,
    pub installed_count: usize,
    pub owned_count: usize,
    pub download_url: Option<String>,
    pub error: Option<String>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum OnGameLaunch {
    Nothing,
    #[default]
    Minimize,
    Quit,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub open_at_login: bool,
    pub start_minimized: bool,
    pub on_game_launch: OnGameLaunch,
    /// Rescan launchers on start (the last scan shows instantly either way).
    pub scan_on_startup: bool,
    /// Steam store lookups for owned-game names, store search and box art downloads.
    pub online_lookups: bool,
    pub show_art: bool,
    pub path_overrides: std::collections::BTreeMap<LauncherId, PathBuf>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            open_at_login: false,
            start_minimized: false,
            on_game_launch: OnGameLaunch::Minimize,
            scan_on_startup: true,
            online_lookups: true,
            show_art: true,
            path_overrides: Default::default(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct StoreResult {
    pub launcher: LauncherId,
    pub id: String,
    pub title: String,
    pub image: Option<String>,
    pub price: Option<String>,
}
