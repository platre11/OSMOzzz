/// Connecteurs natifs Rust — Linear, Jira, GitLab, Vercel, Railway, Render (et futurs connecteurs).
///
/// Pattern :
///   - `tools()` → Vec<Value>  — définitions MCP exposées à Claude
///   - `handle(name, args)` → Result<String, String>  — exécution sans appel à send()
///
/// La couche sécurité (sanitize_proxy_response + audit_log) est appliquée
/// CENTRALEMENT dans le dispatcher de mcp.rs, après le retour du connecteur.
pub mod gcal;
pub mod gitlab;
pub mod stripe;
pub mod jira;
pub mod linear;
pub mod railway;
pub mod render;
pub mod vercel;

use serde_json::Value;

/// Agrège les définitions de tools de tous les connecteurs natifs.
pub fn all_tools() -> Vec<Value> {
    let mut tools = Vec::new();
    tools.extend(linear::tools());
    tools.extend(jira::tools());
    tools.extend(gitlab::tools());
    tools.extend(vercel::tools());
    tools.extend(railway::tools());
    tools.extend(render::tools());
    tools.extend(gcal::tools());
    tools.extend(stripe::tools());
    tools
}

/// Dispatche vers le bon connecteur selon le préfixe du nom d'outil.
/// Retourne None si le tool n'appartient à aucun connecteur natif.
pub async fn handle(name: &str, args: &Value) -> Option<Result<String, String>> {
    if name.starts_with("linear_")  { return Some(linear::handle(name, args).await); }
    if name.starts_with("jira_")    { return Some(jira::handle(name, args).await); }
    if name.starts_with("gitlab_")  { return Some(gitlab::handle(name, args).await); }
    if name.starts_with("vercel_")  { return Some(vercel::handle(name, args).await); }
    if name.starts_with("railway_") { return Some(railway::handle(name, args).await); }
    if name.starts_with("render_")  { return Some(render::handle(name, args).await); }
    if name.starts_with("gcal_")    { return Some(gcal::handle(name, args).await); }
    if name.starts_with("stripe_")  { return Some(stripe::handle(name, args).await); }
    None
}
