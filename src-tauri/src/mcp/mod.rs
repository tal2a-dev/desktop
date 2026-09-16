//! Unified MCP manager: JSON store + live writes into agent configs.
//!
//! Adapted from cc-switch `mcp/` + `services/mcp.rs` without the provider
//! switcher or SQLite. Canonical records live in
//! `~/.config/napi-desktop/mcp-servers.json` and are projected into
//! `~/.claude.json` / Codex / Gemini / Grok / OpenCode / Hermes / Cursor.

mod commands;
mod error;
mod live;
mod paths;
mod service;
mod store;
mod types;
mod validation;

pub use commands::{
    delete_mcp_server, fetch_registry_servers, get_mcp_servers, import_mcp_from_apps,
    install_mcp_server, scan_mcp_servers, toggle_mcp_app, upsert_mcp_server,
    validate_mcp_command,
};

#[cfg(test)]
pub use commands::{mcp_key, merge_mcp_server};
#[cfg(test)]
pub use validation::redact_url;
