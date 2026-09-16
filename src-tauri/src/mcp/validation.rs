use serde_json::Value;

use super::error::McpError;

/// Strip query + fragment so tokens in MCP URLs never reach the UI.
pub fn redact_url(url: &str) -> String {
    let no_fragment = url.split('#').next().unwrap_or(url);
    no_fragment.split('?').next().unwrap_or(no_fragment).to_string()
}

/// Allow stdio/http/sse; omitted type is treated as stdio.
pub fn validate_server_spec(spec: &Value) -> Result<(), McpError> {
    if !spec.is_object() {
        return Err(McpError::msg("MCP server spec must be a JSON object"));
    }
    let t_opt = spec.get("type").and_then(|x| x.as_str());
    let is_stdio = t_opt.map(|t| t == "stdio").unwrap_or(true);
    let is_http = t_opt.map(|t| t == "http").unwrap_or(false);
    let is_sse = t_opt.map(|t| t == "sse").unwrap_or(false);

    if !(is_stdio || is_http || is_sse) {
        return Err(McpError::msg(
            "MCP server type must be 'stdio', 'http', or 'sse' (or omitted for stdio)",
        ));
    }

    if is_stdio {
        let cmd = spec.get("command").and_then(|x| x.as_str()).unwrap_or("");
        if cmd.trim().is_empty() {
            return Err(McpError::msg("stdio MCP server is missing command"));
        }
    }
    if is_http {
        let url = spec.get("url").and_then(|x| x.as_str()).unwrap_or("");
        if url.trim().is_empty() {
            return Err(McpError::msg("http MCP server is missing url"));
        }
    }
    if is_sse {
        let url = spec.get("url").and_then(|x| x.as_str()).unwrap_or("");
        if url.trim().is_empty() {
            return Err(McpError::msg("sse MCP server is missing url"));
        }
    }
    Ok(())
}

pub fn extract_server_spec(entry: &Value) -> Result<Value, McpError> {
    let obj = entry
        .as_object()
        .ok_or_else(|| McpError::msg("MCP server entry must be a JSON object"))?;
    if let Some(server) = obj.get("server") {
        if !server.is_object() {
            return Err(McpError::msg("MCP server.server must be a JSON object"));
        }
        return Ok(server.clone());
    }
    Ok(entry.clone())
}

pub fn strip_ui_fields(spec: &Value) -> Value {
    let mut obj = match spec.as_object() {
        Some(map) => map.clone(),
        None => return spec.clone(),
    };
    for key in [
        "enabled",
        "source",
        "id",
        "name",
        "description",
        "tags",
        "homepage",
        "docs",
        "apps",
    ] {
        obj.remove(key);
    }
    Value::Object(obj)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn stdio_requires_command() {
        assert!(validate_server_spec(&json!({"type":"stdio"})).is_err());
        assert!(validate_server_spec(&json!({"command":"npx"})).is_ok());
    }

    #[test]
    fn http_requires_url() {
        assert!(validate_server_spec(&json!({"type":"http"})).is_err());
        assert!(validate_server_spec(&json!({"type":"http","url":"https://x"})).is_ok());
    }
}
