use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use anyhow::{anyhow, Result};
use directories::ProjectDirs;

#[derive(Deserialize, Serialize)]
pub struct GqueuesConfig {
    #[serde(rename = "apiEndpoint")]
    pub api_endpoint: String,
    #[serde(rename = "accessToken")]
    pub access_token: String,
}

#[derive(Deserialize, Serialize)]
pub struct Settings {
    pub gqueues: GqueuesConfig,
}

pub fn load_config() -> Result<GqueuesConfig> {
    let toml_path = get_config_path("config.toml")?;
    
    if toml_path.exists() {
        let content = fs::read_to_string(&toml_path)?;
        let settings: Settings = toml::from_str(&content)?;
        return Ok(settings.gqueues);
    }

    // Migration / Fallback logic
    let json_path = get_config_path("config.json")?;
    let legacy_local_path = std::path::Path::new(".gemini/settings.local.json");

    let (legacy_path, content) = if json_path.exists() {
        (Some(json_path), fs::read_to_string(&get_config_path("config.json")?)?)
    } else if legacy_local_path.exists() {
        (Some(legacy_local_path.to_path_buf()), fs::read_to_string(legacy_local_path)?)
    } else {
        return Err(anyhow!("Configuration file not found. Please create one at {:?}", toml_path));
    };

    // Parse legacy JSON
    let settings: Settings = serde_json::from_str(&content)?;
    
    // Auto-migrate to TOML
    let toml_content = toml::to_string_pretty(&settings)?;
    fs::write(&toml_path, toml_content)?;
    log::info!("Migrated configuration from JSON to TOML at {:?}", toml_path);

    // Optionally delete old JSON if it was in the XDG path
    if let Some(path) = legacy_path {
        if path.extension().and_then(|s| s.to_str()) == Some("json") && path.parent() == toml_path.parent() {
            let _ = fs::remove_file(path);
        }
    }

    Ok(settings.gqueues)
}

fn get_config_path(filename: &str) -> Result<PathBuf> {
    let proj_dirs = ProjectDirs::from("com", "gqt", "gqt")
        .ok_or_else(|| anyhow!("Could not determine project directories"))?;
    let config_dir = proj_dirs.config_dir();
    if !config_dir.exists() {
        fs::create_dir_all(config_dir)?;
    }
    Ok(config_dir.join(filename))
}
