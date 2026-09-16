use std::path::PathBuf;

use super::types::AppType;

pub fn home_dir() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("."))
}

pub fn store_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| home_dir().join(".config"))
        .join("napi-desktop")
        .join("mcp-servers.json")
}

pub fn claude_mcp_path() -> PathBuf {
    home_dir().join(".claude.json")
}

pub fn claude_dir() -> PathBuf {
    home_dir().join(".claude")
}

pub fn cursor_dir() -> PathBuf {
    home_dir().join(".cursor")
}

pub fn cursor_mcp_path() -> PathBuf {
    cursor_dir().join("mcp.json")
}

pub fn codex_dir() -> PathBuf {
    home_dir().join(".codex")
}

pub fn codex_config_path() -> PathBuf {
    codex_dir().join("config.toml")
}

pub fn gemini_dir() -> PathBuf {
    home_dir().join(".gemini")
}

pub fn gemini_settings_path() -> PathBuf {
    gemini_dir().join("settings.json")
}

pub fn grok_dir() -> PathBuf {
    home_dir().join(".grok")
}

pub fn grok_config_path() -> PathBuf {
    grok_dir().join("config.toml")
}

pub fn opencode_dir() -> PathBuf {
    home_dir().join(".config").join("opencode")
}

pub fn opencode_config_path() -> PathBuf {
    opencode_dir().join("opencode.json")
}

pub fn hermes_dir() -> PathBuf {
    if let Some(raw) = std::env::var_os("HERMES_HOME") {
        let value = raw.to_string_lossy();
        let trimmed = value.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    #[cfg(windows)]
    {
        if let Some(local) = dirs::data_local_dir() {
            return local.join("hermes");
        }
    }
    home_dir().join(".hermes")
}

pub fn hermes_config_path() -> PathBuf {
    hermes_dir().join("config.yaml")
}

pub fn app_dir(app: AppType) -> PathBuf {
    match app {
        AppType::Claude => claude_dir(),
        AppType::Cursor => cursor_dir(),
        AppType::Codex => codex_dir(),
        AppType::Gemini => gemini_dir(),
        AppType::GrokBuild => grok_dir(),
        AppType::OpenCode => opencode_dir(),
        AppType::Hermes => hermes_dir(),
    }
}

pub fn app_config_path(app: AppType) -> PathBuf {
    match app {
        AppType::Claude => claude_mcp_path(),
        AppType::Cursor => cursor_mcp_path(),
        AppType::Codex => codex_config_path(),
        AppType::Gemini => gemini_settings_path(),
        AppType::GrokBuild => grok_config_path(),
        AppType::OpenCode => opencode_config_path(),
        AppType::Hermes => hermes_config_path(),
    }
}

/// Skip live writes when the agent has never been initialized on this machine.
pub fn should_sync(app: AppType) -> bool {
    let dir = app_dir(app);
    let path = app_config_path(app);
    dir.exists() || path.exists()
}
