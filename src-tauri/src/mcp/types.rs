use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AppType {
    Claude,
    Cursor,
    Codex,
    Gemini,
    GrokBuild,
    OpenCode,
    Hermes,
}

impl AppType {
    pub const ALL: [AppType; 7] = [
        AppType::Claude,
        AppType::Cursor,
        AppType::Codex,
        AppType::Gemini,
        AppType::GrokBuild,
        AppType::OpenCode,
        AppType::Hermes,
    ];

    pub fn as_str(self) -> &'static str {
        match self {
            AppType::Claude => "claude",
            AppType::Cursor => "cursor",
            AppType::Codex => "codex",
            AppType::Gemini => "gemini",
            AppType::GrokBuild => "grokbuild",
            AppType::OpenCode => "opencode",
            AppType::Hermes => "hermes",
        }
    }

    pub fn display_name(self) -> &'static str {
        match self {
            AppType::Claude => "Claude Code",
            AppType::Cursor => "Cursor",
            AppType::Codex => "Codex",
            AppType::Gemini => "Gemini",
            AppType::GrokBuild => "Grok Build",
            AppType::OpenCode => "OpenCode",
            AppType::Hermes => "Hermes",
        }
    }
}

impl FromStr for AppType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_lowercase().as_str() {
            "claude" | "claude-code" => Ok(AppType::Claude),
            "cursor" => Ok(AppType::Cursor),
            "codex" => Ok(AppType::Codex),
            "gemini" => Ok(AppType::Gemini),
            "grokbuild" | "grok" => Ok(AppType::GrokBuild),
            "opencode" => Ok(AppType::OpenCode),
            "hermes" => Ok(AppType::Hermes),
            other => Err(format!("Unknown MCP app: {other}")),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
pub struct McpApps {
    #[serde(default)]
    pub claude: bool,
    #[serde(default)]
    pub cursor: bool,
    #[serde(default)]
    pub codex: bool,
    #[serde(default)]
    pub gemini: bool,
    #[serde(default)]
    pub grokbuild: bool,
    #[serde(default)]
    pub opencode: bool,
    #[serde(default)]
    pub hermes: bool,
}

impl McpApps {
    pub fn is_enabled_for(&self, app: AppType) -> bool {
        match app {
            AppType::Claude => self.claude,
            AppType::Cursor => self.cursor,
            AppType::Codex => self.codex,
            AppType::Gemini => self.gemini,
            AppType::GrokBuild => self.grokbuild,
            AppType::OpenCode => self.opencode,
            AppType::Hermes => self.hermes,
        }
    }

    pub fn set_enabled_for(&mut self, app: AppType, enabled: bool) {
        match app {
            AppType::Claude => self.claude = enabled,
            AppType::Cursor => self.cursor = enabled,
            AppType::Codex => self.codex = enabled,
            AppType::Gemini => self.gemini = enabled,
            AppType::GrokBuild => self.grokbuild = enabled,
            AppType::OpenCode => self.opencode = enabled,
            AppType::Hermes => self.hermes = enabled,
        }
    }

    pub fn enabled_apps(&self) -> Vec<AppType> {
        AppType::ALL
            .into_iter()
            .filter(|app| self.is_enabled_for(*app))
            .collect()
    }

    pub fn for_app(app: AppType) -> Self {
        let mut apps = Self::default();
        apps.set_enabled_for(app, true);
        apps
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServer {
    pub id: String,
    pub name: String,
    pub server: serde_json::Value,
    pub apps: McpApps,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub homepage: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub docs: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub tags: Vec<String>,
}

impl McpServer {
    pub fn new(id: String, spec: serde_json::Value, apps: McpApps) -> Self {
        let name = spec
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or(&id)
            .to_string();
        Self {
            id: id.clone(),
            name,
            server: spec,
            apps,
            description: None,
            homepage: None,
            docs: None,
            tags: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScannedMcpServer {
    pub name: String,
    pub agent: String,
    pub kind: String,
    pub target: String,
}
