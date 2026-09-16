use std::path::PathBuf;

use anyhow::{anyhow, Result};

pub fn get_home_dir() -> PathBuf {
    if let Ok(home) = std::env::var("NAPI_DESKTOP_TEST_HOME") {
        let trimmed = home.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("."))
}

/// NAPI Desktop config root. SSOT lives at `<this>/skills/`.
pub fn get_app_config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| get_home_dir().join(".config"))
        .join("napi-desktop")
}

pub fn get_hermes_dir() -> PathBuf {
    if let Some(raw) = std::env::var_os("HERMES_HOME") {
        let value = raw.to_string_lossy();
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    #[cfg(target_os = "windows")]
    {
        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            let trimmed = local.to_string_lossy();
            if !trimmed.trim().is_empty() {
                return PathBuf::from(local).join("hermes");
            }
        }
        return get_home_dir()
            .join("AppData")
            .join("Local")
            .join("hermes");
    }
    #[cfg(not(target_os = "windows"))]
    {
        get_home_dir().join(".hermes")
    }
}

pub fn get_mcode_dir() -> PathBuf {
    for key in ["MINIMAX_DATA_DIR", "MAVIS_DATA_DIR"] {
        if let Ok(value) = std::env::var(key) {
            let trimmed = value.trim();
            if !trimmed.is_empty() {
                return PathBuf::from(trimmed);
            }
        }
    }
    get_home_dir().join(".minimax")
}

pub fn get_pi_agent_dir() -> Result<PathBuf> {
    if let Some(value) = std::env::var_os("PI_CODING_AGENT_DIR") {
        if !value.is_empty() {
            let path = PathBuf::from(value);
            if !path.is_absolute() {
                return Err(anyhow!(
                    "PI_CODING_AGENT_DIR must be an absolute directory: {}",
                    path.display()
                ));
            }
            return Ok(path);
        }
    }
    Ok(get_home_dir().join(".pi").join("agent"))
}
