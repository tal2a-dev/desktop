use std::sync::Mutex;

use indexmap::IndexMap;
use rusqlite::{params, Connection};

use crate::skills::error::AppError;
use crate::skills::paths::get_app_config_dir;
use crate::skills::types::{InstalledSkill, SkillApps, SkillRepo};

macro_rules! lock_conn {
    ($mutex:expr) => {
        $mutex
            .lock()
            .map_err(|e| AppError::Database(format!("Mutex lock failed: {}", e)))?
    };
}

pub struct Database {
    pub(crate) conn: Mutex<Connection>,
}

impl Database {
    pub fn init() -> Result<Self, AppError> {
        let dir = get_app_config_dir();
        std::fs::create_dir_all(&dir).map_err(|e| AppError::io(&dir, e))?;
        let db_path = dir.join("skills.db");
        let conn = Connection::open(&db_path).map_err(|e| AppError::Database(e.to_string()))?;
        conn.execute("PRAGMA foreign_keys = ON;", [])
            .map_err(|e| AppError::Database(e.to_string()))?;
        let db = Self {
            conn: Mutex::new(conn),
        };
        db.create_tables()?;
        Ok(db)
    }

    fn create_tables(&self) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        conn.execute(
            "CREATE TABLE IF NOT EXISTS skills (
            id TEXT PRIMARY KEY,
            name TEXT NOT NULL,
            description TEXT,
            directory TEXT NOT NULL,
            repo_owner TEXT,
            repo_name TEXT,
            repo_branch TEXT DEFAULT 'main',
            readme_url TEXT,
            enabled_claude BOOLEAN NOT NULL DEFAULT 0,
            enabled_codex BOOLEAN NOT NULL DEFAULT 0,
            enabled_gemini BOOLEAN NOT NULL DEFAULT 0,
            enabled_grokbuild BOOLEAN NOT NULL DEFAULT 0,
            enabled_opencode BOOLEAN NOT NULL DEFAULT 0,
            enabled_mcode BOOLEAN NOT NULL DEFAULT 0,
            enabled_hermes BOOLEAN NOT NULL DEFAULT 0,
            installed_at INTEGER NOT NULL DEFAULT 0,
            content_hash TEXT,
            updated_at INTEGER NOT NULL DEFAULT 0
        )",
            [],
        )?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS skill_repos (
            owner TEXT NOT NULL, name TEXT NOT NULL, branch TEXT NOT NULL DEFAULT 'main',
            enabled BOOLEAN NOT NULL DEFAULT 1, PRIMARY KEY (owner, name)
        )",
            [],
        )?;
        conn.execute(
            "CREATE TABLE IF NOT EXISTS settings (key TEXT PRIMARY KEY, value TEXT)",
            [],
        )?;
        Ok(())
    }

    fn row_to_skill(row: &rusqlite::Row<'_>) -> rusqlite::Result<InstalledSkill> {
        Ok(InstalledSkill {
            id: row.get(0)?,
            name: row.get(1)?,
            description: row.get(2)?,
            directory: row.get(3)?,
            repo_owner: row.get(4)?,
            repo_name: row.get(5)?,
            repo_branch: row.get(6)?,
            readme_url: row.get(7)?,
            apps: SkillApps {
                claude: row.get(8)?,
                codex: row.get(9)?,
                gemini: row.get(10)?,
                grokbuild: row.get(11)?,
                opencode: row.get(12)?,
                hermes: row.get(13)?,
                pi: false,
                mcode: row.get(17)?,
            },
            installed_at: row.get(14)?,
            content_hash: row.get(15)?,
            updated_at: row.get::<_, i64>(16).unwrap_or(0),
        })
    }

    pub fn get_all_installed_skills(&self) -> Result<IndexMap<String, InstalledSkill>, AppError> {
        let conn = lock_conn!(self.conn);
        let mut stmt = conn.prepare(
            "SELECT id, name, description, directory, repo_owner, repo_name, repo_branch,
                    readme_url, enabled_claude, enabled_codex, enabled_gemini, enabled_grokbuild,
                    enabled_opencode, enabled_hermes, installed_at, content_hash, updated_at, enabled_mcode
             FROM skills ORDER BY name ASC",
        )?;
        let skill_iter = stmt.query_map([], Self::row_to_skill)?;
        let mut skills = IndexMap::new();
        for skill_res in skill_iter {
            let skill = skill_res?;
            skills.insert(skill.id.clone(), skill);
        }
        Ok(skills)
    }

    pub fn get_installed_skill(&self, id: &str) -> Result<Option<InstalledSkill>, AppError> {
        let conn = lock_conn!(self.conn);
        let mut stmt = conn.prepare(
            "SELECT id, name, description, directory, repo_owner, repo_name, repo_branch,
                    readme_url, enabled_claude, enabled_codex, enabled_gemini, enabled_grokbuild,
                    enabled_opencode, enabled_hermes, installed_at, content_hash, updated_at, enabled_mcode
             FROM skills WHERE id = ?1",
        )?;
        match stmt.query_row([id], Self::row_to_skill) {
            Ok(skill) => Ok(Some(skill)),
            Err(rusqlite::Error::QueryReturnedNoRows) => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    pub fn save_skill(&self, skill: &InstalledSkill) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        conn.execute(
            "INSERT OR REPLACE INTO skills
             (id, name, description, directory, repo_owner, repo_name, repo_branch,
              readme_url, enabled_claude, enabled_codex, enabled_gemini, enabled_grokbuild, enabled_opencode, enabled_hermes,
              installed_at, content_hash, updated_at, enabled_mcode)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18)",
            params![
                skill.id,
                skill.name,
                skill.description,
                skill.directory,
                skill.repo_owner,
                skill.repo_name,
                skill.repo_branch,
                skill.readme_url,
                skill.apps.claude,
                skill.apps.codex,
                skill.apps.gemini,
                skill.apps.grokbuild,
                skill.apps.opencode,
                skill.apps.hermes,
                skill.installed_at,
                skill.content_hash,
                skill.updated_at,
                skill.apps.mcode,
            ],
        )?;
        Ok(())
    }

    pub fn update_skill_metadata(&self, skill: &InstalledSkill) -> Result<bool, AppError> {
        let conn = lock_conn!(self.conn);
        let affected = conn.execute(
            "UPDATE skills
             SET name = ?1,
                 description = ?2,
                 directory = ?3,
                 repo_owner = ?4,
                 repo_name = ?5,
                 repo_branch = ?6,
                 readme_url = ?7,
                 installed_at = ?8,
                 content_hash = ?9,
                 updated_at = ?10
             WHERE id = ?11 AND installed_at = ?12",
            params![
                skill.name,
                skill.description,
                skill.directory,
                skill.repo_owner,
                skill.repo_name,
                skill.repo_branch,
                skill.readme_url,
                skill.installed_at,
                skill.content_hash,
                skill.updated_at,
                skill.id,
                skill.installed_at,
            ],
        )?;
        Ok(affected > 0)
    }

    pub fn delete_skill(&self, id: &str) -> Result<bool, AppError> {
        let conn = lock_conn!(self.conn);
        let affected = conn.execute("DELETE FROM skills WHERE id = ?1", params![id])?;
        Ok(affected > 0)
    }

    pub fn clear_skills(&self) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        conn.execute("DELETE FROM skills", [])?;
        Ok(())
    }

    pub fn update_skill_apps(&self, id: &str, apps: &SkillApps) -> Result<bool, AppError> {
        let conn = lock_conn!(self.conn);
        let affected = conn.execute(
            "UPDATE skills SET enabled_claude = ?1, enabled_codex = ?2, enabled_gemini = ?3, enabled_grokbuild = ?4, enabled_opencode = ?5, enabled_hermes = ?6, enabled_mcode = ?8 WHERE id = ?7",
            params![apps.claude, apps.codex, apps.gemini, apps.grokbuild, apps.opencode, apps.hermes, id, apps.mcode],
        )?;
        Ok(affected > 0)
    }

    pub fn update_skill_hash(
        &self,
        id: &str,
        content_hash: &str,
        updated_at: i64,
    ) -> Result<bool, AppError> {
        let conn = lock_conn!(self.conn);
        let affected = conn.execute(
            "UPDATE skills SET content_hash = ?1, updated_at = ?2 WHERE id = ?3",
            params![content_hash, updated_at, id],
        )?;
        Ok(affected > 0)
    }

    pub fn get_skill_repos(&self) -> Result<Vec<SkillRepo>, AppError> {
        let conn = lock_conn!(self.conn);
        let mut stmt =
            conn.prepare("SELECT owner, name, branch, enabled FROM skill_repos ORDER BY owner ASC, name ASC")?;
        let repo_iter = stmt.query_map([], |row| {
            Ok(SkillRepo {
                owner: row.get(0)?,
                name: row.get(1)?,
                branch: row.get(2)?,
                enabled: row.get(3)?,
            })
        })?;
        let mut repos = Vec::new();
        for repo_res in repo_iter {
            repos.push(repo_res?);
        }
        Ok(repos)
    }

    pub fn save_skill_repo(&self, repo: &SkillRepo) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        conn.execute(
            "INSERT OR REPLACE INTO skill_repos (owner, name, branch, enabled) VALUES (?1, ?2, ?3, ?4)",
            params![repo.owner, repo.name, repo.branch, repo.enabled],
        )?;
        Ok(())
    }

    pub fn delete_skill_repo(&self, owner: &str, name: &str) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        conn.execute(
            "DELETE FROM skill_repos WHERE owner = ?1 AND name = ?2",
            params![owner, name],
        )?;
        Ok(())
    }

    pub fn get_setting(&self, key: &str) -> Result<Option<String>, AppError> {
        let conn = lock_conn!(self.conn);
        let mut stmt = conn.prepare("SELECT value FROM settings WHERE key = ?1")?;
        let mut rows = stmt.query(params![key])?;
        if let Some(row) = rows.next()? {
            Ok(Some(row.get(0)?))
        } else {
            Ok(None)
        }
    }

    pub fn get_bool_flag(&self, key: &str) -> Result<bool, AppError> {
        Ok(matches!(
            self.get_setting(key)?.as_deref(),
            Some("true") | Some("1")
        ))
    }

    pub fn set_setting(&self, key: &str, value: &str) -> Result<(), AppError> {
        let conn = lock_conn!(self.conn);
        conn.execute(
            "INSERT OR REPLACE INTO settings (key, value) VALUES (?1, ?2)",
            params![key, value],
        )?;
        Ok(())
    }

    pub fn init_default_skill_repos(&self) -> Result<usize, AppError> {
        const INITIALIZED_KEY: &str = "default_skill_repos_initialized";
        if self.get_bool_flag(INITIALIZED_KEY)? {
            return Ok(0);
        }
        if !self.get_skill_repos()?.is_empty() {
            self.set_setting(INITIALIZED_KEY, "true")?;
            return Ok(0);
        }
        let defaults = [
            ("anthropics", "skills", "main"),
            ("ComposioHQ", "awesome-claude-skills", "master"),
            ("cexll", "myclaude", "master"),
            ("JimLiu", "baoyu-skills", "main"),
        ];
        let mut count = 0;
        for (owner, name, branch) in defaults {
            self.save_skill_repo(&SkillRepo {
                owner: owner.to_string(),
                name: name.to_string(),
                branch: branch.to_string(),
                enabled: true,
            })?;
            count += 1;
        }
        self.set_setting(INITIALIZED_KEY, "true")?;
        Ok(count)
    }
}
