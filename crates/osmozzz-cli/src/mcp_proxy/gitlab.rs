/// Connecteur GitLab — @zereight/mcp-gitlab (1300+ stars, 115 tools)
/// Config : ~/.osmozzz/gitlab.toml
use super::LazyProxy;

pub struct GitlabConfig {
    pub token:    String,
    pub base_url: String,
}

impl GitlabConfig {
    pub fn load() -> Option<Self> {
        let path = dirs_next::home_dir()?.join(".osmozzz/gitlab.toml");
        let content = std::fs::read_to_string(path).ok()?;
        let table: toml::Value = content.parse().ok()?;
        let token = table.get("token")?.as_str()?.to_string();
        let base_url = table.get("base_url")
            .and_then(|v| v.as_str())
            .unwrap_or("https://gitlab.com")
            .to_string();
        Some(Self { token, base_url })
    }
}

pub fn lazy() -> Option<LazyProxy> {
    let cfg = GitlabConfig::load().or_else(|| {
        eprintln!("[OSMOzzz MCP] GitLab non configuré (~/.osmozzz/gitlab.toml absent)");
        None
    })?;

    let api_url = format!("{}/api/v4", cfg.base_url.trim_end_matches('/'));

    Some(LazyProxy::new(
        "gitlab",
        "@zereight/mcp-gitlab",
        vec![
            ("GITLAB_PERSONAL_ACCESS_TOKEN".to_string(), cfg.token),
            ("GITLAB_API_URL".to_string(), api_url),
        ],
    ))
}
