use crate::env_checker::EnvConflict;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;



#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupInfo {
    pub backup_path: String,
    pub timestamp: String,
    pub conflicts: Vec<EnvConflict>,
}

/// Delete environment variables with automatic backup
pub fn delete_env_vars(conflicts: Vec<EnvConflict>) -> Result<BackupInfo, String> {
    // Step 1: Create backup
    let backup_info = create_backup(&conflicts)?;

    // Step 2: Delete variables
    for conflict in &conflicts {
        match delete_single_env(conflict) {
            Ok(_) => {}
            Err(e) => {
                // If deletion fails, we keep the backup but return error
                return Err(format!(
                    "Couldn't remove {}: {}. Backup: {}",
                    conflict.var_name, e, backup_info.backup_path
                ));
            }
        }
    }

    Ok(backup_info)
}

/// Create backup file before deletion
fn create_backup(conflicts: &[EnvConflict]) -> Result<BackupInfo, String> {
    // Get backup directory
    let backup_dir = get_backup_dir()?;
    fs::create_dir_all(&backup_dir).map_err(|e| format!("创建备份目录失败: {e}"))?;

    // Generate backup file name with timestamp
    let timestamp = Utc::now().format("%Y%m%d_%H%M%S").to_string();
    let backup_file = backup_dir.join(format!("env-backup-{timestamp}.json"));

    // Create backup data
    let backup_info = BackupInfo {
        backup_path: backup_file.to_string_lossy().to_string(),
        timestamp: timestamp.clone(),
        conflicts: conflicts.to_vec(),
    };

    // Write backup file
    let json = serde_json::to_string_pretty(&backup_info)
        .map_err(|e| format!("序列化备份数据失败: {e}"))?;

    fs::write(&backup_file, json).map_err(|e| format!("写入备份文件失败: {e}"))?;

    Ok(backup_info)
}

/// Get backup directory path
fn get_backup_dir() -> Result<PathBuf, String> {
    let home = dirs::home_dir().ok_or("无法获取用户主目录")?;
    Ok(home.join(".config").join("napi-desktop").join("backups"))
}

fn unset_process_var(name: &str) {
    // SAFETY: Tauri invoke handlers run on the async runtime; we only mutate
    // this process's env so later agent launches inherit a clean map.
    unsafe { std::env::remove_var(name) };
}

fn strip_var_from_file(file_path: &str, var_name: &str) -> Result<bool, String> {
    let Ok(content) = fs::read_to_string(file_path) else {
        return Ok(false);
    };
    let mut changed = false;
    let new_content: Vec<String> = content
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            if trimmed.starts_with('#') {
                return true;
            }
            let export_line = trimmed.strip_prefix("export ").unwrap_or(trimmed);
            if let Some(eq_pos) = export_line.find('=') {
                if export_line[..eq_pos].trim() == var_name {
                    changed = true;
                    return false;
                }
            }
            true
        })
        .map(|s| s.to_string())
        .collect();
    if changed {
        fs::write(file_path, new_content.join("\n"))
            .map_err(|e| format!("Couldn't write {file_path}: {e}"))?;
    }
    Ok(changed)
}

fn common_shell_files() -> Vec<String> {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("/tmp"));
    let home = home.to_string_lossy();
    vec![
        format!("{home}/.zshenv"),
        format!("{home}/.zshrc"),
        format!("{home}/.zprofile"),
        format!("{home}/.bashrc"),
        format!("{home}/.bash_profile"),
        format!("{home}/.profile"),
        format!("{home}/.config/fish/config.fish"),
    ]
}

fn delete_single_env(conflict: &EnvConflict) -> Result<(), String> {
    unset_process_var(&conflict.var_name);

    match conflict.source_type.as_str() {
        "file" => {
            let file_path = conflict
                .source_path
                .rsplit_once(':')
                .map(|(p, _)| p)
                .unwrap_or(conflict.source_path.as_str());
            strip_var_from_file(file_path, &conflict.var_name)?;
            Ok(())
        }
        "system" => {
            // Inherited env (launchd / parent IDE). Unset above. Also strip
            // matching export lines from common shell profiles so the next
            // Terminal launch does not put it back.
            for path in common_shell_files() {
                let _ = strip_var_from_file(&path, &conflict.var_name);
            }
            Ok(())
        }
        _ => Err(format!("Unknown env source: {}", conflict.source_type)),
    }
}

/// Restore environment variables from backup
#[allow(dead_code)]
pub fn restore_from_backup(backup_path: String) -> Result<(), String> {
    // Read backup file
    let content = fs::read_to_string(&backup_path).map_err(|e| format!("读取备份文件失败: {e}"))?;

    let backup_info: BackupInfo =
        serde_json::from_str(&content).map_err(|e| format!("解析备份文件失败: {e}"))?;

    // Restore each variable
    for conflict in &backup_info.conflicts {
        restore_single_env(conflict)?;
    }

    Ok(())
}

#[allow(dead_code)]
fn restore_single_env(conflict: &EnvConflict) -> Result<(), String> {
    match conflict.source_type.as_str() {
        "file" => {
            // Parse file path from source_path
            let parts: Vec<&str> = conflict.source_path.split(':').collect();
            if parts.is_empty() {
                return Err("无效的文件路径格式".to_string());
            }

            let file_path = parts[0];

            // Read file content
            let mut content = fs::read_to_string(file_path)
                .map_err(|e| format!("读取文件失败 {file_path}: {e}"))?;

            // Append the environment variable line
            let export_line = format!("\nexport {}={}", conflict.var_name, conflict.var_value);
            content.push_str(&export_line);

            // Write back to file
            fs::write(file_path, content).map_err(|e| format!("写入文件失败 {file_path}: {e}"))?;

            Ok(())
        }
        _ => Err(format!(
            "无法恢复类型为 {} 的环境变量",
            conflict.source_type
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_backup_dir_creation() {
        let backup_dir = get_backup_dir();
        assert!(backup_dir.is_ok());
    }

    #[test]
    fn test_delete_process_env_unsets_current_process() {
        let name = "NAPI_DESKTOP_TEST_ENV_DELETE";
        unsafe { std::env::set_var(name, "to-remove") };
        let conflict = EnvConflict {
            var_name: name.into(),
            var_value: "to-remove".into(),
            source_type: "system".into(),
            source_path: "Process Environment".into(),
        };
        delete_single_env(&conflict).unwrap();
        assert!(std::env::var(name).is_err());
    }
}
