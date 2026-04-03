/// Connecteurs natifs Rust — Linear, Jira, GitLab, Vercel, Railway, Render (et futurs connecteurs).
///
/// Pattern :
///   - `tools()` → Vec<Value>  — définitions MCP exposées à Claude
///   - `handle(name, args)` → Result<String, String>  — exécution sans appel à send()
///
/// La couche sécurité (sanitize_proxy_response + audit_log) est appliquée
/// CENTRALEMENT dans le dispatcher de mcp.rs, après le retour du connecteur.
pub mod calendly;
pub mod cloudflare;
pub mod shopify;
pub mod n8n;
pub mod discord;
pub mod figma;
pub mod github;
pub mod supabase;
pub mod notion;
pub mod slack;
pub mod gcal;
pub mod gitlab;
pub mod hubspot;
pub mod posthog;
pub mod reddit;
pub mod resend;
pub mod sentry;
pub mod stripe;
pub mod jira;
pub mod linear;
pub mod railway;
pub mod render;
pub mod twilio;
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
    tools.extend(hubspot::tools());
    tools.extend(posthog::tools());
    tools.extend(resend::tools());
    tools.extend(sentry::tools());
    tools.extend(discord::tools());
    tools.extend(twilio::tools());
    tools.extend(figma::tools());
    tools.extend(notion::tools());
    tools.extend(slack::tools());
    tools.extend(reddit::tools());
    tools.extend(calendly::tools());
    tools.extend(n8n::tools());
    tools.extend(supabase::tools());
    tools.extend(cloudflare::tools());
    tools.extend(github::tools());
    tools.extend(shopify::tools());
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
    if name.starts_with("stripe_")   { return Some(stripe::handle(name, args).await); }
    if name.starts_with("hubspot_")  { return Some(hubspot::handle(name, args).await); }
    if name.starts_with("posthog_")  { return Some(posthog::handle(name, args).await); }
    if name.starts_with("resend_")   { return Some(resend::handle(name, args).await); }
    if name.starts_with("sentry_")   { return Some(sentry::handle(name, args).await); }
    if name.starts_with("discord_")   { return Some(discord::handle(name, args).await); }
    if name.starts_with("twilio_")    { return Some(twilio::handle(name, args).await); }
    if name.starts_with("figma_")     { return Some(figma::handle(name, args).await); }
    if name.starts_with("notion_")    { return Some(notion::handle(name, args).await); }
    if name.starts_with("slack_")     { return Some(slack::handle(name, args).await); }
    if name.starts_with("reddit_")    { return Some(reddit::handle(name, args).await); }
    if name.starts_with("calendly_")  { return Some(calendly::handle(name, args).await); }
    if name.starts_with("n8n_")       { return Some(n8n::handle(name, args).await); }
    if name.starts_with("supabase_")    { return Some(supabase::handle(name, args).await); }
    if name.starts_with("cloudflare_") { return Some(cloudflare::handle(name, args).await); }
    if name.starts_with("github_")     { return Some(github::handle(name, args).await); }
    if name.starts_with("shopify_")    { return Some(shopify::handle(name, args).await); }
    None
}
