use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::str::FromStr;

use super::live;
use super::paths;
use super::service::McpService;
use super::types::{AppType, McpApps, McpServer, ScannedMcpServer};

pub fn mcp_key(registry_name: &str) -> String {
    registry_name
        .rsplit('/')
        .next()
        .unwrap_or(registry_name)
        .to_string()
}

pub fn merge_mcp_server(
    root: &mut serde_json::Value,
    key: &str,
    url: &str,
    transport: &str,
) -> Result<(), String> {
    let obj = root
        .as_object_mut()
        .ok_or("config root is not a JSON object")?;
    let servers = obj
        .entry("mcpServers")
        .or_insert_with(|| serde_json::json!({}));
    let map = servers
        .as_object_mut()
        .ok_or("mcpServers is not a JSON object")?;
    map.insert(
        key.to_string(),
        serde_json::json!({
            "type": if transport.is_empty() { "http" } else { transport },
            "url": url,
        }),
    );
    Ok(())
}

fn scanned_from_spec(name: &str, agent: &str, spec: &serde_json::Value) -> ScannedMcpServer {
    let kind = spec
        .get("type")
        .and_then(|v| v.as_str())
        .unwrap_or("stdio")
        .to_string();
    ScannedMcpServer {
        name: name.to_string(),
        agent: agent.to_string(),
        kind: kind.clone(),
        target: live::scan_target(&kind, spec),
    }
}

/// Every MCP server configured in any agent, credentials stripped.
#[tauri::command]
pub fn scan_mcp_servers() -> Vec<ScannedMcpServer> {
    let mut out = Vec::new();
    for app in AppType::ALL {
        let Ok(map) = live::import_servers(app) else {
            continue;
        };
        for (name, spec) in map {
            out.push(scanned_from_spec(&name, app.display_name(), &spec));
        }
    }
    out.sort_by(|a, b| a.agent.cmp(&b.agent).then(a.name.cmp(&b.name)));
    out
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryServer {
    pub name: String,
    pub title: String,
    pub description: String,
    pub version: String,
    pub url: String,
    pub transport: String,
    pub installed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryPage {
    pub servers: Vec<RegistryServer>,
    pub next_cursor: Option<String>,
}

const MCP_REGISTRY: &str = "https://registry.modelcontextprotocol.io/v0/servers";

#[tauri::command]
pub async fn fetch_registry_servers(
    cursor: Option<String>,
    limit: Option<usize>,
) -> Result<RegistryPage, String> {
    let limit = limit.unwrap_or(30).clamp(1, 100);
    let mut url = format!("{}?limit={}", MCP_REGISTRY, limit);
    if let Some(c) = cursor.filter(|c| !c.is_empty()) {
        url.push_str(&format!("&cursor={}", urlencoding::encode(&c)));
    }

    let client = reqwest::Client::new();
    let resp = crate::http_get_with_retry(&client, &url).await?;
    if !resp.status().is_success() {
        return Err(format!("Registry returned HTTP {}", resp.status()));
    }
    let body: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;

    let local: Vec<String> = scan_mcp_servers().into_iter().map(|s| s.name).collect();

    let servers = body["servers"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|entry| {
            let s = entry.get("server")?;
            let name = s["name"].as_str()?.to_string();
            let remote = s["remotes"].as_array().and_then(|r| r.first());
            let url = remote
                .and_then(|r| r["url"].as_str())
                .unwrap_or("")
                .to_string();
            let transport = remote
                .and_then(|r| r["type"].as_str())
                .unwrap_or("")
                .to_string();
            let key = mcp_key(&name);
            let installed = local.iter().any(|l| l == &name || l == &key);
            Some(RegistryServer {
                title: s["title"].as_str().unwrap_or("").to_string(),
                description: s["description"].as_str().unwrap_or("").to_string(),
                version: s["version"].as_str().unwrap_or("").to_string(),
                name,
                url,
                transport,
                installed,
            })
        })
        .collect();

    Ok(RegistryPage {
        servers,
        next_cursor: body["metadata"]["nextCursor"].as_str().map(String::from),
    })
}

/// Install a registry server into Claude Code and the unified store.
#[tauri::command]
pub fn install_mcp_server(name: String, url: String, transport: String) -> Result<String, String> {
    crate::validate_base_url(&url)?;
    let path = paths::claude_mcp_path();

    let mut root: serde_json::Value = match std::fs::read_to_string(&path) {
        Ok(text) if !text.trim().is_empty() => serde_json::from_str(&text).map_err(|e| {
            format!("~/.claude.json is not valid JSON: {e}. Refusing to overwrite it.")
        })?,
        _ => serde_json::json!({}),
    };

    let key = mcp_key(&name);
    merge_mcp_server(&mut root, &key, &url, &transport)?;

    let out = serde_json::to_string_pretty(&root).map_err(|e| e.to_string())?;
    crate::atomic_write(&path, out.as_bytes())?;

    let unified_type = if transport.is_empty() || transport == "streamable-http" {
        "http"
    } else {
        transport.as_str()
    };
    let spec = serde_json::json!({
        "type": unified_type,
        "url": url,
    });
    let mut apps = McpApps::default();
    apps.claude = true;
    let _ = McpService::upsert_server(McpServer::new(key.clone(), spec, apps));

    Ok(format!("Installed {key} into Claude Code"))
}

#[tauri::command]
pub fn get_mcp_servers() -> Result<IndexMap<String, McpServer>, String> {
    McpService::get_all_servers().map_err(Into::into)
}

#[tauri::command]
pub fn upsert_mcp_server(server: McpServer) -> Result<(), String> {
    McpService::upsert_server(server).map_err(Into::into)
}

#[tauri::command]
pub fn delete_mcp_server(id: String) -> Result<bool, String> {
    McpService::delete_server(&id).map_err(Into::into)
}

#[tauri::command]
pub fn toggle_mcp_app(server_id: String, app: String, enabled: bool) -> Result<(), String> {
    let app_ty = AppType::from_str(&app)?;
    McpService::toggle_app(&server_id, app_ty, enabled).map_err(Into::into)
}

#[tauri::command]
pub fn import_mcp_from_apps() -> Result<usize, String> {
    McpService::import_from_all_apps().map_err(Into::into)
}

#[tauri::command]
pub fn validate_mcp_command(cmd: String) -> Result<bool, String> {
    if cmd.trim().is_empty() {
        return Ok(false);
    }
    if cmd.contains('/') || cmd.contains('\\') {
        return Ok(Path::new(&cmd).exists());
    }
    let path_var = std::env::var_os("PATH").unwrap_or_default();
    #[cfg(windows)]
    let exts: Vec<String> = std::env::var("PATHEXT")
        .unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".into())
        .split(';')
        .map(|s| s.trim().to_uppercase())
        .collect();

    for p in std::env::split_paths(&path_var) {
        let candidate = p.join(&cmd);
        if candidate.is_file() {
            return Ok(true);
        }
        #[cfg(windows)]
        {
            for ext in &exts {
                let cand = p.join(format!("{}{}", cmd, ext));
                if cand.is_file() {
                    return Ok(true);
                }
            }
        }
    }
    Ok(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_redact_url_strips_credentials() {
        assert_eq!(
            crate::mcp::redact_url("https://host/mcp?token=sk-secret&x=1"),
            "https://host/mcp"
        );
        assert_eq!(crate::mcp::redact_url("https://host/mcp#frag"), "https://host/mcp");
        assert_eq!(crate::mcp::redact_url("https://host/mcp"), "https://host/mcp");
        assert_eq!(crate::mcp::redact_url(""), "");
    }

    #[test]
    fn test_mcp_key_takes_registry_tail() {
        assert_eq!(mcp_key("ac.inference.sh/mcp"), "mcp");
        assert_eq!(mcp_key("ai.agentgates/mcp"), "mcp");
        assert_eq!(mcp_key("obsidian"), "obsidian");
    }

    #[test]
    fn test_merge_mcp_server_preserves_other_keys() {
        let mut root: serde_json::Value = serde_json::from_str(
            r#"{"mcpServers":{"obsidian":{"type":"http","url":"https://x"}},"other":{"keep":1}}"#,
        )
        .unwrap();
        merge_mcp_server(
            &mut root,
            "mcp",
            "https://api.inference.sh/mcp",
            "streamable-http",
        )
        .unwrap();
        assert_eq!(
            root["mcpServers"]["mcp"]["url"],
            "https://api.inference.sh/mcp"
        );
        assert_eq!(root["mcpServers"]["mcp"]["type"], "streamable-http");
        assert_eq!(root["mcpServers"]["obsidian"]["url"], "https://x");
        assert_eq!(root["other"]["keep"], 1);
    }

    #[test]
    fn test_merge_mcp_server_defaults_transport() {
        let mut root = serde_json::json!({});
        merge_mcp_server(&mut root, "thing", "https://host/mcp", "").unwrap();
        assert_eq!(root["mcpServers"]["thing"]["type"], "http");
    }
}
