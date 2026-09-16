pub mod commands;
pub mod db;
pub mod error;
pub mod http;
pub mod paths;
pub mod service;
pub mod settings;
pub mod types;

use std::sync::Arc;

use crate::skills::db::Database;
use crate::skills::service::SkillService;

pub struct SkillsState {
    pub db: Arc<Database>,
}

pub struct SkillServiceState(pub Arc<SkillService>);

pub fn init_state() -> Result<(SkillsState, SkillServiceState), String> {
    let db = Database::init().map_err(|e| e.to_string())?;
    if let Err(e) = db.init_default_skill_repos() {
        log::warn!("Failed to initialize default skill repos: {e}");
    }
    let db = Arc::new(db);
    match crate::skills::service::migrate_skills_to_ssot(&db) {
        Ok(n) if n > 0 => log::info!("Migrated {n} skills into SSOT"),
        Ok(_) => {}
        Err(e) => log::warn!("SSOT skill migration skipped: {e}"),
    }
    if let Err(e) = SkillService::backfill_content_hashes(&db) {
        log::warn!("Skill hash backfill skipped: {e}");
    }
    Ok((
        SkillsState { db },
        SkillServiceState(Arc::new(SkillService::new())),
    ))
}
