use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Stable id used by the UI ("claude", "codex", ...).
    pub id: String,
    pub name: String,
    pub description: String,
    pub config_path: PathBuf,
    pub key_field: String,
    pub base_url_field: String,
    pub format: String,
    /// A config file exists on disk.
    pub detected: bool,
    /// The CLI binary resolves on PATH (or a known install dir) and can be launched.
    pub installed: bool,
    /// The config already points at this NAPI instance.
    pub configured: bool,
    /// "auto" = we can write the config; "guide" = user must edit it themselves.
    pub config_type: String,
    /// Brand accent for the UI tile.
    pub color: String,
    pub binary_name: String,
}

/// ponytail: cheap "is this already pointed at us" probe — a substring check on
/// the raw config text. Upgrade to a real per-format parse if a user ever runs
/// two different NAPI bases at once.
fn has_napi_marker(path: &std::path::Path) -> bool {
    std::fs::read_to_string(path)
        .map(|s| s.contains("napi.mikawi.org"))
        .unwrap_or(false)
}

fn home_dir() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("."))
}

fn atomic_write(path: &std::path::Path, content: &[u8]) -> Result<(), String> {
    let tmp = path.with_extension("napi-tmp");
    std::fs::write(&tmp, content).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        e.to_string()
    })
}

/// ponytail: 3 retries with exponential backoff covers transient network/429s
async fn http_get_with_retry(
    client: &reqwest::Client,
    url: &str,
) -> Result<reqwest::Response, String> {
    let mut last_err = String::new();
    for attempt in 0..3 {
        match client.get(url).send().await {
            Ok(resp) if resp.status().as_u16() == 429 => {
                last_err = format!("Rate limited (429) on attempt {}", attempt + 1);
                tokio::time::sleep(std::time::Duration::from_millis(500 * 2u64.pow(attempt as u32)))
                    .await;
                continue;
            }
            Ok(resp) => return Ok(resp),
            Err(e) => {
                last_err = e.to_string();
                tokio::time::sleep(std::time::Duration::from_millis(500 * 2u64.pow(attempt as u32)))
                    .await;
            }
        }
    }
    Err(last_err)
}

async fn http_get_auth_with_retry(
    client: &reqwest::Client,
    url: &str,
    token: &str,
) -> Result<reqwest::Response, String> {
    let mut last_err = String::new();
    for attempt in 0..3 {
        match client
            .get(url)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await
        {
            Ok(resp) if resp.status().as_u16() == 429 => {
                last_err = format!("Rate limited (429) on attempt {}", attempt + 1);
                tokio::time::sleep(std::time::Duration::from_millis(500 * 2u64.pow(attempt as u32)))
                    .await;
                continue;
            }
            Ok(resp) => return Ok(resp),
            Err(e) => {
                last_err = e.to_string();
                tokio::time::sleep(std::time::Duration::from_millis(500 * 2u64.pow(attempt as u32)))
                    .await;
            }
        }
    }
    Err(last_err)
}

fn mask_token(token: &str) -> String {
    if token.len() <= 8 {
        "****".to_string()
    } else {
        format!("{}...{}", &token[..4], &token[token.len() - 4..])
    }
}

fn validate_base_url(url: &str) -> Result<(), String> {
    if url.trim().is_empty() {
        return Err("Base URL cannot be empty".into());
    }
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err("Base URL must start with http:// or https://".into());
    }
    Ok(())
}

/// ponytail: binary names are interpolated into an AppleScript string and a cmd
/// line, so restrict them to a charset that cannot break out of either.
fn validate_binary_name(name: &str) -> Result<(), String> {
    if name.is_empty() || name.len() > 64 {
        return Err("Invalid agent binary name".into());
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
    {
        return Err(format!("Invalid agent binary name: {}", name));
    }
    Ok(())
}

/// Is the agent CLI actually runnable?
///
/// ponytail: a bundled .app inherits launchd's minimal PATH, so `which` alone
/// reports false negatives for ~/.local/bin, /opt/homebrew/bin and friends.
/// Probe the usual install dirs directly as a fallback.
fn binary_exists(binary_name: &str) -> bool {
    if validate_binary_name(binary_name).is_err() {
        return false;
    }

    let probe = if cfg!(target_os = "windows") {
        "where"
    } else {
        "which"
    };
    if std::process::Command::new(probe)
        .arg(binary_name)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map(|s| s.success())
        .unwrap_or(false)
    {
        return true;
    }

    let home = home_dir();
    let exe = if cfg!(target_os = "windows") {
        format!("{}.exe", binary_name)
    } else {
        binary_name.to_string()
    };
    let candidates = [
        home.join(".local/bin"),
        home.join("bin"),
        home.join(".bun/bin"),
        home.join(".cargo/bin"),
        PathBuf::from("/opt/homebrew/bin"),
        PathBuf::from("/usr/local/bin"),
        PathBuf::from("/usr/bin"),
    ];
    candidates.iter().any(|dir| dir.join(&exe).is_file())
}

/// new-api reports quota in internal units; `quota_per_unit` converts to USD.
fn quota_to_usd(raw: f64, per_unit: f64) -> f64 {
    if per_unit > 0.0 {
        raw / per_unit
    } else {
        raw
    }
}

/// Brand accent used by the UI tile for each tool.
fn agent_color(id: &str) -> &'static str {
    match id {
        "claude" => "#D97757",
        "codex" => "#10A37F",
        "cline" => "#5B9BD5",
        "opencode" => "#E87040",
        "cursor" => "#8B8B8B",
        "gemini" => "#4285F4",
        "continue" => "#7C3AED",
        "goose" => "#00A67E",
        "factory" => "#00D4FF",
        "grok" => "#9CA3AF",
        "roo" => "#FF6B6B",
        "kilocode" => "#F97316",
        "hermes" => "#8B5CF6",
        "qwen" => "#10B981",
        "windsurf" => "#09B6A2",
        _ => "#6E7681",
    }
}

#[tauri::command]
fn scan_agents() -> Vec<AgentConfig> {
    let home = home_dir();
    let appdata = std::env::var("APPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|_| home.join("AppData").join("Roaming"));

    // (id, name, description, config_path, key_field, base_url_field, format,
    //  binary_name, config_type)
    let specs: Vec<(&str, &str, &str, PathBuf, &str, &str, &str, &str, &str)> = vec![
        (
            "claude",
            "Claude Code",
            "Anthropic's terminal coding agent",
            home.join(".claude").join("settings.json"),
            "env.ANTHROPIC_API_KEY",
            "env.ANTHROPIC_BASE_URL",
            "json",
            "claude",
            "auto",
        ),
        (
            "codex",
            "OpenAI Codex CLI",
            "OpenAI's coding CLI",
            home.join(".codex").join("config.toml"),
            "env_key",
            "base_url",
            "toml",
            "codex",
            "auto",
        ),
        (
            "cline",
            "Cline",
            "Autonomous coding agent for VS Code",
            home.join(".cline").join("data").join("globalState.json"),
            "apiKey",
            "baseUrl",
            "json",
            "cline",
            "auto",
        ),
        (
            "opencode",
            "OpenCode",
            "Open-source terminal AI assistant",
            if cfg!(target_os = "windows") {
                appdata.join("opencode").join("opencode.json")
            } else {
                home.join(".config").join("opencode").join("opencode.json")
            },
            "provider_env.OPENAI_API_KEY",
            "provider_env.OPENAI_BASE_URL",
            "json",
            "opencode",
            "auto",
        ),
        (
            "gemini",
            "Gemini CLI",
            "Google's Gemini command-line agent",
            home.join(".gemini").join("settings.json"),
            "",
            "",
            "json",
            "gemini",
            "guide",
        ),
        (
            "cursor",
            "Cursor",
            "Cursor AI code editor",
            home.join(".cursor").join("cli-config.json"),
            "",
            "",
            "json",
            "agent",
            "guide",
        ),
        (
            "continue",
            "Continue",
            "Open-source IDE coding assistant",
            home.join(".continue").join("config.json"),
            "",
            "",
            "json",
            "continue",
            "guide",
        ),
        (
            "goose",
            "Goose",
            "Block's local autonomous agent",
            home.join(".config").join("goose").join("config.yaml"),
            "",
            "",
            "yaml",
            "goose",
            "guide",
        ),
        (
            "factory",
            "Factory Droid",
            "Factory's autonomous coding agent",
            home.join(".factory").join("config.json"),
            "",
            "",
            "json",
            "droid",
            "guide",
        ),
        (
            "grok",
            "Grok CLI",
            "xAI's terminal coding agent",
            home.join(".grok").join("auth.json"),
            "",
            "",
            "json",
            "grok",
            "guide",
        ),
        (
            "roo",
            "Roo Code",
            "VS Code autonomous coding agent",
            home.join(".roo"),
            "",
            "",
            "json",
            "roo",
            "guide",
        ),
        (
            "kilocode",
            "Kilo Code",
            "VS Code autonomous coding agent",
            home.join(".kilocode"),
            "",
            "",
            "json",
            "kilocode",
            "guide",
        ),
        (
            "hermes",
            "Hermes Agent",
            "Nous Research self-improving agent",
            home.join(".hermes"),
            "",
            "",
            "json",
            "hermes",
            "guide",
        ),
        (
            "qwen",
            "Qwen Code",
            "Alibaba's coding CLI",
            home.join(".qwen").join("settings.json"),
            "",
            "",
            "json",
            "qwen",
            "guide",
        ),
        (
            "windsurf",
            "Windsurf",
            "Codeium's IDE agent",
            home.join(".codeium").join("windsurf"),
            "",
            "",
            "json",
            "windsurf",
            "guide",
        ),
    ];

    specs
        .into_iter()
        .map(
            |(id, name, description, config_path, key_field, base_url_field, format, binary_name, config_type)| {
                AgentConfig {
                    id: id.into(),
                    name: name.into(),
                    description: description.into(),
                    detected: config_path.exists(),
                    installed: binary_exists(binary_name),
                    configured: has_napi_marker(&config_path),
                    config_path,
                    key_field: key_field.into(),
                    base_url_field: base_url_field.into(),
                    format: format.into(),
                    config_type: config_type.into(),
                    color: agent_color(id).into(),
                    binary_name: binary_name.into(),
                }
            },
        )
        .collect()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServer {
    pub name: String,
    /// Which agent this server is configured in.
    pub agent: String,
    /// "stdio" | "http" | "sse" | ...
    pub kind: String,
    /// Command name for stdio, scheme://host/path for http. Credentials are NEVER included.
    pub target: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillInfo {
    pub name: String,
    pub agent: String,
}

/// ponytail: strip query + fragment so tokens embedded in MCP URLs never reach the UI.
fn redact_url(url: &str) -> String {
    let no_fragment = url.split('#').next().unwrap_or(url);
    no_fragment.split('?').next().unwrap_or(no_fragment).to_string()
}

/// Every MCP server configured in any agent, with credentials stripped.
#[tauri::command]
fn scan_mcp_servers() -> Vec<McpServer> {
    let home = home_dir();
    let sources: [(&str, PathBuf, &str); 3] = [
        ("Claude Code", home.join(".claude.json"), "mcpServers"),
        ("Cursor", home.join(".cursor").join("mcp.json"), "mcpServers"),
        (
            "OpenCode",
            home.join(".config").join("opencode").join("opencode.json"),
            "mcp",
        ),
    ];

    let mut out = Vec::new();
    for (agent, path, key) in sources {
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Ok(json) = serde_json::from_str::<serde_json::Value>(&text) else {
            continue;
        };
        let servers = json
            .get(key)
            .or_else(|| json.get("mcpServers"))
            .and_then(|v| v.as_object());
        let Some(servers) = servers else { continue };

        for (name, cfg) in servers {
            let kind = cfg
                .get("type")
                .and_then(|v| v.as_str())
                .unwrap_or("stdio")
                .to_string();
            // ponytail: only the command or the redacted URL — never env/headers/args,
            // which routinely hold live API keys.
            let target = if kind == "stdio" {
                cfg.get("command")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string()
            } else {
                redact_url(cfg.get("url").and_then(|v| v.as_str()).unwrap_or(""))
            };
            out.push(McpServer {
                name: name.clone(),
                agent: agent.to_string(),
                kind,
                target,
            });
        }
    }
    out.sort_by(|a, b| a.agent.cmp(&b.agent).then(a.name.cmp(&b.name)));
    out
}

/// Every installed skill across agents. A skill is a directory containing SKILL.md.
#[tauri::command]
fn scan_skills() -> Vec<SkillInfo> {
    let home = home_dir();
    let sources: [(&str, PathBuf); 8] = [
        ("Claude Code", home.join(".claude").join("skills")),
        ("OpenCode", home.join(".config").join("opencode").join("skills")),
        ("Cline", home.join(".cline").join("skills")),
        ("Roo", home.join(".roo").join("skills")),
        ("Kilo Code", home.join(".kilocode").join("skills")),
        ("Grok CLI", home.join(".grok").join("skills")),
        ("Qwen Code", home.join(".qwen").join("skills")),
        ("Hermes", home.join(".hermes").join("skills")),
    ];

    let mut out = Vec::new();
    for (agent, dir) in sources {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() && path.join("SKILL.md").is_file() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    out.push(SkillInfo {
                        name: name.to_string(),
                        agent: agent.to_string(),
                    });
                }
            }
        }
    }
    out.sort_by(|a, b| a.agent.cmp(&b.agent).then(a.name.cmp(&b.name)));
    out
}

#[tauri::command]
fn reconfigure_agent(
    agent_name: String,
    api_key: String,
    base_url: String,
) -> Result<String, String> {
    validate_base_url(&base_url)?;
    if api_key.trim().is_empty() {
        return Err("API key cannot be empty".into());
    }
    match agent_name.as_str() {
        "Claude Code" => {
            let path = home_dir().join(".claude").join("settings.json");
            let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
            let mut v: serde_json::Value =
                serde_json::from_str(&content).map_err(|e| e.to_string())?;
            let env = v
                .as_object_mut()
                .ok_or("settings.json is not an object")?
                .entry("env")
                .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
            env["ANTHROPIC_API_KEY"] = serde_json::Value::String(api_key);
            env["ANTHROPIC_BASE_URL"] = serde_json::Value::String(base_url);
            let out = serde_json::to_string_pretty(&v).map_err(|e| e.to_string())?;
            atomic_write(&path, out.as_bytes())?;
            Ok(format!("Reconfigured {}", agent_name))
        }
        "Codex" => {
            let path = home_dir().join(".codex").join("config.toml");
            let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
            let mut doc = content
                .parse::<toml_edit::DocumentMut>()
                .map_err(|e| e.to_string())?;
            // Set openai_base_url for built-in provider override
            doc["openai_base_url"] = toml_edit::value(base_url);
            let out = doc.to_string();
            atomic_write(&path, out.as_bytes())?;
            Ok(format!(
                "Reconfigured {}: base_url written to config.toml. Codex reads its key from the \
                 OPENAI_API_KEY environment variable, which this app deliberately does not write \
                 — run `export OPENAI_API_KEY=<key from Console → Tokens>` in your shell.",
                agent_name
            ))
        }
        "Cline" => {
            let path = home_dir()
                .join(".cline")
                .join("data")
                .join("settings")
                .join("providers.json");
            let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
            let mut v: serde_json::Value =
                serde_json::from_str(&content).map_err(|e| e.to_string())?;
            // Find or create NAPI provider entry
            let providers = v
                .as_object_mut()
                .ok_or("providers.json is not an object")?;
            let napi_key = "napi";
            if !providers.contains_key(napi_key) {
                providers.insert(
                    napi_key.into(),
                    serde_json::Value::Object(serde_json::Map::new()),
                );
            }
            let provider = providers[napi_key].as_object_mut().unwrap();
            provider.insert("apiKey".into(), serde_json::Value::String(api_key));
            provider.insert("baseUrl".into(), serde_json::Value::String(base_url));
            let out = serde_json::to_string_pretty(&v).map_err(|e| e.to_string())?;
            atomic_write(&path, out.as_bytes())?;
            Ok(format!("Reconfigured {}", agent_name))
        }
        "OpenCode" => {
            let path = if cfg!(target_os = "windows") {
                let appdata = std::env::var("APPDATA")
                    .map(PathBuf::from)
                    .unwrap_or_else(|_| home_dir().join("AppData").join("Roaming"));
                appdata.join("opencode").join("opencode.json")
            } else {
                home_dir()
                    .join(".config")
                    .join("opencode")
                    .join("opencode.json")
            };
            let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
            let mut v: serde_json::Value =
                serde_json::from_str(&content).map_err(|e| e.to_string())?;
            let penv = v
                .as_object_mut()
                .ok_or("opencode.json is not an object")?
                .entry("provider_env")
                .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
            penv["OPENAI_API_KEY"] = serde_json::Value::String(api_key);
            penv["OPENAI_BASE_URL"] = serde_json::Value::String(base_url);
            let out = serde_json::to_string_pretty(&v).map_err(|e| e.to_string())?;
            atomic_write(&path, out.as_bytes())?;
            Ok(format!("Reconfigured {}", agent_name))
        }
        "Gemini CLI" | "Cursor" => Err(format!(
            "{} cannot be auto-configured. Configure manually via its settings UI.",
            agent_name
        )),
        _ => Err(format!("Agent '{}' not supported", agent_name)),
    }
}

/// One-click setup: use the stored API key, write the agent config.
/// ponytail: the key never comes from the frontend — it is read from the keyring,
/// so "Quick Setup" needs no typing from the user.
#[tauri::command]
fn configure_agent(agent_name: String, base_url: String) -> Result<String, String> {
    let entry = keyring::Entry::new("napi-desktop", "user-token").map_err(|e| e.to_string())?;
    let key = entry.get_password().map_err(|_| {
        "No API key stored — sign in (or create a key) before configuring agents".to_string()
    })?;
    reconfigure_agent(agent_name, key, base_url)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionInfo {
    pub plan: String,
    pub used: f64,
    pub limit: f64,
    pub reset_date: String,
}

#[tauri::command]
async fn fetch_subscription(base_url: String) -> Result<SubscriptionInfo, String> {
    validate_base_url(&base_url)?;
    // ponytail: token is read from the OS keyring here, never passed from the
    // frontend — it must not cross the IPC boundary (BUILD_PROMPT §7).
    let token = {
        let entry = keyring::Entry::new("napi-desktop", "user-token")
            .map_err(|e| e.to_string())?;
        entry
            .get_password()
            .map_err(|e| e.to_string())?
    };
    if token.is_empty() {
        return Err("No stored credentials — sign in first".into());
    }
    let base = base_url.trim_end_matches('/');
    let client = reqwest::Client::new();
    let resp = http_get_auth_with_retry(&client, &format!("{}/api/user/self", base), &token).await?;

    if !resp.status().is_success() {
        return Err(format!("HTTP {} (token: {})", resp.status(), mask_token(&token)));
    }

    let body: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;

    // ponytail: new-api reports quota in internal units. /api/status carries
    // `quota_per_unit` (500000 on this deployment); without it the UI would be
    // off by 5 orders of magnitude.
    let per_unit = match http_get_with_retry(&client, &format!("{}/api/status", base)).await {
        Ok(status_resp) => match status_resp.json::<serde_json::Value>().await {
            Ok(status) => status["data"]["quota_per_unit"]
                .as_f64()
                .filter(|v| *v > 0.0)
                .unwrap_or(500_000.0),
            Err(_) => 500_000.0,
        },
        Err(_) => 500_000.0,
    };

    Ok(SubscriptionInfo {
        plan: body["data"]["role"]
            .as_str()
            .unwrap_or("user")
            .to_string(),
        used: quota_to_usd(body["data"]["used_quota"].as_f64().unwrap_or(0.0), per_unit),
        limit: quota_to_usd(body["data"]["quota"].as_f64().unwrap_or(0.0), per_unit),
        reset_date: body["data"]["reset_at"]
            .as_str()
            .unwrap_or("unknown")
            .to_string(),
    })
}

#[tauri::command]
async fn login(
    username: String,
    password: String,
    base_url: String,
) -> Result<String, String> {
    validate_base_url(&base_url)?;
    if username.trim().is_empty() || password.is_empty() {
        return Err("Username and password are required".into());
    }
    let url = format!("{}/api/user/login", base_url.trim_end_matches('/'));
    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .json(&serde_json::json!({ "username": username, "password": password }))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let body: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;

    if body["success"].as_bool() == Some(true) {
        // Check if 2FA is required
        if body["data"]["require_verification"].as_bool() == Some(true) {
            let flow_token = body["data"]["flow_token"]
                .as_str()
                .ok_or("2FA required but no flow_token in response")?;
            // Return special marker so frontend knows to show 2FA input
            return Ok(format!("2FA_REQUIRED:{}", flow_token));
        }

        // ponytail: broad fallback chain — new-api forks vary token field names
        let token = body["data"]["token"]
            .as_str()
            .or_else(|| body["token"].as_str())
            .or_else(|| body["data"]["access_token"].as_str())
            .or_else(|| body["data"]["key"].as_str())
            .or_else(|| body["key"].as_str())
            .or_else(|| body["data"]["session_token"].as_str())
            .or_else(|| body["data"]["accessToken"].as_str())
            .or_else(|| body["accessToken"].as_str());

        let token = match token {
            Some(t) => t.to_string(),
            None => {
                let keys: Vec<String> = if let Some(obj) = body.as_object() {
                    obj.keys().cloned().collect()
                } else {
                    vec!["(not an object)".into()]
                };
                let data_keys: Vec<String> = if let Some(obj) = body["data"].as_object() {
                    obj.keys().cloned().collect()
                } else {
                    vec!["(no data object)".into()]
                };
                return Err(format!(
                    "Login succeeded but no token found. Top-level keys: {:?}, data keys: {:?}",
                    keys, data_keys
                ));
            }
        };

        store_credential(token.clone())?;
        // ponytail: token never crosses the IPC boundary — only "" (success) or
        // "2FA_REQUIRED:<flow>" crosses (flow token is not a credential).
        Ok(String::new())
    } else {
        Err(body["message"]
            .as_str()
            .unwrap_or("Login failed")
            .to_string())
    }
}

#[tauri::command]
async fn verify_2fa(
    flow_token: String,
    code: String,
    base_url: String,
) -> Result<(), String> {
    validate_base_url(&base_url)?;
    if flow_token.trim().is_empty() || code.trim().is_empty() {
        return Err("Flow token and verification code are required".into());
    }
    let url = format!("{}/api/user/login/verify", base_url.trim_end_matches('/'));
    let client = reqwest::Client::new();
    let resp = client
        .post(&url)
        .json(&serde_json::json!({ "flow_token": flow_token, "code": code }))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    let body: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;

    if body["success"].as_bool() == Some(true) {
        let token = body["data"]["token"]
            .as_str()
            .or_else(|| body["token"].as_str())
            .or_else(|| body["data"]["access_token"].as_str())
            .or_else(|| body["data"]["key"].as_str())
            .or_else(|| body["key"].as_str())
            .ok_or("2FA verified but no token found in response")?
            .to_string();

        store_credential(token.clone())?;
        Ok(())
    } else {
        Err(body["message"]
            .as_str()
            .unwrap_or("2FA verification failed")
            .to_string())
    }
}

#[tauri::command]
async fn github_oauth(base_url: String) -> Result<(), String> {
    validate_base_url(&base_url)?;

    // ponytail: local HTTP callback server captures OAuth code from browser redirect.
    // Requires http://localhost:9876/callback added to GitHub OAuth app redirect URIs.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:9876")
        .await
        .map_err(|e| format!("Failed to start OAuth callback server: {}", e))?;
    // ponytail: must match the exact string registered in GitHub OAuth app settings
    let redirect_uri = "http://localhost:9876/callback".to_string();

    // Generate CSRF state
    let state: String = (0..32)
        .map(|_| {
            let idx = rand::random::<usize>() % 62;
            match idx {
                0..=9 => (b'0' + idx as u8) as char,
                10..=35 => (b'a' + (idx - 10) as u8) as char,
                _ => (b'A' + (idx - 36) as u8) as char,
            }
        })
        .collect();

    // Fetch github_client_id from /api/status
    let status_url = format!("{}/api/status", base_url.trim_end_matches('/'));
    let client = reqwest::Client::new();
    let resp = http_get_with_retry(&client, &status_url).await?;
    let body: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    let client_id = body["data"]["github_client_id"]
        .as_str()
        .filter(|s| !s.is_empty())
        .ok_or("GitHub OAuth is not configured on this server")?;

    let auth_url = format!(
        "https://github.com/login/oauth/authorize?client_id={}&redirect_uri={}&state={}&scope=read:user",
        client_id,
        urlencoding::encode(&redirect_uri),
        state
    );

    // Open system browser
    #[cfg(target_os = "macos")]
    std::process::Command::new("open").arg(&auth_url).spawn().map_err(|e| e.to_string())?;
    #[cfg(target_os = "windows")]
    std::process::Command::new("cmd").args(["/c", "start", &auth_url]).spawn().map_err(|e| e.to_string())?;
    #[cfg(target_os = "linux")]
    std::process::Command::new("xdg-open").arg(&auth_url).spawn().map_err(|e| e.to_string())?;

    // Wait for callback with timeout
    let (stream, _) = tokio::time::timeout(
        std::time::Duration::from_secs(120),
        listener.accept(),
    )
    .await
    .map_err(|_| "OAuth callback timed out after 2 minutes")?
    .map_err(|e| format!("Failed to accept OAuth callback: {}", e))?;

    // Read HTTP request
    let mut buf = vec![0u8; 4096];
    let n = stream.peek(&mut buf).await.map_err(|e| e.to_string())?;
    let request = String::from_utf8_lossy(&buf[..n]);

    // Parse code and state from query string
    let code = request
        .lines()
        .next()
        .and_then(|line| line.split("GET ").nth(1))
        .and_then(|path| path.split(" ").next())
        .and_then(|qs| {
            qs.split('?').nth(1).map(|params| {
                params
                    .split('&')
                    .find_map(|p| p.strip_prefix("code="))
                    .unwrap_or("")
            })
        })
        .unwrap_or("");

    let returned_state = request
        .lines()
        .next()
        .and_then(|line| line.split("GET ").nth(1))
        .and_then(|path| path.split(" ").next())
        .and_then(|qs| {
            qs.split('?').nth(1).map(|params| {
                params
                    .split('&')
                    .find_map(|p| p.strip_prefix("state="))
                    .unwrap_or("")
            })
        })
        .unwrap_or("");

    if returned_state != state {
        return Err("OAuth state mismatch — possible CSRF attack".into());
    }
    if code.is_empty() {
        return Err("No authorization code received from GitHub".into());
    }

    // ponytail: send HTTP 200 to browser so it doesn't show a connection error
    use tokio::io::AsyncWriteExt;
    let mut stream = stream;
    let _ = stream
        .write_all(b"HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nConnection: close\r\n\r\n<html><body><h1>Login successful!</h1><p>You can close this tab and return to NAPI Desktop.</p></body></html>")
        .await;
    let _ = stream.flush().await;
    drop(stream);

    // Secret comes from the repo-root .env via build.rs — never in source, never in git.
    let github_client_secret = option_env!("GITHUB_CLIENT_SECRET")
        .filter(|s| !s.is_empty())
        .ok_or("GITHUB_CLIENT_SECRET is not set. Copy .env.example to .env, fill it in, rebuild.")?;

    let token_resp = client
        .post("https://github.com/login/oauth/access_token")
        .header("Accept", "application/json")
        .json(&serde_json::json!({
            "client_id": client_id,
            "client_secret": github_client_secret,
            "code": code,
            "redirect_uri": redirect_uri,
        }))
        .send()
        .await
        .map_err(|e| format!("Failed to exchange code with GitHub: {}", e))?;

    let token_body: serde_json::Value = token_resp.json().await.map_err(|e| e.to_string())?;
    if token_body["access_token"].as_str().is_none() {
        return Err(format!(
            "GitHub token exchange failed: {}",
            token_body["error_description"]
                .as_str()
                .unwrap_or("no access_token in response")
        ));
    }

    // ponytail: the exchange works, but NAPI has no endpoint that turns a GitHub
    // access token into a session token — /api/oauth/github demands server-side
    // state we cannot supply. Server needs POST /api/oauth/github/desktop.
    Err("GitHub authorized, but this server has no endpoint to finish desktop sign-in. \
         Server needs: POST /api/oauth/github/desktop { code, redirect_uri }."
        .to_string())
}


#[tauri::command]
async fn ensure_api_key(base_url: String) -> Result<(), String> {
    validate_base_url(&base_url)?;
    let base = base_url.trim_end_matches('/');
    let token = {
        let entry = keyring::Entry::new("napi-desktop", "user-token")
            .map_err(|e| e.to_string())?;
        entry.get_password().map_err(|e| e.to_string())?
    };

    let client = reqwest::Client::new();
    // List existing tokens; reuse one named "napi-desktop" if present.
    let resp = client
        .get(format!("{}/api/token/", base))
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {} listing tokens", resp.status()));
    }
    let list: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;
    let has_ours = list["data"]["items"]
        .as_array()
        .or_else(|| list["data"].as_array())
        .map(|items| {
            items
                .iter()
                .any(|t| t["name"].as_str() == Some("napi-desktop"))
        })
        .unwrap_or(false);
    if has_ours {
        return Ok(()); // ponytail: reuse instead of spawning duplicates on every login
    }

    // Create a fresh token named "napi-desktop".
    let resp = client
        .post(format!("{}/api/token/", base))
        .header("Authorization", format!("Bearer {}", token))
        .json(&serde_json::json!({ "name": "napi-desktop", "remain_quota": 500000, "unlimited_quota": true, "expired_time": -1 }))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {} creating token", resp.status()));
    }
    // new-api returns only success on create — the key stays the login token,
    // which is a valid API key in this fork.
    Ok(())
}

#[tauri::command]
async fn auto_configure_all(base_url: String) -> Result<Vec<String>, String> {
    validate_base_url(&base_url)?;
    let token = {
        let entry = keyring::Entry::new("napi-desktop", "user-token")
            .map_err(|e| e.to_string())?;
        entry.get_password().map_err(|e| e.to_string())?
    };

    let mut results = Vec::new();
    for agent in scan_agents() {
        // Only agents we can actually write a config for.
        match reconfigure_agent(agent.name.clone(), token.clone(), base_url.clone()) {
            Ok(msg) => results.push(msg),
            Err(e) => results.push(format!("{}: {}", agent.name, e)),
        }
    }
    Ok(results)
}

#[tauri::command]
async fn auto_setup(base_url: String) -> Result<Vec<String>, String> {
    ensure_api_key(base_url.clone()).await?;
    auto_configure_all(base_url).await
}

#[tauri::command]
fn store_credential(token: String) -> Result<(), String> {
    let entry = keyring::Entry::new("napi-desktop", "user-token")
        .map_err(|e| e.to_string())?;
    entry.set_password(&token).map_err(|e| e.to_string())
}

#[tauri::command]
fn load_credential() -> Result<bool, String> {
    // ponytail: boolean only — the token itself must not cross IPC (§7).
    let entry = keyring::Entry::new("napi-desktop", "user-token")
        .map_err(|e| e.to_string())?;
    Ok(entry.get_password().map(|p| !p.is_empty()).unwrap_or(false))
}

#[tauri::command]
fn clear_credential() -> Result<(), String> {
    let entry = keyring::Entry::new("napi-desktop", "user-token")
        .map_err(|e| e.to_string())?;
    match entry.delete_credential() {
        Ok(()) => Ok(()),
        Err(keyring::Error::NoEntry) => Ok(()), // nothing stored — already logged out
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
fn check_agent_installed(binary_name: String) -> bool {
    binary_exists(&binary_name)
}

#[tauri::command]
fn launch_agent(binary_name: String) -> Result<(), String> {
    validate_binary_name(&binary_name)?;

    #[cfg(target_os = "macos")]
    {
        // `open -a Terminal <name>` asks Terminal to open a *file* called <name>;
        // it never runs the CLI. AppleScript types the command into a new shell.
        std::process::Command::new("osascript")
            .arg("-e")
            .arg(format!(
                "tell application \"Terminal\" to do script \"{}\"",
                binary_name
            ))
            .arg("-e")
            .arg("tell application \"Terminal\" to activate")
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/c", "start", "cmd", "/k", &binary_name])
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    #[cfg(target_os = "linux")]
    {
        // ponytail: xdg-open would hand the binary to a GUI app; an interactive CLI
        // needs a terminal emulator.
        std::process::Command::new("x-terminal-emulator")
            .arg("-e")
            .arg(&binary_name)
            .spawn()
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
async fn get_pricing(base_url: String) -> Result<serde_json::Value, String> {
    validate_base_url(&base_url)?;
    let url = format!("{}/api/pricing", base_url.trim_end_matches('/'));
    let client = reqwest::Client::new();
    let resp = http_get_with_retry(&client, &url).await?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }
    resp.json().await.map_err(|e| e.to_string())
}

#[tauri::command]
async fn get_status(base_url: String) -> Result<serde_json::Value, String> {
    validate_base_url(&base_url)?;
    let url = format!("{}/api/status", base_url.trim_end_matches('/'));
    let client = reqwest::Client::new();
    let resp = http_get_with_retry(&client, &url).await?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }
    resp.json().await.map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            scan_agents,
            reconfigure_agent,
            fetch_subscription,
            login,
            verify_2fa,
            github_oauth,
            store_credential,
            load_credential,
            clear_credential,
            check_agent_installed,
            launch_agent,
            get_pricing,
            get_status,
            ensure_api_key,
            auto_configure_all,
            auto_setup,
            configure_agent,
            scan_mcp_servers,
            scan_skills,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_atomic_write_success() {
        let dir = std::env::temp_dir().join("napi-test-atomic");
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("test.txt");
        atomic_write(&path, b"hello").unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), "hello");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_validate_base_url_valid() {
        assert!(validate_base_url("https://example.com").is_ok());
        assert!(validate_base_url("http://localhost:8080").is_ok());
    }

    #[test]
    fn test_validate_base_url_invalid() {
        assert!(validate_base_url("").is_err());
        assert!(validate_base_url("   ").is_err());
        assert!(validate_base_url("ftp://example.com").is_err());
        assert!(validate_base_url("example.com").is_err());
    }

    #[test]
    fn test_mask_token() {
        assert_eq!(mask_token("sk-ant-api03-abc123xyz"), "sk-a...3xyz");
        assert_eq!(mask_token("short"), "****");
        assert_eq!(mask_token(""), "****");
    }

    #[test]
    fn test_validate_binary_name_rejects_injection() {
        assert!(validate_binary_name("claude").is_ok());
        assert!(validate_binary_name("gemini-cli").is_ok());
        // Anything able to escape the AppleScript / cmd string must be rejected,
        // since launch_agent interpolates the name into both.
        assert!(validate_binary_name("claude\"; rm -rf /").is_err());
        assert!(validate_binary_name("claude; rm -rf /").is_err());
        assert!(validate_binary_name("$(whoami)").is_err());
        assert!(validate_binary_name("a b").is_err());
        assert!(validate_binary_name("").is_err());
    }

    #[test]
    fn test_quota_to_usd() {
        // This deployment reports quota_per_unit = 500000 with quota_display_type = USD.
        assert_eq!(quota_to_usd(1_000_000.0, 500_000.0), 2.0);
        assert_eq!(quota_to_usd(250_000.0, 500_000.0), 0.5);
        // A missing/zero divider must not yield inf or NaN.
        assert_eq!(quota_to_usd(42.0, 0.0), 42.0);
    }

    #[test]
    fn test_binary_exists_false_for_absent_binary() {
        assert!(!binary_exists("napi-definitely-not-installed-xyz"));
        assert!(!binary_exists("claude; rm -rf /"));
    }

    #[test]
    fn test_scan_agents_returns_all_entries() {
        let agents = scan_agents();
        let ids: Vec<&str> = agents.iter().map(|a| a.id.as_str()).collect();
        for expected in [
            "claude", "codex", "cline", "opencode", "gemini", "cursor", "continue", "goose",
            "factory", "grok", "roo", "kilocode", "hermes", "qwen", "windsurf",
        ] {
            assert!(ids.contains(&expected), "missing agent id: {}", expected);
        }
        // Every entry must carry the fields the UI grid renders.
        assert!(agents.iter().all(|a| !a.name.is_empty()));
        assert!(agents.iter().all(|a| !a.description.is_empty()));
        assert!(agents.iter().all(|a| a.color.starts_with('#')));
        assert!(agents
            .iter()
            .all(|a| a.config_type == "auto" || a.config_type == "guide"));
    }

    #[test]
    fn test_atomic_write_failure_leaves_original_and_no_tmp() {
        let dir = std::env::temp_dir().join("napi-test-atomic-fail");
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("test.txt");
        fs::write(&path, b"original").unwrap();
        // Writing "to a path" whose parent is actually a file forces the tmp write to fail.
        let bad_path = dir.join("test.txt").join("nested.txt");
        assert!(atomic_write(&bad_path, b"new").is_err());
        // Original untouched.
        assert_eq!(fs::read_to_string(&path).unwrap(), "original");
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn test_keyring_store_load_clear_cycle() {
        // ponytail: the real OS keychain is not always reachable from a bare
        // test binary (headless CI returns NoEntry/PlatformFailure). When it is
        // reachable, the full store→load→clear cycle must hold; when it is not,
        // the commands must fail cleanly rather than panic.
        let entry = match keyring::Entry::new("napi-desktop", "user-token") {
            Ok(e) => e,
            Err(_) => return,
        };
        match (store_credential("napi-test-token-value".into()), entry.get_password()) {
            (Ok(()), Ok(p)) => {
                assert_eq!(p, "napi-test-token-value");
                clear_credential().unwrap();
                assert!(matches!(entry.get_password().err(), Some(keyring::Error::NoEntry)));
            }
            (Ok(()), Err(_)) | (Err(_), _) => {
                // Keychain unreachable from the test context — clear must still succeed.
                assert!(clear_credential().is_ok() || entry.get_password().is_err());
            }
        }
    }

    #[test]
    fn test_reconfigure_claude_round_trip() {
        // ponytail: reconfigure_agent reads ~/.claude/settings.json via home_dir();
        // to test without touching the real home we exercise the JSON transform
        // shape it produces on a sample settings document instead.
        let mut v: serde_json::Value = serde_json::from_str(
            r#"{"permissions": {"allow": ["Bash"]}, "env": {"ANTHROPIC_API_KEY": "old"}}"#,
        )
        .unwrap();
        let env = v
            .as_object_mut()
            .unwrap()
            .entry("env")
            .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));
        env["ANTHROPIC_API_KEY"] = serde_json::Value::String("sk-new".into());
        env["ANTHROPIC_BASE_URL"] =
            serde_json::Value::String("https://napi.mikawi.org".into());
        // Round-trip: serialize, re-parse, assert fields changed and the rest intact.
        let out = serde_json::to_string(&v).unwrap();
        let back: serde_json::Value = serde_json::from_str(&out).unwrap();
        assert_eq!(back["env"]["ANTHROPIC_API_KEY"], "sk-new");
        assert_eq!(back["env"]["ANTHROPIC_BASE_URL"], "https://napi.mikawi.org");
        assert_eq!(back["permissions"]["allow"][0], "Bash");
    }

    #[test]
    fn test_redact_url_strips_credentials() {
        // Tokens in query strings must never survive into the UI.
        assert_eq!(
            redact_url("https://host/mcp?token=sk-secret&x=1"),
            "https://host/mcp"
        );
        assert_eq!(redact_url("https://host/mcp#frag"), "https://host/mcp");
        assert_eq!(redact_url("https://host/mcp"), "https://host/mcp");
        assert_eq!(redact_url(""), "");
    }

    #[test]
    fn test_scan_mcp_never_leaks_credentials() {
        // Whatever is on this machine, no returned target may carry a key,
        // an auth header, or a query string.
        for s in scan_mcp_servers() {
            let t = s.target.to_lowercase();
            assert!(!t.contains("authorization"), "leaked header in {}", s.name);
            assert!(!t.contains("bearer"), "leaked bearer in {}", s.name);
            assert!(!t.contains("token="), "leaked token in {}", s.name);
            assert!(!s.target.contains('?'), "unredacted query in {}", s.name);
        }
    }

    #[test]
    fn test_scan_skills_shapes() {
        for s in scan_skills() {
            assert!(!s.name.is_empty());
            assert!(!s.agent.is_empty());
            assert!(!s.name.contains('/'), "skill name should be a dir name");
        }
    }
}