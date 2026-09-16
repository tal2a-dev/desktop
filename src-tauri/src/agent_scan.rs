//! Probe CLIs, desktop apps, and editor extensions. Ignore ephemeral PATH shims
//! (cmux / Grok session `/var/folders/.../cmux-cli-shims`).

use std::path::{Path, PathBuf};
use std::process::Command;

use crate::agent_write;
use crate::home_dir;
use crate::tool_lifecycle;

pub struct Probe {
    pub cli: bool,
    pub desktop: bool,
    pub extension: bool,
    pub config: bool,
}

impl Probe {
    /// Launch/Uninstall only when a real CLI or desktop app is present.
    /// Editor extensions and leftover nvm globals do not count.
    pub fn installed(&self) -> bool {
        self.cli || self.desktop
    }

    pub fn detected(&self) -> bool {
        self.installed() || self.config || self.extension
    }
}

fn is_ephemeral(path: &Path) -> bool {
    let s = path.to_string_lossy();
    s.contains("/cmux-cli-shims/")
        || s.contains("/var/folders/")
        || s.contains("/tmp/")
        || s.contains("\\Temp\\")
        || s.contains("/.grok/sessions/")
}

pub fn bin_search_dirs() -> Vec<PathBuf> {
    let home = home_dir();
    vec![
        home.join(".local/bin"),
        home.join("bin"),
        home.join(".bun/bin"),
        home.join(".cargo/bin"),
        home.join(".volta/bin"),
        home.join(".asdf/shims"),
        home.join(".grok/bin"),
        home.join(".opencode/bin"),
        PathBuf::from("/opt/homebrew/bin"),
        PathBuf::from("/usr/local/bin"),
        PathBuf::from("/usr/bin"),
    ]
}

fn binaries_for<'a>(id: &'a str, binary_name: &'a str) -> Vec<&'a str> {
    match id {
        "cursor" => vec!["cursor", "agent"],
        "claude" => vec!["claude"],
        "claude-desktop" => vec![],
        "codex" => vec!["codex"],
        "opencode" => vec!["opencode"],
        "gemini" => vec!["gemini"],
        "grok" => vec!["grok"],
        "hermes" => vec!["hermes"],
        "qwen" => vec!["qwen"],
        "pi" => vec!["pi", "pi-ai"],
        "openclaw" => vec!["openclaw"],
        "goose" => vec!["goose"],
        "factory" => vec!["droid"],
        "windsurf" => vec!["windsurf"],
        _ => vec![binary_name],
    }
}

fn cli_in_dirs(names: &[&str]) -> bool {
    for dir in bin_search_dirs() {
        for name in names {
            let p = dir.join(name);
            if (p.is_file() || p.is_symlink()) && !is_ephemeral(&p) {
                return true;
            }
        }
    }
    false
}

fn which_real(name: &str) -> Option<PathBuf> {
    if name.is_empty()
        || !name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'))
    {
        return None;
    }
    // Do not inherit the app process PATH: Grok/cargo shells inject nvm + cmux
    // shims, which made leftover `gemini` look installed.
    let path = bin_search_dirs()
        .iter()
        .map(|p| p.display().to_string())
        .collect::<Vec<_>>()
        .join(":");
    let out = Command::new("sh")
        .env("PATH", &path)
        .arg("-c")
        .arg(format!("command -v {name}"))
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    let path = PathBuf::from(String::from_utf8_lossy(&out.stdout).trim());
    if path.as_os_str().is_empty() || is_ephemeral(&path) {
        return None;
    }
    if path.is_file() || path.is_symlink() {
        Some(path)
    } else {
        None
    }
}

pub fn cli_installed(id: &str, binary_name: &str) -> bool {
    let names = binaries_for(id, binary_name);
    if names.iter().any(|n| which_real(n).is_some()) {
        return true;
    }
    cli_in_dirs(&names)
}

fn desktop_installed(id: &str) -> bool {
    tool_lifecycle::desktop_app_paths(id)
        .iter()
        .any(|p| p.exists())
}

fn extension_dirs() -> Vec<PathBuf> {
    let home = home_dir();
    vec![
        home.join(".vscode/extensions"),
        home.join(".vscode-insiders/extensions"),
        home.join(".cursor/extensions"),
    ]
}

fn extension_needles(id: &str) -> &'static [&'static str] {
    match id {
        "claude" => &["anthropic.claude-code"],
        "cline" => &["saoudrizwan.claude-dev"],
        "continue" => &["continue.continue"],
        "roo" => &["rooveterinaryinc.roo-cline", "roo-code", "rooveterinary.roo"],
        "kilocode" => &["kilocode", "kilo-code"],
        "codex" => &["openai.chatgpt", "openai.codex"],
        _ => &[],
    }
}

fn extension_installed(id: &str) -> bool {
    let needles = extension_needles(id);
    if needles.is_empty() {
        return false;
    }
    for dir in extension_dirs() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for e in entries.flatten() {
            let name = e.file_name();
            let name = name.to_string_lossy().to_ascii_lowercase();
            if needles
                .iter()
                .any(|n| name.contains(&n.to_ascii_lowercase()))
            {
                return true;
            }
        }
    }
    false
}

fn extra_config_exists(id: &str) -> bool {
    let home = home_dir();
    match id {
        "claude" => home.join(".claude.json").is_file() || home.join(".claude").is_dir(),
        "claude-desktop" => {
            let (profile, meta) = agent_write::claude_desktop_paths_from_home(&home);
            profile.is_file() || meta.is_file()
        }
        "codex" => agent_write::config_dir_for("codex").is_dir(),
        "opencode" => {
            agent_write::config_dir_for("opencode").join("opencode.json").is_file()
                || home.join(".opencode").join("opencode.json").is_file()
        }
        "cursor" => home.join(".cursor").is_dir(),
        "continue" => home.join(".continue").is_dir(),
        "cline" => home.join(".cline").is_dir(),
        "gemini" => agent_write::config_dir_for("gemini").is_dir(),
        "grok" => agent_write::config_dir_for("grok").is_dir(),
        _ => false,
    }
}

pub fn probe(id: &str, binary_name: &str, config_path: &Path) -> Probe {
    Probe {
        cli: cli_installed(id, binary_name),
        desktop: desktop_installed(id),
        extension: extension_installed(id),
        config: config_path.exists() || extra_config_exists(id),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ephemeral_cmux_shim_is_rejected() {
        let p = PathBuf::from(
            "/var/folders/pn/65lbvk3x4hs9381b3d33_2sw0000gn/T/cmux-cli-shims/abc/codex",
        );
        assert!(is_ephemeral(&p));
        assert!(!is_ephemeral(&PathBuf::from("/opt/homebrew/bin/codex")));
        assert!(!is_ephemeral(&PathBuf::from("/Users/mikawi/.local/bin/claude")));
    }

    #[test]
    fn nvm_global_bins_are_not_search_dirs() {
        assert!(bin_search_dirs()
            .iter()
            .all(|p| !p.to_string_lossy().contains("/.nvm/versions/node/")));
    }
}
