/// Connecteur Sentry — REST API v0 officielle.
/// Toutes les fonctions retournent Result<String, String> sans jamais appeler send().
/// La couche sécurité (sanitize_proxy_response + audit_log) est appliquée par le dispatcher (mcp.rs).
use serde_json::{json, Value};

// ─── Config ──────────────────────────────────────────────────────────────────

#[derive(serde::Deserialize)]
struct SentryConfig {
    token: String,
    host:  Option<String>,
}

impl SentryConfig {
    fn load() -> Option<Self> {
        let path = dirs_next::home_dir()?.join(".osmozzz/sentry.toml");
        toml::from_str(&std::fs::read_to_string(path).ok()?).ok()
    }

    fn base_url(&self) -> String {
        let host = self
            .host
            .as_deref()
            .filter(|s| !s.is_empty())
            .unwrap_or("https://sentry.io");
        format!("{}/api/0", host.trim_end_matches('/'))
    }

    /// Construit une URL API Sentry. Le slash final est toujours ajouté.
    fn api(&self, path: &str) -> String {
        let base = self.base_url();
        let p    = path.trim_start_matches('/').trim_end_matches('/');
        format!("{base}/{p}/")
    }
}

// ─── HTTP helpers ─────────────────────────────────────────────────────────────

async fn get(cfg: &SentryConfig, url: &str) -> Result<Value, String> {
    reqwest::Client::new()
        .get(url)
        .header("Authorization", format!("Bearer {}", cfg.token))
        .header("Content-Type", "application/json")
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<Value>()
        .await
        .map_err(|e| e.to_string())
}

async fn post_json(cfg: &SentryConfig, url: &str, body: &Value) -> Result<Value, String> {
    reqwest::Client::new()
        .post(url)
        .header("Authorization", format!("Bearer {}", cfg.token))
        .header("Content-Type", "application/json")
        .json(body)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<Value>()
        .await
        .map_err(|e| e.to_string())
}

async fn put_json(cfg: &SentryConfig, url: &str, body: &Value) -> Result<Value, String> {
    reqwest::Client::new()
        .put(url)
        .header("Authorization", format!("Bearer {}", cfg.token))
        .header("Content-Type", "application/json")
        .json(body)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<Value>()
        .await
        .map_err(|e| e.to_string())
}

/// GET sur une URL Sentry complète — utilisé par sentry_get_sentry_resource.
async fn get_raw(cfg: &SentryConfig, url: &str) -> Result<Value, String> {
    reqwest::Client::new()
        .get(url)
        .header("Authorization", format!("Bearer {}", cfg.token))
        .header("Content-Type", "application/json")
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<Value>()
        .await
        .map_err(|e| e.to_string())
}

// ─── Tool definitions ────────────────────────────────────────────────────────

pub fn tools() -> Vec<Value> {
    vec![
        // ── Organizations ───────────────────────────────────────────────────
        json!({
            "name": "sentry_find_organizations",
            "description": "SENTRY 🔍 — Liste toutes les organisations Sentry accessibles avec le token configuré. Retourne slug, nom et statut de chaque organisation.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }),

        // ── Projects ────────────────────────────────────────────────────────
        json!({
            "name": "sentry_find_projects",
            "description": "SENTRY 📁 — Liste les projets d'une organisation Sentry. Retourne slug, nom, plateforme et statut de chaque projet.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "org_slug": { "type": "string", "description": "Slug de l'organisation" },
                    "cursor":   { "type": "string", "description": "Curseur de pagination (optionnel)" }
                },
                "required": ["org_slug"]
            }
        }),

        // ── Teams ────────────────────────────────────────────────────────────
        json!({
            "name": "sentry_find_teams",
            "description": "SENTRY 👥 — Liste toutes les équipes d'une organisation Sentry. Retourne slug, nom et nombre de membres.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "org_slug": { "type": "string", "description": "Slug de l'organisation" }
                },
                "required": ["org_slug"]
            }
        }),

        // ── Releases ─────────────────────────────────────────────────────────
        json!({
            "name": "sentry_find_releases",
            "description": "SENTRY 🚀 — Liste les releases d'une organisation, filtrable par projet et query texte. Retourne version, date et projets associés.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "org_slug":     { "type": "string", "description": "Slug de l'organisation" },
                    "project_slug": { "type": "string", "description": "Slug du projet (optionnel)" },
                    "query":        { "type": "string", "description": "Filtre texte (optionnel)" },
                    "cursor":       { "type": "string", "description": "Curseur de pagination (optionnel)" }
                },
                "required": ["org_slug"]
            }
        }),

        // ── DSNs ─────────────────────────────────────────────────────────────
        json!({
            "name": "sentry_find_dsns",
            "description": "SENTRY 🔑 — Liste les clés DSN (Data Source Name) d'un projet Sentry. Retourne id, nom, DSN public et secret.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "org_slug":     { "type": "string", "description": "Slug de l'organisation" },
                    "project_slug": { "type": "string", "description": "Slug du projet" }
                },
                "required": ["org_slug", "project_slug"]
            }
        }),

        // ── Issues ───────────────────────────────────────────────────────────
        json!({
            "name": "sentry_list_issues",
            "description": "SENTRY 🐛 — Liste les issues Sentry d'une organisation, filtrable par projet, query texte et limite. Retourne id, titre, statut, assigné et compteur d'occurrences.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "org_slug":     { "type": "string", "description": "Slug de l'organisation" },
                    "project_slug": { "type": "string", "description": "Slug du projet (optionnel)" },
                    "query":        { "type": "string", "description": "Requête de recherche Sentry (ex: 'is:unresolved', 'level:error')" },
                    "limit":        { "type": "integer", "description": "Nombre max de résultats (défaut 25, max 100)" },
                    "cursor":       { "type": "string", "description": "Curseur de pagination (optionnel)" }
                },
                "required": ["org_slug"]
            }
        }),

        json!({
            "name": "sentry_get_issue",
            "description": "SENTRY 🐛 — Récupère le détail complet d'une issue Sentry : titre, statut, assigné, priorité, premier/dernier événement, compteurs.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "issue_id": { "type": "string", "description": "ID de l'issue Sentry" }
                },
                "required": ["issue_id"]
            }
        }),

        json!({
            "name": "sentry_list_issue_events",
            "description": "SENTRY 📋 — Liste les événements d'une issue Sentry (occurrences individuelles). Retourne id, date, message et contexte de chaque événement.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "issue_id": { "type": "string", "description": "ID de l'issue Sentry" },
                    "limit":    { "type": "integer", "description": "Nombre max de résultats (défaut 25)" },
                    "cursor":   { "type": "string", "description": "Curseur de pagination (optionnel)" }
                },
                "required": ["issue_id"]
            }
        }),

        json!({
            "name": "sentry_get_issue_tag_values",
            "description": "SENTRY 🏷️ — Récupère les valeurs d'un tag spécifique pour une issue Sentry (ex: tag 'browser', 'os', 'user'). Utile pour analyser la distribution des environnements touchés.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "issue_id": { "type": "string", "description": "ID de l'issue Sentry" },
                    "tag":      { "type": "string", "description": "Nom du tag (ex: 'browser', 'os', 'environment', 'release', 'user')" }
                },
                "required": ["issue_id", "tag"]
            }
        }),

        json!({
            "name": "sentry_update_issue",
            "description": "SENTRY ✏️ — Met à jour une issue Sentry : statut (resolved/unresolved/ignored), assignation, priorité, marquer comme vu ou mis en favori.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "issue_id":      { "type": "string", "description": "ID de l'issue Sentry" },
                    "status":        { "type": "string", "enum": ["resolved", "unresolved", "ignored"], "description": "Nouveau statut (optionnel)" },
                    "assignedTo":    { "type": "string", "description": "Username ou email de l'assigné (optionnel)" },
                    "hasSeen":       { "type": "boolean", "description": "Marquer comme vu (optionnel)" },
                    "isBookmarked":  { "type": "boolean", "description": "Mettre en favori (optionnel)" },
                    "isSubscribed":  { "type": "boolean", "description": "S'abonner aux notifications (optionnel)" },
                    "priority":      { "type": "string", "enum": ["critical", "high", "medium", "low"], "description": "Priorité de l'issue (optionnel)" }
                },
                "required": ["issue_id"]
            }
        }),

        // ── Events ───────────────────────────────────────────────────────────
        json!({
            "name": "sentry_list_events",
            "description": "SENTRY 📡 — Liste les événements bruts d'un projet Sentry. Retourne id, titre, date, niveau et environnement de chaque événement.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "org_slug":     { "type": "string", "description": "Slug de l'organisation" },
                    "project_slug": { "type": "string", "description": "Slug du projet" },
                    "full":         { "type": "boolean", "description": "Inclure les données complètes de chaque événement (défaut false)" },
                    "cursor":       { "type": "string", "description": "Curseur de pagination (optionnel)" }
                },
                "required": ["org_slug", "project_slug"]
            }
        }),

        // ── Create ───────────────────────────────────────────────────────────
        json!({
            "name": "sentry_create_project",
            "description": "SENTRY ➕ — Crée un nouveau projet Sentry dans une organisation, associé à une équipe. Retourne le slug et DSN du projet créé.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "org_slug":   { "type": "string", "description": "Slug de l'organisation" },
                    "team_slug":  { "type": "string", "description": "Slug de l'équipe propriétaire du projet" },
                    "name":       { "type": "string", "description": "Nom du projet" },
                    "platform":   { "type": "string", "description": "Plateforme (ex: 'javascript', 'python', 'react-native') — optionnel" }
                },
                "required": ["org_slug", "team_slug", "name"]
            }
        }),

        json!({
            "name": "sentry_create_team",
            "description": "SENTRY ➕ — Crée une nouvelle équipe dans une organisation Sentry. Retourne le slug et l'id de l'équipe créée.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "org_slug": { "type": "string", "description": "Slug de l'organisation" },
                    "name":     { "type": "string", "description": "Nom de l'équipe" },
                    "slug":     { "type": "string", "description": "Slug personnalisé (optionnel — généré automatiquement si absent)" }
                },
                "required": ["org_slug", "name"]
            }
        }),

        json!({
            "name": "sentry_create_dsn",
            "description": "SENTRY 🔑 — Crée une nouvelle clé DSN dans un projet Sentry. Retourne le DSN public et secret de la nouvelle clé.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "org_slug":     { "type": "string", "description": "Slug de l'organisation" },
                    "project_slug": { "type": "string", "description": "Slug du projet" },
                    "name":         { "type": "string", "description": "Nom de la clé DSN (ex: 'Production', 'Staging')" }
                },
                "required": ["org_slug", "project_slug", "name"]
            }
        }),

        // ── Whoami ───────────────────────────────────────────────────────────
        json!({
            "name": "sentry_whoami",
            "description": "SENTRY 👤 — Retourne les informations de l'utilisateur authentifié (nom, email, id, date de création du compte).",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }),

        // ── Attachments ──────────────────────────────────────────────────────
        json!({
            "name": "sentry_get_event_attachment",
            "description": "SENTRY 📎 — Récupère le contenu d'une pièce jointe attachée à un événement Sentry (screenshot, log, fichier de crash).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "org_slug":     { "type": "string", "description": "Slug de l'organisation" },
                    "project_slug": { "type": "string", "description": "Slug du projet" },
                    "event_id":     { "type": "string", "description": "ID de l'événement" },
                    "attachment_id":{ "type": "string", "description": "ID de la pièce jointe" }
                },
                "required": ["org_slug", "project_slug", "event_id", "attachment_id"]
            }
        }),

        // ── Resource direct ──────────────────────────────────────────────────
        json!({
            "name": "sentry_get_sentry_resource",
            "description": "SENTRY 🔗 — Appelle directement une URL complète de l'API Sentry. Utile pour suivre les liens de pagination (header Link) retournés par d'autres tools Sentry.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "url": { "type": "string", "description": "URL complète de l'API Sentry (ex: https://sentry.io/api/0/organizations/my-org/issues/?cursor=...)" }
                },
                "required": ["url"]
            }
        }),

        // ── Update project ───────────────────────────────────────────────────
        json!({
            "name": "sentry_update_project",
            "description": "SENTRY ✏️ — Met à jour les paramètres d'un projet Sentry : nom, plateforme, préfixe des sujets d'alerte.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "org_slug":       { "type": "string", "description": "Slug de l'organisation" },
                    "project_slug":   { "type": "string", "description": "Slug du projet" },
                    "name":           { "type": "string", "description": "Nouveau nom du projet (optionnel)" },
                    "platform":       { "type": "string", "description": "Nouvelle plateforme (optionnel)" },
                    "subjectPrefix":  { "type": "string", "description": "Préfixe des sujets d'email d'alerte (optionnel)" }
                },
                "required": ["org_slug", "project_slug"]
            }
        }),

        // ── Search & Analysis ────────────────────────────────────────────────
        json!({
            "name": "sentry_search_issues",
            "description": "SENTRY 🔍 — Recherche avancée d'issues Sentry avec une query Sentry (ex: 'is:unresolved level:error browser:Chrome'). Alias de list_issues avec focus sur la recherche.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "org_slug":     { "type": "string", "description": "Slug de l'organisation" },
                    "project_slug": { "type": "string", "description": "Slug du projet (optionnel)" },
                    "query":        { "type": "string", "description": "Query Sentry (ex: 'is:unresolved assigned:me level:error')" },
                    "limit":        { "type": "integer", "description": "Nombre max de résultats (défaut 25, max 100)" },
                    "cursor":       { "type": "string", "description": "Curseur de pagination" }
                },
                "required": ["org_slug", "query"]
            }
        }),

        json!({
            "name": "sentry_search_events",
            "description": "SENTRY 🔍 — Recherche d'événements Sentry dans un projet avec une query (ex: 'level:error transaction:/api/checkout'). Retourne les événements correspondants.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "org_slug":     { "type": "string", "description": "Slug de l'organisation" },
                    "project_slug": { "type": "string", "description": "Slug du projet" },
                    "query":        { "type": "string", "description": "Query Sentry pour filtrer les événements" },
                    "limit":        { "type": "integer", "description": "Nombre max de résultats (défaut 10)" }
                },
                "required": ["org_slug", "project_slug", "query"]
            }
        }),

        json!({
            "name": "sentry_search_issue_events",
            "description": "SENTRY 🔍 — Recherche dans les événements d'une issue Sentry spécifique avec un filtre texte.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "issue_id": { "type": "string", "description": "ID de l'issue Sentry" },
                    "query":    { "type": "string", "description": "Filtre texte (optionnel)" },
                    "limit":    { "type": "integer", "description": "Nombre max de résultats (défaut 25)" }
                },
                "required": ["issue_id"]
            }
        }),

        json!({
            "name": "sentry_get_issue_details",
            "description": "SENTRY 🐛 — Récupère le détail complet d'une issue Sentry avec sa dernière stacktrace et le contexte d'erreur complet.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "issue_id": { "type": "string", "description": "ID de l'issue Sentry" }
                },
                "required": ["issue_id"]
            }
        }),

        json!({
            "name": "sentry_get_trace_details",
            "description": "SENTRY 🔗 — Récupère les détails d'une trace Sentry (distributed tracing) par son trace_id.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "org_slug":  { "type": "string", "description": "Slug de l'organisation" },
                    "trace_id":  { "type": "string", "description": "ID de la trace (32 caractères hex)" }
                },
                "required": ["org_slug", "trace_id"]
            }
        }),

        json!({
            "name": "sentry_analyze_issue_with_seer",
            "description": "SENTRY 🤖 — Lance l'analyse IA de Sentry (Seer) sur une issue pour obtenir une explication et des suggestions de correction automatiques.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "org_slug":     { "type": "string", "description": "Slug de l'organisation" },
                    "project_slug": { "type": "string", "description": "Slug du projet" },
                    "issue_id":     { "type": "string", "description": "ID de l'issue à analyser" }
                },
                "required": ["org_slug", "project_slug", "issue_id"]
            }
        }),

        json!({
            "name": "sentry_get_profile_details",
            "description": "SENTRY 📊 — Récupère les données de profiling (flamegraph) d'un événement Sentry qui a du profiling activé.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "org_slug":     { "type": "string", "description": "Slug de l'organisation" },
                    "project_slug": { "type": "string", "description": "Slug du projet" },
                    "profile_id":   { "type": "string", "description": "ID du profil (obtenu depuis un événement Sentry)" }
                },
                "required": ["org_slug", "project_slug", "profile_id"]
            }
        }),

        // ── Docs ─────────────────────────────────────────────────────────────
        json!({
            "name": "sentry_get_doc",
            "description": "SENTRY 📖 — Récupère un article de la documentation Sentry par son chemin (ex: 'platforms/javascript/guides/react').",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "path": { "type": "string", "description": "Chemin de la doc Sentry (ex: 'platforms/javascript' ou 'product/issues')" }
                },
                "required": ["path"]
            }
        }),

        json!({
            "name": "sentry_search_docs",
            "description": "SENTRY 📖 — Recherche dans la documentation Sentry par mots-clés. Retourne les articles et sections correspondants.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Termes de recherche dans la documentation Sentry" }
                },
                "required": ["query"]
            }
        }),
    ]
}

// ─── Handler ─────────────────────────────────────────────────────────────────

pub async fn handle(name: &str, args: &Value) -> Result<String, String> {
    let cfg = SentryConfig::load()
        .ok_or_else(|| "Sentry non configuré — créer ~/.osmozzz/sentry.toml avec token".to_string())?;

    match name {
        // ── Organizations ────────────────────────────────────────────────────
        "sentry_find_organizations" => {
            let url  = cfg.api("organizations");
            let resp = get(&cfg, &url).await?;

            let orgs = resp.as_array().cloned().unwrap_or_default();
            if orgs.is_empty() {
                return Ok("Aucune organisation Sentry trouvée.".to_string());
            }

            let mut out = format!("{} organisation(s) :\n", orgs.len());
            for org in &orgs {
                let slug   = org["slug"].as_str().unwrap_or("—");
                let oname  = org["name"].as_str().unwrap_or("—");
                let status = org["status"]["name"].as_str().unwrap_or("—");
                let date   = org["dateCreated"].as_str().unwrap_or("—");
                out.push_str(&format!("• [{slug}] {oname} — statut: {status} — créé: {date}\n"));
            }
            Ok(out.trim_end().to_string())
        }

        // ── Projects ─────────────────────────────────────────────────────────
        "sentry_find_projects" => {
            let org_slug = args["org_slug"].as_str().ok_or("Paramètre 'org_slug' requis")?;
            let mut url  = cfg.api(&format!("organizations/{org_slug}/projects"));
            if let Some(cursor) = args["cursor"].as_str() {
                url = format!("{url}?cursor={cursor}");
            }
            let resp = get(&cfg, &url).await?;

            let projects = resp.as_array().cloned().unwrap_or_default();
            if projects.is_empty() {
                return Ok(format!("Aucun projet dans l'organisation {org_slug}."));
            }

            let mut out = format!("{} projet(s) dans {org_slug} :\n", projects.len());
            for p in &projects {
                let slug     = p["slug"].as_str().unwrap_or("—");
                let pname    = p["name"].as_str().unwrap_or("—");
                let platform = p["platform"].as_str().unwrap_or("—");
                let status   = p["status"].as_str().unwrap_or("—");
                out.push_str(&format!("• [{slug}] {pname} — plateforme: {platform} — statut: {status}\n"));
            }
            Ok(out.trim_end().to_string())
        }

        // ── Teams ─────────────────────────────────────────────────────────────
        "sentry_find_teams" => {
            let org_slug = args["org_slug"].as_str().ok_or("Paramètre 'org_slug' requis")?;
            let url      = cfg.api(&format!("organizations/{org_slug}/teams"));
            let resp     = get(&cfg, &url).await?;

            let teams = resp.as_array().cloned().unwrap_or_default();
            if teams.is_empty() {
                return Ok(format!("Aucune équipe dans l'organisation {org_slug}."));
            }

            let mut out = format!("{} équipe(s) dans {org_slug} :\n", teams.len());
            for t in &teams {
                let slug        = t["slug"].as_str().unwrap_or("—");
                let tname       = t["name"].as_str().unwrap_or("—");
                let member_count = t["memberCount"].as_u64().unwrap_or(0);
                let date         = t["dateCreated"].as_str().unwrap_or("—");
                out.push_str(&format!("• [{slug}] {tname} — {member_count} membre(s) — créé: {date}\n"));
            }
            Ok(out.trim_end().to_string())
        }

        // ── Releases ─────────────────────────────────────────────────────────
        "sentry_find_releases" => {
            let org_slug = args["org_slug"].as_str().ok_or("Paramètre 'org_slug' requis")?;
            let mut params = Vec::new();
            if let Some(proj) = args["project_slug"].as_str() {
                params.push(format!("project={proj}"));
            }
            if let Some(q) = args["query"].as_str() {
                params.push(format!("query={}", urlencoding_simple(q)));
            }
            if let Some(cursor) = args["cursor"].as_str() {
                params.push(format!("cursor={cursor}"));
            }
            let query_str = if params.is_empty() { String::new() } else { format!("?{}", params.join("&")) };
            let url  = format!("{}{}", cfg.api(&format!("organizations/{org_slug}/releases")), query_str);
            let resp = get(&cfg, &url).await?;

            let releases = resp.as_array().cloned().unwrap_or_default();
            if releases.is_empty() {
                return Ok(format!("Aucune release dans l'organisation {org_slug}."));
            }

            let mut out = format!("{} release(s) :\n", releases.len());
            for r in &releases {
                let version    = r["version"].as_str().unwrap_or("—");
                let date       = r["dateCreated"].as_str().unwrap_or("—");
                let deploy_env = r["lastDeploy"]["environment"].as_str().unwrap_or("");
                let projects: Vec<&str> = r["projects"].as_array()
                    .map(|arr| arr.iter().filter_map(|p| p["slug"].as_str()).collect())
                    .unwrap_or_default();
                let proj_str = if projects.is_empty() { "—".to_string() } else { projects.join(", ") };
                if deploy_env.is_empty() {
                    out.push_str(&format!("• {version} — créé: {date} — projets: {proj_str}\n"));
                } else {
                    out.push_str(&format!("• {version} — créé: {date} — env: {deploy_env} — projets: {proj_str}\n"));
                }
            }
            Ok(out.trim_end().to_string())
        }

        // ── DSNs ─────────────────────────────────────────────────────────────
        "sentry_find_dsns" => {
            let org_slug     = args["org_slug"].as_str().ok_or("Paramètre 'org_slug' requis")?;
            let project_slug = args["project_slug"].as_str().ok_or("Paramètre 'project_slug' requis")?;
            let url          = cfg.api(&format!("projects/{org_slug}/{project_slug}/keys"));
            let resp         = get(&cfg, &url).await?;

            let keys = resp.as_array().cloned().unwrap_or_default();
            if keys.is_empty() {
                return Ok(format!("Aucun DSN pour le projet {project_slug}."));
            }

            let mut out = format!("{} clé(s) DSN pour {project_slug} :\n", keys.len());
            for k in &keys {
                let id         = k["id"].as_str().unwrap_or("—");
                let kname      = k["name"].as_str().unwrap_or("—");
                let dsn_public = k["dsn"]["public"].as_str().unwrap_or("—");
                let active     = k["isActive"].as_bool().unwrap_or(false);
                let status_str = if active { "actif" } else { "inactif" };
                out.push_str(&format!("• [{id}] {kname} ({status_str})\n  DSN public : {dsn_public}\n"));
            }
            Ok(out.trim_end().to_string())
        }

        // ── Issues ────────────────────────────────────────────────────────────
        "sentry_list_issues" => {
            let org_slug = args["org_slug"].as_str().ok_or("Paramètre 'org_slug' requis")?;
            let limit    = args["limit"].as_u64().unwrap_or(25).min(100);
            let mut params = vec![format!("limit={limit}")];
            if let Some(proj) = args["project_slug"].as_str() {
                params.push(format!("project={proj}"));
            }
            if let Some(q) = args["query"].as_str() {
                params.push(format!("query={}", urlencoding_simple(q)));
            }
            if let Some(cursor) = args["cursor"].as_str() {
                params.push(format!("cursor={cursor}"));
            }
            let query_str = format!("?{}", params.join("&"));
            let url  = format!("{}{}", cfg.api(&format!("organizations/{org_slug}/issues")), query_str);
            let resp = get(&cfg, &url).await?;

            let issues = resp.as_array().cloned().unwrap_or_default();
            if issues.is_empty() {
                return Ok(format!("Aucune issue dans l'organisation {org_slug}."));
            }

            let mut out = format!("{} issue(s) :\n", issues.len());
            for issue in &issues {
                let id       = issue["id"].as_str().unwrap_or("—");
                let title    = issue["title"].as_str().unwrap_or("—");
                let status   = issue["status"].as_str().unwrap_or("—");
                let count    = issue["count"].as_str().unwrap_or("0");
                let project  = issue["project"]["slug"].as_str().unwrap_or("—");
                let assigned = issue["assignedTo"]["name"].as_str()
                    .or_else(|| issue["assignedTo"]["email"].as_str())
                    .unwrap_or("non assigné");
                let first_seen = issue["firstSeen"].as_str().unwrap_or("—");
                let last_seen  = issue["lastSeen"].as_str().unwrap_or("—");
                out.push_str(&format!(
                    "• [{id}] {title}\n  statut: {status} | projet: {project} | occurrences: {count} | assigné: {assigned}\n  première fois: {first_seen} | dernière fois: {last_seen}\n"
                ));
            }
            Ok(out.trim_end().to_string())
        }

        "sentry_get_issue" => {
            let issue_id = args["issue_id"].as_str().ok_or("Paramètre 'issue_id' requis")?;
            let url      = cfg.api(&format!("issues/{issue_id}"));
            let resp     = get(&cfg, &url).await?;

            let id         = resp["id"].as_str().unwrap_or("—");
            let title      = resp["title"].as_str().unwrap_or("—");
            let culprit    = resp["culprit"].as_str().unwrap_or("—");
            let status     = resp["status"].as_str().unwrap_or("—");
            let level      = resp["level"].as_str().unwrap_or("—");
            let priority   = resp["priority"].as_str().unwrap_or("—");
            let count      = resp["count"].as_str().unwrap_or("0");
            let user_count = resp["userCount"].as_u64().unwrap_or(0);
            let project    = resp["project"]["slug"].as_str().unwrap_or("—");
            let first_seen = resp["firstSeen"].as_str().unwrap_or("—");
            let last_seen  = resp["lastSeen"].as_str().unwrap_or("—");
            let assigned   = resp["assignedTo"]["name"].as_str()
                .or_else(|| resp["assignedTo"]["email"].as_str())
                .unwrap_or("non assigné");

            let tags: Vec<String> = resp["tags"].as_array()
                .map(|arr| arr.iter().map(|t| {
                    let k = t["key"].as_str().unwrap_or("?");
                    let v = t["value"].as_str().unwrap_or("?");
                    format!("{k}={v}")
                }).collect())
                .unwrap_or_default();

            Ok(format!(
                "Issue {id}\nTitre       : {title}\nCulprit     : {culprit}\nStatut      : {status}\nNiveau      : {level}\nPriorité    : {priority}\nProjet      : {project}\nAssigné     : {assigned}\nOccurrences : {count} ({user_count} utilisateur(s))\nPremière    : {first_seen}\nDernière    : {last_seen}\nTags        : {tags}",
                tags = if tags.is_empty() { "aucun".to_string() } else { tags.join(", ") }
            ))
        }

        "sentry_list_issue_events" => {
            let issue_id = args["issue_id"].as_str().ok_or("Paramètre 'issue_id' requis")?;
            let limit    = args["limit"].as_u64().unwrap_or(25).min(100);
            let mut params = vec![format!("limit={limit}")];
            if let Some(cursor) = args["cursor"].as_str() {
                params.push(format!("cursor={cursor}"));
            }
            let query_str = format!("?{}", params.join("&"));
            let url  = format!("{}{}", cfg.api(&format!("issues/{issue_id}/events")), query_str);
            let resp = get(&cfg, &url).await?;

            let events = resp.as_array().cloned().unwrap_or_default();
            if events.is_empty() {
                return Ok(format!("Aucun événement pour l'issue {issue_id}."));
            }

            let mut out = format!("{} événement(s) pour l'issue {issue_id} :\n", events.len());
            for ev in &events {
                let id       = ev["id"].as_str().unwrap_or("—");
                let ev_title = ev["title"].as_str().unwrap_or("—");
                let date     = ev["dateCreated"].as_str().unwrap_or("—");
                let platform = ev["platform"].as_str().unwrap_or("—");
                let env      = ev["environment"].as_str().unwrap_or("—");
                out.push_str(&format!("• [{id}] {ev_title}\n  date: {date} | plateforme: {platform} | env: {env}\n"));
            }
            Ok(out.trim_end().to_string())
        }

        "sentry_get_issue_tag_values" => {
            let issue_id = args["issue_id"].as_str().ok_or("Paramètre 'issue_id' requis")?;
            let tag      = args["tag"].as_str().ok_or("Paramètre 'tag' requis")?;
            let url      = cfg.api(&format!("issues/{issue_id}/tags/{tag}/values"));
            let resp     = get(&cfg, &url).await?;

            let values = resp.as_array().cloned().unwrap_or_default();
            if values.is_empty() {
                return Ok(format!("Aucune valeur pour le tag '{tag}' de l'issue {issue_id}."));
            }

            let mut out = format!("{} valeur(s) pour le tag '{tag}' :\n", values.len());
            for v in &values {
                let value   = v["value"].as_str().unwrap_or("—");
                let count   = v["count"].as_u64().unwrap_or(0);
                let first   = v["firstSeen"].as_str().unwrap_or("—");
                let last    = v["lastSeen"].as_str().unwrap_or("—");
                out.push_str(&format!("• {value} — {count} occurrences (première: {first}, dernière: {last})\n"));
            }
            Ok(out.trim_end().to_string())
        }

        "sentry_update_issue" => {
            let issue_id = args["issue_id"].as_str().ok_or("Paramètre 'issue_id' requis")?;
            let mut body = json!({});

            if let Some(s) = args["status"].as_str()       { body["status"]       = json!(s); }
            if let Some(a) = args["assignedTo"].as_str()   { body["assignedTo"]   = json!(a); }
            if let Some(s) = args["hasSeen"].as_bool()     { body["hasSeen"]      = json!(s); }
            if let Some(b) = args["isBookmarked"].as_bool(){ body["isBookmarked"] = json!(b); }
            if let Some(s) = args["isSubscribed"].as_bool(){ body["isSubscribed"] = json!(s); }
            if let Some(p) = args["priority"].as_str()     { body["priority"]     = json!(p); }

            if body.as_object().map(|o| o.is_empty()).unwrap_or(true) {
                return Err("Au moins un paramètre de mise à jour est requis".to_string());
            }

            let url  = cfg.api(&format!("issues/{issue_id}"));
            let resp = put_json(&cfg, &url, &body).await?;

            let id     = resp["id"].as_str().unwrap_or(issue_id);
            let status = resp["status"].as_str().unwrap_or("—");
            let assigned = resp["assignedTo"]["name"].as_str()
                .or_else(|| resp["assignedTo"]["email"].as_str())
                .unwrap_or("non assigné");
            Ok(format!("Issue {id} mise à jour.\nStatut  : {status}\nAssigné : {assigned}"))
        }

        // ── Events ────────────────────────────────────────────────────────────
        "sentry_list_events" => {
            let org_slug     = args["org_slug"].as_str().ok_or("Paramètre 'org_slug' requis")?;
            let project_slug = args["project_slug"].as_str().ok_or("Paramètre 'project_slug' requis")?;
            let full         = args["full"].as_bool().unwrap_or(false);
            let mut params   = Vec::new();
            if full {
                params.push("full=1".to_string());
            }
            if let Some(cursor) = args["cursor"].as_str() {
                params.push(format!("cursor={cursor}"));
            }
            let query_str = if params.is_empty() { String::new() } else { format!("?{}", params.join("&")) };
            let url  = format!("{}{}", cfg.api(&format!("projects/{org_slug}/{project_slug}/events")), query_str);
            let resp = get(&cfg, &url).await?;

            let events = resp.as_array().cloned().unwrap_or_default();
            if events.is_empty() {
                return Ok(format!("Aucun événement pour le projet {project_slug}."));
            }

            let mut out = format!("{} événement(s) dans {project_slug} :\n", events.len());
            for ev in &events {
                let id        = ev["id"].as_str().unwrap_or("—");
                let ev_title  = ev["title"].as_str().unwrap_or("—");
                let date      = ev["dateCreated"].as_str().unwrap_or("—");
                let level     = ev["level"].as_str().unwrap_or("—");
                let platform  = ev["platform"].as_str().unwrap_or("—");
                out.push_str(&format!("• [{id}] {ev_title}\n  date: {date} | niveau: {level} | plateforme: {platform}\n"));
            }
            Ok(out.trim_end().to_string())
        }

        // ── Create ────────────────────────────────────────────────────────────
        "sentry_create_project" => {
            let org_slug  = args["org_slug"].as_str().ok_or("Paramètre 'org_slug' requis")?;
            let team_slug = args["team_slug"].as_str().ok_or("Paramètre 'team_slug' requis")?;
            let proj_name = args["name"].as_str().ok_or("Paramètre 'name' requis")?;

            let mut body = json!({ "name": proj_name });
            if let Some(platform) = args["platform"].as_str() {
                body["platform"] = json!(platform);
            }

            let url  = cfg.api(&format!("teams/{org_slug}/{team_slug}/projects"));
            let resp = post_json(&cfg, &url, &body).await?;

            let id   = resp["id"].as_str().unwrap_or("—");
            let slug = resp["slug"].as_str().unwrap_or("—");
            let dsn  = resp["options"]["sentry:csp_ignored_sources"].as_str()
                .unwrap_or("(voir sentry_find_dsns)");
            Ok(format!(
                "Projet créé.\nID   : {id}\nNom  : {proj_name}\nSlug : {slug}\nDSN  : {dsn}\nOrg  : {org_slug} | Équipe : {team_slug}"
            ))
        }

        "sentry_create_team" => {
            let org_slug  = args["org_slug"].as_str().ok_or("Paramètre 'org_slug' requis")?;
            let team_name = args["name"].as_str().ok_or("Paramètre 'name' requis")?;

            let mut body = json!({ "name": team_name });
            if let Some(slug) = args["slug"].as_str() {
                body["slug"] = json!(slug);
            }

            let url  = cfg.api(&format!("organizations/{org_slug}/teams"));
            let resp = post_json(&cfg, &url, &body).await?;

            let id   = resp["id"].as_str().unwrap_or("—");
            let slug = resp["slug"].as_str().unwrap_or("—");
            let date = resp["dateCreated"].as_str().unwrap_or("—");
            Ok(format!(
                "Équipe créée.\nID   : {id}\nNom  : {team_name}\nSlug : {slug}\nCréé : {date}\nOrg  : {org_slug}"
            ))
        }

        "sentry_create_dsn" => {
            let org_slug     = args["org_slug"].as_str().ok_or("Paramètre 'org_slug' requis")?;
            let project_slug = args["project_slug"].as_str().ok_or("Paramètre 'project_slug' requis")?;
            let key_name     = args["name"].as_str().ok_or("Paramètre 'name' requis")?;

            let body = json!({ "name": key_name });
            let url  = cfg.api(&format!("projects/{org_slug}/{project_slug}/keys"));
            let resp = post_json(&cfg, &url, &body).await?;

            let id         = resp["id"].as_str().unwrap_or("—");
            let dsn_public = resp["dsn"]["public"].as_str().unwrap_or("—");
            let dsn_secret = resp["dsn"]["secret"].as_str().unwrap_or("—");
            Ok(format!(
                "Clé DSN créée.\nID         : {id}\nNom        : {key_name}\nDSN public : {dsn_public}\nDSN secret : {dsn_secret}\nProjet     : {project_slug} | Org : {org_slug}"
            ))
        }

        // ── Whoami ────────────────────────────────────────────────────────────
        "sentry_whoami" => {
            let url  = cfg.api("users/me");
            let resp = get(&cfg, &url).await?;

            let id       = resp["id"].as_str().unwrap_or("—");
            let username = resp["username"].as_str().unwrap_or("—");
            let email    = resp["email"].as_str().unwrap_or("—");
            let name     = resp["name"].as_str().unwrap_or("—");
            let date     = resp["dateCreated"].as_str().unwrap_or("—");
            let is_staff = resp["isSuperuser"].as_bool().unwrap_or(false);
            Ok(format!(
                "Utilisateur Sentry authentifié\nID       : {id}\nUsername : {username}\nNom      : {name}\nEmail    : {email}\nCréé     : {date}\nSuperuser: {}",
                if is_staff { "oui" } else { "non" }
            ))
        }

        // ── Attachments ───────────────────────────────────────────────────────
        "sentry_get_event_attachment" => {
            let org_slug       = args["org_slug"].as_str().ok_or("Paramètre 'org_slug' requis")?;
            let project_slug   = args["project_slug"].as_str().ok_or("Paramètre 'project_slug' requis")?;
            let event_id       = args["event_id"].as_str().ok_or("Paramètre 'event_id' requis")?;
            let attachment_id  = args["attachment_id"].as_str().ok_or("Paramètre 'attachment_id' requis")?;

            let url  = cfg.api(&format!("projects/{org_slug}/{project_slug}/events/{event_id}/attachments/{attachment_id}"));
            let resp = get(&cfg, &url).await?;

            let id      = resp["id"].as_str().unwrap_or("—");
            let aname   = resp["name"].as_str().unwrap_or("—");
            let size    = resp["size"].as_u64().unwrap_or(0);
            let mime    = resp["mimetype"].as_str().unwrap_or("—");
            let date    = resp["dateCreated"].as_str().unwrap_or("—");
            let headers = resp["headers"].as_object().map(|_| "(présents)").unwrap_or("—");
            Ok(format!(
                "Pièce jointe {id}\nNom       : {aname}\nTaille    : {size} octets\nMIME      : {mime}\nCréé      : {date}\nHeaders   : {headers}\nÉvénement : {event_id}\nProjet    : {project_slug} | Org : {org_slug}"
            ))
        }

        // ── Resource direct ───────────────────────────────────────────────────
        "sentry_get_sentry_resource" => {
            let url  = args["url"].as_str().ok_or("Paramètre 'url' requis")?;
            let resp = get_raw(&cfg, url).await?;
            Ok(serde_json::to_string_pretty(&resp).unwrap_or_else(|_| resp.to_string()))
        }

        // ── Update project ────────────────────────────────────────────────────
        "sentry_update_project" => {
            let org_slug     = args["org_slug"].as_str().ok_or("Paramètre 'org_slug' requis")?;
            let project_slug = args["project_slug"].as_str().ok_or("Paramètre 'project_slug' requis")?;
            let mut body     = json!({});

            if let Some(n) = args["name"].as_str()          { body["name"]          = json!(n); }
            if let Some(p) = args["platform"].as_str()      { body["platform"]      = json!(p); }
            if let Some(s) = args["subjectPrefix"].as_str() { body["subjectPrefix"] = json!(s); }

            if body.as_object().map(|o| o.is_empty()).unwrap_or(true) {
                return Err("Au moins un paramètre parmi 'name', 'platform' ou 'subjectPrefix' est requis".to_string());
            }

            let url  = cfg.api(&format!("projects/{org_slug}/{project_slug}"));
            let resp = put_json(&cfg, &url, &body).await?;

            let id       = resp["id"].as_str().unwrap_or("—");
            let pname    = resp["name"].as_str().unwrap_or("—");
            let platform = resp["platform"].as_str().unwrap_or("—");
            let slug     = resp["slug"].as_str().unwrap_or(project_slug);
            Ok(format!(
                "Projet {id} mis à jour.\nNom        : {pname}\nSlug       : {slug}\nPlateforme : {platform}\nOrg        : {org_slug}"
            ))
        }

        // ── Search & Analysis ────────────────────────────────────────────────
        "sentry_search_issues" => {
            let org_slug     = args["org_slug"].as_str().ok_or("Paramètre 'org_slug' requis")?;
            let query        = args["query"].as_str().ok_or("Paramètre 'query' requis")?;
            let limit        = args["limit"].as_u64().unwrap_or(25).min(100);
            let mut url_str  = format!(
                "{}?query={}&limit={}",
                cfg.api(&format!("organizations/{org_slug}/issues")),
                urlencoding_simple(query), limit
            );
            if let Some(proj) = args["project_slug"].as_str() {
                url_str.push_str(&format!("&project={proj}"));
            }
            if let Some(cursor) = args["cursor"].as_str() {
                url_str.push_str(&format!("&cursor={cursor}"));
            }
            let resp   = get(&cfg, &url_str).await?;
            let issues = resp.as_array().cloned().unwrap_or_default();
            if issues.is_empty() { return Ok("Aucune issue correspondante.".to_string()); }
            let mut out = format!("{} issue(s) pour query '{query}' :\n", issues.len());
            for i in &issues {
                let id     = i["id"].as_str().unwrap_or("—");
                let title  = i["title"].as_str().unwrap_or("—");
                let status = i["status"].as_str().unwrap_or("—");
                let count  = i["count"].as_str().unwrap_or("0");
                out.push_str(&format!("• [{id}] {title} — statut: {status} — occurrences: {count}\n"));
            }
            Ok(out.trim_end().to_string())
        }

        "sentry_search_events" => {
            let org_slug     = args["org_slug"].as_str().ok_or("Paramètre 'org_slug' requis")?;
            let project_slug = args["project_slug"].as_str().ok_or("Paramètre 'project_slug' requis")?;
            let query        = args["query"].as_str().ok_or("Paramètre 'query' requis")?;
            let limit        = args["limit"].as_u64().unwrap_or(10).min(100);
            let url = format!(
                "{}?query={}&limit={}",
                cfg.api(&format!("projects/{org_slug}/{project_slug}/events")),
                urlencoding_simple(query), limit
            );
            let resp   = get(&cfg, &url).await?;
            let events = resp.as_array().cloned().unwrap_or_default();
            if events.is_empty() { return Ok("Aucun événement correspondant.".to_string()); }
            let mut out = format!("{} événement(s) :\n", events.len());
            for e in &events {
                let id    = e["id"].as_str().unwrap_or("—");
                let title = e["title"].as_str().unwrap_or("—");
                let date  = e["dateCreated"].as_str().unwrap_or("—");
                let level = e["level"].as_str().unwrap_or("—");
                out.push_str(&format!("• [{id}] [{level}] {title} — {date}\n"));
            }
            Ok(out.trim_end().to_string())
        }

        "sentry_search_issue_events" => {
            let issue_id = args["issue_id"].as_str().ok_or("Paramètre 'issue_id' requis")?;
            let limit    = args["limit"].as_u64().unwrap_or(25);
            let mut url  = format!("{}?limit={limit}", cfg.api(&format!("issues/{issue_id}/events")));
            if let Some(q) = args["query"].as_str() {
                url.push_str(&format!("&query={}", urlencoding_simple(q)));
            }
            let resp   = get(&cfg, &url).await?;
            let events = resp.as_array().cloned().unwrap_or_default();
            if events.is_empty() { return Ok(format!("Aucun événement pour l'issue {issue_id}.")); }
            let mut out = format!("{} événement(s) pour issue {issue_id} :\n", events.len());
            for e in &events {
                let id   = e["id"].as_str().unwrap_or("—");
                let date = e["dateCreated"].as_str().unwrap_or("—");
                let msg  = e["message"].as_str().unwrap_or("—");
                out.push_str(&format!("• [{id}] {date} — {msg}\n"));
            }
            Ok(out.trim_end().to_string())
        }

        "sentry_get_issue_details" => {
            let issue_id = args["issue_id"].as_str().ok_or("Paramètre 'issue_id' requis")?;
            // Récupère l'issue ET le dernier événement pour avoir la stacktrace
            let url_issue = cfg.api(&format!("issues/{issue_id}"));
            let url_event = cfg.api(&format!("issues/{issue_id}/events/latest"));
            let (issue, event) = tokio::try_join!(
                get(&cfg, &url_issue),
                get(&cfg, &url_event)
            ).map_err(|e| e)?;

            let title     = issue["title"].as_str().unwrap_or("—");
            let status    = issue["status"].as_str().unwrap_or("—");
            let level     = issue["level"].as_str().unwrap_or("—");
            let assignee  = issue["assignee"]["name"].as_str().unwrap_or("non assigné");
            let first_seen = issue["firstSeen"].as_str().unwrap_or("—");
            let last_seen  = issue["lastSeen"].as_str().unwrap_or("—");
            let count      = issue["count"].as_str().unwrap_or("0");

            // Extraire la stacktrace du dernier événement
            let stacktrace = event["entries"].as_array()
                .and_then(|entries| entries.iter().find(|e| e["type"].as_str() == Some("exception")))
                .and_then(|exc| exc["data"]["values"].as_array())
                .and_then(|vals| vals.first())
                .map(|val| {
                    let exc_type  = val["type"].as_str().unwrap_or("—");
                    let exc_value = val["value"].as_str().unwrap_or("—");
                    let frames: Vec<String> = val["stacktrace"]["frames"].as_array()
                        .map(|f| f.iter().rev().take(8).map(|frame| {
                            let file = frame["filename"].as_str().unwrap_or("—");
                            let line = frame["lineNo"].as_u64().unwrap_or(0);
                            let func = frame["function"].as_str().unwrap_or("—");
                            format!("  {file}:{line} in {func}")
                        }).collect())
                        .unwrap_or_default();
                    format!("{exc_type}: {exc_value}\nStacktrace (top 8 frames):\n{}", frames.join("\n"))
                })
                .unwrap_or_else(|| "(stacktrace non disponible)".to_string());

            Ok(format!(
                "Issue {issue_id} — {title}\nStatut: {status} | Level: {level} | Assigné: {assignee}\nOccurrences: {count} | Première: {first_seen} | Dernière: {last_seen}\n\n{stacktrace}"
            ))
        }

        "sentry_get_trace_details" => {
            let org_slug = args["org_slug"].as_str().ok_or("Paramètre 'org_slug' requis")?;
            let trace_id = args["trace_id"].as_str().ok_or("Paramètre 'trace_id' requis")?;
            let url      = cfg.api(&format!("organizations/{org_slug}/events-trace/{trace_id}"));
            let resp     = get(&cfg, &url).await?;

            let transactions = resp["transactions"].as_array().cloned().unwrap_or_default();
            let errors       = resp["errors"].as_array().cloned().unwrap_or_default();

            let mut out = format!("Trace {trace_id}\n{} transaction(s), {} erreur(s)\n\n", transactions.len(), errors.len());
            for t in transactions.iter().take(20) {
                let span_id  = t["span_id"].as_str().unwrap_or("—");
                let txn      = t["transaction"].as_str().unwrap_or("—");
                let duration = t["duration"].as_f64().unwrap_or(0.0);
                out.push_str(&format!("• [{span_id}] {txn} ({duration:.1}ms)\n"));
            }
            if !errors.is_empty() {
                out.push_str("\nErreurs:\n");
                for e in &errors {
                    let title = e["title"].as_str().unwrap_or("—");
                    let span  = e["span"].as_str().unwrap_or("—");
                    out.push_str(&format!("• [{span}] {title}\n"));
                }
            }
            Ok(out.trim_end().to_string())
        }

        "sentry_analyze_issue_with_seer" => {
            let org_slug     = args["org_slug"].as_str().ok_or("Paramètre 'org_slug' requis")?;
            let project_slug = args["project_slug"].as_str().ok_or("Paramètre 'project_slug' requis")?;
            let issue_id     = args["issue_id"].as_str().ok_or("Paramètre 'issue_id' requis")?;
            let url          = cfg.api(&format!("projects/{org_slug}/{project_slug}/issues/{issue_id}/autofix/start"));
            let resp         = post_json(&cfg, &url, &json!({})).await?;

            let run_id = resp["run_id"].as_str().or_else(|| resp["id"].as_str()).unwrap_or("—");
            let status = resp["status"].as_str().unwrap_or("démarré");
            Ok(format!(
                "Analyse Seer lancée pour l'issue {issue_id}\nRun ID : {run_id}\nStatut  : {status}\n\nVérifier le résultat dans l'interface Sentry ou via sentry_get_issue."
            ))
        }

        "sentry_get_profile_details" => {
            let org_slug     = args["org_slug"].as_str().ok_or("Paramètre 'org_slug' requis")?;
            let project_slug = args["project_slug"].as_str().ok_or("Paramètre 'project_slug' requis")?;
            let profile_id   = args["profile_id"].as_str().ok_or("Paramètre 'profile_id' requis")?;
            let url          = cfg.api(&format!("projects/{org_slug}/{project_slug}/profiling/profiles/{profile_id}"));
            let resp         = get(&cfg, &url).await?;

            let version   = resp["version"].as_str().unwrap_or("—");
            let platform  = resp["platform"].as_str().unwrap_or("—");
            let duration  = resp["duration_ns"].as_u64().unwrap_or(0);
            let duration_ms = duration / 1_000_000;
            let ts        = resp["timestamp"].as_str().unwrap_or("—");
            let txn       = resp["transaction"]["name"].as_str().unwrap_or("—");

            Ok(format!(
                "Profil {profile_id}\nTransaction : {txn}\nPlateforme  : {platform}\nVersion     : {version}\nDurée       : {duration_ms}ms\nDate        : {ts}"
            ))
        }

        // ── Docs ─────────────────────────────────────────────────────────────
        "sentry_get_doc" => {
            let path    = args["path"].as_str().ok_or("Paramètre 'path' requis")?;
            let clean   = path.trim_start_matches('/');
            let url     = format!("https://docs.sentry.io/{clean}/");
            // On retourne l'URL et un résumé — l'API docs Sentry n'est pas publique
            Ok(format!("Documentation Sentry : {url}\n\nConsulter cette URL pour la documentation sur '{path}'."))
        }

        "sentry_search_docs" => {
            let query = args["query"].as_str().ok_or("Paramètre 'query' requis")?;
            let url   = format!("https://docs.sentry.io/?search={}", urlencoding_simple(query));
            Ok(format!("Recherche Sentry Docs pour '{query}' : {url}\n\nSuggestions de rubriques courantes :\n• platforms/javascript — SDK JavaScript\n• platforms/python — SDK Python\n• product/issues — Gestion des issues\n• product/performance — Monitoring performance\n• product/profiling — Profiling\n• product/alerts — Alertes"))
        }

        _ => Err(format!("Tool Sentry inconnu : {name}")),
    }
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

/// Encodage URL minimaliste pour les valeurs de query string (espaces → %20, etc.).
/// Évite d'ajouter une dépendance supplémentaire pour un besoin limité.
fn urlencoding_simple(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for ch in s.chars() {
        match ch {
            ' '  => out.push_str("%20"),
            ':'  => out.push_str("%3A"),
            '/'  => out.push_str("%2F"),
            '?'  => out.push_str("%3F"),
            '#'  => out.push_str("%23"),
            '&'  => out.push_str("%26"),
            '='  => out.push_str("%3D"),
            '+'  => out.push_str("%2B"),
            '%'  => out.push_str("%25"),
            c    => out.push(c),
        }
    }
    out
}
