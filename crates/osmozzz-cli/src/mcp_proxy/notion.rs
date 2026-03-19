/// Connecteur Notion — @notionhq/notion-mcp-server (officiel)
/// Config : ~/.osmozzz/notion.toml
use super::McpSubprocess;

pub struct NotionConfig {
    pub token: String,
}

impl NotionConfig {
    pub fn load() -> Option<Self> {
        let path = dirs_next::home_dir()?.join(".osmozzz/notion.toml");
        let content = std::fs::read_to_string(path).ok()?;
        let table: toml::Value = content.parse().ok()?;
        Some(Self {
            token: table.get("token")?.as_str()?.to_string(),
        })
    }
}

pub fn start() -> Option<McpSubprocess> {
    let cfg = NotionConfig::load().or_else(|| {
        eprintln!("[OSMOzzz MCP] Notion non configuré (~/.osmozzz/notion.toml absent)");
        None
    })?;

    McpSubprocess::start(
        "notion",
        "@notionhq/notion-mcp-server",
        &[
            ("OPENAPI_MCP_HEADERS", &format!(
                "{{\"Authorization\": \"Bearer {}\", \"Notion-Version\": \"2022-06-28\"}}",
                cfg.token
            )),
        ],
    )
}
