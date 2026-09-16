use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::fs;
use std::sync::Mutex;

use super::error::McpError;
use super::paths::store_path;
use super::types::McpServer;

static WRITE_LOCK: Mutex<()> = Mutex::new(());

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct StoreFile {
    #[serde(default)]
    servers: IndexMap<String, McpServer>,
}

fn read_unlocked() -> Result<IndexMap<String, McpServer>, McpError> {
    let path = store_path();
    if !path.exists() {
        return Ok(IndexMap::new());
    }
    let text = fs::read_to_string(&path).map_err(|e| McpError::io(&path, e))?;
    if text.trim().is_empty() {
        return Ok(IndexMap::new());
    }
    let file: StoreFile = serde_json::from_str(&text).map_err(|e| McpError::json(&path, e))?;
    Ok(file.servers)
}

fn write_unlocked(servers: &IndexMap<String, McpServer>) -> Result<(), McpError> {
    let path = store_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| McpError::io(parent, e))?;
    }
    let file = StoreFile {
        servers: servers.clone(),
    };
    let json = serde_json::to_string_pretty(&file)
        .map_err(|e| McpError::msg(format!("serialize mcp store: {e}")))?;
    crate::atomic_write(&path, json.as_bytes()).map_err(McpError::msg)
}

pub fn load_all() -> Result<IndexMap<String, McpServer>, McpError> {
    let _guard = WRITE_LOCK
        .lock()
        .map_err(|e| McpError::msg(e.to_string()))?;
    read_unlocked()
}

pub fn save_all(servers: &IndexMap<String, McpServer>) -> Result<(), McpError> {
    let _guard = WRITE_LOCK
        .lock()
        .map_err(|e| McpError::msg(e.to_string()))?;
    write_unlocked(servers)
}

pub fn upsert(server: McpServer) -> Result<IndexMap<String, McpServer>, McpError> {
    let _guard = WRITE_LOCK
        .lock()
        .map_err(|e| McpError::msg(e.to_string()))?;
    let mut servers = read_unlocked()?;
    servers.insert(server.id.clone(), server);
    write_unlocked(&servers)?;
    Ok(servers)
}

pub fn delete(id: &str) -> Result<Option<McpServer>, McpError> {
    let _guard = WRITE_LOCK
        .lock()
        .map_err(|e| McpError::msg(e.to_string()))?;
    let mut servers = read_unlocked()?;
    let removed = servers.shift_remove(id);
    if removed.is_some() {
        write_unlocked(&servers)?;
    }
    Ok(removed)
}
