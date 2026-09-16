//! Per-agent live config adapters (cc-switch mcp/*, file-based).

use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use super::error::McpError;
use super::paths::{self, should_sync};
use super::types::AppType;
use super::validation::{extract_server_spec, redact_url, strip_ui_fields, validate_server_spec};

pub fn sync_server(app: AppType, id: &str, spec: &Value) -> Result<(), McpError> {
    if !should_sync(app) {
        return Ok(());
    }
    validate_server_spec(spec)?;
    match app {
        AppType::Claude => upsert_json_server(
            &paths::claude_mcp_path(),
            "mcpServers",
            id,
            spec,
            Some(wrap_windows_stdio),
        ),
        AppType::Cursor => {
            upsert_json_server(&paths::cursor_mcp_path(), "mcpServers", id, spec, None)
        }
        AppType::Gemini => upsert_json_server(
            &paths::gemini_settings_path(),
            "mcpServers",
            id,
            spec,
            Some(to_gemini_format),
        ),
        AppType::OpenCode => upsert_json_server(
            &paths::opencode_config_path(),
            "mcp",
            id,
            spec,
            Some(to_opencode_format),
        ),
        AppType::Codex => upsert_toml_server(&paths::codex_config_path(), id, spec, false),
        AppType::GrokBuild => upsert_toml_server(&paths::grok_config_path(), id, spec, true),
        AppType::Hermes => upsert_hermes_server(id, spec),
    }
}

pub fn remove_server(app: AppType, id: &str) -> Result<(), McpError> {
    if !should_sync(app) {
        return Ok(());
    }
    match app {
        AppType::Claude => remove_json_server(&paths::claude_mcp_path(), "mcpServers", id),
        AppType::Cursor => remove_json_server(&paths::cursor_mcp_path(), "mcpServers", id),
        AppType::Gemini => remove_json_server(&paths::gemini_settings_path(), "mcpServers", id),
        AppType::OpenCode => remove_json_server(&paths::opencode_config_path(), "mcp", id),
        AppType::Codex => remove_toml_server(&paths::codex_config_path(), id),
        AppType::GrokBuild => remove_toml_server(&paths::grok_config_path(), id),
        AppType::Hermes => remove_hermes_server(id),
    }
}

pub fn import_servers(app: AppType) -> Result<HashMap<String, Value>, McpError> {
    if !paths::app_config_path(app).exists() {
        return Ok(HashMap::new());
    }
    match app {
        AppType::Claude => read_json_servers(&paths::claude_mcp_path(), "mcpServers", None),
        AppType::Cursor => read_json_servers(&paths::cursor_mcp_path(), "mcpServers", None),
        AppType::Gemini => {
            read_json_servers(&paths::gemini_settings_path(), "mcpServers", Some(from_gemini))
        }
        AppType::OpenCode => {
            read_json_servers(&paths::opencode_config_path(), "mcp", Some(from_opencode))
        }
        AppType::Codex => read_toml_servers(&paths::codex_config_path()),
        AppType::GrokBuild => read_toml_servers(&paths::grok_config_path()),
        AppType::Hermes => read_hermes_servers(),
    }
}

pub fn scan_target(kind: &str, spec: &Value) -> String {
    if kind == "stdio" || kind == "local" {
        spec.get("command")
            .and_then(|v| v.as_str())
            .or_else(|| {
                spec.get("command")
                    .and_then(|v| v.as_array())
                    .and_then(|a| a.first())
                    .and_then(|v| v.as_str())
            })
            .unwrap_or("")
            .to_string()
    } else {
        let url = spec
            .get("url")
            .or_else(|| spec.get("httpUrl"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        redact_url(url)
    }
}

fn atomic_write_path(path: &Path, bytes: &[u8]) -> Result<(), McpError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| McpError::io(parent, e))?;
    }
    crate::atomic_write(path, bytes).map_err(McpError::msg)
}

fn read_json_value(path: &Path) -> Result<Value, McpError> {
    if !path.exists() {
        return Ok(json!({}));
    }
    let text = fs::read_to_string(path).map_err(|e| McpError::io(path, e))?;
    if text.trim().is_empty() {
        return Ok(json!({}));
    }
    serde_json::from_str(&text).map_err(|e| McpError::json(path, e))
}

fn write_json_value(path: &Path, value: &Value) -> Result<(), McpError> {
    let json = serde_json::to_string_pretty(value)
        .map_err(|e| McpError::msg(format!("serialize {}: {e}", path.display())))?;
    atomic_write_path(path, json.as_bytes())
}

fn upsert_json_server(
    path: &Path,
    key: &str,
    id: &str,
    spec: &Value,
    transform: Option<fn(&Value) -> Result<Value, McpError>>,
) -> Result<(), McpError> {
    let mut root = read_json_value(path)?;
    let obj = root
        .as_object_mut()
        .ok_or_else(|| McpError::msg(format!("{} root is not a JSON object", path.display())))?;
    if !obj.contains_key(key) {
        obj.insert(key.to_string(), json!({}));
    }
    let map = obj.get_mut(key).and_then(|v| v.as_object_mut()).ok_or_else(|| {
        McpError::msg(format!("{} '{key}' is not a JSON object", path.display()))
    })?;
    let written = if let Some(tx) = transform {
        tx(&strip_ui_fields(spec))?
    } else {
        strip_ui_fields(spec)
    };
    map.insert(id.to_string(), written);
    write_json_value(path, &root)
}

fn remove_json_server(path: &Path, key: &str, id: &str) -> Result<(), McpError> {
    if !path.exists() {
        return Ok(());
    }
    let mut root = read_json_value(path)?;
    if let Some(map) = root.get_mut(key).and_then(|v| v.as_object_mut()) {
        map.remove(id);
        write_json_value(path, &root)?;
    }
    Ok(())
}

fn read_json_servers(
    path: &Path,
    key: &str,
    transform: Option<fn(&Value) -> Result<Value, McpError>>,
) -> Result<HashMap<String, Value>, McpError> {
    let root = read_json_value(path)?;
    let Some(map) = root.get(key).and_then(|v| v.as_object()) else {
        return Ok(HashMap::new());
    };
    let mut out = HashMap::new();
    for (id, spec) in map {
        let extracted = extract_server_spec(spec).unwrap_or_else(|_| spec.clone());
        let unified = if let Some(tx) = transform {
            match tx(&extracted) {
                Ok(v) => v,
                Err(_) => continue,
            }
        } else {
            extracted
        };
        if validate_server_spec(&unified).is_ok() {
            out.insert(id.clone(), unified);
        }
    }
    Ok(out)
}

#[cfg(windows)]
const WINDOWS_WRAP_COMMANDS: &[&str] = &["npx", "npm", "yarn", "pnpm", "node", "bun", "deno"];

fn wrap_windows_stdio(spec: &Value) -> Result<Value, McpError> {
    let obj = spec
        .as_object()
        .cloned()
        .ok_or_else(|| McpError::msg("MCP spec must be an object"))?;
    #[cfg(windows)]
    {
        let mut obj = obj;
        wrap_command_for_windows(&mut obj);
        return Ok(Value::Object(obj));
    }
    #[cfg(not(windows))]
    Ok(Value::Object(obj))
}

#[cfg(windows)]
fn wrap_command_for_windows(obj: &mut Map<String, Value>) {
    let server_type = obj.get("type").and_then(|v| v.as_str()).unwrap_or("stdio");
    if server_type != "stdio" {
        return;
    }
    let Some(cmd) = obj.get("command").and_then(|v| v.as_str()) else {
        return;
    };
    if cmd.eq_ignore_ascii_case("cmd") || cmd.eq_ignore_ascii_case("cmd.exe") {
        return;
    }
    let cmd_name = Path::new(cmd)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or(cmd);
    if !WINDOWS_WRAP_COMMANDS
        .iter()
        .any(|&c| cmd_name.eq_ignore_ascii_case(c))
    {
        return;
    }
    let original_args = obj
        .get("args")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();
    let mut new_args = vec![Value::String("/c".into()), Value::String(cmd.into())];
    new_args.extend(original_args);
    obj.insert("command".into(), Value::String("cmd".into()));
    obj.insert("args".into(), Value::Array(new_args));
}

fn to_gemini_format(spec: &Value) -> Result<Value, McpError> {
    let mut obj = spec
        .as_object()
        .cloned()
        .ok_or_else(|| McpError::msg("MCP spec must be an object"))?;
    let transport = obj.get("type").and_then(|v| v.as_str()).unwrap_or("stdio");
    if transport == "http" {
        if let Some(url) = obj.remove("url") {
            obj.insert("httpUrl".into(), url);
        }
    }
    obj.remove("type");
    Ok(Value::Object(obj))
}

fn from_gemini(spec: &Value) -> Result<Value, McpError> {
    let mut obj = spec
        .as_object()
        .cloned()
        .ok_or_else(|| McpError::msg("Gemini MCP spec must be an object"))?;
    if let Some(http_url) = obj.remove("httpUrl") {
        obj.insert("url".into(), http_url);
        obj.insert("type".into(), json!("http"));
    }
    if obj.get("type").is_none() {
        if obj.contains_key("command") {
            obj.insert("type".into(), json!("stdio"));
        } else if obj.contains_key("url") {
            obj.insert("type".into(), json!("sse"));
        }
    }
    Ok(Value::Object(obj))
}

fn to_opencode_format(spec: &Value) -> Result<Value, McpError> {
    let obj = spec
        .as_object()
        .ok_or_else(|| McpError::msg("MCP spec must be a JSON object"))?;
    let typ = obj.get("type").and_then(|v| v.as_str()).unwrap_or("stdio");
    let mut result = Map::new();
    match typ {
        "stdio" => {
            result.insert("type".into(), json!("local"));
            let cmd = obj.get("command").and_then(|v| v.as_str()).unwrap_or("");
            let mut command_arr = vec![json!(cmd)];
            if let Some(args) = obj.get("args").and_then(|v| v.as_array()) {
                command_arr.extend(args.iter().cloned());
            }
            result.insert("command".into(), Value::Array(command_arr));
            if let Some(env) = obj.get("env") {
                if env.as_object().is_some_and(|o| !o.is_empty()) {
                    result.insert("environment".into(), env.clone());
                }
            }
            result.insert("enabled".into(), json!(true));
        }
        "sse" | "http" => {
            result.insert("type".into(), json!("remote"));
            if let Some(url) = obj.get("url") {
                result.insert("url".into(), url.clone());
            }
            if let Some(headers) = obj.get("headers") {
                if headers.as_object().is_some_and(|o| !o.is_empty()) {
                    result.insert("headers".into(), headers.clone());
                }
            }
            result.insert("enabled".into(), json!(true));
        }
        _ => return Err(McpError::msg(format!("Unknown MCP type: {typ}"))),
    }
    Ok(Value::Object(result))
}

fn from_opencode(spec: &Value) -> Result<Value, McpError> {
    let obj = spec
        .as_object()
        .ok_or_else(|| McpError::msg("OpenCode MCP spec must be a JSON object"))?;
    let typ = obj.get("type").and_then(|v| v.as_str()).unwrap_or("local");
    let mut result = Map::new();
    match typ {
        "local" | "stdio" => {
            result.insert("type".into(), json!("stdio"));
            if let Some(cmd_arr) = obj.get("command").and_then(|v| v.as_array()) {
                if let Some(cmd) = cmd_arr.first().and_then(|v| v.as_str()) {
                    result.insert("command".into(), json!(cmd));
                }
                if cmd_arr.len() > 1 {
                    result.insert("args".into(), Value::Array(cmd_arr[1..].to_vec()));
                }
            } else if let Some(cmd) = obj.get("command").and_then(|v| v.as_str()) {
                result.insert("command".into(), json!(cmd));
                if let Some(args) = obj.get("args") {
                    result.insert("args".into(), args.clone());
                }
            }
            if let Some(env) = obj.get("environment").or_else(|| obj.get("env")) {
                if env.as_object().is_some_and(|o| !o.is_empty()) {
                    result.insert("env".into(), env.clone());
                }
            }
        }
        "remote" | "sse" | "http" => {
            let out_type = if typ == "http" { "http" } else { "sse" };
            result.insert("type".into(), json!(out_type));
            if let Some(url) = obj.get("url") {
                result.insert("url".into(), url.clone());
            }
            if let Some(headers) = obj.get("headers") {
                result.insert("headers".into(), headers.clone());
            }
        }
        _ => return Err(McpError::msg(format!("Unknown OpenCode MCP type: {typ}"))),
    }
    Ok(Value::Object(result))
}

fn json_value_to_toml_item(value: &Value) -> Option<toml_edit::Item> {
    match value {
        Value::String(s) => Some(toml_edit::value(s.as_str())),
        Value::Number(n) => n
            .as_i64()
            .map(toml_edit::value)
            .or_else(|| n.as_f64().map(toml_edit::value)),
        Value::Bool(b) => Some(toml_edit::value(*b)),
        Value::Array(arr) => {
            let mut toml_arr = toml_edit::Array::default();
            for item in arr {
                match item {
                    Value::String(s) => toml_arr.push(s.as_str()),
                    Value::Number(n) if n.is_i64() => toml_arr.push(n.as_i64().unwrap()),
                    Value::Number(n) if n.is_f64() => toml_arr.push(n.as_f64().unwrap()),
                    Value::Bool(b) => toml_arr.push(*b),
                    _ => return None,
                }
            }
            Some(toml_edit::Item::Value(toml_edit::Value::Array(toml_arr)))
        }
        Value::Object(obj) => {
            let mut inline = toml_edit::InlineTable::new();
            for (k, v) in obj {
                if let Some(s) = v.as_str() {
                    inline.insert(k, s.into());
                } else {
                    return None;
                }
            }
            Some(toml_edit::Item::Value(toml_edit::Value::InlineTable(inline)))
        }
        Value::Null => None,
    }
}

fn json_server_to_toml_table(spec: &Value, grok: bool) -> Result<toml_edit::Table, McpError> {
    let obj = spec
        .as_object()
        .ok_or_else(|| McpError::msg("MCP spec must be an object"))?;
    let mut t = toml_edit::Table::new();
    let typ = obj.get("type").and_then(|v| v.as_str()).unwrap_or("stdio");
    if !grok {
        t["type"] = toml_edit::value(typ);
    }
    for (key, value) in obj {
        if key == "type" {
            continue;
        }
        let out_key = if key == "headers" && !grok {
            "http_headers"
        } else {
            key.as_str()
        };
        if let Some(item) = json_value_to_toml_item(value) {
            t[out_key] = item;
        }
    }
    Ok(t)
}

fn upsert_toml_server(path: &Path, id: &str, spec: &Value, grok: bool) -> Result<(), McpError> {
    let mut doc = read_toml_doc(path)?;
    if let Some(mcp_item) = doc.get_mut("mcp") {
        if let Some(tbl) = mcp_item.as_table_like_mut() {
            tbl.remove("servers");
        }
    }
    if doc
        .get_mut("mcp_servers")
        .and_then(toml_edit::Item::as_table_like_mut)
        .is_none()
    {
        doc["mcp_servers"] = toml_edit::table();
    }
    let table = json_server_to_toml_table(&strip_ui_fields(spec), grok)?;
    let servers = doc
        .get_mut("mcp_servers")
        .and_then(toml_edit::Item::as_table_like_mut)
        .ok_or_else(|| McpError::msg("mcp_servers is not a table"))?;
    servers.insert(id, toml_edit::Item::Table(table));
    atomic_write_path(path, doc.to_string().as_bytes())
}

fn remove_toml_server(path: &Path, id: &str) -> Result<(), McpError> {
    if !path.exists() {
        return Ok(());
    }
    let mut doc = match read_toml_doc(path) {
        Ok(doc) => doc,
        Err(_) => return Ok(()),
    };
    if let Some(item) = doc.get_mut("mcp_servers") {
        if let Some(tbl) = item.as_table_like_mut() {
            tbl.remove(id);
        }
    }
    if let Some(mcp_item) = doc.get_mut("mcp") {
        if let Some(tbl) = mcp_item.as_table_like_mut() {
            if let Some(servers) = tbl.get_mut("servers").and_then(|v| v.as_table_like_mut()) {
                servers.remove(id);
            }
        }
    }
    atomic_write_path(path, doc.to_string().as_bytes())
}

fn read_toml_doc(path: &Path) -> Result<toml_edit::DocumentMut, McpError> {
    if !path.exists() {
        return Ok(toml_edit::DocumentMut::new());
    }
    let text = fs::read_to_string(path).map_err(|e| McpError::io(path, e))?;
    if text.trim().is_empty() {
        return Ok(toml_edit::DocumentMut::new());
    }
    text.parse::<toml_edit::DocumentMut>()
        .map_err(|e| McpError::msg(format!("parse {}: {e}", path.display())))
}

fn toml_to_json(value: &toml::Value) -> Option<Value> {
    match value {
        toml::Value::String(s) => Some(json!(s)),
        toml::Value::Integer(i) => Some(json!(i)),
        toml::Value::Float(f) => Some(json!(f)),
        toml::Value::Boolean(b) => Some(json!(b)),
        toml::Value::Datetime(d) => Some(json!(d.to_string())),
        toml::Value::Array(arr) => Some(Value::Array(arr.iter().filter_map(toml_to_json).collect())),
        toml::Value::Table(tbl) => {
            let mut obj = Map::new();
            for (k, v) in tbl {
                if let Some(jv) = toml_to_json(v) {
                    obj.insert(k.clone(), jv);
                }
            }
            Some(Value::Object(obj))
        }
    }
}

fn read_toml_servers(path: &Path) -> Result<HashMap<String, Value>, McpError> {
    let text = fs::read_to_string(path).map_err(|e| McpError::io(path, e))?;
    if text.trim().is_empty() {
        return Ok(HashMap::new());
    }
    let root: toml::Table = toml::from_str(&text)
        .map_err(|e| McpError::msg(format!("parse {}: {e}", path.display())))?;
    let mut out = HashMap::new();
    let mut import_tbl = |tbl: &toml::value::Table| {
        for (id, entry) in tbl {
            let Some(entry_tbl) = entry.as_table() else {
                continue;
            };
            let mut spec = Map::new();
            for (k, v) in entry_tbl {
                let key = if k == "http_headers" { "headers" } else { k };
                if let Some(jv) = toml_to_json(v) {
                    spec.insert(key.to_string(), jv);
                }
            }
            if !spec.contains_key("type") {
                let default = if spec.contains_key("url") {
                    "http"
                } else {
                    "stdio"
                };
                spec.insert("type".into(), json!(default));
            }
            let spec_v = Value::Object(spec);
            if validate_server_spec(&spec_v).is_ok() {
                out.insert(id.clone(), spec_v);
            }
        }
    };
    if let Some(tbl) = root.get("mcp_servers").and_then(|v| v.as_table()) {
        import_tbl(tbl);
    }
    if let Some(tbl) = root
        .get("mcp")
        .and_then(|v| v.as_table())
        .and_then(|m| m.get("servers"))
        .and_then(|v| v.as_table())
    {
        import_tbl(tbl);
    }
    Ok(out)
}

fn hermes_path() -> PathBuf {
    paths::hermes_config_path()
}

fn yaml_to_json(value: &serde_yaml::Value) -> Result<Value, McpError> {
    serde_json::to_value(value).map_err(|e| McpError::msg(format!("yaml→json: {e}")))
}

fn json_to_yaml(value: &Value) -> Result<serde_yaml::Value, McpError> {
    serde_yaml::to_value(value).map_err(|e| McpError::msg(format!("json→yaml: {e}")))
}

fn read_hermes_root() -> Result<serde_yaml::Mapping, McpError> {
    let path = hermes_path();
    if !path.exists() {
        return Ok(serde_yaml::Mapping::new());
    }
    let text = fs::read_to_string(&path).map_err(|e| McpError::io(&path, e))?;
    if text.trim().is_empty() {
        return Ok(serde_yaml::Mapping::new());
    }
    let value: serde_yaml::Value =
        serde_yaml::from_str(&text).map_err(|e| McpError::msg(format!("parse {}: {e}", path.display())))?;
    match value {
        serde_yaml::Value::Mapping(m) => Ok(m),
        serde_yaml::Value::Null => Ok(serde_yaml::Mapping::new()),
        _ => Err(McpError::msg("Hermes config.yaml root must be a mapping")),
    }
}

fn write_hermes_root(root: &serde_yaml::Mapping) -> Result<(), McpError> {
    let path = hermes_path();
    let text = serde_yaml::to_string(root)
        .map_err(|e| McpError::msg(format!("serialize {}: {e}", path.display())))?;
    atomic_write_path(&path, text.as_bytes())
}

fn to_hermes_format(spec: &Value) -> Result<Value, McpError> {
    let obj = spec
        .as_object()
        .ok_or_else(|| McpError::msg("MCP spec must be a JSON object"))?;
    let typ = obj.get("type").and_then(|v| v.as_str()).unwrap_or("stdio");
    let mut result = Map::new();
    match typ {
        "stdio" => {
            if let Some(command) = obj.get("command") {
                result.insert("command".into(), command.clone());
            }
            if let Some(args) = obj.get("args") {
                if args.as_array().is_some_and(|a| !a.is_empty()) {
                    result.insert("args".into(), args.clone());
                }
            }
            if let Some(env) = obj.get("env") {
                if env.as_object().is_some_and(|o| !o.is_empty()) {
                    result.insert("env".into(), env.clone());
                }
            }
        }
        "sse" | "http" => {
            if let Some(url) = obj.get("url") {
                result.insert("url".into(), url.clone());
            }
            if let Some(headers) = obj.get("headers") {
                if headers.as_object().is_some_and(|o| !o.is_empty()) {
                    result.insert("headers".into(), headers.clone());
                }
            }
        }
        _ => return Err(McpError::msg(format!("Unknown MCP type: {typ}"))),
    }
    result.insert("enabled".into(), json!(true));
    Ok(Value::Object(result))
}

fn from_hermes_format(id: &str, spec: &Value) -> Result<Value, McpError> {
    let obj = spec
        .as_object()
        .ok_or_else(|| McpError::msg("Hermes MCP spec must be a JSON object"))?;
    let mut result = Map::new();
    if obj.contains_key("command") {
        result.insert("type".into(), json!("stdio"));
        result.insert("command".into(), obj["command"].clone());
        if let Some(args) = obj.get("args") {
            if args.as_array().is_some_and(|a| !a.is_empty()) {
                result.insert("args".into(), args.clone());
            }
        }
        if let Some(env) = obj.get("env") {
            if env.as_object().is_some_and(|o| !o.is_empty()) {
                result.insert("env".into(), env.clone());
            }
        }
    } else if obj.contains_key("url") {
        result.insert("type".into(), json!("sse"));
        result.insert("url".into(), obj["url"].clone());
        if let Some(headers) = obj.get("headers") {
            result.insert("headers".into(), headers.clone());
        }
    } else {
        return Err(McpError::msg(format!(
            "Hermes MCP server '{id}' has neither command nor url"
        )));
    }
    Ok(Value::Object(result))
}

const HERMES_EXTRA_FIELDS: &[&str] = &[
    "enabled",
    "timeout",
    "connect_timeout",
    "tools",
    "sampling",
    "roots",
    "auth",
];

fn upsert_hermes_server(id: &str, spec: &Value) -> Result<(), McpError> {
    let hermes_spec = to_hermes_format(spec)?;
    let mut root = read_hermes_root()?;
    let key = serde_yaml::Value::String("mcp_servers".into());
    let mut servers = match root.remove(&key) {
        Some(serde_yaml::Value::Mapping(m)) => m,
        _ => serde_yaml::Mapping::new(),
    };
    let id_yaml = serde_yaml::Value::String(id.to_string());
    let merged = if let Some(existing) = servers.get(&id_yaml) {
        let existing_json = yaml_to_json(existing)?;
        let mut result = Map::new();
        if let Some(existing_obj) = existing_json.as_object() {
            for &field in HERMES_EXTRA_FIELDS {
                if let Some(val) = existing_obj.get(field) {
                    result.insert(field.to_string(), val.clone());
                }
            }
        }
        if let Some(new_obj) = hermes_spec.as_object() {
            for (k, v) in new_obj {
                if HERMES_EXTRA_FIELDS.contains(&k.as_str()) && result.contains_key(k) {
                    continue;
                }
                result.insert(k.clone(), v.clone());
            }
        }
        Value::Object(result)
    } else {
        hermes_spec
    };
    servers.insert(id_yaml, json_to_yaml(&merged)?);
    root.insert(key, serde_yaml::Value::Mapping(servers));
    write_hermes_root(&root)
}

fn remove_hermes_server(id: &str) -> Result<(), McpError> {
    if !hermes_path().exists() {
        return Ok(());
    }
    let mut root = read_hermes_root()?;
    let key = serde_yaml::Value::String("mcp_servers".into());
    if let Some(serde_yaml::Value::Mapping(servers)) = root.get_mut(&key) {
        servers.remove(serde_yaml::Value::String(id.to_string()));
        write_hermes_root(&root)?;
    }
    Ok(())
}

fn read_hermes_servers() -> Result<HashMap<String, Value>, McpError> {
    let root = read_hermes_root()?;
    let key = serde_yaml::Value::String("mcp_servers".into());
    let Some(serde_yaml::Value::Mapping(servers)) = root.get(&key) else {
        return Ok(HashMap::new());
    };
    let mut out = HashMap::new();
    for (k, v) in servers {
        let Some(id) = k.as_str() else { continue };
        let spec_json = yaml_to_json(v)?;
        if let Ok(unified) = from_hermes_format(id, &spec_json) {
            if validate_server_spec(&unified).is_ok() {
                out.insert(id.to_string(), unified);
            }
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opencode_roundtrip_stdio() {
        let spec = json!({"type":"stdio","command":"npx","args":["-y","pkg"]});
        let oc = to_opencode_format(&spec).unwrap();
        assert_eq!(oc["type"], "local");
        let back = from_opencode(&oc).unwrap();
        assert_eq!(back["command"], "npx");
        assert_eq!(back["args"][0], "-y");
    }

    #[test]
    fn gemini_httpurl_roundtrip() {
        let spec = json!({"type":"http","url":"https://x"});
        let g = to_gemini_format(&spec).unwrap();
        assert_eq!(g["httpUrl"], "https://x");
        assert!(g.get("type").is_none());
        let back = from_gemini(&g).unwrap();
        assert_eq!(back["type"], "http");
        assert_eq!(back["url"], "https://x");
    }
}
