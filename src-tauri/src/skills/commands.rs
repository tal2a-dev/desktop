//! Skills Tauri commands — same names as cc-switch UnifiedSkillsPanel.

use std::str::FromStr;

use tauri::State;

use crate::skills::service::{
    DiscoverableSkill, ImportSkillSelection, MigrationResult, Skill, SkillBackupEntry, SkillService,
    SkillUninstallResult, SkillUpdateInfo, SkillsShSearchResult,
};
use crate::skills::types::{
    AppType, InstalledSkill, SkillRepo, SkillStorageLocation, UnmanagedSkill,
};
use crate::skills::{SkillServiceState, SkillsState};

fn parse_app_type(app: &str) -> Result<AppType, String> {
    AppType::from_str(app).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_installed_skills(state: State<'_, SkillsState>) -> Result<Vec<InstalledSkill>, String> {
    SkillService::get_all_installed(&state.db).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_skill_backups() -> Result<Vec<SkillBackupEntry>, String> {
    SkillService::list_backups().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_skill_backup(backup_id: String) -> Result<bool, String> {
    SkillService::delete_backup(&backup_id).map_err(|e| e.to_string())?;
    Ok(true)
}

#[tauri::command]
pub async fn install_skill_unified(
    skill: DiscoverableSkill,
    current_app: String,
    service: State<'_, SkillServiceState>,
    state: State<'_, SkillsState>,
) -> Result<InstalledSkill, String> {
    let app_type = parse_app_type(&current_app)?;
    service
        .0
        .install(&state.db, &skill, &app_type)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn uninstall_skill_unified(
    id: String,
    state: State<'_, SkillsState>,
) -> Result<SkillUninstallResult, String> {
    SkillService::uninstall(&state.db, &id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn restore_skill_backup(
    backup_id: String,
    current_app: String,
    state: State<'_, SkillsState>,
) -> Result<InstalledSkill, String> {
    let app_type = parse_app_type(&current_app)?;
    SkillService::restore_from_backup(&state.db, &backup_id, &app_type).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn toggle_skill_app(
    id: String,
    app: String,
    enabled: bool,
    state: State<'_, SkillsState>,
) -> Result<bool, String> {
    let app_type = parse_app_type(&app)?;
    SkillService::toggle_app(&state.db, &id, &app_type, enabled).map_err(|e| e.to_string())?;
    Ok(true)
}

#[tauri::command]
pub fn scan_unmanaged_skills(
    state: State<'_, SkillsState>,
) -> Result<Vec<UnmanagedSkill>, String> {
    SkillService::scan_unmanaged(&state.db).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn import_skills_from_apps(
    imports: Vec<ImportSkillSelection>,
    state: State<'_, SkillsState>,
) -> Result<Vec<InstalledSkill>, String> {
    SkillService::import_from_apps(&state.db, imports).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn discover_available_skills(
    service: State<'_, SkillServiceState>,
    state: State<'_, SkillsState>,
) -> Result<Vec<DiscoverableSkill>, String> {
    let repos = state.db.get_skill_repos().map_err(|e| e.to_string())?;
    service
        .0
        .discover_available(repos)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn check_skill_updates(
    service: State<'_, SkillServiceState>,
    state: State<'_, SkillsState>,
) -> Result<Vec<SkillUpdateInfo>, String> {
    service
        .0
        .check_updates(&state.db)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn update_skill(
    id: String,
    service: State<'_, SkillServiceState>,
    state: State<'_, SkillsState>,
) -> Result<InstalledSkill, String> {
    service
        .0
        .update_skill(&state.db, &id)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn migrate_skill_storage(
    target: SkillStorageLocation,
    state: State<'_, SkillsState>,
) -> Result<MigrationResult, String> {
    SkillService::migrate_storage(&state.db, target).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn search_skills_sh(
    query: String,
    limit: usize,
    offset: usize,
) -> Result<SkillsShSearchResult, String> {
    SkillService::search_skills_sh(&query, limit, offset)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn get_skills(
    service: State<'_, SkillServiceState>,
    state: State<'_, SkillsState>,
) -> Result<Vec<Skill>, String> {
    let repos = state.db.get_skill_repos().map_err(|e| e.to_string())?;
    service
        .0
        .list_skills(repos, &state.db)
        .await
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_skill_repos(state: State<'_, SkillsState>) -> Result<Vec<SkillRepo>, String> {
    state.db.get_skill_repos().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_skill_repo(repo: SkillRepo, state: State<'_, SkillsState>) -> Result<bool, String> {
    SkillService::validate_repo_ref(&repo.owner, &repo.name, &repo.branch)
        .map_err(|e| e.to_string())?;
    state.db.save_skill_repo(&repo).map_err(|e| e.to_string())?;
    Ok(true)
}

#[tauri::command]
pub fn remove_skill_repo(
    owner: String,
    name: String,
    state: State<'_, SkillsState>,
) -> Result<bool, String> {
    state
        .db
        .delete_skill_repo(&owner, &name)
        .map_err(|e| e.to_string())?;
    Ok(true)
}

#[tauri::command]
pub fn install_skills_from_zip(
    file_path: String,
    current_app: String,
    state: State<'_, SkillsState>,
) -> Result<Vec<InstalledSkill>, String> {
    let app_type = parse_app_type(&current_app)?;
    let path = std::path::Path::new(&file_path);
    SkillService::install_from_zip(&state.db, path, &app_type).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn open_zip_file_dialog() -> Result<Option<String>, String> {
    let picked = rfd::FileDialog::new()
        .add_filter("ZIP / Skill", &["zip", "skill"])
        .pick_file();
    Ok(picked.map(|p| p.to_string_lossy().into_owned()))
}

#[tauri::command]
pub fn open_external(url: String) -> Result<(), String> {
    let trimmed = url.trim();
    if !trimmed.starts_with("https://") && !trimmed.starts_with("http://") {
        return Err("Unsupported URL scheme".into());
    }
    #[cfg(target_os = "macos")]
    let (program, prefix): (&str, &[&str]) = ("open", &[]);
    #[cfg(target_os = "windows")]
    let (program, prefix): (&str, &[&str]) = ("cmd", &["/c", "start", ""]);
    #[cfg(target_os = "linux")]
    let (program, prefix): (&str, &[&str]) = ("xdg-open", &[]);
    crate::silent_command(program)
        .args(prefix)
        .arg(trimmed)
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("Could not open a browser: {e}"))
}
