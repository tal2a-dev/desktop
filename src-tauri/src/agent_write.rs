//! Live-config writers for every coding agent we know, cc-switch-style.
//! Values always come from NAPI (key + base_url), never a provider catalog.

use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

use crate::{atomic_write, home_dir};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
    #[serde(default = "default_true")]
    pub enable_claude_plugin_integration: bool,
    #[serde(default = "default_true")]
    pub skip_claude_onboarding: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claude_config_dir: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub codex_config_dir: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub gemini_config_dir: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grok_config_dir: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub opencode_config_dir: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub openclaw_config_dir: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hermes_config_dir: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub pi_config_dir: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub app_config_dir: Option<String>,
    #[serde(default = "default_true")]
    pub log_enabled: bool,
    #[serde(default = "default_log_level")]
    pub log_level: String,
    /// NAPI `tokens.id` the desktop writes into agent configs.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub selected_token_id: Option<i64>,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            enable_claude_plugin_integration: true,
            skip_claude_onboarding: true,
            claude_config_dir: None,
            codex_config_dir: None,
            gemini_config_dir: None,
            grok_config_dir: None,
            opencode_config_dir: None,
            openclaw_config_dir: None,
            hermes_config_dir: None,
            pi_config_dir: None,
            app_config_dir: None,
            log_enabled: true,
            log_level: default_log_level(),
            selected_token_id: None,
        }
    }
}

fn default_true() -> bool {
    true
}

fn default_log_level() -> String {
    "info".into()
}

pub fn default_app_config_dir() -> PathBuf {
    dirs::config_dir()
        .unwrap_or_else(|| home_dir().join(".config"))
        .join("napi-desktop")
}

fn settings_path() -> PathBuf {
    default_app_config_dir().join("app-settings.json")
}

pub fn load_app_settings() -> AppSettings {
    let Ok(raw) = fs::read_to_string(settings_path()) else {
        return AppSettings::default();
    };
    serde_json::from_str(&raw).unwrap_or_default()
}

fn expand_tilde(raw: &str) -> PathBuf {
    let trimmed = raw.trim();
    if trimmed == "~" {
        return home_dir();
    }
    if let Some(rest) = trimmed.strip_prefix("~/") {
        return home_dir().join(rest);
    }
    if let Some(rest) = trimmed.strip_prefix("~\\") {
        return home_dir().join(rest);
    }
    PathBuf::from(trimmed)
}

fn override_dir(raw: &Option<String>) -> Option<PathBuf> {
    raw.as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(expand_tilde)
}

fn default_dir_for(app: &str) -> PathBuf {
    let home = home_dir();
    match app {
        "claude" => home.join(".claude"),
        "codex" => home.join(".codex"),
        "gemini" => home.join(".gemini"),
        "grok" => home.join(".grok"),
        "opencode" => {
            if cfg!(target_os = "windows") {
                std::env::var("APPDATA")
                    .map(PathBuf::from)
                    .unwrap_or_else(|_| home.join("AppData").join("Roaming"))
                    .join("opencode")
            } else {
                home.join(".config").join("opencode")
            }
        }
        "openclaw" => home.join(".openclaw"),
        "hermes" => home.join(".hermes"),
        "pi" => home.join(".pi").join("agent"),
        "app" => default_app_config_dir(),
        other => home.join(format!(".{other}")),
    }
}

/// Resolved config directory for an agent (or `"app"` for NAPI Desktop).
/// Empty / missing overrides fall back to the platform default. `~` is expanded.
pub fn config_dir_for(app: &str) -> PathBuf {
    let settings = load_app_settings();
    let raw = match app {
        "claude" => &settings.claude_config_dir,
        "codex" => &settings.codex_config_dir,
        "gemini" => &settings.gemini_config_dir,
        "grok" => &settings.grok_config_dir,
        "opencode" => &settings.opencode_config_dir,
        "openclaw" => &settings.openclaw_config_dir,
        "hermes" => &settings.hermes_config_dir,
        "pi" => &settings.pi_config_dir,
        "app" => &settings.app_config_dir,
        _ => return default_dir_for(app),
    };
    override_dir(raw).unwrap_or_else(|| default_dir_for(app))
}

pub fn abs_dir_string(path: PathBuf) -> String {
    if path.is_absolute() {
        return path.to_string_lossy().into_owned();
    }
    std::env::current_dir()
        .map(|cwd| cwd.join(&path))
        .unwrap_or(path)
        .to_string_lossy()
        .into_owned()
}

fn blank_to_none(value: &Option<String>) -> Option<String> {
    value
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

pub fn save_app_settings(settings: &AppSettings) -> Result<(), String> {
    let mut settings = settings.clone();
    settings.claude_config_dir = blank_to_none(&settings.claude_config_dir);
    settings.codex_config_dir = blank_to_none(&settings.codex_config_dir);
    settings.gemini_config_dir = blank_to_none(&settings.gemini_config_dir);
    settings.grok_config_dir = blank_to_none(&settings.grok_config_dir);
    settings.opencode_config_dir = blank_to_none(&settings.opencode_config_dir);
    settings.openclaw_config_dir = blank_to_none(&settings.openclaw_config_dir);
    settings.hermes_config_dir = blank_to_none(&settings.hermes_config_dir);
    settings.pi_config_dir = blank_to_none(&settings.pi_config_dir);
    settings.app_config_dir = blank_to_none(&settings.app_config_dir);
    let level = settings.log_level.trim().to_ascii_lowercase();
    settings.log_level = match level.as_str() {
        "error" | "warn" | "info" | "debug" | "trace" => level,
        _ => "info".into(),
    };
    let path = settings_path();
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let out = serde_json::to_string_pretty(&settings).map_err(|e| e.to_string())?;
    atomic_write(&path, out.as_bytes())
}

fn ensure_parent(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn load_json(path: &Path) -> Value {
    fs::read_to_string(path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| json!({}))
}

fn write_json(path: &Path, value: &Value) -> Result<(), String> {
    ensure_parent(path)?;
    let out = serde_json::to_string_pretty(value).map_err(|e| e.to_string())?;
    atomic_write(path, out.as_bytes())
}

fn upsert_json_env(path: &Path, pairs: &[(&str, &str)]) -> Result<(), String> {
    let mut v = load_json(path);
    let env = v
        .as_object_mut()
        .ok_or("config is not a JSON object")?
        .entry("env")
        .or_insert_with(|| json!({}));
    let map = env.as_object_mut().ok_or("env is not an object")?;
    for (k, val) in pairs {
        map.insert((*k).into(), Value::String((*val).into()));
    }
    write_json(path, &v)
}

fn write_env_file(path: &Path, pairs: &[(&str, &str)]) -> Result<(), String> {
    ensure_parent(path)?;
    let mut map = std::collections::BTreeMap::new();
    if let Ok(existing) = fs::read_to_string(path) {
        for line in existing.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((k, v)) = line.split_once('=') {
                map.insert(k.trim().to_string(), v.trim().to_string());
            }
        }
    }
    for (k, v) in pairs {
        map.insert((*k).into(), (*v).into());
    }
    let mut out = String::new();
    for (k, v) in map {
        out.push_str(&format!("{k}={v}\n"));
    }
    atomic_write(path, out.as_bytes())
}

fn apply_claude_plugin() -> Result<(), String> {
    let path = config_dir_for("claude").join("config.json");
    let mut v = load_json(&path);
    if let Some(map) = v.as_object_mut() {
        map.insert("primaryApiKey".into(), Value::String("any".into()));
    }
    write_json(&path, &v)
}

/// Codex CLI, Codex Desktop, and local ChatGPT Work all read `CODEX_HOME`
/// (`~/.codex`). Provider `base_url` must include `/v1` (Codex appends
/// `/responses`). An inline `model_providers = {}` cannot hold a nested
/// table — convert it to a dotted `[model_providers.napi]` table.
fn apply_codex_toml(existing: &str, api_key: &str, base_url: &str) -> String {
    let mut doc = existing
        .parse::<toml_edit::DocumentMut>()
        .unwrap_or_else(|_| toml_edit::DocumentMut::new());
    let v1 = openai_compat_v1(base_url);
    doc["openai_base_url"] = toml_edit::value(v1.as_str());
    doc["model_provider"] = toml_edit::value("napi");

    let mut providers = toml_edit::Table::new();
    providers.set_implicit(true);
    match doc.remove("model_providers") {
        Some(toml_edit::Item::Table(tbl)) => {
            for (k, v) in tbl {
                if k.as_str() != "napi" && !item_points_at_9router(&v) {
                    providers.insert(&k, v);
                }
            }
        }
        Some(toml_edit::Item::Value(toml_edit::Value::InlineTable(inline))) => {
            for (k, v) in inline.iter() {
                if k != "napi" && !value_points_at_9router(v) {
                    providers.insert(k, toml_edit::Item::Value(v.clone()));
                }
            }
        }
        _ => {}
    }

    let mut napi = toml_edit::Table::new();
    napi.set_implicit(false);
    napi["name"] = toml_edit::value("NAPI");
    napi["base_url"] = toml_edit::value(v1.as_str());
    napi["wire_api"] = toml_edit::value("responses");
    napi["env_key"] = toml_edit::value("OPENAI_API_KEY");
    napi["experimental_bearer_token"] = toml_edit::value(api_key);
    napi["requires_openai_auth"] = toml_edit::value(false);
    napi["supports_websockets"] = toml_edit::value(false);
    providers.insert("napi", toml_edit::Item::Table(napi));
    doc["model_providers"] = toml_edit::Item::Table(providers);

    if let Some(tbl) = doc["env"].as_table_mut() {
        tbl["OPENAI_API_KEY"] = toml_edit::value(api_key);
        tbl["OPENAI_BASE_URL"] = toml_edit::value(v1.as_str());
    } else {
        let mut env = toml_edit::Table::new();
        env["OPENAI_API_KEY"] = toml_edit::value(api_key);
        env["OPENAI_BASE_URL"] = toml_edit::value(v1.as_str());
        doc["env"] = toml_edit::Item::Table(env);
    }
    doc.to_string()
}

fn write_codex(api_key: &str, base_url: &str) -> Result<(), String> {
    let dir = config_dir_for("codex");
    let path = dir.join("config.toml");
    ensure_parent(&path)?;
    let content = fs::read_to_string(&path).unwrap_or_default();
    let next = apply_codex_toml(&content, api_key, base_url);
    atomic_write(&path, next.as_bytes())?;

    let v1 = openai_compat_v1(base_url);
    write_env_file(
        &dir.join(".env"),
        &[
            ("OPENAI_API_KEY", api_key),
            ("OPENAI_BASE_URL", v1.as_str()),
        ],
    )?;

    let auth = dir.join("auth.json");
    write_json(
        &auth,
        &json!({
            "OPENAI_API_KEY": api_key,
            "auth_mode": "apikey",
        }),
    )?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&auth, fs::Permissions::from_mode(0o600));
    }
    Ok(())
}

fn remap_provider_prefix(value: &mut Value, from: &str, to: &str) {
    match value {
        Value::String(s) if s.starts_with(from) => {
            *s = format!("{to}{}", &s[from.len()..]);
        }
        Value::Array(items) => {
            for item in items {
                remap_provider_prefix(item, from, to);
            }
        }
        Value::Object(map) => {
            for item in map.values_mut() {
                remap_provider_prefix(item, from, to);
            }
        }
        _ => {}
    }
}

fn item_points_at_9router(item: &toml_edit::Item) -> bool {
    item.to_string()
        .to_ascii_lowercase()
        .contains("9router.mikawi.org")
}

fn value_points_at_9router(value: &toml_edit::Value) -> bool {
    value
        .to_string()
        .to_ascii_lowercase()
        .contains("9router.mikawi.org")
}

/// NAPI origin, no `/v1`. Anthropic SDK / Claude Code / Gemini CLI / Goose
/// append the protocol path themselves (`/v1/messages`, `/v1beta/models/…`).
fn napi_origin(base: &str) -> String {
    let b = base.trim_end_matches('/');
    match b.strip_suffix("/v1") {
        Some(origin) => origin.trim_end_matches('/').to_string(),
        None => b.to_string(),
    }
}

/// OpenAI-compatible base: `{origin}/v1`. OpenAI SDK, Codex, Pi, Cline, etc.
/// then append `/chat/completions` or `/responses`.
fn openai_compat_v1(base: &str) -> String {
    format!("{}/v1", napi_origin(base))
}

/// Every agent surface Overwrite All writes. Independent of whether the CLI
/// or desktop app is installed on this machine — production overwrite
/// pre-stages NAPI so a later install is already pointed at the gateway.
pub const OVERWRITE_AGENT_IDS: &[&str] = &[
    "claude",
    "claude-desktop",
    "codex",
    "cline",
    "opencode",
    "gemini",
    "cursor",
    "continue",
    "goose",
    "factory",
    "grok",
    "roo",
    "kilocode",
    "hermes",
    "qwen",
    "openclaw",
    "pi",
    "windsurf",
];

pub fn agent_display_name(id: &str) -> &str {
    match id {
        "claude" => "Claude Code",
        "claude-desktop" => "Claude Desktop",
        "codex" => "Codex",
        "cline" => "Cline",
        "opencode" => "OpenCode",
        "gemini" => "Gemini CLI",
        "cursor" => "Cursor",
        "continue" => "Continue",
        "goose" => "Goose",
        "factory" => "Factory Droid",
        "grok" => "Grok CLI",
        "roo" => "Roo Code",
        "kilocode" => "Kilo Code",
        "hermes" => "Hermes Agent",
        "qwen" => "Qwen Code",
        "openclaw" => "OpenClaw",
        "pi" => "Pi",
        "windsurf" => "Windsurf",
        other => other,
    }
}

/// NAPI's Claude Desktop 3P profile. Not cc-switch's `00000000-0000-4000-8000-000000157210`.
const CLAUDE_DESKTOP_PROFILE_ID: &str = "6e617069-6465-4000-8000-000000000001";
const CLAUDE_DESKTOP_PROFILE_NAME: &str = "NAPI";
const CLAUDE_DESKTOP_CONFIG_FILE: &str = "claude_desktop_config.json";

#[derive(Debug, Clone)]
struct ClaudeDesktopPaths {
    normal_config_path: PathBuf,
    threep_config_path: PathBuf,
    profile_path: PathBuf,
    meta_path: PathBuf,
}

fn claude_desktop_paths_from_dirs(normal_dir: PathBuf, threep_dir: PathBuf) -> ClaudeDesktopPaths {
    let library = threep_dir.join("configLibrary");
    ClaudeDesktopPaths {
        normal_config_path: normal_dir.join(CLAUDE_DESKTOP_CONFIG_FILE),
        threep_config_path: threep_dir.join(CLAUDE_DESKTOP_CONFIG_FILE),
        profile_path: library.join(format!("{CLAUDE_DESKTOP_PROFILE_ID}.json")),
        meta_path: library.join("_meta.json"),
    }
}

/// Claude Desktop Chat / Cowork / in-app Code tab. Official 3P layout:
/// `Claude` + `Claude-3p` under Application Support (macOS), LocalAppData
/// (Windows), or XDG config (Linux).
pub fn claude_desktop_paths_from_home(home: &Path) -> (PathBuf, PathBuf) {
    let paths = claude_desktop_paths_for_home(home);
    (paths.profile_path, paths.meta_path)
}

fn claude_desktop_paths_for_home(home: &Path) -> ClaudeDesktopPaths {
    #[cfg(target_os = "macos")]
    {
        let app_support = home.join("Library").join("Application Support");
        return claude_desktop_paths_from_dirs(
            app_support.join("Claude"),
            app_support.join("Claude-3p"),
        );
    }
    #[cfg(windows)]
    {
        let local = std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .unwrap_or_else(|| home.join("AppData").join("Local"));
        return claude_desktop_paths_from_dirs(local.join("Claude"), local.join("Claude-3p"));
    }
    #[cfg(not(any(target_os = "macos", windows)))]
    {
        let config = std::env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .filter(|p| p.is_absolute())
            .unwrap_or_else(|| home.join(".config"));
        claude_desktop_paths_from_dirs(config.join("Claude"), config.join("Claude-3p"))
    }
}

/// OpenAI `{ data: [{ id }] }` and Anthropic `{ data: [{ id, display_name }] }`.
fn parse_v1_model_ids(body: &Value) -> Vec<String> {
    let Some(data) = body.get("data").and_then(Value::as_array) else {
        return Vec::new();
    };
    let mut ids = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for item in data {
        let Some(id) = item.get("id").and_then(Value::as_str).map(str::trim) else {
            continue;
        };
        if id.is_empty() || !seen.insert(id.to_string()) {
            continue;
        }
        ids.push(id.to_string());
    }
    ids
}

fn strip_1m_marker(id: &str) -> (String, bool) {
    let trimmed = id.trim();
    for marker in ["[1m]", "[1M]"] {
        if let Some(stripped) = trimmed
            .strip_suffix(marker)
            .map(str::trim_end)
            .filter(|s| !s.is_empty())
        {
            return (stripped.to_string(), true);
        }
    }
    (trimmed.to_string(), false)
}

/// Claude Desktop 1.12603.1+ fail-all: one non-Anthropic route rejects the
/// whole `inferenceModels` group (9router #935, deepseek-v4-pro).
fn is_claude_desktop_route(model: &str) -> bool {
    let normalized = model.trim().to_ascii_lowercase();
    if normalized.contains("[1m]") {
        return false;
    }
    let Some(tail) = normalized
        .strip_prefix("anthropic/claude-")
        .or_else(|| normalized.strip_prefix("claude-"))
    else {
        return false;
    };
    ["sonnet-", "opus-", "haiku-", "fable-", "mythos-"]
        .iter()
        .any(|prefix| tail.strip_prefix(prefix).is_some_and(|rest| !rest.is_empty()))
}

fn anthropic_family_tier(name: &str) -> Option<&'static str> {
    let n = name.to_ascii_lowercase();
    let tail = n
        .strip_prefix("anthropic/claude-")
        .or_else(|| n.strip_prefix("claude-"))?;
    if tail.starts_with("sonnet-") {
        Some("sonnet")
    } else if tail.starts_with("opus-") {
        Some("opus")
    } else if tail.starts_with("haiku-") {
        Some("haiku")
    } else if tail.starts_with("fable-") {
        Some("fable")
    } else if tail.starts_with("mythos-") {
        Some("mythos")
    } else {
        None
    }
}

/// 9router Cowork writer: `inferenceModels: models.map(name => ({ name }))`.
/// Source here is NAPI `GET /v1/models`, not 9router's static catalog.
/// Cowork only accepts Anthropic route ids; other NAPI ids are dropped so
/// one grok/gpt entry cannot blank the picker.
fn inference_models_from_catalog(ids: &[String]) -> Vec<Value> {
    let mut out = Vec::new();
    let mut family_defaulted = std::collections::HashSet::new();
    for raw in ids {
        let (name, supports_1m) = strip_1m_marker(raw);
        if !is_claude_desktop_route(&name) {
            continue;
        }
        let mut item = json!({ "name": name });
        if supports_1m {
            item["supports1m"] = json!(true);
        }
        if let Some(tier) = anthropic_family_tier(&name) {
            item["anthropicFamilyTier"] = json!(tier);
            if family_defaulted.insert(tier) {
                item["isFamilyDefault"] = json!(true);
            }
        }
        out.push(item);
    }
    out
}

fn build_claude_desktop_gateway_profile(
    origin: &str,
    api_key: &str,
    catalog: &[String],
) -> Value {
    let mut profile = json!({
        "coworkEgressAllowedHosts": ["*"],
        "disableDeploymentModeChooser": true,
        "inferenceGatewayApiKey": api_key,
        "inferenceGatewayAuthScheme": "bearer",
        "inferenceGatewayBaseUrl": origin,
        "inferenceProvider": "gateway"
    });
    let models = inference_models_from_catalog(catalog);
    if !models.is_empty() {
        profile["inferenceModels"] = Value::Array(models);
    }
    profile
}

async fn fetch_v1_model_ids(base_url: &str, api_key: &str) -> Result<Vec<String>, String> {
    let url = format!("{}/v1/models", napi_origin(base_url));
    let client = reqwest::Client::new();
    let resp = crate::http_get_auth_with_retry(&client, &url, api_key).await?;
    if !resp.status().is_success() {
        return Err(format!("GET /v1/models returned {}", resp.status()));
    }
    let body: Value = resp.json().await.map_err(|e| e.to_string())?;
    Ok(parse_v1_model_ids(&body))
}

fn fetch_v1_model_ids_blocking(base_url: &str, api_key: &str) -> Result<Vec<String>, String> {
    tauri::async_runtime::block_on(fetch_v1_model_ids(base_url, api_key))
}

fn write_claude_desktop_deployment_mode(path: &Path) -> Result<(), String> {
    let mut v = load_json(path);
    if !v.is_object() {
        v = json!({});
    }
    if let Some(map) = v.as_object_mut() {
        map.insert("deploymentMode".into(), Value::String("3p".into()));
    }
    write_json(path, &v)
}

fn write_claude_desktop_meta(path: &Path, applied: bool) -> Result<(), String> {
    let mut v = load_json(path);
    if !v.is_object() {
        v = json!({});
    }
    let map = v.as_object_mut().ok_or("_meta.json is not an object")?;
    let mut entries = map
        .get("entries")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    entries.retain(|entry| {
        entry.get("id").and_then(Value::as_str) != Some(CLAUDE_DESKTOP_PROFILE_ID)
    });
    if applied {
        entries.push(json!({
            "id": CLAUDE_DESKTOP_PROFILE_ID,
            "name": CLAUDE_DESKTOP_PROFILE_NAME
        }));
        map.insert(
            "appliedId".into(),
            Value::String(CLAUDE_DESKTOP_PROFILE_ID.into()),
        );
    } else {
        let ours = map
            .get("appliedId")
            .and_then(Value::as_str)
            == Some(CLAUDE_DESKTOP_PROFILE_ID);
        if ours {
            if let Some(next) = entries
                .iter()
                .find_map(|entry| entry.get("id").and_then(Value::as_str))
            {
                map.insert("appliedId".into(), Value::String(next.into()));
            } else {
                map.remove("appliedId");
            }
        }
    }
    map.insert("entries".into(), Value::Array(entries));
    write_json(path, &v)
}

fn apply_claude_desktop_3p_at(
    paths: &ClaudeDesktopPaths,
    api_key: &str,
    base_url: &str,
    catalog: &[String],
) -> Result<(), String> {
    let origin = napi_origin(base_url);
    write_claude_desktop_deployment_mode(&paths.normal_config_path)?;
    write_claude_desktop_deployment_mode(&paths.threep_config_path)?;
    write_json(
        &paths.profile_path,
        &build_claude_desktop_gateway_profile(&origin, api_key, catalog),
    )?;
    write_claude_desktop_meta(&paths.meta_path, true)
}

fn write_claude_desktop(api_key: &str, base_url: &str) -> Result<(), String> {
    let paths = claude_desktop_paths_for_home(&home_dir());
    let catalog = fetch_v1_model_ids_blocking(base_url, api_key).unwrap_or_default();
    apply_claude_desktop_3p_at(&paths, api_key, base_url, &catalog)
}

fn clear_claude_desktop_napi() -> Result<String, String> {
    let paths = claude_desktop_paths_for_home(&home_dir());
    for path in [&paths.normal_config_path, &paths.threep_config_path] {
        if !path.is_file() {
            continue;
        }
        let mut v = load_json(path);
        if let Some(map) = v.as_object_mut() {
            if map.get("deploymentMode").and_then(Value::as_str) == Some("3p") {
                map.insert("deploymentMode".into(), Value::String("1p".into()));
            }
        }
        write_json(path, &v)?;
    }
    let _ = delete_file(&paths.profile_path);
    if paths.meta_path.is_file() {
        write_claude_desktop_meta(&paths.meta_path, false)?;
    }
    Ok("stripped NAPI 3P gateway from Claude Desktop".into())
}

/// Write NAPI into every known agent config surface. Does not skip agents
/// that are not currently installed — configs are created so later installs
/// already talk to NAPI.
pub fn write_all_agents(api_key: &str, base_url: &str) -> Vec<String> {
    OVERWRITE_AGENT_IDS
        .iter()
        .map(|id| {
            let name = agent_display_name(id);
            match write_agent(id, api_key, base_url) {
                Ok(msg) => format!("{name}: {msg}"),
                Err(e) => format!("{name}: {e}"),
            }
        })
        .collect()
}

fn apply_claude_onboarding() -> Result<(), String> {
    let path = home_dir().join(".claude.json");
    let mut v = load_json(&path);
    if let Some(map) = v.as_object_mut() {
        map.insert("hasCompletedOnboarding".into(), Value::Bool(true));
    }
    write_json(&path, &v)
}

/// Resolve scan `id` or display name to a stable id.
pub fn resolve_agent_id(name: &str) -> String {
    match name {
        "Claude Code" | "claude" => "claude",
        "Claude Desktop" | "claude-desktop" => "claude-desktop",
        "OpenAI Codex CLI" | "Codex" | "codex" => "codex",
        "Cline" | "cline" => "cline",
        "OpenCode" | "opencode" => "opencode",
        "Gemini CLI" | "gemini" => "gemini",
        "Cursor" | "cursor" => "cursor",
        "Continue" | "continue" => "continue",
        "Goose" | "goose" => "goose",
        "Factory Droid" | "factory" => "factory",
        "Grok CLI" | "grok" => "grok",
        "Roo Code" | "roo" => "roo",
        "Kilo Code" | "kilocode" => "kilocode",
        "Hermes Agent" | "hermes" => "hermes",
        "OpenClaw" | "openclaw" => "openclaw",
        "Pi" | "pi" => "pi",
        "Qwen Code" | "qwen" => "qwen",
        "Windsurf" | "windsurf" => "windsurf",
        other => other,
    }
    .to_string()
}

pub fn write_agent(agent: &str, api_key: &str, base_url: &str) -> Result<String, String> {
    let id = resolve_agent_id(agent);
    let home = home_dir();
    let settings = load_app_settings();
    // Desktop persists the NAPI origin (`https://napi.mikawi.org`). Clients
    // that speak Anthropic/Gemini append `/v1/messages` or `/v1beta/…`
    // themselves. OpenAI-compat clients need `{origin}/v1`.
    let origin = napi_origin(base_url);
    let v1 = openai_compat_v1(base_url);

    match id.as_str() {
        "claude" => {
            let path = config_dir_for("claude").join("settings.json");
            upsert_json_env(
                &path,
                &[
                    ("ANTHROPIC_API_KEY", api_key),
                    ("ANTHROPIC_BASE_URL", origin.as_str()),
                    ("ANTHROPIC_AUTH_TOKEN", api_key),
                ],
            )?;
            if settings.enable_claude_plugin_integration {
                let _ = apply_claude_plugin();
            }
            if settings.skip_claude_onboarding {
                let _ = apply_claude_onboarding();
            }
        }
        "claude-desktop" => write_claude_desktop(api_key, base_url)?,
        "codex" => write_codex(api_key, base_url)?,
        "cline" => {
            let path = home.join(".cline").join("data").join("globalState.json");
            let mut v = load_json(&path);
            let providers = v
                .as_object_mut()
                .ok_or("globalState.json is not an object")?
                .entry("clineProviders")
                .or_insert_with(|| json!({}));
            let provider = providers
                .as_object_mut()
                .ok_or("clineProviders is not an object")?
                .entry("napi")
                .or_insert_with(|| json!({}))
                .as_object_mut()
                .ok_or("napi provider entry is not an object")?;
            provider.insert("name".into(), Value::String("NAPI".into()));
            provider.insert("apiKey".into(), Value::String(api_key.into()));
            provider.insert("baseUrl".into(), Value::String(v1.clone()));
            write_json(&path, &v)?;
        }
        "opencode" => {
            let path = config_dir_for("opencode").join("opencode.json");
            let mut v = load_json(&path);
            {
                let root = v
                    .as_object_mut()
                    .ok_or("opencode.json is not an object")?;
                let omni_models = root
                    .get("provider")
                    .and_then(|p| p.get("omniroute"))
                    .and_then(|p| p.get("models"))
                    .cloned();
                let providers = root.entry("provider").or_insert_with(|| json!({}));
                if let Some(map) = providers.as_object_mut() {
                    let mut napi_entry = json!({
                        "name": "NAPI",
                        "npm": "@ai-sdk/openai-compatible",
                        "options": {
                            "apiKey": api_key,
                            "baseURL": v1,
                        }
                    });
                    if let Some(models) = omni_models {
                        napi_entry["models"] = models;
                    }
                    map.insert("napi".into(), napi_entry);
                }
                let enabled = root
                    .entry("enabled_providers")
                    .or_insert_with(|| json!([]));
                if let Some(arr) = enabled.as_array_mut() {
                    arr.retain(|x| {
                        let s = x.as_str().unwrap_or("");
                        s != "napi" && s != "omniroute"
                    });
                    arr.insert(0, json!("napi"));
                }
                let penv = root.entry("provider_env").or_insert_with(|| json!({}));
                if let Some(map) = penv.as_object_mut() {
                    map.insert("OPENAI_API_KEY".into(), Value::String(api_key.into()));
                    map.insert("OPENAI_BASE_URL".into(), Value::String(v1.clone()));
                    map.insert("ANTHROPIC_API_KEY".into(), Value::String(api_key.into()));
                    map.insert("ANTHROPIC_BASE_URL".into(), Value::String(origin.clone()));
                }
            }
            remap_provider_prefix(&mut v, "omniroute/", "napi/");
            write_json(&path, &v)?;
            let slim = config_dir_for("opencode").join("oh-my-opencode-slim.json");
            if slim.is_file() {
                let mut slim_v = load_json(&slim);
                remap_provider_prefix(&mut slim_v, "omniroute/", "napi/");
                write_json(&slim, &slim_v)?;
            }
        }
        "gemini" => {
            write_env_file(
                &config_dir_for("gemini").join(".env"),
                &[
                    ("GEMINI_API_KEY", api_key),
                    ("GEMINI_BASE_URL", origin.as_str()),
                    ("GOOGLE_GEMINI_BASE_URL", origin.as_str()),
                ],
            )?;
        }
        "grok" => {
            let path = config_dir_for("grok").join("config.toml");
            ensure_parent(&path)?;
            let content = fs::read_to_string(&path).unwrap_or_default();
            let mut doc = content
                .parse::<toml_edit::DocumentMut>()
                .unwrap_or_else(|_| toml_edit::DocumentMut::new());
            doc["base_url"] = toml_edit::value(v1.as_str());
            doc["api_key"] = toml_edit::value(api_key);
            atomic_write(&path, doc.to_string().as_bytes())?;
        }
        "hermes" => {
            let path = config_dir_for("hermes").join("config.yaml");
            ensure_parent(&path)?;
            let mut v = if path.exists() {
                let raw = fs::read_to_string(&path).unwrap_or_default();
                serde_yaml::from_str::<Value>(&raw).unwrap_or_else(|_| json!({}))
            } else {
                json!({})
            };
            if let Some(map) = v.as_object_mut() {
                map.insert("openai_api_key".into(), Value::String(api_key.into()));
                map.insert("openai_base_url".into(), Value::String(v1.clone()));
                map.insert("base_url".into(), Value::String(v1.clone()));
            }
            let out = serde_yaml::to_string(&v).map_err(|e| e.to_string())?;
            atomic_write(&path, out.as_bytes())?;
        }
        "qwen" => {
            upsert_json_env(
                &home.join(".qwen").join("settings.json"),
                &[
                    ("OPENAI_API_KEY", api_key),
                    ("OPENAI_BASE_URL", v1.as_str()),
                    ("QWEN_API_KEY", api_key),
                ],
            )?;
        }
        "continue" => {
            let path = home.join(".continue").join("config.json");
            let mut v = load_json(&path);
            if let Some(map) = v.as_object_mut() {
                map.insert(
                    "models".into(),
                    json!([{
                        "title": "NAPI",
                        "provider": "openai",
                        "model": "gpt-4o",
                        "apiKey": api_key,
                        "apiBase": v1,
                    }]),
                );
            }
            write_json(&path, &v)?;
        }
        "goose" => {
            write_env_file(
                &home.join(".config").join("goose").join(".env"),
                &[
                    ("OPENAI_API_KEY", api_key),
                    ("OPENAI_HOST", origin.as_str()),
                    ("GOOSE_PROVIDER", "openai"),
                ],
            )?;
        }
        "factory" => {
            upsert_json_env(
                &home.join(".factory").join("config.json"),
                &[
                    ("OPENAI_API_KEY", api_key),
                    ("OPENAI_BASE_URL", v1.as_str()),
                ],
            )?;
        }
        "cursor" => {
            upsert_json_env(
                &home.join(".cursor").join("cli-config.json"),
                &[
                    ("OPENAI_API_KEY", api_key),
                    ("OPENAI_BASE_URL", v1.as_str()),
                    ("ANTHROPIC_API_KEY", api_key),
                    ("ANTHROPIC_BASE_URL", origin.as_str()),
                ],
            )?;
        }
        "roo" => {
            upsert_json_env(
                &home.join(".roo").join("napi.json"),
                &[
                    ("OPENAI_API_KEY", api_key),
                    ("OPENAI_BASE_URL", v1.as_str()),
                ],
            )?;
        }
        "kilocode" => {
            upsert_json_env(
                &home.join(".kilocode").join("napi.json"),
                &[
                    ("OPENAI_API_KEY", api_key),
                    ("OPENAI_BASE_URL", v1.as_str()),
                ],
            )?;
        }
        "openclaw" => {
            upsert_json_env(
                &config_dir_for("openclaw").join("openclaw.json"),
                &[
                    ("OPENAI_API_KEY", api_key),
                    ("OPENAI_BASE_URL", v1.as_str()),
                ],
            )?;
        }
        "pi" => {
            let path = config_dir_for("pi").join("models.json");
            let mut v = load_json(&path);
            if let Some(map) = v.as_object_mut() {
                let providers = map.entry("providers").or_insert_with(|| json!({}));
                if let Some(p) = providers.as_object_mut() {
                    p.insert(
                        "napi".into(),
                        json!({
                            "name": "NAPI",
                            "apiKey": api_key,
                            "baseUrl": v1,
                        }),
                    );
                }
            }
            write_json(&path, &v)?;
        }
        "windsurf" => {
            upsert_json_env(
                &home.join(".codeium").join("windsurf").join("napi.json"),
                &[
                    ("OPENAI_API_KEY", api_key),
                    ("OPENAI_BASE_URL", v1.as_str()),
                ],
            )?;
        }
        other => return Err(format!("Agent '{other}' is not supported")),
    }

    Ok(format!("Wrote NAPI into {id}"))
}

fn looks_like_napi(value: &str) -> bool {
    let v = value.to_ascii_lowercase();
    v.contains("napi.mikawi.org") || v.contains("napi-desktop")
}

fn strip_json_env_keys(path: &Path, keys: &[&str]) -> Result<bool, String> {
    if !path.is_file() {
        return Ok(false);
    }
    let mut v = load_json(path);
    let Some(map) = v.as_object_mut() else {
        return Ok(false);
    };
    let Some(env) = map.get_mut("env").and_then(|e| e.as_object_mut()) else {
        return Ok(false);
    };
    let mut changed = false;
    for k in keys {
        if env.remove(*k).is_some() {
            changed = true;
        }
    }
    if changed {
        write_json(path, &v)?;
    }
    Ok(changed)
}

fn strip_dotenv_keys(path: &Path, keys: &[&str]) -> Result<bool, String> {
    if !path.is_file() {
        return Ok(false);
    }
    let raw = fs::read_to_string(path).map_err(|e| e.to_string())?;
    let drop: std::collections::HashSet<&str> = keys.iter().copied().collect();
    let mut changed = false;
    let mut out = String::new();
    for line in raw.lines() {
        let trimmed = line.trim();
        if let Some((k, _)) = trimmed.split_once('=') {
            if drop.contains(k.trim()) {
                changed = true;
                continue;
            }
        }
        out.push_str(line);
        out.push('\n');
    }
    if changed {
        atomic_write(path, out.as_bytes())?;
    }
    Ok(changed)
}

fn delete_file(path: &Path) -> Result<bool, String> {
    if !path.exists() {
        return Ok(false);
    }
    fs::remove_file(path).map_err(|e| e.to_string())?;
    Ok(true)
}

fn clear_codex_napi() -> Result<String, String> {
    let dir = config_dir_for("codex");
    let path = dir.join("config.toml");
    if path.is_file() {
        let content = fs::read_to_string(&path).unwrap_or_default();
        if let Ok(mut doc) = content.parse::<toml_edit::DocumentMut>() {
            if let Some(mp) = doc
                .get_mut("model_providers")
                .and_then(|item| item.as_table_like_mut())
            {
                mp.remove("napi");
            }
            if doc
                .get("model_provider")
                .and_then(|item| item.as_str())
                == Some("napi")
            {
                doc.remove("model_provider");
            }
            if doc
                .get("openai_base_url")
                .and_then(|item| item.as_str())
                .is_some_and(looks_like_napi)
            {
                doc.remove("openai_base_url");
            }
            if let Some(env) = doc.get_mut("env").and_then(|item| item.as_table_mut()) {
                for key in ["OPENAI_API_KEY", "OPENAI_BASE_URL"] {
                    let drop = env
                        .get(key)
                        .and_then(|item| item.as_str())
                        .is_some_and(looks_like_napi)
                        || key == "OPENAI_API_KEY";
                    if drop {
                        env.remove(key);
                    }
                }
            }
            atomic_write(&path, doc.to_string().as_bytes())?;
        }
    }
    let _ = strip_dotenv_keys(
        &dir.join(".env"),
        &["OPENAI_API_KEY", "OPENAI_BASE_URL"],
    );
    let auth = dir.join("auth.json");
    if auth.is_file() {
        let v = load_json(&auth);
        let only_apikey = v.get("OPENAI_API_KEY").is_some()
            && v.get("tokens").is_none()
            && v.get("refresh_token").is_none();
        if only_apikey {
            let _ = delete_file(&auth);
        }
    }
    Ok("stripped NAPI from ~/.codex".into())
}

fn clear_opencode_napi() -> Result<String, String> {
    let path = config_dir_for("opencode").join("opencode.json");
    if path.is_file() {
        let mut v = load_json(&path);
        let has_omni = v
            .pointer("/provider/omniroute")
            .is_some();
        if let Some(map) = v.as_object_mut() {
            if let Some(providers) = map.get_mut("provider").and_then(|p| p.as_object_mut()) {
                providers.remove("napi");
            }
            if let Some(arr) = map
                .get_mut("enabled_providers")
                .and_then(|e| e.as_array_mut())
            {
                arr.retain(|x| x.as_str() != Some("napi"));
                if has_omni && !arr.iter().any(|x| x.as_str() == Some("omniroute")) {
                    arr.insert(0, json!("omniroute"));
                }
            }
            if let Some(penv) = map.get_mut("provider_env").and_then(|e| e.as_object_mut()) {
                for k in [
                    "OPENAI_API_KEY",
                    "OPENAI_BASE_URL",
                    "ANTHROPIC_API_KEY",
                    "ANTHROPIC_BASE_URL",
                ] {
                    let drop = penv
                        .get(k)
                        .and_then(|x| x.as_str())
                        .is_some_and(looks_like_napi)
                        || k.ends_with("_KEY");
                    if drop {
                        penv.remove(k);
                    }
                }
            }
        }
        if has_omni {
            remap_provider_prefix(&mut v, "napi/", "omniroute/");
        }
        write_json(&path, &v)?;
    }
    let slim = config_dir_for("opencode").join("oh-my-opencode-slim.json");
    if slim.is_file() {
        let mut slim_v = load_json(&slim);
        remap_provider_prefix(&mut slim_v, "napi/", "omniroute/");
        write_json(&slim, &slim_v)?;
    }
    Ok("stripped NAPI from OpenCode".into())
}

/// Remove the NAPI overlay from an agent's original config. Does not delete
/// the rest of the user's project/history files.
pub fn clear_napi(agent: &str) -> Result<String, String> {
    let id = resolve_agent_id(agent);
    let home = home_dir();
    match id.as_str() {
        "claude" => {
            strip_json_env_keys(
                &config_dir_for("claude").join("settings.json"),
                &[
                    "ANTHROPIC_API_KEY",
                    "ANTHROPIC_BASE_URL",
                    "ANTHROPIC_AUTH_TOKEN",
                ],
            )?;
            let plugin = config_dir_for("claude").join("config.json");
            if plugin.is_file() {
                let mut v = load_json(&plugin);
                if v.get("primaryApiKey").and_then(|x| x.as_str()) == Some("any") {
                    if let Some(map) = v.as_object_mut() {
                        map.remove("primaryApiKey");
                    }
                    write_json(&plugin, &v)?;
                }
            }
            Ok("stripped NAPI env from Claude settings".into())
        }
        "claude-desktop" => clear_claude_desktop_napi(),
        "codex" => clear_codex_napi(),
        "cline" => {
            let path = home.join(".cline").join("data").join("globalState.json");
            if path.is_file() {
                let mut v = load_json(&path);
                if let Some(p) = v
                    .pointer_mut("/clineProviders")
                    .and_then(|x| x.as_object_mut())
                {
                    p.remove("napi");
                }
                write_json(&path, &v)?;
            }
            Ok("removed Cline napi provider".into())
        }
        "opencode" => clear_opencode_napi(),
        "gemini" => {
            strip_dotenv_keys(
                &config_dir_for("gemini").join(".env"),
                &["GEMINI_API_KEY", "GEMINI_BASE_URL", "GOOGLE_GEMINI_BASE_URL"],
            )?;
            Ok("stripped NAPI from Gemini .env".into())
        }
        "grok" => {
            let path = config_dir_for("grok").join("config.toml");
            if path.is_file() {
                let content = fs::read_to_string(&path).unwrap_or_default();
                if let Ok(mut doc) = content.parse::<toml_edit::DocumentMut>() {
                    if doc
                        .get("base_url")
                        .and_then(|i| i.as_str())
                        .is_some_and(looks_like_napi)
                    {
                        doc.remove("base_url");
                        doc.remove("api_key");
                    }
                    atomic_write(&path, doc.to_string().as_bytes())?;
                }
            }
            Ok("stripped NAPI from Grok config".into())
        }
        "hermes" => {
            let path = config_dir_for("hermes").join("config.yaml");
            if path.is_file() {
                let raw = fs::read_to_string(&path).unwrap_or_default();
                if let Ok(mut v) = serde_yaml::from_str::<Value>(&raw) {
                    if let Some(map) = v.as_object_mut() {
                        for k in ["openai_api_key", "openai_base_url", "base_url"] {
                            map.remove(k);
                        }
                    }
                    let out = serde_yaml::to_string(&v).map_err(|e| e.to_string())?;
                    atomic_write(&path, out.as_bytes())?;
                }
            }
            Ok("stripped NAPI from Hermes config".into())
        }
        "qwen" => {
            strip_json_env_keys(
                &home.join(".qwen").join("settings.json"),
                &["OPENAI_API_KEY", "OPENAI_BASE_URL", "QWEN_API_KEY"],
            )?;
            Ok("stripped NAPI from Qwen settings".into())
        }
        "continue" => {
            let path = home.join(".continue").join("config.json");
            if path.is_file() {
                let mut v = load_json(&path);
                if let Some(models) = v.get_mut("models").and_then(|m| m.as_array_mut()) {
                    models.retain(|m| {
                        m.get("title").and_then(|t| t.as_str()) != Some("NAPI")
                            && !m
                                .get("apiBase")
                                .and_then(|t| t.as_str())
                                .is_some_and(looks_like_napi)
                    });
                }
                write_json(&path, &v)?;
            }
            Ok("removed NAPI model from Continue".into())
        }
        "goose" => {
            strip_dotenv_keys(
                &home.join(".config").join("goose").join(".env"),
                &["OPENAI_API_KEY", "OPENAI_HOST", "GOOSE_PROVIDER"],
            )?;
            Ok("stripped NAPI from Goose .env".into())
        }
        "factory" => {
            strip_json_env_keys(
                &home.join(".factory").join("config.json"),
                &["OPENAI_API_KEY", "OPENAI_BASE_URL"],
            )?;
            Ok("stripped NAPI from Factory config".into())
        }
        "cursor" => {
            strip_json_env_keys(
                &home.join(".cursor").join("cli-config.json"),
                &[
                    "OPENAI_API_KEY",
                    "OPENAI_BASE_URL",
                    "ANTHROPIC_API_KEY",
                    "ANTHROPIC_BASE_URL",
                ],
            )?;
            Ok("stripped NAPI env from Cursor".into())
        }
        "roo" => {
            delete_file(&home.join(".roo").join("napi.json"))?;
            Ok("removed Roo napi.json".into())
        }
        "kilocode" => {
            delete_file(&home.join(".kilocode").join("napi.json"))?;
            Ok("removed Kilo Code napi.json".into())
        }
        "openclaw" => {
            strip_json_env_keys(
                &config_dir_for("openclaw").join("openclaw.json"),
                &["OPENAI_API_KEY", "OPENAI_BASE_URL"],
            )?;
            Ok("stripped NAPI from OpenClaw".into())
        }
        "pi" => {
            let path = config_dir_for("pi").join("models.json");
            if path.is_file() {
                let mut v = load_json(&path);
                if let Some(p) = v
                    .pointer_mut("/providers")
                    .and_then(|x| x.as_object_mut())
                {
                    p.remove("napi");
                }
                write_json(&path, &v)?;
            }
            Ok("removed Pi napi provider".into())
        }
        "windsurf" => {
            delete_file(&home.join(".codeium").join("windsurf").join("napi.json"))?;
            Ok("removed Windsurf napi.json".into())
        }
        other => Ok(format!("{other}: no NAPI overlay to strip")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_dir_for_defaults() {
        let home = home_dir();
        assert_eq!(default_dir_for("claude"), home.join(".claude"));
        assert_eq!(default_dir_for("codex"), home.join(".codex"));
        assert_eq!(default_dir_for("gemini"), home.join(".gemini"));
        assert_eq!(default_dir_for("grok"), home.join(".grok"));
        assert_eq!(default_dir_for("openclaw"), home.join(".openclaw"));
        assert_eq!(default_dir_for("hermes"), home.join(".hermes"));
        assert_eq!(default_dir_for("pi"), home.join(".pi").join("agent"));
        assert_eq!(default_dir_for("app"), default_app_config_dir());
        if cfg!(target_os = "windows") {
            assert!(default_dir_for("opencode").ends_with("opencode"));
        } else {
            assert_eq!(
                default_dir_for("opencode"),
                home.join(".config").join("opencode")
            );
        }
    }

    #[test]
    fn expand_tilde_home() {
        let home = home_dir();
        assert_eq!(expand_tilde("~"), home);
        assert_eq!(expand_tilde("~/foo"), home.join("foo"));
        assert_eq!(expand_tilde("  ~/bar  "), home.join("bar"));
        assert_eq!(expand_tilde("/abs/path"), PathBuf::from("/abs/path"));
    }

    #[test]
    fn app_settings_defaults_are_on() {
        let s = AppSettings::default();
        assert!(s.enable_claude_plugin_integration);
        assert!(s.skip_claude_onboarding);
        assert!(s.log_enabled);
        assert_eq!(s.log_level, "info");
        assert!(s.claude_config_dir.is_none());
        assert!(s.app_config_dir.is_none());
    }

    #[test]
    fn looks_like_napi_host() {
        assert!(looks_like_napi("https://napi.mikawi.org/v1"));
        assert!(!looks_like_napi("https://api.openai.com"));
    }

    #[test]
    fn strip_json_env_removes_napi_keys() {
        let dir = std::env::temp_dir().join(format!(
            "napi-strip-env-{}",
            std::process::id()
        ));
        let _ = fs::create_dir_all(&dir);
        let path = dir.join("settings.json");
        fs::write(
            &path,
            r#"{"env":{"ANTHROPIC_API_KEY":"sk-x","KEEP":"yes"},"other":1}"#,
        )
        .unwrap();
        strip_json_env_keys(&path, &["ANTHROPIC_API_KEY"]).unwrap();
        let v: serde_json::Value =
            serde_json::from_str(&fs::read_to_string(&path).unwrap()).unwrap();
        assert!(v["env"].get("ANTHROPIC_API_KEY").is_none());
        assert_eq!(v["env"]["KEEP"], "yes");
        assert_eq!(v["other"], 1);
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn openai_compat_v1_appends_once() {
        assert_eq!(
            openai_compat_v1("https://napi.mikawi.org"),
            "https://napi.mikawi.org/v1"
        );
        assert_eq!(
            openai_compat_v1("https://napi.mikawi.org/v1/"),
            "https://napi.mikawi.org/v1"
        );
    }

    #[test]
    fn napi_origin_strips_v1() {
        assert_eq!(
            napi_origin("https://napi.mikawi.org"),
            "https://napi.mikawi.org"
        );
        assert_eq!(
            napi_origin("https://napi.mikawi.org/"),
            "https://napi.mikawi.org"
        );
        assert_eq!(
            napi_origin("https://napi.mikawi.org/v1"),
            "https://napi.mikawi.org"
        );
        assert_eq!(
            napi_origin("https://napi.mikawi.org/v1/"),
            "https://napi.mikawi.org"
        );
    }

    #[test]
    fn apply_codex_toml_replaces_inline_empty_table() {
        let existing = "openai_base_url = \"https://napi.mikawi.org\"\nmodel_provider = \"napi\"\nmodel_providers = {}\n";
        let out = apply_codex_toml(existing, "sk-test", "https://napi.mikawi.org");
        assert!(
            out.contains("[model_providers.napi]"),
            "expected dotted napi table, got:\n{out}"
        );
        assert!(out.contains("base_url = \"https://napi.mikawi.org/v1\""));
        assert!(out.contains("openai_base_url = \"https://napi.mikawi.org/v1\""));
        assert!(out.contains("wire_api = \"responses\""));
        assert!(!out.contains("model_providers = {}"));
        assert!(!out.contains("api_key ="));
    }

    #[test]
    fn apply_codex_toml_keeps_unrelated_providers() {
        let existing = r#"
[model_providers.local]
name = "local"
base_url = "http://127.0.0.1:4000/v1"
"#;
        let out = apply_codex_toml(existing, "sk-test", "https://napi.mikawi.org/v1");
        assert!(out.contains("[model_providers.local]"), "got:\n{out}");
        assert!(out.contains("[model_providers.napi]"), "got:\n{out}");
        assert!(out.contains("http://127.0.0.1:4000/v1"));
    }

    #[test]
    fn apply_codex_toml_drops_9router_tables() {
        let existing = r#"
[model_providers.other]
name = "other"
base_url = "https://9router.mikawi.org/v1"
"#;
        let out = apply_codex_toml(existing, "sk-test", "https://napi.mikawi.org");
        assert!(
            !out.contains("9router.mikawi.org"),
            "9router must not survive a NAPI overwrite:\n{out}"
        );
        assert!(out.contains("[model_providers.napi]"));
    }

    #[test]
    fn remap_omniroute_prefix_to_napi() {
        let mut v = json!({
            "agent": { "explorer": { "model": "omniroute/ox-alpha-free" } },
            "enabled_providers": ["omniroute"]
        });
        remap_provider_prefix(&mut v, "omniroute/", "napi/");
        assert_eq!(v["agent"]["explorer"]["model"], "napi/ox-alpha-free");
        assert_eq!(v["enabled_providers"][0], "omniroute");
    }

    #[test]
    fn overwrite_ids_cover_every_writer_arm() {
        for id in OVERWRITE_AGENT_IDS {
            assert_eq!(resolve_agent_id(id), *id, "id {id} must resolve to itself");
            assert_ne!(
                agent_display_name(id),
                "",
                "display name required for {id}"
            );
        }
        assert!(OVERWRITE_AGENT_IDS.contains(&"claude-desktop"));
        assert!(OVERWRITE_AGENT_IDS.contains(&"claude"));
        assert!(OVERWRITE_AGENT_IDS.contains(&"codex"));
        assert_eq!(OVERWRITE_AGENT_IDS.len(), 18);
    }

    #[test]
    fn claude_desktop_3p_writes_gateway_profile_without_v1() {
        let dir = std::env::temp_dir().join(format!(
            "napi-claude-3p-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let paths = claude_desktop_paths_from_dirs(dir.join("Claude"), dir.join("Claude-3p"));
        apply_claude_desktop_3p_at(&paths, "sk-test", "https://napi.mikawi.org", &[]).unwrap();

        let normal: Value =
            serde_json::from_str(&fs::read_to_string(&paths.normal_config_path).unwrap()).unwrap();
        let threep: Value =
            serde_json::from_str(&fs::read_to_string(&paths.threep_config_path).unwrap()).unwrap();
        let profile: Value =
            serde_json::from_str(&fs::read_to_string(&paths.profile_path).unwrap()).unwrap();
        let meta: Value =
            serde_json::from_str(&fs::read_to_string(&paths.meta_path).unwrap()).unwrap();

        assert_eq!(normal["deploymentMode"], "3p");
        assert_eq!(threep["deploymentMode"], "3p");
        assert_eq!(profile["inferenceProvider"], "gateway");
        assert_eq!(profile["inferenceGatewayAuthScheme"], "bearer");
        assert_eq!(
            profile["inferenceGatewayBaseUrl"],
            "https://napi.mikawi.org"
        );
        assert_eq!(profile["inferenceGatewayApiKey"], "sk-test");
        assert!(profile.get("inferenceModels").is_none());
        assert!(!profile.to_string().contains("/v1"));
        assert_eq!(meta["appliedId"], CLAUDE_DESKTOP_PROFILE_ID);
        assert_ne!(
            CLAUDE_DESKTOP_PROFILE_ID,
            "00000000-0000-4000-8000-000000157210"
        );
        let entries = meta["entries"].as_array().unwrap();
        assert!(entries.iter().any(|e| {
            e["id"] == CLAUDE_DESKTOP_PROFILE_ID && e["name"] == CLAUDE_DESKTOP_PROFILE_NAME
        }));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn claude_desktop_3p_preserves_mcp_and_foreign_profiles() {
        let dir = std::env::temp_dir().join(format!(
            "napi-claude-3p-keep-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("Claude")).unwrap();
        let paths = claude_desktop_paths_from_dirs(dir.join("Claude"), dir.join("Claude-3p"));
        fs::write(
            &paths.normal_config_path,
            r#"{"mcpServers":{"fs":{"command":"npx"}}}"#,
        )
        .unwrap();
        fs::create_dir_all(paths.meta_path.parent().unwrap()).unwrap();
        fs::write(
            &paths.meta_path,
            r#"{"appliedId":"other","entries":[{"id":"other","name":"Keep"}]}"#,
        )
        .unwrap();

        apply_claude_desktop_3p_at(
            &paths,
            "sk-test",
            "https://napi.mikawi.org/v1",
            &[],
        )
        .unwrap();

        let normal: Value =
            serde_json::from_str(&fs::read_to_string(&paths.normal_config_path).unwrap()).unwrap();
        assert_eq!(normal["deploymentMode"], "3p");
        assert_eq!(normal["mcpServers"]["fs"]["command"], "npx");

        let profile: Value =
            serde_json::from_str(&fs::read_to_string(&paths.profile_path).unwrap()).unwrap();
        assert_eq!(
            profile["inferenceGatewayBaseUrl"],
            "https://napi.mikawi.org"
        );

        let meta: Value =
            serde_json::from_str(&fs::read_to_string(&paths.meta_path).unwrap()).unwrap();
        assert_eq!(meta["appliedId"], CLAUDE_DESKTOP_PROFILE_ID);
        let entries = meta["entries"].as_array().unwrap();
        assert!(entries.iter().any(|e| e["id"] == "other"));
        assert!(entries.iter().any(|e| e["id"] == CLAUDE_DESKTOP_PROFILE_ID));
        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn parse_v1_models_openai_and_anthropic_envelopes() {
        let openai = json!({
            "object": "list",
            "data": [
                { "id": "claude-sonnet-5", "object": "model" },
                { "id": "claude-opus-5", "object": "model" },
                { "id": "grok-4", "object": "model" },
                { "id": "claude-sonnet-5", "object": "model" }
            ]
        });
        assert_eq!(
            parse_v1_model_ids(&openai),
            vec!["claude-sonnet-5", "claude-opus-5", "grok-4"]
        );

        let anthropic = json!({
            "data": [
                { "id": "claude-haiku-4-5", "display_name": "Haiku 4.5", "type": "model" }
            ],
            "has_more": false
        });
        assert_eq!(parse_v1_model_ids(&anthropic), vec!["claude-haiku-4-5"]);
        assert!(parse_v1_model_ids(&json!({ "success": false })).is_empty());
    }

    #[test]
    fn inference_models_keeps_live_claude_routes_drops_fail_all_ids() {
        let catalog = vec![
            "claude-sonnet-5".into(),
            "claude-opus-5".into(),
            "claude-sonnet-5[1m]".into(),
            "grok-4".into(),
            "gpt-4o".into(),
            "deepseek-v4-pro".into(),
            "claude-fable-5".into(),
        ];
        let models = inference_models_from_catalog(&catalog);
        let names: Vec<&str> = models
            .iter()
            .map(|m| m["name"].as_str().unwrap())
            .collect();
        assert_eq!(
            names,
            vec![
                "claude-sonnet-5",
                "claude-opus-5",
                "claude-sonnet-5",
                "claude-fable-5"
            ]
        );
        assert!(!models.iter().any(|m| m["name"] == "grok-4"));
        assert!(!models.iter().any(|m| m["name"] == "deepseek-v4-pro"));
        let one_m = models
            .iter()
            .find(|m| m.get("supports1m") == Some(&json!(true)))
            .unwrap();
        assert_eq!(one_m["name"], "claude-sonnet-5");
        assert_eq!(models[0]["anthropicFamilyTier"], "sonnet");
        assert_eq!(models[0]["isFamilyDefault"], true);
        assert_eq!(models[1]["anthropicFamilyTier"], "opus");
    }

    #[test]
    fn claude_desktop_3p_writes_catalog_from_endpoint_ids() {
        let dir = std::env::temp_dir().join(format!(
            "napi-claude-3p-models-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        let paths = claude_desktop_paths_from_dirs(dir.join("Claude"), dir.join("Claude-3p"));
        let catalog = vec![
            "claude-sonnet-5".into(),
            "claude-opus-5".into(),
            "grok-4".into(),
        ];
        apply_claude_desktop_3p_at(
            &paths,
            "sk-test",
            "https://napi.mikawi.org",
            &catalog,
        )
        .unwrap();
        let profile: Value =
            serde_json::from_str(&fs::read_to_string(&paths.profile_path).unwrap()).unwrap();
        let models = profile["inferenceModels"].as_array().unwrap();
        assert_eq!(models[0]["name"], "claude-sonnet-5");
        assert_eq!(models[1]["name"], "claude-opus-5");
        assert_eq!(models.len(), 2);
        let _ = fs::remove_dir_all(&dir);
    }
}
