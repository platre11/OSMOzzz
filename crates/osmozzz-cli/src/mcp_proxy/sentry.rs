/// Connecteur Sentry — @sentry/mcp-server (officiel Sentry, 612 stars, 21 tools)
/// Config : ~/.osmozzz/sentry.toml
use super::LazyProxy;

pub struct SentryConfig {
    pub token: String,
    pub host:  Option<String>,
}

impl SentryConfig {
    pub fn load() -> Option<Self> {
        let path = dirs_next::home_dir()?.join(".osmozzz/sentry.toml");
        let content = std::fs::read_to_string(path).ok()?;
        let table: toml::Value = content.parse().ok()?;
        let token = table.get("token")?.as_str()?.to_string();
        let host = table.get("host").and_then(|v| v.as_str()).map(|s| s.to_string());
        Some(Self { token, host })
    }
}

pub fn lazy() -> Option<LazyProxy> {
    let cfg = SentryConfig::load().or_else(|| {
        eprintln!("[OSMOzzz MCP] Sentry non configuré (~/.osmozzz/sentry.toml absent)");
        None
    })?;

    let mut extra_args = vec![
        format!("--access-token={}", cfg.token),
    ];
    if let Some(host) = cfg.host {
        extra_args.push(format!("--host={}", host));
    }

    Some(LazyProxy::new_with_args(
        "sentry",
        "@sentry/mcp-server",
        vec![],
        extra_args,
    ))
}
