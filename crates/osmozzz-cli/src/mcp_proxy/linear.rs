/// Connecteur Linear — @linear/mcp-server (officiel Linear)
/// Config : ~/.osmozzz/linear.toml
use super::McpSubprocess;

pub struct LinearConfig {
    pub api_key: String,
}

impl LinearConfig {
    pub fn load() -> Option<Self> {
        let path = dirs_next::home_dir()?.join(".osmozzz/linear.toml");
        let content = std::fs::read_to_string(path).ok()?;
        let table: toml::Value = content.parse().ok()?;
        Some(Self {
            api_key: table.get("api_key")?.as_str()?.to_string(),
        })
    }
}

pub fn start() -> Option<McpSubprocess> {
    let cfg = LinearConfig::load().or_else(|| {
        eprintln!("[OSMOzzz MCP] Linear non configuré (~/.osmozzz/linear.toml absent)");
        None
    })?;

    McpSubprocess::start(
        "linear",
        "@tacticlaunch/mcp-linear",
        &[
            ("LINEAR_API_TOKEN", &cfg.api_key),
        ],
    )
}
