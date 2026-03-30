/// Connecteur Cloudflare — @cloudflare/mcp-server-cloudflare (officiel Cloudflare, 3.6k stars, 89 tools)
/// Config : ~/.osmozzz/cloudflare.toml
use super::LazyProxy;

pub struct CloudflareConfig {
    pub api_token:  String,
    pub account_id: String,
}

impl CloudflareConfig {
    pub fn load() -> Option<Self> {
        let path = dirs_next::home_dir()?.join(".osmozzz/cloudflare.toml");
        let content = std::fs::read_to_string(path).ok()?;
        let table: toml::Value = content.parse().ok()?;
        Some(Self {
            api_token:  table.get("api_token")?.as_str()?.to_string(),
            account_id: table.get("account_id")?.as_str()?.to_string(),
        })
    }
}

pub fn lazy() -> Option<LazyProxy> {
    let cfg = CloudflareConfig::load().or_else(|| {
        eprintln!("[OSMOzzz MCP] Cloudflare non configuré (~/.osmozzz/cloudflare.toml absent)");
        None
    })?;

    Some(LazyProxy::new_with_args(
        "cloudflare",
        "@cloudflare/mcp-server-cloudflare",
        vec![
            ("CLOUDFLARE_API_TOKEN".to_string(), cfg.api_token),
        ],
        vec![
            "run".to_string(),
            cfg.account_id,
        ],
    ))
}
