use indexmap::IndexMap;

use super::error::McpError;
use super::live;
use super::store;
use super::types::{AppType, McpServer};
use super::validation::validate_server_spec;

pub struct McpService;

impl McpService {
    pub fn get_all_servers() -> Result<IndexMap<String, McpServer>, McpError> {
        let servers = store::load_all()?;
        if servers.is_empty() {
            // First launch: seed the store from live agent configs, no write-back.
            let imported = Self::import_from_all_apps()?;
            if imported > 0 {
                return store::load_all();
            }
        }
        Ok(servers)
    }

    pub fn upsert_server(server: McpServer) -> Result<(), McpError> {
        if server.id.trim().is_empty() {
            return Err(McpError::msg("MCP server id cannot be empty"));
        }
        validate_server_spec(&server.server)?;

        let prev_apps = store::load_all()?
            .get(&server.id)
            .map(|s| s.apps.clone())
            .unwrap_or_default();

        store::upsert(server.clone())?;

        for app in AppType::ALL {
            let was = prev_apps.is_enabled_for(app);
            let now = server.apps.is_enabled_for(app);
            if now {
                live::sync_server(app, &server.id, &server.server)?;
            } else if was && !now {
                live::remove_server(app, &server.id)?;
            }
        }
        Ok(())
    }

    pub fn delete_server(id: &str) -> Result<bool, McpError> {
        let Some(server) = store::delete(id)? else {
            return Ok(false);
        };
        for app in server.apps.enabled_apps() {
            live::remove_server(app, id)?;
        }
        Ok(true)
    }

    pub fn toggle_app(server_id: &str, app: AppType, enabled: bool) -> Result<(), McpError> {
        let mut servers = store::load_all()?;
        let Some(server) = servers.get_mut(server_id) else {
            return Err(McpError::msg(format!("MCP server '{server_id}' not found")));
        };
        server.apps.set_enabled_for(app, enabled);
        let spec = server.server.clone();
        let id = server.id.clone();
        store::save_all(&servers)?;
        if enabled {
            live::sync_server(app, &id, &spec)?;
        } else {
            live::remove_server(app, &id)?;
        }
        Ok(())
    }

    /// Import from live configs. Never write back; only enable apps / insert new ids.
    pub fn import_from_all_apps() -> Result<usize, McpError> {
        let mut servers = store::load_all()?;
        let mut new_count = 0;
        let mut failures = Vec::new();

        for app in AppType::ALL {
            let map = match live::import_servers(app) {
                Ok(m) => m,
                Err(e) => {
                    failures.push(format!("{}: {e}", app.as_str()));
                    continue;
                }
            };
            for (id, spec) in map {
                if let Some(existing) = servers.get_mut(&id) {
                    if !existing.apps.is_enabled_for(app) {
                        existing.apps.set_enabled_for(app, true);
                    }
                } else {
                    servers.insert(id.clone(), McpServer::new(id, spec, super::types::McpApps::for_app(app)));
                    new_count += 1;
                }
            }
        }

        store::save_all(&servers)?;
        if !failures.is_empty() && new_count == 0 && servers.is_empty() {
            return Err(McpError::msg(format!(
                "MCP import failed: {}",
                failures.join("; ")
            )));
        }
        Ok(new_count)
    }
}
