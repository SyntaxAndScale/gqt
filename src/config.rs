use serde::Deserialize;
use std::fs;
use anyhow::Result;

#[derive(Deserialize)]
pub struct GqueuesConfig {
    #[serde(rename = "apiEndpoint")]
    pub api_endpoint: String,
    #[serde(rename = "accessToken")]
    pub access_token: String,
}

#[derive(Deserialize)]
pub struct Settings {
    pub gqueues: GqueuesConfig,
}

pub fn load_config() -> Result<GqueuesConfig> {
    let content = fs::read_to_string(".gemini/settings.local.json")?;
    let settings: Settings = serde_json::from_str(&content)?;
    Ok(settings.gqueues)
}
