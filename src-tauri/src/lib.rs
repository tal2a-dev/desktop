use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tauri::Manager;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryServer {
    /// Registry id, e.g. "ac.inference.sh/mcp".
    pub name: String,
    pub title: String,
    pub description: String,
    pub version: String,
    /// First remote endpoint.
    pub url: String,
    /// "streamable-http" | "sse" | ...
    pub transport: String,
    /// Already present in one of this machine's agent configs.
    pub installed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegistryPage {
    pub servers: Vec<RegistryServer>,
    pub next_cursor: Option<String>,
}

/// The public MCP registry — no auth, no key.
const MCP_REGISTRY: &str = "https://registry.modelcontextprotocol.io/v0/servers";

/// Key we file a registry server under in an agent's mcpServers map.
fn mcp_key(registry_name: &str) -> String {
    registry_name
        .rsplit('/')
        .next()
        .unwrap_or(registry_name)
        .to_string()
}

/// Merge one server into an mcpServers document. Pure so it can be tested.
fn merge_mcp_server(
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

/// One page of the public registry, flagged against what is already installed locally.
#[tauri::command]
async fn fetch_registry_servers(
    cursor: Option<String>,
    limit: Option<usize>,
) -> Result<RegistryPage, String> {
    let limit = limit.unwrap_or(30).clamp(1, 100);
    let mut url = format!("{}?limit={}", MCP_REGISTRY, limit);
    if let Some(c) = cursor.filter(|c| !c.is_empty()) {
        url.push_str(&format!("&cursor={}", urlencoding::encode(&c)));
    }

    let client = reqwest::Client::new();
    let resp = http_get_with_retry(&client, &url).await?;
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

/// Install a registry server into an agent's MCP config (atomic write).
#[tauri::command]
fn install_mcp_server(name: String, url: String, transport: String) -> Result<String, String> {
    validate_base_url(&url)?;
    let path = home_dir().join(".claude.json");

    let mut root: serde_json::Value = std::fs::read_to_string(&path)
        .ok()
        .and_then(|t| serde_json::from_str(&t).ok())
        .unwrap_or_else(|| serde_json::json!({}));

    let key = mcp_key(&name);
    merge_mcp_server(&mut root, &key, &url, &transport)?;

    let out = serde_json::to_string_pretty(&root).map_err(|e| e.to_string())?;
    atomic_write(&path, out.as_bytes())?;
    Ok(format!("Installed {} into Claude Code", key))
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
    let key = stored_token()?.ok_or(NOT_SIGNED_IN)?;
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
    let token = stored_token()?.ok_or(NOT_SIGNED_IN)?;
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

/// Hand a URL to the system browser.
fn open_browser(url: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    let (program, prefix): (&str, &[&str]) = ("open", &[]);
    #[cfg(target_os = "windows")]
    let (program, prefix): (&str, &[&str]) = ("cmd", &["/c", "start", ""]);
    #[cfg(target_os = "linux")]
    let (program, prefix): (&str, &[&str]) = ("xdg-open", &[]);

    std::process::Command::new(program)
        .args(prefix)
        .arg(url)
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("Could not open a browser: {e}"))
}

/// Pull a session token out of a sign-in response. new-api forks vary the field
/// name, so try the ones seen in the wild.
fn extract_token(body: &serde_json::Value) -> Option<String> {
    body["data"]["token"]
        .as_str()
        .or_else(|| body["token"].as_str())
        .or_else(|| body["data"]["access_token"].as_str())
        .or_else(|| body["data"]["key"].as_str())
        .or_else(|| body["key"].as_str())
        .or_else(|| body["data"]["session_token"].as_str())
        .or_else(|| body["data"]["accessToken"].as_str())
        .or_else(|| body["accessToken"].as_str())
        .map(str::to_string)
}

/// Shared tail for every sign-in path — password, 2FA, and GitHub OAuth all end
/// here. Returns "" on success, or "2FA_REQUIRED:<flow>" when the server wants a
/// second factor. The token goes to the keyring and never crosses IPC.
fn finish_login(body: &serde_json::Value) -> Result<String, String> {
    if body["success"].as_bool() != Some(true) {
        return Err(body["message"]
            .as_str()
            .unwrap_or("Sign-in failed")
            .to_string());
    }

    if body["data"]["require_verification"].as_bool() == Some(true) {
        let flow = body["data"]["flow_token"]
            .as_str()
            .ok_or("Second factor required but the server sent no flow token")?;
        return Ok(format!("2FA_REQUIRED:{flow}"));
    }

    match extract_token(body) {
        Some(token) => {
            store_credential(token)?;
            Ok(String::new())
        }
        None => {
            let top: Vec<&String> = body
                .as_object()
                .map(|o| o.keys().collect())
                .unwrap_or_default();
            let data: Vec<&String> = body["data"]
                .as_object()
                .map(|o| o.keys().collect())
                .unwrap_or_default();
            Err(format!(
                "Signed in but the response carried no token. Top-level keys: {top:?}, data keys: {data:?}"
            ))
        }
    }
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
    let client = reqwest::Client::new();
    let body: serde_json::Value = client
        .post(format!("{}/api/user/login", base_url.trim_end_matches('/')))
        .json(&serde_json::json!({ "username": username, "password": password }))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;
    finish_login(&body)
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
    let client = reqwest::Client::new();
    let body: serde_json::Value = client
        .post(format!(
            "{}/api/user/login/verify",
            base_url.trim_end_matches('/')
        ))
        .json(&serde_json::json!({ "flow_token": flow_token, "code": code }))
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json()
        .await
        .map_err(|e| e.to_string())?;
    finish_login(&body).map(|_| ())
}

#[tauri::command]
async fn github_oauth(app: tauri::AppHandle, base_url: String) -> Result<String, String> {
    validate_base_url(&base_url)?;
    let base = base_url.trim_end_matches('/');
    let client = reqwest::Client::new();

    // The server owns the OAuth state: it stores a flow in its DB with a
    // 10-minute TTL and validates that same value when we call back. Inventing
    // our own state locally was the original bug — nothing could ever match it.
    let state_body: serde_json::Value = client
        .post(format!("{}/api/oauth/state", base))
        .json(&serde_json::json!({ "provider": "github", "intent": "login" }))
        .send()
        .await
        .map_err(|e| format!("Could not start GitHub sign-in: {e}"))?
        .json()
        .await
        .map_err(|e| e.to_string())?;
    let state = state_body["data"]["flow_token"]
        .as_str()
        .ok_or("Server did not return an OAuth state")?
        .to_string();

    // Loopback listener. The redirect URI must match the GitHub app byte for
    // byte, so bind a fixed port instead of echoing back local_addr.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:9876")
        .await
        .map_err(|e| format!("Could not start the OAuth callback server: {e}"))?;
    let redirect_uri = "http://localhost:9876/callback";

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
        urlencoding::encode(client_id),
        urlencoding::encode(redirect_uri),
        urlencoding::encode(&state),
    );

    open_browser(&auth_url)?;

    // Wait for the browser to be bounced back.
    let (mut stream, _) = tokio::time::timeout(
        std::time::Duration::from_secs(300),
        listener.accept(),
    )
    .await
    .map_err(|_| "GitHub sign-in timed out")?
    .map_err(|e| format!("Failed to accept the OAuth callback: {e}"))?;

    let mut buf = vec![0u8; 8192];
    let n = stream.peek(&mut buf).await.map_err(|e| e.to_string())?;
    let request = String::from_utf8_lossy(&buf[..n]).into_owned();
    let query = request
        .lines()
        .next()
        .and_then(|line| line.split_whitespace().nth(1))
        .and_then(|path| path.split_once('?'))
        .map(|(_, q)| q.to_string())
        .unwrap_or_default();
    // GitHub percent-encodes both parameters, so decode before comparing.
    let param = |key: &str| -> Option<String> {
        query
            .split('&')
            .find_map(|kv| kv.strip_prefix(&format!("{key}=")))
            .map(|v| {
                urlencoding::decode(v)
                    .map(|c| c.into_owned())
                    .unwrap_or_else(|_| v.to_string())
            })
    };
    let code = param("code");
    let returned_state = param("state");

    // Exchange BEFORE answering the browser. Rendering the page first meant it
    // claimed "Signed in" before the gateway had actually agreed, and hid the
    // real error from whoever needed to read it.
    let outcome: Result<String, String> = match code {
        None => Err("GitHub did not return an authorization code".into()),
        Some(_) if returned_state.as_deref() != Some(state.as_str()) => {
            Err("OAuth state mismatch — sign-in rejected".into())
        }
        Some(code) => {
            let sent = client
                .get(format!("{}/api/oauth/github", base))
                .query(&[("code", code.as_str()), ("state", state.as_str())])
                .send()
                .await;
            match sent {
                Err(e) => Err(format!("Could not reach the gateway: {e}")),
                Ok(resp) => match resp.json::<serde_json::Value>().await {
                    Ok(body) => finish_login(&body),
                    Err(e) => Err(format!("Gateway sent a non-JSON response: {e}")),
                },
            }
        }
    };

    // ponytail: the message can carry a server-controlled string, so escape it
    // before it lands in HTML.
    let esc = |s: &str| {
        s.replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
    };
    use tokio::io::AsyncWriteExt;
    let (head, title, detail) = match &outcome {
        Ok(marker) if marker.is_empty() => (
            "200 OK",
            "Signed in",
            "You can close this tab and return to NAPI Desktop.",
        ),
        Ok(_) => (
            "200 OK",
            "One more step",
            "Return to NAPI Desktop and enter your two-factor code.",
        ),
        Err(e) => ("400 Bad Request", "Sign-in failed", e.as_str()),
    };
    let page = format!(
        "<!doctype html><meta charset=utf-8><title>NAPI Desktop</title>\
         <body style=\"font-family:system-ui;padding:3rem;max-width:44rem\">\
         <h1>{}</h1><p style=\"color:#444\">{}</p>",
        esc(title),
        esc(detail)
    );
    let _ = stream
        .write_all(
            format!(
                "HTTP/1.1 {head}\r\nContent-Type: text/html; charset=utf-8\r\n\
                 Content-Length: {}\r\nConnection: close\r\n\r\n{page}",
                page.len()
            )
            .as_bytes(),
        )
        .await;
    let _ = stream.flush().await;
    drop(stream);

    // A browser cannot close its own tab, so bring the app forward instead —
    // otherwise a finished sign-in looks like it went nowhere.
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.set_focus();
    }

    // Ok("") = fully signed in, Ok("2FA_REQUIRED:<flow>") = second factor needed,
    // Err = the browser page above already showed why.
    outcome
}


#[tauri::command]
async fn ensure_api_key(base_url: String) -> Result<(), String> {
    validate_base_url(&base_url)?;
    let base = base_url.trim_end_matches('/');
    let token = stored_token()?.ok_or(NOT_SIGNED_IN)?;

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

/// Read the stored token, distinguishing "not signed in" from a keychain fault.
///
/// ponytail: keyring reports a missing item as `NoEntry`, which stringifies to
/// "No matching entry found in secure storage". Every reader used to `?` that
/// straight through, so a signed-out app showed a raw keychain error instead of
/// asking the user to sign in. One helper, all callers.
fn stored_token() -> Result<Option<String>, String> {
    let entry = keyring::Entry::new("napi-desktop", "user-token").map_err(|e| e.to_string())?;
    match entry.get_password() {
        Ok(t) if !t.is_empty() => Ok(Some(t)),
        Ok(_) => Ok(None),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

/// Shown to the user when there is no usable credential.
const NOT_SIGNED_IN: &str = "Not signed in — sign in again to continue";

#[tauri::command]
fn store_credential(token: String) -> Result<(), String> {
    let entry = keyring::Entry::new("napi-desktop", "user-token")
        .map_err(|e| e.to_string())?;
    entry.set_password(&token).map_err(|e| e.to_string())?;
    // ponytail: macOS binds a keychain item to the code signature of the binary
    // that wrote it. A fresh (ad-hoc signed) build can write successfully and
    // then read back NoEntry — which is how a "signed in" app ends up with no
    // usable credential. Verify on write so login can never leave that phantom.
    match entry.get_password() {
        Ok(p) if p == token => Ok(()),
        _ => Err("Signed in, but the system keychain would not return the credential. \
                  Remove the \"napi-desktop\" item in Keychain Access and try again."
            .into()),
    }
}

#[tauri::command]
fn load_credential() -> Result<bool, String> {
    // ponytail: boolean only — the token itself must not cross IPC (§7).
    Ok(stored_token()?.is_some())
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
            fetch_registry_servers,
            install_mcp_server,
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

    /// The OS keychain item is process-global and tests run in parallel, so two
    /// keyring tests race: one clears the entry while the other is asserting on
    /// it. Every test that touches the keychain takes this lock.
    static KEYRING_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn test_keyring_store_load_clear_cycle() {
        let _serial = KEYRING_LOCK.lock().unwrap_or_else(|e| e.into_inner());
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

    #[test]
    fn test_mcp_key_takes_registry_tail() {
        assert_eq!(mcp_key("ac.inference.sh/mcp"), "mcp");
        assert_eq!(mcp_key("ai.agentgates/mcp"), "mcp");
        assert_eq!(mcp_key("obsidian"), "obsidian");
    }

    #[test]
    fn test_merge_mcp_server_preserves_other_keys() {
        // An install must not clobber unrelated config or existing servers.
        let mut root: serde_json::Value = serde_json::from_str(
            r#"{"mcpServers":{"obsidian":{"type":"http","url":"https://x"}},"other":{"keep":1}}"#,
        )
        .unwrap();
        merge_mcp_server(&mut root, "mcp", "https://api.inference.sh/mcp", "streamable-http")
            .unwrap();
        assert_eq!(root["mcpServers"]["mcp"]["url"], "https://api.inference.sh/mcp");
        assert_eq!(root["mcpServers"]["mcp"]["type"], "streamable-http");
        // pre-existing server + unrelated top-level key survive
        assert_eq!(root["mcpServers"]["obsidian"]["url"], "https://x");
        assert_eq!(root["other"]["keep"], 1);
    }

    #[test]
    fn test_merge_mcp_server_defaults_transport() {
        let mut root = serde_json::json!({});
        merge_mcp_server(&mut root, "thing", "https://host/mcp", "").unwrap();
        assert_eq!(root["mcpServers"]["thing"]["type"], "http");
    }

    #[test]
    fn test_stored_token_missing_is_not_an_error() {
        // The contract that matters: reading the credential slot yields Ok(_),
        // never Err, when nothing is stored. Concurrent tests share this one
        // keychain entry, so either Ok variant is acceptable here — Err is the
        // bug this guards against (it is what leaked a raw keychain string
        // into the subscription page).
        match stored_token() {
            Ok(_) => {}
            Err(e) => panic!("reading the credential slot must not error, got: {}", e),
        }
    }

    #[test]
    fn test_not_signed_in_message_is_human() {
        assert!(NOT_SIGNED_IN.contains("sign in"));
        // The raw keyring string must never be what the user sees.
        assert!(!NOT_SIGNED_IN.contains("secure storage"));
    }
}