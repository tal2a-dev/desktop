use std::fmt;
use std::io;
use std::path::Path;

#[derive(Debug)]
pub enum McpError {
    Message(String),
    Io { path: String, source: io::Error },
    Json { path: String, source: serde_json::Error },
}

impl McpError {
    pub fn msg(msg: impl Into<String>) -> Self {
        Self::Message(msg.into())
    }

    pub fn io(path: &Path, source: io::Error) -> Self {
        Self::Io {
            path: path.display().to_string(),
            source,
        }
    }

    pub fn json(path: &Path, source: serde_json::Error) -> Self {
        Self::Json {
            path: path.display().to_string(),
            source,
        }
    }
}

impl fmt::Display for McpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Message(m) => write!(f, "{m}"),
            Self::Io { path, source } => write!(f, "{path}: {source}"),
            Self::Json { path, source } => write!(f, "{path}: {source}"),
        }
    }
}

impl std::error::Error for McpError {}

impl From<McpError> for String {
    fn from(value: McpError) -> Self {
        value.to_string()
    }
}
