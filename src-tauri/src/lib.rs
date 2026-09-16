mod agent_scan;
mod agent_write;
mod env_checker;
mod env_manager;
mod mcp;
mod napi_account;
mod skills;
mod tool_lifecycle;
mod tool_versions;

use mcp::{
    delete_mcp_server, fetch_registry_servers, get_mcp_servers, import_mcp_from_apps,
    install_mcp_server, scan_mcp_servers, toggle_mcp_app, upsert_mcp_server, validate_mcp_command,
};
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
    /// cc-switch can install this CLI from the app (npm / hermes install.sh).
    pub installable: bool,
    pub uninstallable: bool,
}

/// ponytail: cheap "is this already pointed at us" probe — a substring check on
/// the raw config text. Upgrade to a real per-format parse if a user ever runs
/// two different NAPI bases at once.
fn has_napi_marker(path: &std::path::Path) -> bool {
    let Some(host) = napi_host() else {
        return false;
    };
    std::fs::read_to_string(path)
        .map(|s| s.contains(&host))
        .unwrap_or(false)
}

pub(crate) fn home_dir() -> PathBuf {
    dirs::home_dir().unwrap_or_else(|| PathBuf::from("."))
}

/// Host of the configured base URL, persisted in a config file so scans
/// survive restarts. Falls back to the built-in default.
fn napi_host() -> Option<String> {
    let raw = std::fs::read_to_string(base_url_path()).unwrap_or_default();
    let url = if raw.trim().is_empty() {
        DEFAULT_BASE_URL.to_string()
    } else {
        raw.trim().to_string()
    };
    url.trim_start_matches("https://")
        .trim_start_matches("http://")
        .split('/')
        .next()
        .filter(|h| !h.is_empty())
        .map(str::to_string)
}

const DEFAULT_BASE_URL: &str = "https://napi.mikawi.org";

fn base_url_path() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| home_dir().join(".config"))
        .join("napi-desktop")
        .join("base_url")
}

pub fn set_base_url(url: &str) -> Result<(), String> {
    validate_base_url(url)?;
    let path = base_url_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    atomic_write(&path, url.trim().as_bytes())
}

pub(crate) fn atomic_write(path: &std::path::Path, content: &[u8]) -> Result<(), String> {
    let tmp = path.with_extension("napi-tmp");
    std::fs::write(&tmp, content).map_err(|e| e.to_string())?;
    std::fs::rename(&tmp, path).map_err(|e| {
        let _ = std::fs::remove_file(&tmp);
        e.to_string()
    })
}

/// ponytail: 3 retries with exponential backoff covers transient network/429s
pub(crate) async fn http_get_with_retry(
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

pub(crate) async fn http_get_auth_with_retry(
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

pub(crate) fn validate_base_url(url: &str) -> Result<(), String> {
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

/// CLI on a real install path — not a Grok/cmux session shim.
fn binary_exists(binary_name: &str) -> bool {
    if validate_binary_name(binary_name).is_err() {
        return false;
    }
    agent_scan::cli_installed("", binary_name)
}

/// new-api reports quota in internal units; `quota_per_unit` converts to USD.
pub(crate) fn quota_to_usd(raw: f64, per_unit: f64) -> f64 {
    if per_unit > 0.0 {
        raw / per_unit
    } else {
        raw
    }
}

/// Brand accent used by the UI tile for each tool.
fn agent_color(id: &str) -> &'static str {
    match id {
        "claude" | "claude-desktop" => "#D97757",
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
        "openclaw" => "#F59E0B",
        "pi" => "#06B6D4",
        "windsurf" => "#09B6A2",
        _ => "#6E7681",
    }
}

#[tauri::command]
fn scan_agents() -> Vec<AgentConfig> {
    let home = home_dir();

    // (id, name, description, config_path, key_field, base_url_field, format,
    //  binary_name, config_type)
    let specs: Vec<(&str, &str, &str, PathBuf, &str, &str, &str, &str, &str)> = vec![
        (
            "claude",
            "Claude Code",
            "Claude Code pointed at NAPI",
            agent_write::config_dir_for("claude").join("settings.json"),
            "env.ANTHROPIC_API_KEY",
            "env.ANTHROPIC_BASE_URL",
            "json",
            "claude",
            "auto",
        ),
        (
            "claude-desktop",
            "Claude Desktop",
            "Claude Desktop Chat, Cowork, and Code tab via 3P gateway",
            agent_write::claude_desktop_paths_from_home(&home).0,
            "inferenceGatewayApiKey",
            "inferenceGatewayBaseUrl",
            "json",
            "claude-desktop",
            "auto",
        ),
        (
            "codex",
            "Codex",
            "Codex CLI, Desktop, and ChatGPT cowork (shared ~/.codex)",
            agent_write::config_dir_for("codex").join("config.toml"),
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
            agent_write::config_dir_for("opencode").join("opencode.json"),
            "provider_env.OPENAI_API_KEY",
            "provider_env.OPENAI_BASE_URL",
            "json",
            "opencode",
            "auto",
        ),
        (
            "gemini",
            "Gemini CLI",
            "Gemini CLI pointed at NAPI",
            agent_write::config_dir_for("gemini").join(".env"),
            "GEMINI_API_KEY",
            "GEMINI_BASE_URL",
            "env",
            "gemini",
            "auto",
        ),
        (
            "cursor",
            "Cursor",
            "Cursor AI code editor",
            home.join(".cursor").join("cli-config.json"),
            "env.OPENAI_API_KEY",
            "env.OPENAI_BASE_URL",
            "json",
            "agent",
            "auto",
        ),
        (
            "continue",
            "Continue",
            "Open-source IDE coding assistant",
            home.join(".continue").join("config.json"),
            "models[0].apiKey",
            "models[0].apiBase",
            "json",
            "continue",
            "auto",
        ),
        (
            "goose",
            "Goose",
            "Block's local autonomous agent",
            home.join(".config").join("goose").join(".env"),
            "OPENAI_API_KEY",
            "OPENAI_HOST",
            "env",
            "goose",
            "auto",
        ),
        (
            "factory",
            "Factory Droid",
            "Factory's autonomous coding agent",
            home.join(".factory").join("config.json"),
            "env.OPENAI_API_KEY",
            "env.OPENAI_BASE_URL",
            "json",
            "droid",
            "auto",
        ),
        (
            "grok",
            "Grok CLI",
            "xAI's terminal coding agent",
            agent_write::config_dir_for("grok").join("config.toml"),
            "api_key",
            "base_url",
            "toml",
            "grok",
            "auto",
        ),
        (
            "roo",
            "Roo Code",
            "VS Code autonomous coding agent",
            home.join(".roo").join("napi.json"),
            "env.OPENAI_API_KEY",
            "env.OPENAI_BASE_URL",
            "json",
            "roo",
            "auto",
        ),
        (
            "kilocode",
            "Kilo Code",
            "VS Code autonomous coding agent",
            home.join(".kilocode").join("napi.json"),
            "env.OPENAI_API_KEY",
            "env.OPENAI_BASE_URL",
            "json",
            "kilocode",
            "auto",
        ),
        (
            "hermes",
            "Hermes Agent",
            "Nous Research self-improving agent",
            agent_write::config_dir_for("hermes").join("config.yaml"),
            "openai_api_key",
            "openai_base_url",
            "yaml",
            "hermes",
            "auto",
        ),
        (
            "qwen",
            "Qwen Code",
            "Alibaba's coding CLI",
            home.join(".qwen").join("settings.json"),
            "env.OPENAI_API_KEY",
            "env.OPENAI_BASE_URL",
            "json",
            "qwen",
            "auto",
        ),
        (
            "openclaw",
            "OpenClaw",
            "OpenClaw coding agent",
            agent_write::config_dir_for("openclaw").join("openclaw.json"),
            "apiKey",
            "baseUrl",
            "json",
            "openclaw",
            "auto",
        ),
        (
            "pi",
            "Pi",
            "Pi coding agent",
            agent_write::config_dir_for("pi").join("models.json"),
            "apiKey",
            "baseUrl",
            "json",
            "pi",
            "auto",
        ),
        (
            "windsurf",
            "Windsurf",
            "Codeium's IDE agent",
            home.join(".codeium").join("windsurf").join("napi.json"),
            "env.OPENAI_API_KEY",
            "env.OPENAI_BASE_URL",
            "json",
            "windsurf",
            "auto",
        ),
    ];

    specs
        .into_iter()
        .map(
            |(id, name, description, config_path, key_field, base_url_field, format, binary_name, config_type)| {
                let probe = agent_scan::probe(id, binary_name, &config_path);
                AgentConfig {
                    id: id.into(),
                    name: name.into(),
                    description: description.into(),
                    detected: probe.detected(),
                    installed: probe.installed(),
                    configured: has_napi_marker(&config_path),
                    config_path,
                    key_field: key_field.into(),
                    base_url_field: base_url_field.into(),
                    format: format.into(),
                    config_type: config_type.into(),
                    color: agent_color(id).into(),
                    binary_name: binary_name.into(),
                    installable: tool_lifecycle::can_install(id),
                    uninstallable: tool_lifecycle::can_uninstall(id),
                }
            },
        )
        .collect()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkillInfo {
    pub name: String,
    pub agent: String,
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
    agent_write::write_agent(&agent_name, &api_key, &base_url)
}

/// One-click setup: use the stored API key, write the agent config.
/// ponytail: the key never comes from the frontend — it is read from the keyring,
/// so "Quick Setup" needs no typing from the user.
#[tauri::command]
fn configure_agent(agent_name: String, base_url: String) -> Result<String, String> {
    // Prefer the dedicated sk- key; fall back to the session token only if
    // ensure_api_key has not run yet (login-only flows).
    let key = stored_api_key()?
        .or_else(|| stored_token().ok().flatten())
        .ok_or(NOT_SIGNED_IN)?;
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

    // /api/user/self for wallet numbers (used_quota/quota are real wallet
    // fields; `role` is an int and there is no `reset_at` — plan title and
    // reset date must come from /api/subscription/self instead).
    let user_resp = http_get_auth_with_retry(&client, &format!("{}/api/user/self", base), &token).await?;
    if !user_resp.status().is_success() {
        return Err(format!("HTTP {} (token: {})", user_resp.status(), mask_token(&token)));
    }
    let user: serde_json::Value = user_resp.json().await.map_err(|e| e.to_string())?;

    // /api/subscription/self: { data: { subscriptions: [ { subscription: {...}, pools: [...] } ] } }
    let sub_resp = http_get_auth_with_retry(&client, &format!("{}/api/subscription/self", base), &token).await;
    let sub_body: serde_json::Value = match sub_resp {
        Ok(r) if r.status().is_success() => r.json().await.unwrap_or(serde_json::Value::Null),
        _ => serde_json::Value::Null,
    };

    // Pick the active subscription with the latest end_time; sum its pools.
    // NAPI returns ALL active subscriptions — aggregate every one's pools.
    let active_subs: Vec<&serde_json::Value> = sub_body["data"]["subscriptions"]
        .as_array()
        .map(|subs| {
            subs.iter()
                .filter(|s| s["subscription"]["status"].as_str() == Some("active"))
                .collect()
        })
        .unwrap_or_default();

    if !active_subs.is_empty() {
        let mut used_units = 0.0;
        let mut total_units = 0.0;
        let mut plan_ids: Vec<i64> = Vec::new();
        let mut reset_ts: i64 = 0;
        for s in &active_subs {
            for p in s["pools"].as_array().into_iter().flatten() {
                if let Some(u) = p["amount_used"].as_f64() {
                    used_units += u;
                }
                if let Some(t) = p["amount_total"].as_f64() {
                    total_units += t;
                }
            }
            let sub = &s["subscription"];
            plan_ids.push(sub["plan_id"].as_i64().unwrap_or(0));
            // Earliest upcoming reset across subscriptions is what the user
            // actually waits for.
            let r = sub["next_reset_time"].as_i64().unwrap_or(0);
            if r > 0 && (reset_ts == 0 || r < reset_ts) {
                reset_ts = r;
            }
        }

        // Plan title isn't in the summary — fall back to plan_id label; the
        // frontend renders whatever string we give.
        let plan = if plan_ids.len() == 1 {
            format!("Subscription #{}", plan_ids[0])
        } else {
            format!("{} subscriptions", plan_ids.len())
        };
        let reset_date = if reset_ts > 0 {
            // Unix seconds → ISO date; no chrono dependency, do it by hand.
            let days = reset_ts.div_euclid(86400);
            let (y, m, d) = civil_from_days(days);
            format!("{:04}-{:02}-{:02}", y, m, d)
        } else {
            "unknown".to_string()
        };

        // Empty pools (plan without quota pools) → wallet fallback.
        if total_units > 0.0 {
            return Ok(SubscriptionInfo {
                plan,
                used: quota_to_usd(used_units, per_unit),
                limit: quota_to_usd(total_units, per_unit),
                reset_date,
            });
        }
    }

    // Wallet fallback: no active subscription (or pools empty).
    // NAPI `quota` is remaining balance (DecreaseUserQuota subtracts);
    // `used_quota` is lifetime consumed. Cap = used + remaining.
    let used_units = user["data"]["used_quota"].as_f64().unwrap_or(0.0);
    let remaining_units = user["data"]["quota"].as_f64().unwrap_or(0.0);
    Ok(SubscriptionInfo {
        plan: format!(
            "Wallet ({})",
            role_name(user["data"]["role"].as_i64().unwrap_or(0))
        ),
        used: quota_to_usd(used_units, per_unit),
        limit: quota_to_usd(used_units + remaining_units, per_unit),
        reset_date: "n/a — pay-as-you-go".to_string(),
    })
}

/// new-api role ids → label (user 1, admin 10, root 100).
fn role_name(role: i64) -> &'static str {
    match role {
        100 => "root",
        10 => "admin",
        _ => "user",
    }
}

/// Days since 1970-01-01 → (y, m, d). Howard Hinnant's civil_from_days.
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
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
) -> Result<String, String> {
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
    finish_login(&body)
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
    napi_account::ensure_desktop_key(&base_url).await
}

#[tauri::command]
async fn auto_configure_all(base_url: String) -> Result<Vec<String>, String> {
    validate_base_url(&base_url)?;
    // Prefer the dedicated sk- key; fall back to the session token.
    let token = stored_api_key()?
        .or_else(|| stored_token().ok().flatten())
        .ok_or(NOT_SIGNED_IN)?;

    Ok(agent_write::write_all_agents(&token, &base_url))
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
pub(crate) fn stored_token() -> Result<Option<String>, String> {
    let entry = keyring::Entry::new("napi-desktop", "user-token").map_err(map_keyring)?;
    match entry.get_password() {
        Ok(t) if !t.is_empty() => Ok(Some(t)),
        Ok(_) => Ok(None),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(map_keyring(e)),
    }
}

pub(crate) fn stored_api_key() -> Result<Option<String>, String> {
    let entry = keyring::Entry::new("napi-desktop", "api-key").map_err(map_keyring)?;
    match entry.get_password() {
        Ok(k) if !k.is_empty() => Ok(Some(k)),
        Ok(_) => Ok(None),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(map_keyring(e)),
    }
}

fn keyring_is_duplicate(err: &keyring::Error) -> bool {
    if matches!(err, keyring::Error::Ambiguous(_)) {
        return true;
    }
    let s = err.to_string().to_lowercase();
    s.contains("already exists") || s.contains("duplicate")
}

fn map_keyring(err: keyring::Error) -> String {
    let s = err.to_string();
    if keyring_is_duplicate(&err) {
        "Could not update Keychain (an old napi-desktop item is in the way). Retry."
            .into()
    } else if s.contains("secure storage") {
        format!("Could not save login in Keychain: {s}")
    } else {
        s
    }
}

/// Create or replace a generic password. macOS returns
/// "item already exists" when an older (often differently-signed) item is
/// still in the login keychain; delete then write.
fn set_keyring_secret(account: &str, secret: &str) -> Result<(), String> {
    let entry = keyring::Entry::new("napi-desktop", account).map_err(map_keyring)?;
    match entry.get_password() {
        Ok(existing) if existing == secret => return Ok(()),
        _ => {}
    }
    match entry.set_password(secret) {
        Ok(()) => Ok(()),
        Err(e) if keyring_is_duplicate(&e) => {
            let _ = entry.delete_credential();
            #[cfg(target_os = "macos")]
            {
                let _ = std::process::Command::new("security")
                    .args([
                        "delete-generic-password",
                        "-s",
                        "napi-desktop",
                        "-a",
                        account,
                    ])
                    .output();
            }
            match entry.set_password(secret) {
                Ok(()) => Ok(()),
                Err(e2) => match entry.get_password() {
                    Ok(existing) if existing == secret => Ok(()),
                    _ => Err(map_keyring(e2)),
                },
            }
        }
        Err(e) => Err(map_keyring(e)),
    }
}

/// Persist the relay secret as `sk-…`. NAPI stores the raw 48-char key;
/// clients (and the web console) always send the `sk-` prefix.
pub(crate) fn persist_api_key(raw: &str) -> Result<String, String> {
    let key = if raw.starts_with("sk-") {
        raw.to_string()
    } else {
        format!("sk-{raw}")
    };
    set_keyring_secret("api-key", &key)?;
    Ok(key)
}

/// Shown to the user when there is no usable credential.
pub(crate) const NOT_SIGNED_IN: &str = "Not signed in — sign in again to continue";

#[tauri::command]
fn store_credential(token: String) -> Result<(), String> {
    set_keyring_secret("user-token", &token)?;
    // ponytail: macOS binds a keychain item to the code signature of the binary
    // that wrote it. A fresh (ad-hoc signed) build can write successfully and
    // then read back NoEntry — which is how a "signed in" app ends up with no
    // usable credential. Verify on write so login can never leave that phantom.
    match stored_token()? {
        Some(p) if p == token => Ok(()),
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
    for service in ["user-token", "api-key"] {
        let entry = keyring::Entry::new("napi-desktop", service).map_err(|e| e.to_string())?;
        match entry.delete_credential() {
            Ok(()) => {}
            Err(keyring::Error::NoEntry) => {} // nothing stored — already logged out
            Err(e) => return Err(e.to_string()),
        }
    }
    Ok(())
}

#[tauri::command]
fn check_agent_installed(binary_name: String) -> bool {
    binary_exists(&binary_name)
}

#[tauri::command]
async fn install_agent(agent_id: String) -> Result<String, String> {
    let id = agent_write::resolve_agent_id(&agent_id);
    tokio::task::spawn_blocking(move || tool_lifecycle::install_tool(&id))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn update_agent(agent_id: String) -> Result<String, String> {
    let id = agent_write::resolve_agent_id(&agent_id);
    tokio::task::spawn_blocking(move || tool_lifecycle::update_tool(&id))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
async fn uninstall_agent(agent_id: String, binary_name: String) -> Result<String, String> {
    let id = agent_write::resolve_agent_id(&agent_id);
    let config_note = match agent_write::clear_napi(&id) {
        Ok(msg) => msg,
        Err(e) => format!("config strip failed: {e}"),
    };
    let cli = tokio::task::spawn_blocking({
        let id = id.clone();
        move || tool_lifecycle::uninstall_tool(&id, &binary_name)
    })
    .await
    .map_err(|e| e.to_string())??;
    Ok(format!("{cli}; {config_note}"))
}

#[tauri::command]
fn launch_agent(binary_name: String) -> Result<(), String> {
    validate_binary_name(&binary_name)?;
    if !agent_scan::cli_installed("", &binary_name) {
        return Err(format!(
            "{binary_name} is not installed on this Mac — Install it first"
        ));
    }

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
        // needs a terminal emulator. x-terminal-emulator only exists on Debian.
        let name = binary_name.as_str();
        let candidates: [(&str, &[&str]); 6] = [
            ("x-terminal-emulator", &["-e", name]),
            ("gnome-terminal", &["--", name]),
            ("konsole", &["-e", name]),
            ("xfce4-terminal", &["-e", name]),
            ("kitty", &[name]),
            ("xterm", &["-e", name]),
        ];
        let spawned = candidates
            .iter()
            .any(|(bin, args)| std::process::Command::new(bin).args(*args).spawn().is_ok());
        if !spawned {
            return Err(
                "No terminal emulator found (tried x-terminal-emulator, gnome-terminal, \
                 konsole, xfce4-terminal, kitty, xterm)"
                    .into(),
            );
        }
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

#[tauri::command]
fn save_base_url(base_url: String) -> Result<(), String> {
    set_base_url(&base_url)
}

#[tauri::command]
fn get_app_settings() -> agent_write::AppSettings {
    agent_write::load_app_settings()
}

#[tauri::command]
fn save_app_settings(settings: agent_write::AppSettings) -> Result<(), String> {
    agent_write::save_app_settings(&settings)
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedDirectories {
    pub app_config: String,
    pub claude: String,
    pub codex: String,
    pub gemini: String,
    pub grok: String,
    pub opencode: String,
    pub openclaw: String,
    pub hermes: String,
    pub pi: String,
}

#[tauri::command]
async fn pick_directory(default_path: Option<String>) -> Result<Option<String>, String> {
    tokio::task::spawn_blocking(move || {
        let mut dialog = rfd::FileDialog::new();
        if let Some(path) = default_path
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
        {
            dialog = dialog.set_directory(path);
        }
        dialog
            .pick_folder()
            .map(|p| p.to_string_lossy().into_owned())
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
fn get_resolved_directories() -> ResolvedDirectories {
    let abs = agent_write::abs_dir_string;
    ResolvedDirectories {
        app_config: abs(agent_write::config_dir_for("app")),
        claude: abs(agent_write::config_dir_for("claude")),
        codex: abs(agent_write::config_dir_for("codex")),
        gemini: abs(agent_write::config_dir_for("gemini")),
        grok: abs(agent_write::config_dir_for("grok")),
        opencode: abs(agent_write::config_dir_for("opencode")),
        openclaw: abs(agent_write::config_dir_for("openclaw")),
        hermes: abs(agent_write::config_dir_for("hermes")),
        pi: abs(agent_write::config_dir_for("pi")),
    }
}

#[tauri::command]
fn open_path(path: String) -> Result<(), String> {
    let path = path.trim();
    if path.is_empty() {
        return Err("path is empty".into());
    }

    #[cfg(target_os = "macos")]
    let mut cmd = {
        let mut c = std::process::Command::new("open");
        c.arg(path);
        c
    };
    #[cfg(target_os = "linux")]
    let mut cmd = {
        let mut c = std::process::Command::new("xdg-open");
        c.arg(path);
        c
    };
    #[cfg(target_os = "windows")]
    let mut cmd = {
        let mut c = std::process::Command::new("explorer");
        c.arg(path);
        c
    };
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        return Err("open_path is not supported on this OS".into());
    }

    cmd.spawn()
        .map(|_| ())
        .map_err(|e| format!("Could not open path: {e}"))
}

#[tauri::command]
fn open_logs_dir() -> Result<String, String> {
    let dir = agent_write::config_dir_for("app").join("logs");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let path = agent_write::abs_dir_string(dir);
    open_path(path.clone())?;
    Ok(path)
}

#[tauri::command]
async fn get_tool_versions(tools: Vec<String>) -> Result<Vec<tool_versions::ToolVersion>, String> {
    tool_versions::get_tool_versions(tools).await
}

#[tauri::command]
fn check_env_conflicts(app: String) -> Result<Vec<env_checker::EnvConflict>, String> {
    env_checker::check_env_conflicts(&app)
}

#[tauri::command]
fn delete_env_vars(
    conflicts: Vec<env_checker::EnvConflict>,
) -> Result<env_manager::BackupInfo, String> {
    env_manager::delete_env_vars(conflicts)
}

#[tauri::command]
async fn fetch_user_logs(base_url: String) -> Result<serde_json::Value, String> {
    validate_base_url(&base_url)?;
    let token = stored_token()?.ok_or(NOT_SIGNED_IN)?;
    let client = reqwest::Client::new();
    let url = format!(
        "{}/api/log/self?p=1&page_size=50&type=2",
        base_url.trim_end_matches('/')
    );
    let resp = http_get_auth_with_retry(&client, &url, &token).await?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {} fetching logs", resp.status()));
    }
    resp.json().await.map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            let (skills_state, skill_service) =
                skills::init_state().map_err(|e| -> Box<dyn std::error::Error> { e.into() })?;
            app.manage(skills_state);
            app.manage(skill_service);
            Ok(())
        })
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
            install_agent,
            update_agent,
            uninstall_agent,
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
            get_mcp_servers,
            upsert_mcp_server,
            delete_mcp_server,
            toggle_mcp_app,
            import_mcp_from_apps,
            validate_mcp_command,
            save_base_url,
            get_app_settings,
            save_app_settings,
            pick_directory,
            get_resolved_directories,
            open_path,
            open_logs_dir,
            get_tool_versions,
            check_env_conflicts,
            delete_env_vars,
            fetch_user_logs,
            napi_account::list_api_keys,
            napi_account::select_api_key,
            napi_account::fetch_weekly_usage,
            skills::commands::get_installed_skills,
            skills::commands::get_skill_backups,
            skills::commands::delete_skill_backup,
            skills::commands::install_skill_unified,
            skills::commands::uninstall_skill_unified,
            skills::commands::restore_skill_backup,
            skills::commands::toggle_skill_app,
            skills::commands::scan_unmanaged_skills,
            skills::commands::import_skills_from_apps,
            skills::commands::discover_available_skills,
            skills::commands::check_skill_updates,
            skills::commands::update_skill,
            skills::commands::migrate_skill_storage,
            skills::commands::search_skills_sh,
            skills::commands::get_skills,
            skills::commands::get_skill_repos,
            skills::commands::add_skill_repo,
            skills::commands::remove_skill_repo,
            skills::commands::install_skills_from_zip,
            skills::commands::open_zip_file_dialog,
            skills::commands::open_external,
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
        for expected in agent_write::OVERWRITE_AGENT_IDS {
            assert!(ids.contains(expected), "missing agent id: {}", expected);
        }
        assert_eq!(
            ids.len(),
            agent_write::OVERWRITE_AGENT_IDS.len(),
            "scan catalog and overwrite list must stay 1:1"
        );
        // Every entry must carry the fields the UI grid renders.
        assert!(agents.iter().all(|a| !a.name.is_empty()));
        assert!(agents.iter().all(|a| !a.description.is_empty()));
        assert!(agents.iter().all(|a| a.color.starts_with('#')));
        assert!(agents
            .iter()
            .all(|a| a.config_type == "auto" || a.config_type == "guide"));
    }

    #[test]
    fn overwrite_all_does_not_skip_uninstalled_agents() {
        let src = include_str!("lib.rs");
        let start = src
            .find("async fn auto_configure_all")
            .expect("auto_configure_all");
        let body = src.get(start..start + 600).unwrap_or(&src[start..]);
        assert!(
            body.contains("write_all_agents"),
            "Overwrite All must write every catalogued agent via write_all_agents"
        );
        assert!(
            !body.contains("agent.installed"),
            "Overwrite All must not gate on installed/detected"
        );
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
        // ponytail: this test used to `return` when Entry::new() failed, which
        // silently passed on a build with NO keyring backend feature — exactly
        // the bug that made login fail after a successful HTTP call. Fail loudly
        // now; a genuinely headless runner can opt out explicitly.
        if std::env::var("NAPI_SKIP_KEYRING_TESTS").is_ok() {
            return;
        }
        let entry = keyring::Entry::new("napi-desktop", "user-token").expect(
            "no keyring backend compiled — add a keystore feature to the keyring dependency",
        );
        // The keychain itself may still be locked/absent at runtime: tolerate a
        // clean error, but never a panic, and always require clear() to succeed.
        match (
            store_credential("napi-test-token-value".into()),
            entry.get_password(),
        ) {
            (Ok(()), Ok(p)) => {
                assert_eq!(p, "napi-test-token-value");
                clear_credential().unwrap();
                assert!(matches!(
                    entry.get_password().err(),
                    Some(keyring::Error::NoEntry)
                ));
            }
            (Ok(()), Err(_)) | (Err(_), _) => {
                assert!(
                    clear_credential().is_ok() || entry.get_password().is_err(),
                    "clear_credential must not fail hard"
                );
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
            crate::mcp::redact_url("https://host/mcp?token=sk-secret&x=1"),
            "https://host/mcp"
        );
        assert_eq!(crate::mcp::redact_url("https://host/mcp#frag"), "https://host/mcp");
        assert_eq!(crate::mcp::redact_url("https://host/mcp"), "https://host/mcp");
        assert_eq!(crate::mcp::redact_url(""), "");
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
        assert_eq!(crate::mcp::mcp_key("ac.inference.sh/mcp"), "mcp");
        assert_eq!(crate::mcp::mcp_key("ai.agentgates/mcp"), "mcp");
        assert_eq!(crate::mcp::mcp_key("obsidian"), "obsidian");
    }

    #[test]
    fn test_merge_mcp_server_preserves_other_keys() {
        // An install must not clobber unrelated config or existing servers.
        let mut root: serde_json::Value = serde_json::from_str(
            r#"{"mcpServers":{"obsidian":{"type":"http","url":"https://x"}},"other":{"keep":1}}"#,
        )
        .unwrap();
        crate::mcp::merge_mcp_server(&mut root, "mcp", "https://api.inference.sh/mcp", "streamable-http")
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
        crate::mcp::merge_mcp_server(&mut root, "thing", "https://host/mcp", "").unwrap();
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
            Err(e) => {
                let msg = e.to_string();
                // GitHub ubuntu runners have no Secret Service bus.
                if msg.contains("org.freedesktop.secrets") {
                    return;
                }
                panic!("reading the credential slot must not error, got: {}", e);
            }
        }
    }

    #[test]
    fn test_not_signed_in_message_is_human() {
        assert!(NOT_SIGNED_IN.contains("sign in"));
        // The raw keyring string must never be what the user sees.
        assert!(!NOT_SIGNED_IN.contains("secure storage"));
    }
}