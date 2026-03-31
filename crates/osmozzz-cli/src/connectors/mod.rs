/// Connecteurs natifs Rust — Linear & Jira (et futurs connecteurs).
///
/// Pattern :
///   - `tools()` → Vec<Value>  — définitions MCP exposées à Claude
///   - `handle(name, args)` → Result<String, String>  — exécution sans appel à send()
///
/// La couche sécurité (sanitize_proxy_response + audit_log) est appliquée
/// CENTRALEMENT dans le dispatcher de mcp.rs, après le retour du connecteur.
pub mod gitlab;
pub mod jira;
pub mod linear;

use serde_json::Value;

/// Agrège les définitions de tools de tous les connecteurs natifs.
pub fn all_tools() -> Vec<Value> {
    let mut tools = Vec::new();
    tools.extend(linear::tools());
    tools.extend(jira::tools());
    tools.extend(gitlab::tools());
    tools
}

/// Dispatche vers le bon connecteur selon le préfixe du nom d'outil.
/// Retourne None si le tool n'appartient à aucun connecteur natif.
pub async fn handle(name: &str, args: &Value) -> Option<Result<String, String>> {
    if name.starts_with("linear_") { return Some(linear::handle(name, args).await); }
    if name.starts_with("jira_")   { return Some(jira::handle(name, args).await); }
    if name.starts_with("gitlab_") { return Some(gitlab::handle(name, args).await); }
    None
}
