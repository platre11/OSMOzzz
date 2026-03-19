/// Connecteur Jira — @aashari/mcp-server-atlassian-jira
/// Config : ~/.osmozzz/jira.toml
use super::McpSubprocess;

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

pub fn start() -> Option<McpSubprocess> {
    let cfg = JiraConfig::load().or_else(|| {
        eprintln!("[OSMOzzz MCP] Jira non configuré (~/.osmozzz/jira.toml absent)");
        None
    })?;

    // Le package attend le nom du site uniquement (sans https://)
    let site_name = cfg.base_url
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .split('.')
        .next()
        .unwrap_or(&cfg.base_url)
        .to_string();

    McpSubprocess::start(
        "jira",
        "@aashari/mcp-server-atlassian-jira",
        &[
            ("ATLASSIAN_SITE_NAME",  &site_name),
            ("ATLASSIAN_USER_EMAIL", &cfg.email),
            ("ATLASSIAN_API_TOKEN",  &cfg.token),
        ],
    )
}
