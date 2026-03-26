/// Connecteur Supabase — @supabase/mcp-server-supabase (officiel)
/// Config : ~/.osmozzz/supabase.toml
use super::LazyProxy;

pub struct SupabaseConfig {
    pub access_token: String,
    pub project_id:   Option<String>,
}

impl SupabaseConfig {
    pub fn load() -> Option<Self> {
        let path = dirs_next::home_dir()?.join(".osmozzz/supabase.toml");
        let content = std::fs::read_to_string(path).ok()?;
        let table: toml::Value = content.parse().ok()?;
        Some(Self {
            access_token: table.get("access_token")?.as_str()?.to_string(),
            project_id:   table.get("project_id").and_then(|v| v.as_str()).map(String::from),
        })
    }
}

pub fn lazy() -> Option<LazyProxy> {
    let cfg = SupabaseConfig::load().or_else(|| {
        eprintln!("[OSMOzzz MCP] Supabase non configuré (~/.osmozzz/supabase.toml absent)");
        None
    })?;

    let mut env_vars = vec![
        ("SUPABASE_ACCESS_TOKEN".to_string(), cfg.access_token),
    ];
    if let Some(pid) = cfg.project_id {
        env_vars.push(("SUPABASE_PROJECT_REF".to_string(), pid));
    }

    Some(LazyProxy::new(
        "supabase",
        "@supabase/mcp-server-supabase",
        env_vars,
    ))
}
