/// Connecteur Supabase — @supabase/mcp-server-supabase (officiel)
/// Config : ~/.osmozzz/supabase.toml
use super::McpSubprocess;

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

pub fn start() -> Option<McpSubprocess> {
    let cfg = SupabaseConfig::load().or_else(|| {
        eprintln!("[OSMOzzz MCP] Supabase non configuré (~/.osmozzz/supabase.toml absent)");
        None
    })?;

    let mut env_vars: Vec<(&str, String)> = vec![
        ("SUPABASE_ACCESS_TOKEN", cfg.access_token.clone()),
    ];

    let project_ref_owned;
    if let Some(ref pid) = cfg.project_id {
        project_ref_owned = pid.clone();
        env_vars.push(("SUPABASE_PROJECT_REF", project_ref_owned));
    }

    let env_refs: Vec<(&str, &str)> = env_vars.iter().map(|(k, v)| (*k, v.as_str())).collect();

    McpSubprocess::start(
        "supabase",
        "@supabase/mcp-server-supabase",
        &env_refs,
    )
}
