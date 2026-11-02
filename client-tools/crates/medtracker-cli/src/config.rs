use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
pub struct ProfileConfig {
    pub base_url: String,
}

pub fn load_profile(profile: &str) -> Result<Option<ProfileConfig>> {
    let path = profile_path(profile)?;
    match fs::read_to_string(path) {
        Ok(body) => serde_json::from_str(&body)
            .context("invalid profile config; log in again with auth login --token")
            .map(Some),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error)
            .context("failed to read profile config; log in again with auth login --token"),
    }
}

pub fn save_profile(profile: &str, base_url: &str) -> Result<PathBuf> {
    let path = profile_path(profile)?;
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).context("failed to create config directory")?;
    }

    let config = ProfileConfig {
        base_url: base_url.to_string(),
    };
    let body = serde_json::to_string_pretty(&config).context("failed to encode profile config")?;
    fs::write(&path, body).context("failed to write profile config")?;

    Ok(path)
}

pub fn profile_path(profile: &str) -> Result<PathBuf> {
    let dirs = ProjectDirs::from("com", "damacus", "MedTracker")
        .context("could not locate OS config directory")?;
    Ok(dirs.config_dir().join(format!("{profile}.json")))
}
