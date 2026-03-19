/// Connecteur GitHub — @modelcontextprotocol/server-github (officiel)
/// Config : ~/.osmozzz/github.toml
use super::McpSubprocess;

pub struct GithubConfig {
    pub token: String,
}

impl GithubConfig {
    pub fn load() -> Option<Self> {
        let path = dirs_next::home_dir()?.join(".osmozzz/github.toml");
        let content = std::fs::read_to_string(path).ok()?;
        let table: toml::Value = content.parse().ok()?;
        Some(Self {
            token: table.get("token")?.as_str()?.to_string(),
        })
    }
}

pub fn start() -> Option<McpSubprocess> {
    let cfg = GithubConfig::load().or_else(|| {
        eprintln!("[OSMOzzz MCP] GitHub non configuré (~/.osmozzz/github.toml absent)");
        None
    })?;

    McpSubprocess::start(
        "github",
        "@modelcontextprotocol/server-github",
        &[
            ("GITHUB_PERSONAL_ACCESS_TOKEN", &cfg.token),
        ],
    )
}
