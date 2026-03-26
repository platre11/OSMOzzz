/// Connecteur Jira — @aashari/mcp-server-atlassian-jira
/// Config : ~/.osmozzz/jira.toml
use super::LazyProxy;

pub struct JiraConfig {
    pub base_url: String,
    pub email:    String,
    pub token:    String,
}

impl JiraConfig {
    pub fn load() -> Option<Self> {
        let path = dirs_next::home_dir()?.join(".osmozzz/jira.toml");
        let content = std::fs::read_to_string(path).ok()?;
        let table: toml::Value = content.parse().ok()?;
        Some(Self {
            base_url: table.get("base_url")?.as_str()?.to_string(),
            email:    table.get("email")?.as_str()?.to_string(),
            token:    table.get("token")?.as_str()?.to_string(),
        })
    }
}

pub fn lazy() -> Option<LazyProxy> {
    let cfg = JiraConfig::load().or_else(|| {
        eprintln!("[OSMOzzz MCP] Jira non configuré (~/.osmozzz/jira.toml absent)");
        None
    })?;

    let site_name = cfg.base_url
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .split('.')
        .next()
        .unwrap_or(&cfg.base_url)
        .to_string();

    Some(LazyProxy::new(
        "jira",
        "@aashari/mcp-server-atlassian-jira",
        vec![
            ("ATLASSIAN_SITE_NAME".to_string(),  site_name),
            ("ATLASSIAN_USER_EMAIL".to_string(), cfg.email),
            ("ATLASSIAN_API_TOKEN".to_string(),  cfg.token),
        ],
    ))
}
