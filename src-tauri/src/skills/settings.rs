use std::fs;
use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use serde::{Deserialize, Serialize};

use crate::skills::error::AppError;
use crate::skills::paths::get_app_config_dir;
use crate::skills::types::{SkillStorageLocation, SyncMethod};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SkillsSettingsFile {
    #[serde(default)]
    skill_sync_method: SyncMethod,
    #[serde(default)]
    skill_storage_location: SkillStorageLocation,
}

impl Default for SkillsSettingsFile {
    fn default() -> Self {
        Self {
            skill_sync_method: SyncMethod::default(),
            skill_storage_location: SkillStorageLocation::default(),
        }
    }
}

fn settings_path() -> PathBuf {
    get_app_config_dir().join("skills-settings.json")
}

fn settings() -> &'static Mutex<SkillsSettingsFile> {
    static SETTINGS: OnceLock<Mutex<SkillsSettingsFile>> = OnceLock::new();
    SETTINGS.get_or_init(|| {
        let loaded = fs::read_to_string(settings_path())
            .ok()
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .unwrap_or_default();
        Mutex::new(loaded)
    })
}

fn persist(current: &SkillsSettingsFile) -> Result<(), AppError> {
    let path = settings_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| AppError::io(parent, e))?;
    }
    let raw = serde_json::to_string_pretty(current)
        .map_err(|e| AppError::Config(e.to_string()))?;
    let tmp = path.with_extension("json.tmp");
    fs::write(&tmp, raw).map_err(|e| AppError::io(&tmp, e))?;
    fs::rename(&tmp, &path).map_err(|e| {
        let _ = fs::remove_file(&tmp);
        AppError::io(&path, e)
    })
}

pub fn get_skill_sync_method() -> SyncMethod {
    settings()
        .lock()
        .map(|s| s.skill_sync_method)
        .unwrap_or_default()
}

pub fn get_skill_storage_location() -> SkillStorageLocation {
    settings()
        .lock()
        .map(|s| s.skill_storage_location)
        .unwrap_or_default()
}

pub fn set_skill_storage_location(location: SkillStorageLocation) -> Result<(), AppError> {
    let mut guard = settings().lock()?;
    guard.skill_storage_location = location;
    persist(&guard)
}

pub fn get_claude_override_dir() -> Option<PathBuf> {
    None
}
pub fn get_codex_override_dir() -> Option<PathBuf> {
    None
}
pub fn get_gemini_override_dir() -> Option<PathBuf> {
    None
}
pub fn get_grok_override_dir() -> Option<PathBuf> {
    None
}
pub fn get_opencode_override_dir() -> Option<PathBuf> {
    None
}
pub fn get_openclaw_override_dir() -> Option<PathBuf> {
    None
}
pub fn get_hermes_override_dir() -> Option<PathBuf> {
    None
}
