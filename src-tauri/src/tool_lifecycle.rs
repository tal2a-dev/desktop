//! CLI install/update commands copied from cc-switch `commands/misc.rs`
//! (`npm_install_command_for`, `HERMES_INSTALL_UNIX`, `official_update_args`).

use std::process::Command;

const CLAUDE_INSTALL_UNIX: &str =
    "bash -c 'tmp=$(mktemp) && curl -fsSL https://claude.ai/install.sh -o $tmp && bash $tmp; status=$?; rm -f $tmp; exit $status'";
const OPENCODE_INSTALL_UNIX: &str =
    "bash -c 'tmp=$(mktemp) && curl -fsSL https://opencode.ai/install -o $tmp && bash $tmp; status=$?; rm -f $tmp; exit $status'";
const GROK_INSTALL_UNIX: &str =
    "bash -c 'tmp=$(mktemp) && curl -fsSL https://x.ai/cli/install.sh -o $tmp && bash $tmp; status=$?; rm -f $tmp; exit $status'";
const HERMES_INSTALL_UNIX: &str =
    "bash -c 'tmp=$(mktemp) && curl -fsSL https://raw.githubusercontent.com/NousResearch/hermes-agent/main/scripts/install.sh -o $tmp && bash $tmp; status=$?; rm -f $tmp; exit $status'";
const HERMES_UPDATE_UNIX: &str =
    "hermes update || bash -c 'tmp=$(mktemp) && curl -fsSL https://raw.githubusercontent.com/NousResearch/hermes-agent/main/scripts/install.sh -o $tmp && bash $tmp; status=$?; rm -f $tmp; exit $status'";

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

fn npm_install_command_for(tool: &str) -> Option<String> {
    npm_package_for(tool).map(|pkg| format!("npm i -g {pkg}@latest"))
}

fn installer_or_npm(installer: &str, tool: &str) -> String {
    match npm_install_command_for(tool) {
        Some(npm) => format!("{installer} || {npm}"),
        None => installer.to_string(),
    }
}

fn install_command(tool: &str) -> Option<String> {
    Some(match tool {
        "claude" => installer_or_npm(CLAUDE_INSTALL_UNIX, tool),
        "grok" => installer_or_npm(GROK_INSTALL_UNIX, tool),
        "opencode" => installer_or_npm(OPENCODE_INSTALL_UNIX, tool),
        "hermes" => HERMES_INSTALL_UNIX.to_string(),
        other => npm_install_command_for(other)?,
    })
}

fn update_command(tool: &str) -> Option<String> {
    if tool == "hermes" {
        return Some(HERMES_UPDATE_UNIX.to_string());
    }
    let install = npm_install_command_for(tool)?;
    let update = match tool {
        "claude" | "codex" | "grok" => format!("{tool} update || {install}"),
        "opencode" => format!("opencode upgrade || {install}"),
        "openclaw" => format!("openclaw update --yes || {install}"),
        _ => install.to_string(),
    };
    Some(update)
}

pub fn can_install(tool: &str) -> bool {
    install_command(tool).is_some()
}

pub fn can_uninstall(tool: &str) -> bool {
    matches!(
        tool,
        "claude"
            | "codex"
            | "cline"
            | "opencode"
            | "gemini"
            | "cursor"
            | "continue"
            | "goose"
            | "factory"
            | "grok"
            | "roo"
            | "kilocode"
            | "hermes"
            | "qwen"
            | "openclaw"
            | "pi"
            | "windsurf"
    )
}

fn brew_formulae(tool: &str) -> &'static [&'static str] {
    match tool {
        "codex" => &["codex"],
        "opencode" => &["opencode"],
        _ => &[],
    }
}

fn brew_casks(tool: &str) -> &'static [&'static str] {
    match tool {
        "claude" => &["claude-code"],
        _ => &[],
    }
}

pub(crate) fn desktop_app_paths(tool: &str) -> Vec<std::path::PathBuf> {
    let home = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    let apps = std::path::PathBuf::from("/Applications");
    match tool {
        "codex" => vec![
            apps.join("ChatGPT.app"),
            home.join(".codex")
                .join("computer-use")
                .join("Codex Computer Use.app"),
        ],
        "claude" | "claude-desktop" => vec![apps.join("Claude.app"), apps.join("ClaudeBar.app")],
        "cursor" => vec![apps.join("Cursor.app")],
        "windsurf" => vec![apps.join("Windsurf.app")],
        "continue" => vec![],
        _ => vec![],
    }
}

fn user_bin_dirs() -> Vec<std::path::PathBuf> {
    let home = dirs::home_dir().unwrap_or_else(|| std::path::PathBuf::from("."));
    vec![
        home.join(".local/bin"),
        home.join("bin"),
        home.join(".bun/bin"),
        home.join(".cargo/bin"),
        home.join(".grok/bin"),
        home.join(".opencode/bin"),
        std::path::PathBuf::from("/opt/homebrew/bin"),
        std::path::PathBuf::from("/usr/local/bin"),
    ]
}

fn try_shell(line: &str, notes: &mut Vec<String>, label: &str) {
    match run_shell(line) {
        Ok(out) if !out.is_empty() => notes.push(out),
        Ok(_) => notes.push(format!("{label}: ok")),
        Err(e) => notes.push(format!("{label}: {e}")),
    }
}

fn brew_available() -> bool {
    Command::new("brew").arg("--version").output().is_ok()
}

#[cfg(target_os = "macos")]
fn quit_mac_app(name: &str) {
    let _ = Command::new("osascript")
        .args(["-e", &format!("tell application \"{name}\" to quit")])
        .output();
}

#[cfg(target_os = "macos")]
fn trash_mac_path(path: &std::path::Path, notes: &mut Vec<String>) {
    if !path.exists() {
        return;
    }
    let posix = path.to_string_lossy().replace('\\', "\\\\").replace('"', "\\\"");
    let script = format!("tell application \"Finder\" to delete (POSIX file \"{posix}\" as alias)");
    match Command::new("osascript").args(["-e", &script]).output() {
        Ok(out) if out.status.success() => {
            notes.push(format!("trashed {}", path.display()));
        }
        Ok(out) => {
            let err = String::from_utf8_lossy(&out.stderr);
            match std::fs::remove_dir_all(path).or_else(|_| std::fs::remove_file(path)) {
                Ok(()) => notes.push(format!("removed {}", path.display())),
                Err(e) => notes.push(format!("{}: {err} / {e}", path.display())),
            }
        }
        Err(e) => notes.push(format!("Finder: {e}")),
    }
}

/// Remove the CLI (Homebrew, npm, user bins) and the desktop app if we know it.
/// Config overlay is stripped by `agent_write::clear_napi` before this runs.
pub fn uninstall_tool(tool: &str, binary_name: &str) -> Result<String, String> {
    let mut notes = Vec::new();

    #[cfg(target_os = "macos")]
    {
        match tool {
            "codex" => {
                quit_mac_app("ChatGPT");
                quit_mac_app("Codex Computer Use");
            }
            "claude" => {
                quit_mac_app("Claude");
                quit_mac_app("ClaudeBar");
            }
            "cursor" => quit_mac_app("Cursor"),
            _ => {}
        }
        for app in desktop_app_paths(tool) {
            trash_mac_path(&app, &mut notes);
        }
    }

    if brew_available() {
        for formula in brew_formulae(tool) {
            try_shell(
                &format!("brew uninstall --formula --force {formula}"),
                &mut notes,
                &format!("brew {formula}"),
            );
        }
        for cask in brew_casks(tool) {
            try_shell(
                &format!("brew uninstall --cask --force {cask}"),
                &mut notes,
                &format!("brew cask {cask}"),
            );
        }
    }

    if let Some(pkg) = npm_package_for(tool) {
        try_shell(&format!("npm uninstall -g {pkg}"), &mut notes, "npm");
    }

    let exe = if cfg!(target_os = "windows") {
        format!("{binary_name}.exe")
    } else {
        binary_name.to_string()
    };
    for dir in user_bin_dirs() {
        let path = dir.join(&exe);
        if path.is_file() || path.is_symlink() {
            match std::fs::remove_file(&path) {
                Ok(()) => notes.push(format!("removed {}", path.display())),
                Err(e) => notes.push(format!("{}: {e}", path.display())),
            }
        }
    }

    if notes.is_empty() {
        Ok(format!("Uninstalled {tool} (nothing left to remove)"))
    } else {
        Ok(notes.join("; "))
    }
}

fn run_shell(line: &str) -> Result<String, String> {
    let output = Command::new("sh")
        .arg("-c")
        .arg(line)
        .output()
        .map_err(|e| e.to_string())?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if output.status.success() {
        Ok(stdout.trim().to_string())
    } else {
        Err(format!(
            "install failed ({}): {}",
            output.status,
            stderr.trim().if_empty(stdout.trim())
        ))
    }
}

trait IfEmpty {
    fn if_empty(self, fallback: &str) -> String;
}

impl IfEmpty for &str {
    fn if_empty(self, fallback: &str) -> String {
        if self.is_empty() {
            fallback.to_string()
        } else {
            self.to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uninstall_maps_codex_desktop_and_brew() {
        assert!(brew_formulae("codex").contains(&"codex"));
        assert!(can_uninstall("codex"));
        let apps = desktop_app_paths("codex");
        assert!(apps.iter().any(|p| p.ends_with("ChatGPT.app")));
        assert!(apps
            .iter()
            .any(|p| p.ends_with("Codex Computer Use.app")));
    }

    #[test]
    fn uninstall_maps_claude_cask_and_apps() {
        assert!(brew_casks("claude").contains(&"claude-code"));
        assert!(can_uninstall("cursor"));
        assert!(can_uninstall("continue"));
        assert!(can_uninstall("cline"));
        assert!(can_uninstall("windsurf"));
        assert!(desktop_app_paths("cursor")
            .iter()
            .any(|p| p.ends_with("Cursor.app")));
    }
}

pub fn install_tool(tool: &str) -> Result<String, String> {
    let line = install_command(tool).ok_or_else(|| {
        format!("{tool} cannot be installed from this app — install the CLI yourself")
    })?;
    let out = run_shell(&line)?;
    Ok(if out.is_empty() {
        format!("Installed {tool}")
    } else {
        out
    })
}

pub fn update_tool(tool: &str) -> Result<String, String> {
    let line = update_command(tool).ok_or_else(|| format!("{tool} has no update command"))?;
    let out = run_shell(&line)?;
    Ok(if out.is_empty() {
        format!("Updated {tool}")
    } else {
        out
    })
}
