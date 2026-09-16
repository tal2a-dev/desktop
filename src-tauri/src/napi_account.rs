//! NAPI token desk + weekly quota_data for the signed-in user.
//! Keys live in the `tokens` table; weekly usage is aggregated `quota_data`.

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::{
    agent_write, http_get_auth_with_retry, http_get_with_retry, persist_api_key, quota_to_usd,
    stored_token, validate_base_url, NOT_SIGNED_IN,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiKeyInfo {
    pub id: i64,
    pub name: String,
    pub status: i64,
    pub remain_quota: f64,
    pub used_quota: f64,
    pub unlimited_quota: bool,
    pub expired_time: i64,
    pub key_masked: String,
    pub selected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WeeklyPoint {
    pub created_at: i64,
    pub model_name: String,
    pub quota: f64,
    pub count: f64,
    pub token_used: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WeeklyUsage {
    pub start: i64,
    pub end: i64,
    pub used_usd: f64,
    pub used_quota: f64,
    pub request_count: f64,
    pub token_used: f64,
    pub points: Vec<WeeklyPoint>,
}

fn items_array(body: &Value) -> Option<&Vec<Value>> {
    body["data"]["items"]
        .as_array()
        .or_else(|| body["data"].as_array())
}

fn f64_field(v: &Value, keys: &[&str]) -> f64 {
    for k in keys {
        if let Some(n) = v[k].as_f64() {
            return n;
        }
        if let Some(n) = v[k].as_i64() {
            return n as f64;
        }
    }
    0.0
}

fn i64_field(v: &Value, keys: &[&str]) -> i64 {
    for k in keys {
        if let Some(n) = v[k].as_i64() {
            return n;
        }
        if let Some(n) = v[k].as_f64() {
            return n as i64;
        }
    }
    0
}

pub async fn list_tokens(base_url: &str) -> Result<Vec<ApiKeyInfo>, String> {
    validate_base_url(base_url)?;
    let token = stored_token()?.ok_or(NOT_SIGNED_IN)?;
    let base = base_url.trim_end_matches('/');
    let client = reqwest::Client::new();
    let url = format!("{base}/api/token/?p=1&size=100");
    let resp = http_get_auth_with_retry(&client, &url, &token).await?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {} listing API keys", resp.status()));
    }
    let body: Value = resp.json().await.map_err(|e| e.to_string())?;
    if body["success"].as_bool() == Some(false) {
        return Err(body["message"]
            .as_str()
            .unwrap_or("Couldn't list API keys")
            .to_string());
    }
    let selected = agent_write::load_app_settings().selected_token_id;
    let empty: Vec<Value> = Vec::new();
    let items = items_array(&body).unwrap_or(&empty);
    Ok(items
        .iter()
        .map(|t| {
            let id = i64_field(t, &["id"]);
            ApiKeyInfo {
                id,
                name: t["name"].as_str().unwrap_or("unnamed").to_string(),
                status: i64_field(t, &["status"]),
                remain_quota: f64_field(t, &["remain_quota"]),
                used_quota: f64_field(t, &["used_quota"]),
                unlimited_quota: t["unlimited_quota"].as_bool().unwrap_or(false),
                expired_time: i64_field(t, &["expired_time"]),
                key_masked: t["key"].as_str().unwrap_or("").to_string(),
                selected: selected == Some(id),
            }
        })
        .collect())
}

async fn fetch_token_secret(base: &str, session: &str, id: i64) -> Result<String, String> {
    let client = reqwest::Client::new();
    let url = format!("{base}/api/token/{id}/key");
    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {session}"))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {} revealing API key", resp.status()));
    }
    let body: Value = resp.json().await.map_err(|e| e.to_string())?;
    if body["success"].as_bool() == Some(false) {
        return Err(body["message"]
            .as_str()
            .unwrap_or("Couldn't reveal API key")
            .to_string());
    }
    body["data"]["key"]
        .as_str()
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .ok_or_else(|| "API key reveal returned no secret".to_string())
}

pub async fn select_token(base_url: &str, token_id: i64) -> Result<ApiKeyInfo, String> {
    validate_base_url(base_url)?;
    if token_id <= 0 {
        return Err("Pick an API key".into());
    }
    let session = stored_token()?.ok_or(NOT_SIGNED_IN)?;
    let base = base_url.trim_end_matches('/');
    let raw = fetch_token_secret(base, &session, token_id).await?;
    persist_api_key(&raw)?;
    let mut settings = agent_write::load_app_settings();
    settings.selected_token_id = Some(token_id);
    agent_write::save_app_settings(&settings)?;
    let keys = list_tokens(base_url).await?;
    keys.into_iter()
        .find(|k| k.id == token_id)
        .ok_or_else(|| "Selected key is not on this account".to_string())
}

pub async fn ensure_desktop_key(base_url: &str) -> Result<(), String> {
    validate_base_url(base_url)?;
    let keys = list_tokens(base_url).await?;
    let settings = agent_write::load_app_settings();
    let have_secret = crate::stored_api_key()?.is_some();
    if let Some(id) = settings.selected_token_id {
        if keys.iter().any(|k| k.id == id && k.status == 1) {
            if have_secret {
                return Ok(());
            }
            select_token(base_url, id).await?;
            return Ok(());
        }
    }
    if let Some(ours) = keys
        .iter()
        .filter(|k| k.name == "napi-desktop" && k.status == 1)
        .max_by_key(|k| k.id)
    {
        if have_secret {
            return Ok(());
        }
        select_token(base_url, ours.id).await?;
        return Ok(());
    }
    if let Some(first) = keys.iter().filter(|k| k.status == 1).max_by_key(|k| k.id) {
        if have_secret {
            return Ok(());
        }
        select_token(base_url, first.id).await?;
        return Ok(());
    }

    let session = stored_token()?.ok_or(NOT_SIGNED_IN)?;
    let base = base_url.trim_end_matches('/');
    let client = reqwest::Client::new();
    let resp = client
        .post(format!("{base}/api/token/"))
        .header("Authorization", format!("Bearer {session}"))
        .json(&serde_json::json!({
            "name": "napi-desktop",
            "remain_quota": 0,
            "unlimited_quota": true,
            "expired_time": -1
        }))
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {} creating API key", resp.status()));
    }
    let body: Value = resp.json().await.map_err(|e| e.to_string())?;
    if body["success"].as_bool() == Some(false) {
        return Err(body["message"]
            .as_str()
            .unwrap_or("Couldn't create API key")
            .to_string());
    }
    let created = list_tokens(base_url).await?;
    let ours = created
        .iter()
        .filter(|k| k.name == "napi-desktop")
        .max_by_key(|k| k.id)
        .ok_or("Token created but not returned — create it in the web console")?;
    select_token(base_url, ours.id).await?;
    Ok(())
}

async fn quota_per_unit(client: &reqwest::Client, base: &str) -> f64 {
    match http_get_with_retry(client, &format!("{base}/api/status")).await {
        Ok(resp) => match resp.json::<Value>().await {
            Ok(status) => status["data"]["quota_per_unit"]
                .as_f64()
                .filter(|v| *v > 0.0)
                .unwrap_or(500_000.0),
            Err(_) => 500_000.0,
        },
        Err(_) => 500_000.0,
    }
}

pub async fn weekly_usage(base_url: &str) -> Result<WeeklyUsage, String> {
    validate_base_url(base_url)?;
    let session = stored_token()?.ok_or(NOT_SIGNED_IN)?;
    let base = base_url.trim_end_matches('/');
    let client = reqwest::Client::new();
    let end = chrono::Utc::now().timestamp();
    let start = end - 7 * 86400;
    let url = format!("{base}/api/data/self?start_timestamp={start}&end_timestamp={end}");
    let resp = http_get_auth_with_retry(&client, &url, &session).await?;
    if !resp.status().is_success() {
        return Err(format!("HTTP {} fetching weekly usage", resp.status()));
    }
    let body: Value = resp.json().await.map_err(|e| e.to_string())?;
    if body["success"].as_bool() == Some(false) {
        return Err(body["message"]
            .as_str()
            .unwrap_or("Couldn't load weekly usage")
            .to_string());
    }
    let points: Vec<WeeklyPoint> = body["data"]
        .as_array()
        .map(|rows| {
            rows.iter()
                .map(|row| WeeklyPoint {
                    created_at: i64_field(row, &["created_at"]),
                    model_name: row["model_name"].as_str().unwrap_or("").to_string(),
                    quota: f64_field(row, &["quota"]),
                    count: f64_field(row, &["count"]),
                    token_used: f64_field(row, &["token_used"]),
                })
                .collect()
        })
        .unwrap_or_default();
    let used_quota: f64 = points.iter().map(|p| p.quota).sum();
    let request_count: f64 = points.iter().map(|p| p.count).sum();
    let token_used: f64 = points.iter().map(|p| p.token_used).sum();
    let per_unit = quota_per_unit(&client, base).await;
    Ok(WeeklyUsage {
        start,
        end,
        used_usd: quota_to_usd(used_quota, per_unit),
        used_quota,
        request_count,
        token_used,
        points,
    })
}

#[tauri::command]
pub async fn list_api_keys(base_url: String) -> Result<Vec<ApiKeyInfo>, String> {
    list_tokens(&base_url).await
}

#[tauri::command]
pub async fn select_api_key(base_url: String, token_id: i64) -> Result<ApiKeyInfo, String> {
    select_token(&base_url, token_id).await
}

#[tauri::command]
pub async fn fetch_weekly_usage(base_url: String) -> Result<WeeklyUsage, String> {
    weekly_usage(&base_url).await
}
