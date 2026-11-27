use crate::context::Ctx;
use crate::types::{DetectedGame, LaunchAction, LauncherId, Platform};
use std::path::PathBuf;

pub mod battlenet;
pub mod ea;
pub mod epic;
pub mod gog;
pub mod riot;
pub mod steam;
pub mod ubisoft;
pub mod xbox;

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Detection {
    pub detected: bool,
    /// Launcher install folder or client executable.
    pub path: Option<PathBuf>,
}

impl Detection {
    pub fn found(path: Option<PathBuf>) -> Self {
        Self { detected: path.is_some(), path }
    }
}

pub trait Provider: Send + Sync {
    fn id(&self) -> LauncherId;
    fn platforms(&self) -> &'static [Platform];
    fn download_url(&self) -> &'static str;
    fn detect(&self, ctx: &Ctx) -> Detection;
    fn scan(&self, ctx: &Ctx, detection: &Detection) -> Vec<DetectedGame>;
    /// How to bring the launcher itself to the front.
    fn open(&self, detection: &Detection) -> LaunchAction;
}

pub fn all() -> Vec<Box<dyn Provider>> {
    vec![
        Box::new(steam::Steam),
        Box::new(epic::Epic),
        Box::new(gog::Gog),
        Box::new(battlenet::BattleNet),
        Box::new(ea::Ea),
        Box::new(ubisoft::Ubisoft),
        Box::new(xbox::Xbox),
        Box::new(riot::Riot),
    ]
}

pub const ALL_DESKTOP: &[Platform] = &[Platform::Windows, Platform::Mac, Platform::Linux];
pub const WINDOWS: &[Platform] = &[Platform::Windows];
pub const WINDOWS_MAC: &[Platform] = &[Platform::Windows, Platform::Mac];

#[cfg(test)]
pub(crate) mod testing {
    use crate::context::Ctx;
    use crate::registry::{FakeRegistry, RegKey};
    use crate::types::Platform;
    use serde_json::Value;
    use std::collections::HashMap;
    use std::path::{Path, PathBuf};
    use std::sync::Mutex;

    pub struct Fakes {
        pub registry: FakeRegistry,
        pub cache: Mutex<HashMap<String, Value>>,
        pub ps: Box<dyn Fn(&str) -> String + Send + Sync>,
        pub fetch: Box<dyn Fn(&str) -> Result<Value, String> + Send + Sync>,
    }

    impl Fakes {
        pub fn new() -> Self {
            Self {
                registry: FakeRegistry::default(),
                cache: Mutex::new(HashMap::new()),
                ps: Box::new(|_| String::new()),
                fetch: Box::new(|_| Err("offline".into())),
            }
        }
        pub fn reg(mut self, keys: Vec<RegKey>) -> Self {
            self.registry = FakeRegistry(keys);
            self
        }
        pub fn ctx(&self, platform: Platform, env: &[(&str, &Path)]) -> Ctx<'_> {
            Ctx {
                platform,
                env: env.iter().map(|(k, v)| (k.to_string(), v.to_string_lossy().into_owned())).collect(),
                home: PathBuf::from("/nonexistent-home"),
                registry: &self.registry,
                powershell: &*self.ps,
                fetch_json: &*self.fetch,
                cache: &self.cache,
                path_override: None,
            }
        }
    }

    /// Writes files into a fresh temp dir.
    pub fn tree(files: &[(&str, &str)]) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        for (rel, content) in files {
            let p = dir.path().join(rel);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, content).unwrap();
        }
        dir
    }
}
