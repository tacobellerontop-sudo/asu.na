use serde_json::Value;
use std::time::Duration;

fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(10))
        .user_agent(concat!("AsuNa-Launcher/", env!("CARGO_PKG_VERSION")))
        .build()
}

pub fn get_json(url: &str) -> Result<Value, String> {
    let body = agent().get(url).call().map_err(|e| e.to_string())?.into_string().map_err(|e| e.to_string())?;
    serde_json::from_str(&body).map_err(|e| e.to_string())
}

pub fn get_bytes(url: &str) -> Result<Vec<u8>, String> {
    let resp = agent().get(url).call().map_err(|e| e.to_string())?;
    let mut buf = Vec::new();
    std::io::Read::read_to_end(&mut resp.into_reader().take(20 * 1024 * 1024), &mut buf).map_err(|e| e.to_string())?;
    Ok(buf)
}

use std::io::Read as _;
