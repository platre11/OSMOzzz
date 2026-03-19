/// Connecteur Slack — @modelcontextprotocol/server-slack (officiel Anthropic)
/// Config : ~/.osmozzz/slack.toml
use super::McpSubprocess;

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

pub fn start() -> Option<McpSubprocess> {
    let cfg = SlackConfig::load().or_else(|| {
        eprintln!("[OSMOzzz MCP] Slack non configuré (~/.osmozzz/slack.toml absent)");
        None
    })?;

    McpSubprocess::start(
        "slack",
        "@modelcontextprotocol/server-slack",
        &[
            ("SLACK_BOT_TOKEN", &cfg.token),
            ("SLACK_TEAM_ID",   &cfg.team_id),
        ],
    )
}
