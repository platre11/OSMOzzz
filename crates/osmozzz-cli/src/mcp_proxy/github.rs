/// Connecteur GitHub — @modelcontextprotocol/server-github (officiel)
/// Config : ~/.osmozzz/github.toml
use super::LazyProxy;

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

pub fn lazy() -> Option<LazyProxy> {
    let cfg = GithubConfig::load().or_else(|| {
        eprintln!("[OSMOzzz MCP] GitHub non configuré (~/.osmozzz/github.toml absent)");
        None
    })?;

    Some(LazyProxy::new(
        "github",
        "@modelcontextprotocol/server-github",
        vec![
            ("GITHUB_PERSONAL_ACCESS_TOKEN".to_string(), cfg.token),
        ],
    ))
}
