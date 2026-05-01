use serde::Deserialize;
use std::fs;
use std::path::PathBuf;
use anyhow::{anyhow, Result};
use directories::ProjectDirs;

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
    let path = get_config_path()?;
    
    // Fallback to project-local for development if XDG doesn't exist
    let content = if path.exists() {
        fs::read_to_string(path)?
    } else if std::path::Path::new(".gemini/settings.local.json").exists() {
        fs::read_to_string(".gemini/settings.local.json")?
    } else {
        return Err(anyhow!("Configuration file not found. Please create one at {:?}", path));
    };

    let settings: Settings = serde_json::from_str(&content)?;
    Ok(settings.gqueues)
}

fn get_config_path() -> Result<PathBuf> {
    let proj_dirs = ProjectDirs::from("com", "gqt", "gqt")
        .ok_or_else(|| anyhow!("Could not determine project directories"))?;
    let config_dir = proj_dirs.config_dir();
    if !config_dir.exists() {
        fs::create_dir_all(config_dir)?;
    }
    Ok(config_dir.join("config.json"))
}
