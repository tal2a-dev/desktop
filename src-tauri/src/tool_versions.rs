//! Local `--version` probes + npm/GitHub latest lookups for the About tab.

use serde::Serialize;
use std::path::PathBuf;
use std::time::Duration;

const PROBE_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Debug, Clone, Serialize)]
pub struct ToolVersion {
    pub name: String,
    pub version: Option<String>,
    pub latest_version: Option<String>,
    pub error: Option<String>,
    pub installed_but_broken: bool,
    pub install_path: Option<String>,
}

fn extra_path() -> String {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    let extras = [
        home.join(".local/bin"),
        home.join(".bun/bin"),
        PathBuf::from("/opt/homebrew/bin"),
        PathBuf::from("/usr/local/bin"),
    ];
    let sep = if cfg!(windows) { ";" } else { ":" };
    let extra = extras
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join(sep);
    let inherited = std::env::var("PATH").unwrap_or_default();
    if inherited.is_empty() {
        extra
    } else {
        format!("{extra}{sep}{inherited}")
    }
}

fn valid_tool_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 64
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | '@' | '/'))
}

fn run_login_shell(script: &str) -> std::io::Result<std::process::Output> {
    #[cfg(target_os = "windows")]
    {
        crate::shell_command(script)
            .env("PATH", extra_path())
            .output()
    }
    #[cfg(not(target_os = "windows"))]
    {
        crate::silent_command("sh")
            .args(["-lc", script])
            .env("PATH", extra_path())
            .output()
    }
}

/// First `major.minor` / `major.minor.patch` token in stdout/stderr.
pub fn extract_semver(text: &str) -> Option<String> {
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i].is_ascii_digit() {
            let start = i;
            let mut dots = 0;
            while i < bytes.len() {
                let c = bytes[i];
                if c.is_ascii_digit() {
                    i += 1;
                } else if c == b'.' {
                    dots += 1;
                    i += 1;
                } else {
                    break;
                }
            }
            let mut end = i;
            if end > start && end < bytes.len() && bytes[end] == b'-' {
                end += 1;
                while end < bytes.len()
                    && (bytes[end].is_ascii_alphanumeric()
                        || bytes[end] == b'.'
                        || bytes[end] == b'-')
                {
                    end += 1;
                }
            }
            if dots >= 1 {
                if let Ok(token) = std::str::from_utf8(&bytes[start..end]) {
                    let token = token.trim_end_matches('.').trim_end_matches('-');
                    if !token.is_empty() {
                        return Some(token.to_string());
                    }
                }
            }
        }
        i += 1;
    }
    None
}

fn probe_local(tool: &str) -> (Option<String>, Option<String>, bool, Option<String>) {
    if !valid_tool_name(tool) {
        return (None, Some(format!("invalid tool name: {tool}")), false, None);
    }

    #[cfg(target_os = "windows")]
    let locate = format!("where {tool}");
    #[cfg(not(target_os = "windows"))]
    let locate = format!("command -v {tool} || which {tool}");
    let which = run_login_shell(&locate);
    let install_path = which.ok().and_then(|out| {
        if !out.status.success() {
            return None;
        }
        let path = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if path.is_empty() {
            None
        } else {
            Some(path)
        }
    });

    let version_out = match run_login_shell(&format!("{tool} --version")) {
        Ok(out) => out,
        Err(e) => {
            return (
                None,
                Some(e.to_string()),
                install_path.is_some(),
                install_path,
            );
        }
    };

    let stdout = String::from_utf8_lossy(&version_out.stdout);
    let stderr = String::from_utf8_lossy(&version_out.stderr);
    let combined = if stdout.trim().is_empty() {
        stderr.as_ref()
    } else {
        stdout.as_ref()
    };

    if version_out.status.success() {
        let version = extract_semver(combined);
        if version.is_none() && combined.trim().is_empty() && install_path.is_none() {
            (
                None,
                Some("not installed".into()),
                false,
                None,
            )
        } else {
            (version, None, false, install_path)
        }
    } else {
        let code = version_out.status.code();
        let err = combined.trim();
        if install_path.is_some() || (code != Some(127) && !err.is_empty()) {
            let msg = if err.is_empty() {
                format!("{tool} --version failed")
            } else {
                err.lines().take(4).collect::<Vec<_>>().join("\n")
            };
            (None, Some(msg), true, install_path)
        } else {
            (None, Some("not installed".into()), false, install_path)
        }
    }
}

fn npm_package_for(tool: &str) -> Option<&'static str> {
    match tool {
        "claude" => Some("@anthropic-ai/claude-code"),
        "codex" => Some("@openai/codex"),
        "gemini" => Some("@google/gemini-cli"),
        "grok" => Some("@xai-official/grok"),
        "opencode" => Some("opencode-ai"),
        "openclaw" => Some("openclaw"),
        "pi" => Some("@earendil-works/pi-coding-agent"),
        "qwen" => Some("@qwen-code/qwen-code"),
        _ => None,
    }
}

async fn fetch_npm_latest(client: &reqwest::Client, package: &str) -> Option<String> {
    let url = format!(
        "https://registry.npmjs.org/-/package/{}/dist-tags",
        package.replace('/', "%2f")
    );
    let resp = client.get(&url).timeout(PROBE_TIMEOUT).send().await.ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let json: serde_json::Value = resp.json().await.ok()?;
    json.get("latest")
        .and_then(|v| v.as_str())
        .map(|s| s.trim_start_matches('v').to_string())
}

async fn fetch_github_latest_tag(client: &reqwest::Client, repo: &str) -> Option<String> {
    let url = format!("https://api.github.com/repos/{repo}/releases/latest");
    let resp = client
        .get(&url)
        .header("User-Agent", "tal2a")
        .header("Accept", "application/vnd.github+json")
        .timeout(PROBE_TIMEOUT)
        .send()
        .await
        .ok()?;
    if !resp.status().is_success() {
        return None;
    }
    let json: serde_json::Value = resp.json().await.ok()?;
    json.get("tag_name")
        .and_then(|v| v.as_str())
        .map(|s| s.trim_start_matches('v').to_string())
}

async fn fetch_latest(client: &reqwest::Client, tool: &str) -> Option<String> {
    if tool == "hermes" {
        return fetch_github_latest_tag(client, "NousResearch/hermes-agent").await;
    }
    let pkg = npm_package_for(tool)?;
    fetch_npm_latest(client, pkg).await
}

pub async fn get_tool_versions(tools: Vec<String>) -> Result<Vec<ToolVersion>, String> {
    let client = reqwest::Client::new();
    let mut results = Vec::with_capacity(tools.len());
    for tool in tools {
        let name = tool.trim().to_string();
        if name.is_empty() {
            continue;
        }
        let (version, error, installed_but_broken, install_path) = probe_local(&name);
        let latest_version = fetch_latest(&client, &name).await;
        results.push(ToolVersion {
            name,
            version,
            latest_version,
            error,
            installed_but_broken,
            install_path,
        });
    }
    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_semver_from_cli_banners() {
        assert_eq!(extract_semver("claude 2.1.156"), Some("2.1.156".into()));
        assert_eq!(extract_semver("v0.21.0 (build)"), Some("0.21.0".into()));
        assert_eq!(
            extract_semver("opencode 1.0.0-beta.2"),
            Some("1.0.0-beta.2".into())
        );
        assert_eq!(extract_semver("not a version"), None);
    }
}
