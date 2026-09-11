use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    pub name: String,
    pub config_path: PathBuf,
    pub key_field: String,
    pub base_url_field: String,
    pub format: String,
    pub detected: bool,
}

#[tauri::command]
fn scan_agents() -> Vec<AgentConfig> {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .unwrap_or_else(|_| ".".to_string());
    let home_path = PathBuf::from(&home);

    let mut agents = Vec::new();

    let claude_path = home_path.join(".claude").join("settings.json");
    agents.push(AgentConfig {
        name: "Claude Code".into(),
        config_path: claude_path.clone(),
        key_field: "env.ANTHROPIC_AUTH_TOKEN".into(),
        base_url_field: "env.ANTHROPIC_BASE_URL".into(),
        format: "json".into(),
        detected: claude_path.exists(),
    });

    let codex_path = home_path.join(".codex").join("config.toml");
    agents.push(AgentConfig {
        name: "Codex".into(),
        config_path: codex_path.clone(),
        key_field: "model_providers.9router.api_key".into(),
        base_url_field: "model_providers.9router.base_url".into(),
        format: "toml".into(),
        detected: codex_path.exists(),
    });

    let opencode_path = if cfg!(target_os = "windows") {
        let appdata = std::env::var("APPDATA").unwrap_or_else(|_| format!("{}\\AppData\\Roaming", home));
        PathBuf::from(appdata).join("opencode").join("opencode.json")
    } else {
        home_path.join(".config").join("opencode").join("opencode.json")
    };
    agents.push(AgentConfig {
        name: "OpenCode".into(),
        config_path: opencode_path.clone(),
        key_field: "provider_env.OPENAI_API_KEY".into(),
        base_url_field: "provider_env.OPENAI_BASE_URL".into(),
        format: "json".into(),
        detected: opencode_path.exists(),
    });

    let gemini_path = home_path.join(".gemini").join("settings.json");
    agents.push(AgentConfig {
        name: "Gemini CLI".into(),
        config_path: gemini_path.clone(),
        key_field: "oauth".into(), // ponytail: OAuth-only, no static key injection
        base_url_field: "n/a".into(),
        format: "json".into(),
        detected: gemini_path.exists(),
    });

    agents
}

#[tauri::command]
fn reconfigure_agent(
    agent_name: String,
    api_key: String,
    base_url: String,
) -> Result<String, String> {
    let home = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .map_err(|e| e.to_string())?;
    let home_path = PathBuf::from(&home);

    match agent_name.as_str() {
        "Claude Code" => {
            let path = home_path.join(".claude").join("settings.json");
            let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
            let mut v: serde_json::Value =
                serde_json::from_str(&content).map_err(|e| e.to_string())?;
            if let Some(env) = v.get_mut("env") {
                env["ANTHROPIC_AUTH_TOKEN"] = serde_json::Value::String(api_key);
                env["ANTHROPIC_BASE_URL"] = serde_json::Value::String(base_url);
            }
            let out = serde_json::to_string_pretty(&v).map_err(|e| e.to_string())?;
            std::fs::write(&path, out).map_err(|e| e.to_string())?;
            Ok(format!("Reconfigured {}", agent_name))
        }
        "Codex" => {
            // ponytail: TOML rewrite via regex; proper toml crate when edge cases hit
            let path = home_path.join(".codex").join("config.toml");
            let content = std::fs::read_to_string(&path).map_err(|e| e.to_string())?;
            let mut out = content.clone();
            if out.contains("[model_providers.9router]") {
                let re_base = regex::Regex::new(r#"(?m)(^\s*base_url\s*=\s*").*(")"#).unwrap();
                out = re_base
                    .replace_all(&out, |caps: &regex::Captures| {
                        format!("{}{}{}", &caps[1], base_url, &caps[2])
                    })
                    .to_string();
            }
            std::fs::write(&path, out).map_err(|e| e.to_string())?;
            Ok(format!("Reconfigured {} (base_url only; key via env)", agent_name))
        }
        _ => Err(format!("Agent '{}' not supported for reconfiguration", agent_name)),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubscriptionInfo {
    pub plan: String,
    pub used: f64,
    pub limit: f64,
    pub reset_date: String,
}

#[tauri::command]
async fn fetch_subscription(token: String, base_url: String) -> Result<SubscriptionInfo, String> {
    // ponytail: hardcoded response shape; adapt when new-api exposes real /api/user/self quota fields
    let url = format!("{}/api/user/self", base_url.trim_end_matches('/'));
    let client = reqwest::Client::new();
    let resp = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", token))
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if !resp.status().is_success() {
        return Err(format!("HTTP {}", resp.status()));
    }

    let body: serde_json::Value = resp.json().await.map_err(|e| e.to_string())?;

    Ok(SubscriptionInfo {
        plan: body["data"]["role"].as_str().unwrap_or("user").to_string(),
        used: body["data"]["used_quota"].as_f64().unwrap_or(0.0),
        limit: body["data"]["quota"].as_f64().unwrap_or(0.0),
        reset_date: body["data"]["reset_at"]
            .as_str()
            .unwrap_or("unknown")
            .to_string(),
    })
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            scan_agents,
            reconfigure_agent,
            fetch_subscription
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

