use crate::registry::Registry;
use crate::types::Platform;
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Mutex;

pub type FetchJson = dyn Fn(&str) -> Result<Value, String> + Send + Sync;
pub type PowerShell = dyn Fn(&str) -> String + Send + Sync;

/// Everything a provider may touch, so tests can swap in fakes.
pub struct Ctx<'a> {
    pub platform: Platform,
    pub env: HashMap<String, String>,
    pub home: PathBuf,
    pub registry: &'a dyn Registry,
    pub powershell: &'a PowerShell,
    pub fetch_json: &'a FetchJson,
    /// Small persistent key/value cache (e.g. Steam app names).
    pub cache: &'a Mutex<HashMap<String, Value>>,
    /// User-provided launcher folder when auto-detection fails.
    pub path_override: Option<PathBuf>,
}

impl Ctx<'_> {
    pub fn env(&self, key: &str) -> Option<String> {
        // Windows env names are case-insensitive.
        self.env
            .get(key)
            .or_else(|| self.env.iter().find(|(k, _)| k.eq_ignore_ascii_case(key)).map(|(_, v)| v))
            .cloned()
    }

    pub fn env_or(&self, key: &str, fallback: &str) -> PathBuf {
        PathBuf::from(self.env(key).unwrap_or_else(|| fallback.to_string()))
    }

    pub fn cache_get(&self, key: &str) -> Option<Value> {
        self.cache.lock().ok()?.get(key).cloned()
    }

    pub fn cache_set(&self, key: &str, value: Value) {
        if let Ok(mut c) = self.cache.lock() {
            c.insert(key.to_string(), value);
        }
    }
}
