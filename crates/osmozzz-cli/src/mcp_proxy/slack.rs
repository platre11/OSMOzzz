/// Connecteur Slack — @modelcontextprotocol/server-slack (officiel Anthropic)
/// Config : ~/.osmozzz/slack.toml
use super::LazyProxy;

pub struct SlackConfig {
    pub token:   String,
    pub team_id: String,
}

impl SlackConfig {
    pub fn load() -> Option<Self> {
        let path = dirs_next::home_dir()?.join(".osmozzz/slack.toml");
        let content = std::fs::read_to_string(path).ok()?;
        let table: toml::Value = content.parse().ok()?;
        Some(Self {
            token:   table.get("token")?.as_str()?.to_string(),
            team_id: table.get("team_id")?.as_str()?.to_string(),
        })
    }
}

pub fn lazy() -> Option<LazyProxy> {
    let cfg = SlackConfig::load().or_else(|| {
        eprintln!("[OSMOzzz MCP] Slack non configuré (~/.osmozzz/slack.toml absent)");
        None
    })?;

    Some(LazyProxy::new(
        "slack",
        "@modelcontextprotocol/server-slack",
        vec![
            ("SLACK_BOT_TOKEN".to_string(), cfg.token),
            ("SLACK_TEAM_ID".to_string(),   cfg.team_id),
        ],
    ))
}
