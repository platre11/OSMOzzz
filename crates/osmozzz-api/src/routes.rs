use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::state::AppState;
use osmozzz_core::Embedder;
use osmozzz_harvester::{GmailConfig, SKIP_DIRS, TEXT_EXTENSIONS, harvest_file};

// ─── Types de réponse ────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct SourceStatus {
    pub count: usize,
    pub last_sync: Option<DateTime<Utc>>,
    pub error: Option<String>,
}

#[derive(Serialize)]
pub struct PerfMetrics {
    pub db_disk_mb: u64,
    pub process_rss_mb: Option<u64>,
    pub total_vectors: usize,
    pub estimated_ram_mb: u64,
}

#[derive(Serialize)]
pub struct StatusResponse {
    pub daemon_status: String,
    pub sources: HashMap<String, SourceStatus>,
    pub perf: PerfMetrics,
}

#[derive(Serialize)]
pub struct SearchDoc {
    pub url: String,
    pub title: Option<String>,
    pub content: String,
    pub date: Option<String>,
}

#[derive(Serialize)]
pub struct SourceGroup {
    pub source: String,
    pub results: Vec<SearchDoc>,
}

#[derive(Serialize)]
pub struct GroupedSearchResponse {
    pub groups: Vec<SourceGroup>,
}

#[derive(Serialize)]
pub struct RecentDoc {
    pub url: String,
    pub title: Option<String>,
    pub content: String,
    pub source: String,
    pub source_ts: Option<i64>,
}

/// Parse "YYYY-MM-DD" → timestamp Unix début de journée (00:00:00)
fn parse_date_ts(s: &str) -> Option<i64> {
    chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()
        .and_then(|d| d.and_hms_opt(0, 0, 0))
        .map(|dt| dt.and_utc().timestamp())
}

/// Parse "YYYY-MM-DD" → timestamp Unix fin de journée (23:59:59)
fn parse_date_ts_end(s: &str) -> Option<i64> {
    chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()
        .and_then(|d| d.and_hms_opt(23, 59, 59))
        .map(|dt| dt.and_utc().timestamp())
}

#[derive(Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub ok: bool,
    pub data: Option<T>,
    pub error: Option<String>,
}

impl<T: Serialize> ApiResponse<T> {
    pub fn ok(data: T) -> Json<Self> {
        Json(Self { ok: true, data: Some(data), error: None })
    }
    pub fn err(msg: impl Into<String>) -> Json<ApiResponse<()>> {
        Json(ApiResponse { ok: false, data: None, error: Some(msg.into()) })
    }
}

// ─── Query params ────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct SearchQuery {
    pub q: String,
    pub source: Option<String>,  // filtre source unique
    pub from:   Option<String>,  // YYYY-MM-DD
    pub to:     Option<String>,  // YYYY-MM-DD
}

#[derive(Deserialize)]
pub struct OpenQuery {
    pub url: String,
}

#[derive(Deserialize)]
pub struct RecentQuery {
    pub source: Option<String>,
    pub q:      Option<String>,  // recherche mot-clé dans la source
    pub from:   Option<String>,  // YYYY-MM-DD
    pub to:     Option<String>,  // YYYY-MM-DD
    #[serde(default = "default_limit")]
    pub limit: usize,
    #[serde(default)]
    pub offset: usize,
}

#[derive(Deserialize)]
pub struct SyncQuery {
    pub source: String,
}

#[derive(Deserialize)]
pub struct ConversationQuery {
    pub phone: String,
    #[serde(default = "default_conv_limit")]
    pub limit: usize,
}

fn default_conv_limit() -> usize { 200 }

#[derive(Serialize)]
pub struct ContactItem {
    pub phone: String,
    pub last_message: String,
    pub last_ts: i64,
    pub count: usize,
}

#[derive(Serialize)]
pub struct MessageItem {
    pub ts: i64,
    pub is_me: bool,
    pub text: String,
    pub date: Option<String>,
}

fn default_limit() -> usize { 200 }

// ─── GET /api/status ─────────────────────────────────────────────────────────

pub async fn get_status(State(state): State<AppState>) -> impl IntoResponse {
    // Sources locales : selon l'OS
    #[cfg(target_os = "macos")]
    let local_sources = ["email", "chrome", "file", "imessage", "safari", "notes", "terminal", "calendar"];
    #[cfg(not(target_os = "macos"))]
    let local_sources = ["email", "chrome", "file", "terminal"];
    let mut sources = HashMap::new();

    for src in &local_sources {
        let count = state.vault.count_source(src).await.unwrap_or(0);
        sources.insert(src.to_string(), SourceStatus { count, last_sync: None, error: None });
    }

    let total_vectors: usize = sources.values().map(|s| s.count).sum();
    // Estimation : 1 vecteur 384d f32 = 1536 bytes ≈ 1.5 KB
    let estimated_ram_mb = (total_vectors as u64 * 1536) / (1024 * 1024);
    let db_disk_mb = state.vault.db_disk_bytes() / (1024 * 1024);
    let process_rss_mb = osmozzz_embedder::Vault::process_rss_mb();

    ApiResponse::ok(StatusResponse {
        daemon_status: "running".to_string(),
        sources,
        perf: PerfMetrics { db_disk_mb, process_rss_mb, total_vectors, estimated_ram_mb },
    })
}

// ─── GET /api/search ─────────────────────────────────────────────────────────

pub async fn get_search(
    State(state): State<AppState>,
    Query(params): Query<SearchQuery>,
) -> impl IntoResponse {
    let q = &params.q;
    let from_ts = params.from.as_deref().and_then(parse_date_ts);
    let to_ts   = params.to.as_deref().and_then(|s| parse_date_ts_end(s));

    // Recherche AND multi-termes si `+` détecté (ex: "qonto + style + sécurité")
    if q.contains('+') {
        if let Ok(Some(results)) = state.vault.search_and_query(q, 20).await {
            #[cfg(target_os = "macos")]
            let order = ["email", "imessage", "chrome", "file", "safari", "notes", "terminal", "calendar"];
            #[cfg(not(target_os = "macos"))]
            let order = ["email", "chrome", "file", "terminal"];
            let mut by_source: std::collections::HashMap<String, Vec<SearchDoc>> = std::collections::HashMap::new();
            for r in &results {
                by_source.entry(r.source.clone()).or_default().push(SearchDoc {
                    url: r.url.clone(),
                    title: r.title.clone(),
                    content: truncate(&r.content, 300),
                    date: None,
                });
            }
            let groups: Vec<SourceGroup> = order.iter()
                .filter_map(|src| by_source.remove(*src).map(|docs| SourceGroup { source: src.to_string(), results: docs }))
                .collect();
            return ApiResponse::ok(GroupedSearchResponse { groups }).into_response();
        }
    }

    let file_q = q.to_lowercase();
    let (grouped_res, live_files) = tokio::join!(
        state.vault.search_grouped_by_keyword(q, 5),
        tokio::task::spawn_blocking(move || live_file_search_sync(file_q, std::collections::HashSet::new(), 5))
    );

    let mut grouped = match grouped_res {
        Ok(g) => g,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, ApiResponse::<GroupedSearchResponse>::err(e.to_string())).into_response(),
    };

    // Merge résultats disque dans le groupe "file" (dédupliqués par path)
    let live_files = live_files.unwrap_or_default();
    if !live_files.is_empty() {
        let file_entries = grouped.entry("file".to_string()).or_insert_with(Vec::new);
        let existing: std::collections::HashSet<String> = file_entries.iter().map(|(_, _, url, _)| url.clone()).collect();
        for fr in live_files {
            if !existing.contains(&fr.path) {
                file_entries.push((0, Some(fr.name), fr.path, fr.snippet));
            }
        }
    }

    #[cfg(target_os = "macos")]
    let source_order = ["email", "imessage", "chrome", "file", "safari", "notes", "terminal", "calendar"];
    #[cfg(not(target_os = "macos"))]
    let source_order = ["email", "chrome", "file", "terminal"];

    let groups: Vec<SourceGroup> = source_order.iter()
        .filter(|src| params.source.as_deref().map_or(true, |f| f == **src))
        .filter_map(|src| {
            grouped.get(*src).map(|results| {
                let filtered: Vec<SearchDoc> = results.iter()
                    .filter(|(ts, _, _, _)| {
                        from_ts.map_or(true, |f| *ts >= f) &&
                        to_ts.map_or(true,   |t| *ts <= t)
                    })
                    .map(|(ts, title, url, content)| SearchDoc {
                        url: url.clone(),
                        title: title.clone(),
                        content: truncate(content, 300),
                        date: if *ts > 0 {
                            chrono::DateTime::from_timestamp(*ts, 0)
                                .map(|dt: chrono::DateTime<chrono::Utc>| dt.format("%d/%m/%Y").to_string())
                        } else { None },
                    })
                    .collect();
                (src, filtered)
            })
        })
        .filter(|(_, results)| !results.is_empty())
        .map(|(src, results)| SourceGroup { source: src.to_string(), results })
        .collect();

    ApiResponse::ok(GroupedSearchResponse { groups }).into_response()
}

// ─── GET /api/recent ─────────────────────────────────────────────────────────

pub async fn get_recent(
    State(state): State<AppState>,
    Query(params): Query<RecentQuery>,
) -> impl IntoResponse {
    let source  = params.source.as_deref().unwrap_or("email");
    let keyword = params.q.as_deref().unwrap_or("");
    let from_ts = params.from.as_deref().and_then(parse_date_ts);
    let to_ts   = params.to.as_deref().and_then(|s| parse_date_ts_end(s));
    let has_filters = !keyword.is_empty() || from_ts.is_some() || to_ts.is_some();

    // Si filtres actifs : passer par la recherche datée (keyword + date range)
    // Sinon : chemin rapide recent_by_source
    if has_filters {
        let raw = match state.vault.search_by_keyword_dated(keyword, params.limit + params.offset + 200, source).await {
            Ok(r) => r,
            Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, ApiResponse::<Vec<RecentDoc>>::err(e.to_string())).into_response(),
        };

        let docs: Vec<RecentDoc> = raw.into_iter()
            .filter(|(ts, _, _, _)| {
                from_ts.map_or(true, |f| *ts >= f) &&
                to_ts.map_or(true,   |t| *ts <= t)
            })
            .skip(params.offset)
            .take(params.limit)
            .map(|(ts, title, url, content): (i64, Option<String>, String, String)| RecentDoc {
                url,
                title,
                content: truncate(&content, 300),
                source: source.to_string(),
                source_ts: if ts > 0 { Some(ts) } else { None },
            })
            .collect();

        return ApiResponse::ok(docs).into_response();
    }

    let results = match state.vault.recent_by_source(source, params.limit + params.offset).await {
        Ok(r) => r,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, ApiResponse::<Vec<RecentDoc>>::err(e.to_string())).into_response(),
    };

    let docs: Vec<RecentDoc> = results
        .into_iter()
        .skip(params.offset)
        .take(params.limit)
        .map(|r| RecentDoc {
            url: r.url,
            title: r.title,
            content: truncate(&r.content, 300),
            source: r.source,
            source_ts: None,
        })
        .collect();

    ApiResponse::ok(docs).into_response()
}

// ─── GET /api/config ─────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct ConnectorStatus {
    pub configured: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub display: Option<String>,
}

#[derive(Serialize)]
pub struct ConfigResponse {
    pub gmail: ConnectorStatus,
    pub notion: ConnectorStatus,
    pub github: ConnectorStatus,
    pub linear: ConnectorStatus,
    pub jira: ConnectorStatus,
    pub slack: ConnectorStatus,
    pub trello: ConnectorStatus,
    pub todoist: ConnectorStatus,
    pub gitlab: ConnectorStatus,
    pub airtable: ConnectorStatus,
    pub obsidian:  ConnectorStatus,
    pub supabase:  ConnectorStatus,
    pub sentry:     ConnectorStatus,
    pub cloudflare: ConnectorStatus,
    pub vercel:   ConnectorStatus,
    pub railway:  ConnectorStatus,
    pub render:   ConnectorStatus,
    pub google:   ConnectorStatus,
    pub stripe:   ConnectorStatus,
    pub hubspot:  ConnectorStatus,
    pub posthog:  ConnectorStatus,
    pub resend:   ConnectorStatus,
    pub discord:  ConnectorStatus,
    pub twilio:   ConnectorStatus,
    pub figma:    ConnectorStatus,
    pub reddit:   ConnectorStatus,
    pub calendly: ConnectorStatus,
    pub n8n:      ConnectorStatus,
    pub shopify:  ConnectorStatus,
}

fn osmozzz_dir() -> Option<std::path::PathBuf> {
    dirs_next::home_dir().map(|h| h.join(".osmozzz"))
}

fn connector_status(filename: &str, display_key: &str) -> ConnectorStatus {
    let path = match osmozzz_dir() {
        Some(d) => d.join(filename),
        None => return ConnectorStatus { configured: false, display: None },
    };
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return ConnectorStatus { configured: false, display: None },
    };
    // Parse le display_key si défini (ex: "token", "username")
    let display = if display_key.is_empty() {
        None
    } else {
        content.lines()
            .find(|l| l.trim_start().starts_with(display_key))
            .and_then(|l| l.split('=').nth(1))
            .map(|v| v.trim().trim_matches('"').to_string())
    };
    ConnectorStatus { configured: true, display }
}

pub async fn get_config() -> impl IntoResponse {
    let gmail_cfg = GmailConfig::load();
    ApiResponse::ok(ConfigResponse {
        gmail:    ConnectorStatus {
            configured: gmail_cfg.is_some(),
            display:    gmail_cfg.map(|c| c.username),
        },
        notion:   connector_status("notion.toml",   ""),
        github:   connector_status("github.toml",   "repos"),
        linear:   connector_status("linear.toml",   ""),
        jira:     connector_status("jira.toml",     "base_url"),
        slack:    connector_status("slack.toml",    "channels"),
        trello:   connector_status("trello.toml",   ""),
        todoist:  connector_status("todoist.toml",  ""),
        gitlab:   connector_status("gitlab.toml",   "base_url"),
        airtable: connector_status("airtable.toml", "bases"),
        obsidian: connector_status("obsidian.toml", "vault_path"),
        supabase: connector_status("supabase.toml", "project_id"),
        sentry:     connector_status("sentry.toml",     ""),
        cloudflare: connector_status("cloudflare.toml", "account_id"),
        vercel:   connector_status("vercel.toml",   ""),
        railway:  connector_status("railway.toml",  ""),
        render:   connector_status("render.toml",   ""),
        google:   connector_status("google.toml",   "username"),
        stripe:   connector_status("stripe.toml",   ""),
        hubspot:  connector_status("hubspot.toml",  ""),
        posthog:  connector_status("posthog.toml",  "project_id"),
        resend:   connector_status("resend.toml",   ""),
        discord:  connector_status("discord.toml",  "guild_id"),
        twilio:   connector_status("twilio.toml",   "account_sid"),
        figma:    connector_status("figma.toml",    "team_id"),
        reddit:   connector_status("reddit.toml",   "username"),
        calendly: connector_status("calendly.toml", ""),
        n8n:      connector_status("n8n.toml",      "api_url"),
        shopify:  connector_status("shopify.toml",  "shop_domain"),
    })
}

// ─── Helper : écrire un fichier de config ─────────────────────────────────────

fn write_config(filename: &str, content: &str) -> Result<(), String> {
    let dir = osmozzz_dir().ok_or("Cannot find home directory")?;
    std::fs::create_dir_all(&dir).map_err(|e| format!("Cannot create config dir: {}", e))?;
    std::fs::write(dir.join(filename), content)
        .map_err(|e| format!("Cannot write {}: {}", filename, e))
}

fn esc(s: &str) -> String { s.replace('"', "\\\"") }

// ─── POST /api/config/gmail ──────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct GmailConfigBody {
    pub username: String,
    pub app_password: String,
}

pub async fn post_config_gmail(Json(body): Json<GmailConfigBody>) -> impl IntoResponse {
    let content = format!(
        "username = \"{}\"\napp_password = \"{}\"\n",
        esc(&body.username), esc(&body.app_password)
    );
    match write_config("gmail.toml", &content) {
        Ok(_)  => ApiResponse::ok("Gmail configuré".to_string()).into_response(),
        Err(e) => ApiResponse::<String>::err(e).into_response(),
    }
}

// ─── POST /api/config/notion ─────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct NotionConfigBody { pub token: String }

pub async fn post_config_notion(Json(body): Json<NotionConfigBody>) -> impl IntoResponse {
    let content = format!("token = \"{}\"\n", esc(&body.token));
    match write_config("notion.toml", &content) {
        Ok(_)  => ApiResponse::ok("Notion configuré".to_string()).into_response(),
        Err(e) => ApiResponse::<String>::err(e).into_response(),
    }
}

// ─── POST /api/config/github ─────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct GithubConfigBody {
    pub token: String,
    pub repos: String, // "owner/repo1, owner/repo2"
}

pub async fn post_config_github(Json(body): Json<GithubConfigBody>) -> impl IntoResponse {
    let repos: Vec<String> = body.repos
        .split(',')
        .map(|s| format!("\"{}\"", esc(s.trim())))
        .filter(|s| s.len() > 2)
        .collect();
    let content = format!(
        "token = \"{}\"\nrepos = [{}]\n",
        esc(&body.token),
        repos.join(", ")
    );
    match write_config("github.toml", &content) {
        Ok(_)  => ApiResponse::ok("GitHub configuré".to_string()).into_response(),
        Err(e) => ApiResponse::<String>::err(e).into_response(),
    }
}

// ─── POST /api/config/linear ─────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct LinearConfigBody { pub api_key: String }

pub async fn post_config_linear(Json(body): Json<LinearConfigBody>) -> impl IntoResponse {
    let content = format!("api_key = \"{}\"\n", esc(&body.api_key));
    match write_config("linear.toml", &content) {
        Ok(_)  => ApiResponse::ok("Linear configuré".to_string()).into_response(),
        Err(e) => ApiResponse::<String>::err(e).into_response(),
    }
}

// ─── POST /api/config/jira ───────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct JiraConfigBody {
    pub base_url: String,
    pub email: String,
    pub token: String,
}

pub async fn post_config_jira(Json(body): Json<JiraConfigBody>) -> impl IntoResponse {
    let base_url = if body.base_url.starts_with("http") {
        body.base_url.clone()
    } else {
        format!("https://{}", body.base_url)
    };
    let content = format!(
        "base_url = \"{}\"\nemail = \"{}\"\ntoken = \"{}\"\n",
        esc(&base_url), esc(&body.email), esc(&body.token)
    );
    match write_config("jira.toml", &content) {
        Ok(_)  => ApiResponse::ok("Jira configuré".to_string()).into_response(),
        Err(e) => ApiResponse::<String>::err(e).into_response(),
    }
}

// ─── POST /api/config/slack ──────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct SlackConfigBody {
    pub token:    String,
    pub team_id:  String,
    pub channels: String, // "general, random, dev"
}

pub async fn post_config_slack(Json(body): Json<SlackConfigBody>) -> impl IntoResponse {
    let channels: Vec<String> = body.channels
        .split(',')
        .map(|s| format!("\"{}\"", esc(s.trim())))
        .filter(|s| s.len() > 2)
        .collect();
    let content = format!(
        "token = \"{}\"\nteam_id = \"{}\"\nchannels = [{}]\n",
        esc(&body.token),
        esc(&body.team_id),
        channels.join(", ")
    );
    match write_config("slack.toml", &content) {
        Ok(_)  => ApiResponse::ok("Slack configuré".to_string()).into_response(),
        Err(e) => ApiResponse::<String>::err(e).into_response(),
    }
}

// ─── POST /api/config/trello ─────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct TrelloConfigBody {
    pub api_key: String,
    pub token: String,
}

pub async fn post_config_trello(Json(body): Json<TrelloConfigBody>) -> impl IntoResponse {
    let content = format!(
        "api_key = \"{}\"\ntoken = \"{}\"\n",
        esc(&body.api_key), esc(&body.token)
    );
    match write_config("trello.toml", &content) {
        Ok(_)  => ApiResponse::ok("Trello configuré".to_string()).into_response(),
        Err(e) => ApiResponse::<String>::err(e).into_response(),
    }
}

// ─── POST /api/config/todoist ────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct TodoistConfigBody { pub token: String }

pub async fn post_config_todoist(Json(body): Json<TodoistConfigBody>) -> impl IntoResponse {
    let content = format!("token = \"{}\"\n", esc(&body.token));
    match write_config("todoist.toml", &content) {
        Ok(_)  => ApiResponse::ok("Todoist configuré".to_string()).into_response(),
        Err(e) => ApiResponse::<String>::err(e).into_response(),
    }
}

// ─── POST /api/config/gitlab ─────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct GitlabConfigBody {
    pub token: String,
    #[serde(default)]
    pub base_url: String,
    #[serde(default)]
    pub groups: String, // "groupe1, groupe2"
}

pub async fn post_config_gitlab(Json(body): Json<GitlabConfigBody>) -> impl IntoResponse {
    let base_url = if body.base_url.is_empty() {
        "https://gitlab.com".to_string()
    } else {
        body.base_url.clone()
    };
    let groups: Vec<String> = body.groups
        .split(',')
        .map(|s| format!("\"{}\"", esc(s.trim())))
        .filter(|s| s.len() > 2)
        .collect();
    let content = format!(
        "token = \"{}\"\nbase_url = \"{}\"\ngroups = [{}]\n",
        esc(&body.token), esc(&base_url), groups.join(", ")
    );
    match write_config("gitlab.toml", &content) {
        Ok(_)  => ApiResponse::ok("GitLab configuré".to_string()).into_response(),
        Err(e) => ApiResponse::<String>::err(e).into_response(),
    }
}

// ─── POST /api/config/airtable ───────────────────────────────────────────────

#[derive(Deserialize)]
pub struct AirtableConfigBody {
    pub token: String,
    pub bases: String, // "appXXXX, appYYYY"
}

pub async fn post_config_airtable(Json(body): Json<AirtableConfigBody>) -> impl IntoResponse {
    let bases: Vec<String> = body.bases
        .split(',')
        .map(|s| format!("\"{}\"", esc(s.trim())))
        .filter(|s| s.len() > 2)
        .collect();
    let content = format!(
        "token = \"{}\"\nbases = [{}]\n",
        esc(&body.token),
        bases.join(", ")
    );
    match write_config("airtable.toml", &content) {
        Ok(_)  => ApiResponse::ok("Airtable configuré".to_string()).into_response(),
        Err(e) => ApiResponse::<String>::err(e).into_response(),
    }
}

// ─── POST /api/config/cloudflare ─────────────────────────────────────────────

#[derive(Deserialize)]
pub struct CloudflareConfigBody {
    pub api_token:  String,
    pub account_id: String,
}

pub async fn post_config_cloudflare(Json(body): Json<CloudflareConfigBody>) -> impl IntoResponse {
    let content = format!(
        "api_token  = \"{}\"\naccount_id = \"{}\"\n",
        esc(&body.api_token),
        esc(&body.account_id),
    );
    match write_config("cloudflare.toml", &content) {
        Ok(_)  => ApiResponse::ok("Cloudflare configuré".to_string()).into_response(),
        Err(e) => ApiResponse::<String>::err(e).into_response(),
    }
}

// ─── POST /api/config/sentry ─────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct SentryConfigBody {
    pub token: String,
    #[serde(default)]
    pub host: String,
}

pub async fn post_config_sentry(Json(body): Json<SentryConfigBody>) -> impl IntoResponse {
    let mut content = format!("token = \"{}\"\n", esc(&body.token));
    if !body.host.is_empty() {
        content.push_str(&format!("host = \"{}\"\n", esc(&body.host)));
    }
    match write_config("sentry.toml", &content) {
        Ok(_)  => ApiResponse::ok("Sentry configuré".to_string()).into_response(),
        Err(e) => ApiResponse::<String>::err(e).into_response(),
    }
}

// ─── POST /api/config/obsidian ───────────────────────────────────────────────

#[derive(Deserialize)]
pub struct ObsidianConfigBody { pub vault_path: String }

pub async fn post_config_obsidian(Json(body): Json<ObsidianConfigBody>) -> impl IntoResponse {
    let content = format!("vault_path = \"{}\"\n", esc(&body.vault_path));
    match write_config("obsidian.toml", &content) {
        Ok(_)  => ApiResponse::ok("Obsidian configuré".to_string()).into_response(),
        Err(e) => ApiResponse::<String>::err(e).into_response(),
    }
}

// ─── POST /api/config/supabase ───────────────────────────────────────────────

#[derive(Deserialize)]
pub struct SupabaseConfigBody {
    pub access_token: String,
    pub project_id:   Option<String>,
}

pub async fn post_config_supabase(Json(body): Json<SupabaseConfigBody>) -> impl IntoResponse {
    let mut content = format!("access_token = \"{}\"\n", esc(&body.access_token));
    if let Some(ref pid) = body.project_id {
        if !pid.is_empty() {
            content.push_str(&format!("project_id = \"{}\"\n", esc(pid)));
        }
    }
    match write_config("supabase.toml", &content) {
        Ok(_)  => ApiResponse::ok("Supabase configuré".to_string()).into_response(),
        Err(e) => ApiResponse::<String>::err(e).into_response(),
    }
}

// ─── DB Security routes ───────────────────────────────────────────────────────

/// GET /api/db/supabase/projects — list projects for the configured access_token
pub async fn get_db_supabase_projects() -> impl IntoResponse {
    let config_path = dirs_next::home_dir().map(|h| h.join(".osmozzz/supabase.toml"));
    let access_token = match config_path.and_then(|p| std::fs::read_to_string(p).ok()) {
        Some(content) => {
            let table: toml::Value = match content.parse() {
                Ok(t) => t,
                Err(_) => return ApiResponse::<String>::err("Impossible de lire supabase.toml".to_string()).into_response(),
            };
            table.get("access_token").and_then(|v| v.as_str()).unwrap_or("").to_string()
        }
        None => return ApiResponse::<String>::err("supabase.toml non configuré".to_string()).into_response(),
    };
    if access_token.is_empty() {
        return ApiResponse::<String>::err("access_token manquant dans supabase.toml".to_string()).into_response();
    }
    let client = reqwest::Client::new();
    match client.get("https://api.supabase.com/v1/projects").bearer_auth(&access_token).send().await {
        Ok(res) => {
            let json: serde_json::Value = res.json().await.unwrap_or(serde_json::json!([]));
            ApiResponse::ok(json).into_response()
        }
        Err(e) => ApiResponse::<String>::err(e.to_string()).into_response(),
    }
}

/// POST /api/db/supabase/project — save project_id to supabase.toml
pub async fn post_db_supabase_project(
    Json(body): Json<serde_json::Value>,
) -> impl IntoResponse {
    let project_id = match body.get("project_id").and_then(|v| v.as_str()) {
        Some(id) => id.to_string(),
        None => return ApiResponse::<String>::err("project_id manquant".to_string()).into_response(),
    };
    let path = match dirs_next::home_dir() {
        Some(h) => h.join(".osmozzz/supabase.toml"),
        None => return ApiResponse::<String>::err("home dir introuvable".to_string()).into_response(),
    };
    let current = std::fs::read_to_string(&path).unwrap_or_default();
    let mut table: toml::value::Table = current.parse::<toml::Value>().ok()
        .and_then(|v| if let toml::Value::Table(t) = v { Some(t) } else { None })
        .unwrap_or_default();
    table.insert("project_id".to_string(), toml::Value::String(project_id));
    let content = toml::to_string(&toml::Value::Table(table)).unwrap_or_default();
    match std::fs::write(&path, content) {
        Ok(_)  => ApiResponse::ok("project_id sauvegardé".to_string()).into_response(),
        Err(e) => ApiResponse::<String>::err(e.to_string()).into_response(),
    }
}

/// GET /api/db/supabase/schema — import tables+columns from Supabase information_schema
pub async fn get_db_supabase_schema() -> impl IntoResponse {
    let config_path = dirs_next::home_dir()
        .map(|h| h.join(".osmozzz/supabase.toml"));

    let (access_token, project_id) = match config_path.and_then(|p| std::fs::read_to_string(p).ok()) {
        Some(content) => {
            let table: toml::Value = match content.parse() {
                Ok(t) => t,
                Err(_) => return ApiResponse::<String>::err("Impossible de lire supabase.toml".to_string()).into_response(),
            };
            let token = table.get("access_token").and_then(|v| v.as_str()).unwrap_or("").to_string();
            let pid   = table.get("project_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
            (token, pid)
        }
        None => return ApiResponse::<String>::err("supabase.toml non configuré".to_string()).into_response(),
    };

    if access_token.is_empty() || project_id.is_empty() {
        return ApiResponse::<String>::err("access_token et project_id requis".to_string()).into_response();
    }

    match crate::db::import_supabase_schema(&access_token, &project_id).await {
        Ok(tables) => ApiResponse::ok(tables).into_response(),
        Err(e)     => ApiResponse::<String>::err(e.to_string()).into_response(),
    }
}

/// GET /api/db/supabase/security — read db_security.toml
pub async fn get_db_supabase_security() -> impl IntoResponse {
    let config = crate::db::DbSecurityConfig::load();
    ApiResponse::ok(config).into_response()
}

/// POST /api/db/supabase/security — save db_security.toml
pub async fn post_db_supabase_security(
    Json(body): Json<crate::db::DbSecurityConfig>,
) -> impl IntoResponse {
    match body.save() {
        Ok(_)  => ApiResponse::ok("Configuration DB sauvegardée".to_string()).into_response(),
        Err(e) => ApiResponse::<String>::err(e.to_string()).into_response(),
    }
}

// ─── POST /api/ban ────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct BanRequest {
    /// "url" = ban one document | "source" = ban all from identifier
    pub kind: String,
    /// Document URL (required for kind="url")
    pub url: Option<String>,
    /// Source type: "email", "imessage", "chrome", "safari", "file"
    pub source: Option<String>,
    /// Identifier: sender email, phone number, domain, or file path
    pub identifier: Option<String>,
}

pub async fn post_ban(
    State(state): State<AppState>,
    Json(body): Json<BanRequest>,
) -> impl IntoResponse {
    match body.kind.as_str() {
        "url" => {
            let url = match &body.url {
                Some(u) if !u.is_empty() => u.clone(),
                _ => return ApiResponse::<String>::err("url requis").into_response(),
            };
            match state.vault.ban_url(&url).await {
                Ok(_)  => ApiResponse::ok("banni".to_string()).into_response(),
                Err(e) => ApiResponse::<String>::err(e.to_string()).into_response(),
            }
        }
        "source" => {
            let source = match &body.source {
                Some(s) if !s.is_empty() => s.clone(),
                _ => return ApiResponse::<String>::err("source requis").into_response(),
            };
            let identifier = match &body.identifier {
                Some(i) if !i.is_empty() => i.clone(),
                _ => return ApiResponse::<String>::err("identifier requis").into_response(),
            };
            match state.vault.ban_source_item(&source, &identifier).await {
                Ok(_)  => ApiResponse::ok("banni".to_string()).into_response(),
                Err(e) => ApiResponse::<String>::err(e.to_string()).into_response(),
            }
        }
        _ => ApiResponse::<String>::err("kind invalide (url|source)").into_response(),
    }
}

// ─── GET /api/messages/contacts ──────────────────────────────────────────────

pub async fn get_imessage_contacts(State(state): State<AppState>) -> impl IntoResponse {
    match state.vault.get_imessage_contacts().await {
        Ok(contacts) => {
            let items: Vec<ContactItem> = contacts.into_iter().map(|(phone, last_message, last_ts, count)| {
                ContactItem { phone, last_message, last_ts, count }
            }).collect();
            ApiResponse::ok(items).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, ApiResponse::<Vec<ContactItem>>::err(e.to_string())).into_response(),
    }
}

// ─── GET /api/messages/conversation ──────────────────────────────────────────

pub async fn get_imessage_conversation(
    State(state): State<AppState>,
    Query(params): Query<ConversationQuery>,
) -> impl IntoResponse {
    match state.vault.get_imessage_conversation(&params.phone, params.limit).await {
        Ok(messages) => {
            let items: Vec<MessageItem> = messages.into_iter().map(|(ts, is_me, text)| {
                MessageItem {
                    ts,
                    is_me,
                    text,
                    date: if ts > 0 {
                        chrono::DateTime::from_timestamp(ts, 0)
                            .map(|dt: chrono::DateTime<chrono::Utc>| dt.format("%d/%m/%Y %H:%M").to_string())
                    } else { None },
                }
            }).collect();
            ApiResponse::ok(items).into_response()
        }
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, ApiResponse::<Vec<MessageItem>>::err(e.to_string())).into_response(),
    }
}

// ─── GET /api/open ───────────────────────────────────────────────────────────

const ALLOWED_PROTOCOLS: &[&str] = &[
    "file://", "https://", "http://",
    "imessage://", "sms://", "notes://",
    "calendar://", "facetime://",
    "x-apple.systempreferences:",
];

fn is_url_allowed(url: &str) -> bool {
    // Chemin local absolu (commence par /)
    if url.starts_with('/') {
        // Interdit path traversal hors du home
        return !url.contains("../");
    }
    // Protocole whitelisté
    ALLOWED_PROTOCOLS.iter().any(|p| url.starts_with(p))
}

pub async fn get_open(Query(params): Query<OpenQuery>) -> impl IntoResponse {
    let url = &params.url;

    if !is_url_allowed(url) {
        return (
            StatusCode::BAD_REQUEST,
            ApiResponse::<String>::err(format!("Protocole non autorisé: {}", url)),
        ).into_response();
    }

    let result = if url.starts_with('/') {
        std::process::Command::new("open").args(["-R", url.as_str()]).spawn()
    } else if url.starts_with("file://") {
        let path = url.trim_start_matches("file://");
        std::process::Command::new("open").args(["-R", path]).spawn()
    } else {
        std::process::Command::new("open").arg(url.as_str()).spawn()
    };

    match result {
        Ok(_)  => ApiResponse::ok("ok").into_response(),
        Err(e) => ApiResponse::<String>::err(e.to_string()).into_response(),
    }
}

// ─── GET /api/blacklist ───────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct BlacklistEntry {
    pub kind: String,
    pub source: String,
    pub identifier: String,
    pub title: Option<String>,
    pub content: Option<String>,
}

#[derive(Serialize)]
pub struct BlacklistResponse {
    pub entries: Vec<BlacklistEntry>,
}

pub async fn get_blacklist(State(state): State<AppState>) -> impl IntoResponse {
    let bl = state.vault.get_blacklist();
    let raw = bl.get_all_entries();

    // Collect URL bans to enrich with vault data
    let url_bans: Vec<String> = raw.iter()
        .filter(|(kind, _, _)| kind == "url")
        .map(|(_, _, id)| id.clone())
        .collect();

    let mut doc_map: std::collections::HashMap<String, (String, Option<String>, String)> = std::collections::HashMap::new();
    if !url_bans.is_empty() {
        if let Ok(docs) = state.vault.get_docs_info_by_urls(&url_bans).await {
            for (url, source, title, content) in docs {
                doc_map.insert(url, (source, title, content));
            }
        }
    }

    let entries: Vec<BlacklistEntry> = raw.into_iter().map(|(kind, source, identifier)| {
        if kind == "url" {
            if let Some((real_source, title, content)) = doc_map.get(&identifier) {
                let snippet = truncate(&content, 200);
                return BlacklistEntry {
                    kind,
                    source: real_source.clone(),
                    identifier,
                    title: title.clone(),
                    content: Some(snippet),
                };
            }
        }
        BlacklistEntry { kind, source, identifier, title: None, content: None }
    }).collect();

    ApiResponse::ok(BlacklistResponse { entries })
}

// ─── POST /api/unban ─────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct UnbanRequest {
    pub kind: String,
    pub url: Option<String>,
    pub source: Option<String>,
    pub identifier: Option<String>,
}

pub async fn post_unban(
    State(state): State<AppState>,
    Json(body): Json<UnbanRequest>,
) -> impl IntoResponse {
    match body.kind.as_str() {
        "url" => {
            let url = match &body.url {
                Some(u) if !u.is_empty() => u.clone(),
                _ => return ApiResponse::<String>::err("url requis").into_response(),
            };
            match state.vault.unban_url(&url).await {
                Ok(_)  => ApiResponse::ok("débanni".to_string()).into_response(),
                Err(e) => ApiResponse::<String>::err(e.to_string()).into_response(),
            }
        }
        "source" => {
            let source = match &body.source {
                Some(s) if !s.is_empty() => s.clone(),
                _ => return ApiResponse::<String>::err("source requis").into_response(),
            };
            let identifier = match &body.identifier {
                Some(i) if !i.is_empty() => i.clone(),
                _ => return ApiResponse::<String>::err("identifier requis").into_response(),
            };
            match state.vault.unban_source_item(&source, &identifier).await {
                Ok(_)  => ApiResponse::ok("débanni".to_string()).into_response(),
                Err(e) => ApiResponse::<String>::err(e.to_string()).into_response(),
            }
        }
        _ => ApiResponse::<String>::err("kind invalide (url|source)").into_response(),
    }
}

// ─── POST /api/compact ───────────────────────────────────────────────────────

pub async fn post_compact(State(state): State<AppState>) -> impl IntoResponse {
    match state.vault.compact().await {
        Ok(_)  => ApiResponse::ok("Compactage terminé".to_string()).into_response(),
        Err(e) => ApiResponse::<String>::err(e.to_string()).into_response(),
    }
}

// ─── POST /api/reindex/imessage ──────────────────────────────────────────────

pub async fn post_reindex_imessage(State(state): State<AppState>) -> impl IntoResponse {
    #[cfg(target_os = "macos")]
    {
        use osmozzz_core::{Embedder, Harvester};
        use osmozzz_harvester::IMessageHarvester;

        // 1. Vide la source
        if let Err(e) = state.vault.delete_by_source("imessage").await {
            return ApiResponse::<String>::err(format!("Erreur suppression: {e}")).into_response();
        }

        // 2. Re-indexe
        let harvester = IMessageHarvester::new();
        match harvester.harvest().await {
            Err(e) => ApiResponse::<String>::err(format!("Erreur harvest: {e}")).into_response(),
            Ok(docs) => {
                let count = docs.len();
                for doc in docs {
                    let _ = state.vault.upsert(&doc).await;
                }
                ApiResponse::ok(format!("{count} documents iMessage indexés")).into_response()
            }
        }
    }
    #[cfg(not(target_os = "macos"))]
    {
        ApiResponse::<String>::err("iMessage disponible sur macOS uniquement".to_string()).into_response()
    }
}

// ─── Réseau P2P ──────────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct PeerResponse {
    pub peer_id: String,
    pub display_name: String,
    pub addresses: Vec<String>,
    pub connected: bool,
    pub last_seen: Option<i64>,
    pub shared_sources: Vec<String>,
}

#[derive(Serialize)]
pub struct InviteResponse {
    pub link: String,
    pub peer_id: String,
}

#[derive(Deserialize)]
pub struct ConnectRequest {
    pub link: String,
    pub display_name: String,
}

#[derive(Deserialize)]
pub struct PermissionsBody {
    pub allowed_sources: Vec<String>,
    pub max_results_per_query: Option<usize>,
}

pub async fn get_network_peers(State(state): State<AppState>) -> impl IntoResponse {
    let Some(p2p) = &state.p2p else {
        return ApiResponse::ok(Vec::<PeerResponse>::new()).into_response();
    };
    let peers = p2p.store.all();
    let connected_ids = p2p.connected_peer_ids().await;
    let response: Vec<PeerResponse> = peers.into_iter().map(|p| {
        let connected = connected_ids.contains(&p.peer_id);
        PeerResponse {
            peer_id: p.peer_id,
            display_name: p.display_name,
            addresses: p.addresses,
            connected,
            last_seen: p.last_seen,
            shared_sources: p.permissions.allowed_source_names(),
        }
    }).collect();
    ApiResponse::ok(response).into_response()
}

pub async fn post_network_invite(State(state): State<AppState>) -> impl IntoResponse {
    let Some(p2p) = &state.p2p else {
        return ApiResponse::<InviteResponse>::err("P2P non initialisé").into_response();
    };
    match p2p.generate_invite_link().await {
        Ok(link) => ApiResponse::ok(InviteResponse { link, peer_id: p2p.identity.id.clone() }).into_response(),
        Err(e)   => ApiResponse::<InviteResponse>::err(e.to_string()).into_response(),
    }
}

pub async fn post_network_connect(
    State(state): State<AppState>,
    Json(body): Json<ConnectRequest>,
) -> impl IntoResponse {
    let Some(p2p) = &state.p2p else {
        return ApiResponse::<String>::err("P2P non initialisé").into_response();
    };
    match p2p.accept_invite(&body.link, &body.display_name) {
        Ok(peer) => {
            // Tente la connexion TCP en arrière-plan
            let node = p2p.clone();
            let address = peer.addresses.first().cloned().unwrap_or_default();
            tokio::spawn(async move {
                if !address.is_empty() {
                    if let Err(e) = node.connect_to_peer(&address).await {
                        tracing::warn!("[P2P] Connexion sortante échouée : {}", e);
                    }
                }
            });
            ApiResponse::ok("Peer ajouté — connexion en cours".to_string()).into_response()
        }
        Err(e) => ApiResponse::<String>::err(e.to_string()).into_response(),
    }
}

pub async fn delete_network_peer(
    State(state): State<AppState>,
    axum::extract::Path(peer_id): axum::extract::Path<String>,
) -> impl IntoResponse {
    let Some(p2p) = &state.p2p else {
        return ApiResponse::<String>::err("P2P non initialisé").into_response();
    };
    match p2p.store.remove(&peer_id) {
        Ok(_) => ApiResponse::ok("Peer supprimé".to_string()).into_response(),
        Err(e) => ApiResponse::<String>::err(e.to_string()).into_response(),
    }
}

pub async fn get_network_permissions(
    State(state): State<AppState>,
    axum::extract::Path(peer_id): axum::extract::Path<String>,
) -> impl IntoResponse {
    let Some(p2p) = &state.p2p else {
        return ApiResponse::<osmozzz_p2p::PeerPermissions>::err("P2P non initialisé").into_response();
    };
    match p2p.store.get(&peer_id) {
        Some(peer) => ApiResponse::ok(peer.permissions).into_response(),
        None => ApiResponse::<osmozzz_p2p::PeerPermissions>::err("Peer introuvable").into_response(),
    }
}

pub async fn post_network_permissions(
    State(state): State<AppState>,
    axum::extract::Path(peer_id): axum::extract::Path<String>,
    Json(body): Json<PermissionsBody>,
) -> impl IntoResponse {
    let Some(p2p) = &state.p2p else {
        return ApiResponse::<String>::err("P2P non initialisé").into_response();
    };
    let allowed = body.allowed_sources.iter()
        .filter_map(|s| source_str_to_enum(s))
        .collect();
    // Préserve les tool_permissions existantes
    let existing_tool_perms = p2p.store.get(&peer_id)
        .map(|p| p.permissions.tool_permissions)
        .unwrap_or_default();
    let perms = osmozzz_p2p::PeerPermissions {
        allowed_sources: allowed,
        max_results_per_query: body.max_results_per_query.unwrap_or(10),
        tool_permissions: existing_tool_perms,
    };
    match p2p.store.update_permissions(&peer_id, perms) {
        Ok(_) => ApiResponse::ok("Permissions mises à jour".to_string()).into_response(),
        Err(e) => ApiResponse::<String>::err(e.to_string()).into_response(),
    }
}

pub async fn get_network_history(State(state): State<AppState>) -> impl IntoResponse {
    let Some(p2p) = &state.p2p else {
        return ApiResponse::ok(Vec::<osmozzz_p2p::QueryHistoryEntry>::new()).into_response();
    };
    let entries = p2p.history.recent(100);
    ApiResponse::ok(entries).into_response()
}

// ─── Identité locale ─────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct IdentityResponse {
    pub peer_id: String,
    pub display_name: String,
}

pub async fn get_network_identity(State(state): State<AppState>) -> impl IntoResponse {
    let Some(p2p) = &state.p2p else {
        return ApiResponse::<IdentityResponse>::err("P2P non initialisé").into_response();
    };
    let display_name = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "OSMOzzz".to_string());
    ApiResponse::ok(IdentityResponse {
        peer_id: p2p.identity.id.clone(),
        display_name,
    }).into_response()
}

// ─── Permissions tools par peer ───────────────────────────────────────────────

#[derive(Deserialize)]
pub struct ToolPermissionsBody {
    pub permissions: std::collections::HashMap<String, String>,
}

pub async fn get_network_tool_permissions(
    State(state): State<AppState>,
    axum::extract::Path(peer_id): axum::extract::Path<String>,
) -> impl IntoResponse {
    let Some(p2p) = &state.p2p else {
        return ApiResponse::<std::collections::HashMap<String, osmozzz_p2p::ToolAccessMode>>::err(
            "P2P non initialisé",
        ).into_response();
    };
    match p2p.store.get(&peer_id) {
        Some(peer) => ApiResponse::ok(peer.permissions.tool_permissions).into_response(),
        None => ApiResponse::<std::collections::HashMap<String, osmozzz_p2p::ToolAccessMode>>::err(
            "Peer introuvable",
        ).into_response(),
    }
}

pub async fn post_network_tool_permissions(
    State(state): State<AppState>,
    axum::extract::Path(peer_id): axum::extract::Path<String>,
    Json(body): Json<ToolPermissionsBody>,
) -> impl IntoResponse {
    let Some(p2p) = &state.p2p else {
        return ApiResponse::<String>::err("P2P non initialisé").into_response();
    };
    let perms: std::collections::HashMap<String, osmozzz_p2p::ToolAccessMode> = body
        .permissions
        .into_iter()
        .filter_map(|(k, v)| {
            let mode = match v.as_str() {
                "auto"     => osmozzz_p2p::ToolAccessMode::Auto,
                "require"  => osmozzz_p2p::ToolAccessMode::Require,
                "disabled" => osmozzz_p2p::ToolAccessMode::Disabled,
                _ => return None,
            };
            Some((k, mode))
        })
        .collect();
    match p2p.store.update_tool_permissions(&peer_id, perms) {
        Ok(_)  => ApiResponse::ok("Permissions outils mises à jour".to_string()).into_response(),
        Err(e) => ApiResponse::<String>::err(e.to_string()).into_response(),
    }
}

// ─── GET /api/configured-connectors ──────────────────────────────────────────
// Retourne la liste des connecteurs pour lesquels un fichier .toml existe.

pub async fn get_configured_connectors() -> impl IntoResponse {
    let home = match dirs_next::home_dir() {
        Some(h) => h.join(".osmozzz"),
        None => return ApiResponse::ok(Vec::<String>::new()).into_response(),
    };
    // (toml_file, connector_id)
    let mapping = [
        ("github.toml",     "github"),
        ("notion.toml",     "notion"),
        ("slack.toml",      "slack"),
        ("linear.toml",     "linear"),
        ("jira.toml",       "jira"),
        ("gitlab.toml",     "gitlab"),
        ("supabase.toml",   "supabase"),
        ("sentry.toml",     "sentry"),
        ("cloudflare.toml", "cloudflare"),
        ("vercel.toml",     "vercel"),
        ("railway.toml",    "railway"),
        ("render.toml",     "render"),
        ("google.toml",     "gcal"),
        ("stripe.toml",     "stripe"),
        ("hubspot.toml",    "hubspot"),
        ("posthog.toml",    "posthog"),
        ("resend.toml",     "resend"),
        ("discord.toml",    "discord"),
        ("twilio.toml",     "twilio"),
        ("figma.toml",      "figma"),
    ];
    let configured: Vec<String> = mapping.iter()
        .filter(|(file, _)| home.join(file).exists())
        .map(|(_, id)| id.to_string())
        .collect();
    ApiResponse::ok(configured).into_response()
}

// ─── POST /api/network/simulate-peer-request ─────────────────────────────────
//
// Injecte une fausse requête P2P dans la file d'actions pour tester le flux
// approbation/rejet sans avoir besoin d'un 2e Mac.

#[derive(Deserialize)]
pub struct SimulateBody {
    #[serde(default = "default_sim_peer_name")]
    pub peer_name: String,
    #[serde(default = "default_sim_tool")]
    pub tool_name: String,
    #[serde(default = "default_sim_query")]
    pub query: String,
}
fn default_sim_peer_name() -> String { "Alice (simulation)".to_string() }
fn default_sim_tool()      -> String { "search_memory".to_string() }
fn default_sim_query()     -> String { "test de connexion P2P".to_string() }

pub async fn post_network_simulate(
    State(state): State<AppState>,
    body: Option<Json<SimulateBody>>,
) -> impl IntoResponse {
    use osmozzz_core::{ActionRequest, ActionStatus};
    let sim = body.map(|Json(b)| b).unwrap_or_else(|| SimulateBody {
        peer_name: default_sim_peer_name(),
        tool_name: default_sim_tool(),
        query:     default_sim_query(),
    });
    let now = chrono::Utc::now().timestamp();
    let params = serde_json::json!({ "query": sim.query });
    let preview = format!(
        "[P2P Simulation — {}] {} — paramètres : {}",
        sim.peer_name,
        sim.tool_name,
        serde_json::to_string(&params).unwrap_or_default()
    );
    let action = ActionRequest {
        id: uuid::Uuid::new_v4().to_string(),
        tool: format!("p2p:{}", sim.tool_name),
        params,
        preview,
        status: ActionStatus::Pending,
        created_at: now,
        expires_at: now + 300,
        execution_result: None,
        mcp_proxy: None,
    };
    let action_id = action.id.clone();
    state.p2p_action_queue.push(action);
    ApiResponse::ok(serde_json::json!({
        "action_id": action_id,
        "message": format!("Requête simulée de \"{}\" → apparaît dans la section Flux P2P de la page Réseau", sim.peer_name)
    })).into_response()
}

// ─── GET /api/network/connected-peers ────────────────────────────────────────

#[derive(Serialize)]
pub struct ConnectedPeer {
    pub peer_id: String,
    pub display_name: String,
}

pub async fn get_network_connected_peers(State(state): State<AppState>) -> impl IntoResponse {
    let Some(p2p) = &state.p2p else {
        return ApiResponse::<Vec<ConnectedPeer>>::err("P2P non initialisé").into_response();
    };
    let peers = p2p.connected_peers_list().await
        .into_iter()
        .map(|(peer_id, display_name)| ConnectedPeer { peer_id, display_name })
        .collect::<Vec<_>>();
    ApiResponse::ok(peers).into_response()
}

// ─── POST /api/network/call-peer-tool ────────────────────────────────────────

#[derive(Deserialize)]
pub struct CallPeerToolBody {
    pub peer_id: String,
    pub tool_name: String,
    pub params: serde_json::Value,
}

#[derive(Serialize)]
pub struct CallPeerToolResponse {
    pub result: Option<String>,
    pub error: Option<String>,
}

pub async fn post_network_call_peer_tool(
    State(state): State<AppState>,
    Json(body): Json<CallPeerToolBody>,
) -> impl IntoResponse {
    let Some(p2p) = &state.p2p else {
        return ApiResponse::<CallPeerToolResponse>::err("P2P non initialisé").into_response();
    };
    match p2p.call_peer_tool(&body.peer_id, &body.tool_name, body.params).await {
        Ok(res) => ApiResponse::ok(CallPeerToolResponse {
            result: res.result,
            error: res.error,
        }).into_response(),
        Err(e) => ApiResponse::<CallPeerToolResponse>::err(e.to_string()).into_response(),
    }
}

fn source_str_to_enum(s: &str) -> Option<osmozzz_p2p::SharedSource> {
    use osmozzz_p2p::SharedSource;
    match s {
        "chrome"   => Some(SharedSource::Chrome),
        "safari"   => Some(SharedSource::Safari),
        "email"    => Some(SharedSource::Email),
        "imessage" => Some(SharedSource::IMessage),
        "notes"    => Some(SharedSource::Notes),
        "calendar" => Some(SharedSource::Calendar),
        "terminal" => Some(SharedSource::Terminal),
        "file"     => Some(SharedSource::File),
        "notion"   => Some(SharedSource::Notion),
        "github"   => Some(SharedSource::Github),
        "linear"   => Some(SharedSource::Linear),
        "jira"     => Some(SharedSource::Jira),
        "slack"    => Some(SharedSource::Slack),
        "trello"   => Some(SharedSource::Trello),
        "todoist"  => Some(SharedSource::Todoist),
        "gitlab"   => Some(SharedSource::Gitlab),
        "airtable" => Some(SharedSource::Airtable),
        "obsidian" => Some(SharedSource::Obsidian),
        _ => None,
    }
}

// ─── GET /api/aliases ────────────────────────────────────────────────────────

#[derive(Serialize, Deserialize, Clone)]
pub struct AliasEntry {
    pub real: String,
    pub alias: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alias_type: Option<String>,
}

#[derive(Serialize)]
pub struct AliasesResponse {
    pub aliases: Vec<AliasEntry>,
    pub types: Vec<String>,
}

#[derive(Deserialize)]
pub struct AliasesBody {
    pub aliases: Vec<AliasEntry>,
    #[serde(default)]
    pub types: Vec<String>,
}

pub async fn get_aliases() -> impl IntoResponse {
    let path = match dirs_next::home_dir() {
        Some(h) => h.join(".osmozzz/aliases.toml"),
        None => return ApiResponse::ok(AliasesResponse { aliases: vec![], types: vec![] }).into_response(),
    };
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return ApiResponse::ok(AliasesResponse { aliases: vec![], types: vec![] }).into_response(),
    };
    let t: toml::Value = match content.parse() {
        Ok(v) => v,
        Err(_) => return ApiResponse::ok(AliasesResponse { aliases: vec![], types: vec![] }).into_response(),
    };
    // Nouveau format : [[entries]] + [meta].types
    let mut entries: Vec<AliasEntry> = Vec::new();
    if let Some(arr) = t.get("entries").and_then(|v| v.as_array()) {
        for item in arr {
            if let (Some(real), Some(alias)) = (
                item.get("real").and_then(|v| v.as_str()),
                item.get("alias").and_then(|v| v.as_str()),
            ) {
                let alias_type = item.get("alias_type").and_then(|v| v.as_str()).map(|s| s.to_string());
                entries.push(AliasEntry { real: real.to_string(), alias: alias.to_string(), alias_type });
            }
        }
    }
    // Compat ancien format [map]
    if entries.is_empty() {
        if let Some(map) = t.get("map").and_then(|v| v.as_table()) {
            for (real, alias) in map {
                if let Some(alias_str) = alias.as_str() {
                    entries.push(AliasEntry { real: real.clone(), alias: alias_str.to_string(), alias_type: None });
                }
            }
        }
    }
    let types: Vec<String> = t.get("meta")
        .and_then(|v| v.get("types"))
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
        .unwrap_or_default();
    ApiResponse::ok(AliasesResponse { aliases: entries, types }).into_response()
}

// ─── POST /api/aliases ───────────────────────────────────────────────────────

pub async fn post_aliases(Json(body): Json<AliasesBody>) -> impl IntoResponse {
    let path = match dirs_next::home_dir() {
        Some(h) => h.join(".osmozzz/aliases.toml"),
        None => return ApiResponse::<String>::err("impossible de trouver le home dir").into_response(),
    };
    // [[entries]] avec alias_type optionnel
    let entries_arr: Vec<toml::Value> = body.aliases.iter().map(|entry| {
        let mut tbl = toml::map::Map::new();
        tbl.insert("real".to_string(), toml::Value::String(entry.real.clone()));
        tbl.insert("alias".to_string(), toml::Value::String(entry.alias.clone()));
        if let Some(t) = &entry.alias_type {
            tbl.insert("alias_type".to_string(), toml::Value::String(t.clone()));
        }
        toml::Value::Table(tbl)
    }).collect();
    // [meta] types
    let mut meta_tbl = toml::map::Map::new();
    meta_tbl.insert("types".to_string(), toml::Value::Array(
        body.types.iter().map(|t| toml::Value::String(t.clone())).collect()
    ));
    let mut doc = toml::map::Map::new();
    doc.insert("meta".to_string(), toml::Value::Table(meta_tbl));
    doc.insert("entries".to_string(), toml::Value::Array(entries_arr));
    let content = match toml::to_string(&toml::Value::Table(doc)) {
        Ok(s) => s,
        Err(e) => return ApiResponse::<String>::err(e.to_string()).into_response(),
    };
    match std::fs::write(&path, content) {
        Ok(_)  => ApiResponse::ok("ok".to_string()).into_response(),
        Err(e) => ApiResponse::<String>::err(e.to_string()).into_response(),
    }
}

// ─── GET /api/privacy ────────────────────────────────────────────────────────

pub async fn get_privacy() -> impl IntoResponse {
    let config = osmozzz_core::filter::PrivacyConfig::load();
    ApiResponse::ok(config)
}

// ─── POST /api/privacy ───────────────────────────────────────────────────────

pub async fn post_privacy(Json(body): Json<osmozzz_core::filter::PrivacyConfig>) -> impl IntoResponse {
    match body.save() {
        Ok(_)  => ApiResponse::ok("ok".to_string()).into_response(),
        Err(e) => ApiResponse::<String>::err(e).into_response(),
    }
}

// ─── POST /api/index ─────────────────────────────────────────────────────────

// ─── GET /api/files/search?q=... ─────────────────────────────────────────────
// Recherche directe sur le disque, SANS index LanceDB.
// Même principe que le MCP tool find_file.

#[derive(Deserialize)]
pub struct FilesSearchQuery {
    pub q: String,
    #[serde(default = "default_files_limit")]
    pub limit: usize,
    /// Extensions filtrées, séparées par virgule : "pdf,md,txt"
    #[serde(default)]
    pub exts: String,
}
fn default_files_limit() -> usize { 40 }

#[derive(Serialize, Clone)]
pub struct FilesSearchResult {
    pub path: String,
    pub name: String,
    pub ext: String,
    pub size_kb: u64,
    pub snippet: String,
}

/// Recherche synchrone sur disque (Desktop/Documents/Downloads).
/// `allowed_exts` vide = toutes les extensions.
fn live_file_search_sync(
    query: String,
    allowed_exts: std::collections::HashSet<String>,
    limit: usize,
) -> Vec<FilesSearchResult> {
    let home = dirs_next::home_dir().unwrap_or_default();
    let roots = vec![home.join("Desktop"), home.join("Documents"), home.join("Downloads")];
    let mut found = Vec::new();

    for root in &roots {
        if !root.exists() { continue; }
        for entry in walkdir::WalkDir::new(root)
            .follow_links(false)
            .max_depth(20)
            .into_iter()
            .filter_entry(|e| {
                let name = e.file_name().to_string_lossy();
                !name.starts_with('.') && !SKIP_DIRS.contains(&&*name.to_string())
            })
            .filter_map(|e| e.ok())
        {
            if found.len() >= limit { break; }
            let path = entry.path();
            if !path.is_file() { continue; }

            let name = path.file_name()
                .and_then(|n| n.to_str()).unwrap_or("").to_string();
            let ext = path.extension()
                .and_then(|e| e.to_str()).unwrap_or("").to_lowercase();

            if !allowed_exts.is_empty() && !allowed_exts.contains(&ext) { continue; }

            let size_bytes = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
            let size_kb = size_bytes / 1024;

            let name_match = name.to_lowercase().contains(&query);

            let extracted_text: Option<String> = if ext == "pdf" && size_bytes < 10 * 1024 * 1024 {
                pdf_extract::extract_text(path).ok()
            } else if TEXT_EXTENSIONS.contains(&ext.as_str()) && size_bytes < 2 * 1024 * 1024 {
                std::fs::read_to_string(path).ok()
            } else { None };

            let content_match = if !name_match {
                extracted_text.as_ref().map(|t| t.to_lowercase().contains(&query)).unwrap_or(false)
            } else { false };

            if !name_match && !content_match { continue; }

            let snippet = if let Some(ref text) = extracted_text {
                let lower = text.to_lowercase();
                if let Some(pos) = lower.find(&query) {
                    let mut start = pos.saturating_sub(120);
                    while !text.is_char_boundary(start) { start += 1; }
                    let mut end = (pos + query.len() + 200).min(text.len());
                    while !text.is_char_boundary(end) { end -= 1; }
                    format!("...{}...", text[start..end].trim())
                } else { String::new() }
            } else { String::new() };

            found.push(FilesSearchResult { path: path.display().to_string(), name, ext, size_kb, snippet });
        }
    }
    found
}

pub async fn get_files_search(
    Query(params): Query<FilesSearchQuery>,
) -> impl IntoResponse {
    let query = params.q.to_lowercase();
    if query.trim().is_empty() {
        return ApiResponse::ok(Vec::<FilesSearchResult>::new()).into_response();
    }

    let allowed_exts: std::collections::HashSet<String> = params.exts
        .split(',')
        .map(|s| s.trim().to_lowercase())
        .filter(|s| !s.is_empty())
        .collect();

    let limit = params.limit;
    let results = tokio::task::spawn_blocking(move || {
        live_file_search_sync(query, allowed_exts, limit)
    }).await.unwrap_or_default();

    ApiResponse::ok(results).into_response()
}

// ─── GET /api/index/preview ───────────────────────────────────────────────────

#[derive(Serialize)]
pub struct IndexPreview {
    /// extension → nombre de fichiers (ex: {"pdf": 12, "md": 45})
    pub extensions: HashMap<String, usize>,
}

pub async fn get_index_preview() -> impl IntoResponse {
    use osmozzz_harvester::SKIP_DIRS;
    use walkdir::WalkDir;

    let home = dirs_next::home_dir().unwrap_or_default();
    let paths = vec![
        home.join("Desktop"),
        home.join("Documents"),
        home.join("Downloads"),
    ];

    let mut counts: HashMap<String, usize> = HashMap::new();

    // Walkdir léger : on ne lit pas les fichiers, juste les extensions
    for root in &paths {
        if !root.exists() { continue; }
        for entry in WalkDir::new(root)
            .follow_links(false)
            .into_iter()
            .filter_entry(|e: &walkdir::DirEntry| {
                let name = e.file_name().to_string_lossy();
                !name.starts_with('.') && !SKIP_DIRS.contains(&&*name)
            })
            .filter_map(|e: Result<walkdir::DirEntry, _>| e.ok())
        {
            let path = entry.path();
            if !path.is_file() { continue; }
            let ext = path.extension()
                .and_then(|e: &std::ffi::OsStr| e.to_str())
                .unwrap_or("other")
                .to_lowercase();
            *counts.entry(ext).or_insert(0) += 1;
        }
    }

    // Filtrer les extensions avec moins de 1 fichier + trier par count desc
    let mut counts: HashMap<String, usize> = counts.into_iter()
        .filter(|(_, c)| *c >= 1)
        .collect();

    // Regrouper les extensions très mineures (< 3 fichiers) dans "other"
    let minor: Vec<String> = counts.iter()
        .filter(|(k, v)| **v < 3 && k.as_str() != "pdf")
        .map(|(k, _)| k.clone())
        .collect();
    let minor_count: usize = minor.iter().map(|k| counts[k]).sum();
    for k in &minor { counts.remove(k); }
    if minor_count > 0 {
        *counts.entry("other".to_string()).or_insert(0) += minor_count;
    }

    ApiResponse::ok(IndexPreview { extensions: counts }).into_response()
}

// ─── POST /api/index ──────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct IndexBody {
    pub path: Option<String>,
    /// Extensions à indexer — vide = toutes (ex: ["pdf", "md", "txt"])
    pub extensions: Option<Vec<String>>,
}

#[derive(Serialize)]
pub struct IndexResult {
    pub indexed: usize,
    pub skipped: usize,
}

// ─── GET /api/index/progress ─────────────────────────────────────────────────

pub async fn get_index_progress(State(state): State<AppState>) -> impl IntoResponse {
    let p = state.index_progress.lock().unwrap().clone();
    ApiResponse::ok(p).into_response()
}

// ─── POST /api/index ─────────────────────────────────────────────────────────

pub async fn post_index(
    State(state): State<AppState>,
    Json(body): Json<IndexBody>,
) -> impl IntoResponse {
    use crate::state::IndexProgress;

    // Refuse if already running
    {
        let p = state.index_progress.lock().unwrap();
        if p.running {
            return ApiResponse::<String>::err("Indexation déjà en cours".to_string()).into_response();
        }
    }

    let roots: Vec<std::path::PathBuf> = if let Some(ref p) = body.path {
        vec![std::path::PathBuf::from(::shellexpand::tilde(p).to_string())]
    } else {
        let home = dirs_next::home_dir().unwrap_or_default();
        vec![home.join("Desktop"), home.join("Documents"), home.join("Downloads")]
    };

    let ext_filter: std::collections::HashSet<String> = body.extensions
        .unwrap_or_default()
        .into_iter()
        .map(|e| e.to_lowercase())
        .collect();

    // Count total files first for progress bar
    let total = {
        let mut n = 0usize;
        for root in &roots {
            if !root.exists() { continue; }
            for entry in walkdir::WalkDir::new(root).follow_links(false).into_iter()
                .filter_entry(|e| {
                    let name = e.file_name().to_string_lossy();
                    !name.starts_with('.') && !SKIP_DIRS.contains(&&*name.to_string())
                })
                .filter_map(|e| e.ok())
            {
                if !entry.path().is_file() { continue; }
                if ext_filter.is_empty() { n += 1; continue; }
                let ext = entry.path().extension()
                    .and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
                if ext_filter.contains(&ext) { n += 1; }
            }
        }
        n
    };

    {
        let mut p = state.index_progress.lock().unwrap();
        *p = IndexProgress { running: true, total, ..Default::default() };
    }

    let progress = state.index_progress.clone();
    let vault = state.vault.clone();

    tokio::spawn(async move {
        let mut indexed = 0usize;
        let mut skipped = 0usize;

        for root in &roots {
            if !root.exists() { continue; }

            for entry in walkdir::WalkDir::new(root).follow_links(false).into_iter()
                .filter_entry(|e| {
                    let name = e.file_name().to_string_lossy();
                    !name.starts_with('.') && !SKIP_DIRS.contains(&&*name.to_string())
                })
                .filter_map(|e| e.ok())
            {
                let path = entry.path();
                if !path.is_file() { continue; }

                // Extension filter
                let ext = path.extension()
                    .and_then(|e| e.to_str()).unwrap_or("").to_lowercase();
                if !ext_filter.is_empty() && !ext_filter.contains(&ext) { continue; }

                let file_size = std::fs::metadata(path).map(|m| m.len()).unwrap_or(0);
                let file_name = path.file_name()
                    .and_then(|n| n.to_str()).unwrap_or("").to_string();

                // Update progress
                {
                    let mut p = progress.lock().unwrap();
                    p.processed += 1;
                    p.current_file = file_name.clone();
                    p.indexed = indexed;
                    p.skipped = skipped;
                }

                let docs = tokio::task::spawn_blocking({
                    let path = path.to_path_buf();
                    move || harvest_file(&path, file_size, &Default::default())
                }).await.unwrap_or_default();

                if docs.is_empty() {
                    skipped += 1;
                    continue;
                }

                let mut any_indexed = false;
                for doc in &docs {
                    // PDFs: store text only (no ONNX = no OOM)
                    let result = if ext == "pdf" {
                        vault.store_text_only(doc).await
                    } else {
                        vault.upsert(doc).await
                    };
                    match result {
                        Ok(_) => { any_indexed = true; }
                        Err(_) => {}
                    }
                }
                if any_indexed { indexed += 1; } else { skipped += 1; }
            }
        }

        let mut p = progress.lock().unwrap();
        p.running = false;
        p.indexed = indexed;
        p.skipped = skipped;
        p.current_file = String::new();
    });

    ApiResponse::ok(IndexResult { indexed: 0, skipped: 0 }).into_response()
}

// ─── Actions orchestrateur ────────────────────────────────────────────────────

use axum::{
    extract::Path,
    response::sse::{Event, KeepAlive, Sse},
};
use std::convert::Infallible;
use tokio_stream::wrappers::BroadcastStream;
use futures::StreamExt;
use osmozzz_core::ActionRequest;

/// GET /api/actions — historique complet (les plus récents en premier)
pub async fn get_actions_all(State(state): State<AppState>) -> impl IntoResponse {
    let actions = state.action_queue.all();
    ApiResponse::ok(actions).into_response()
}

/// GET /api/actions/pending — actions en attente de validation
pub async fn get_actions_pending(State(state): State<AppState>) -> impl IntoResponse {
    let pending = state.action_queue.pending();
    ApiResponse::ok(pending).into_response()
}

/// POST /api/actions — soumet une nouvelle action (appelé par le process MCP)
pub async fn post_action(
    State(state): State<AppState>,
    Json(action): Json<ActionRequest>,
) -> impl IntoResponse {
    state.action_queue.push(action);
    ApiResponse::ok(serde_json::json!({ "ok": true })).into_response()
}

/// POST /api/actions/:id/approve — l'utilisateur approuve l'action, puis exécution immédiate
pub async fn post_action_approve(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.action_queue.approve(&id) {
        Some(action) => {
            // Si mcp_proxy = true, le process MCP gère l'exécution (il poll le statut).
            // On change juste le statut à Approved — pas d'executor ici.
            if action.mcp_proxy == Some(true) {
                return ApiResponse::ok(action).into_response();
            }
            // Exécution en arrière-plan pour ne pas bloquer la réponse HTTP
            let queue = state.action_queue.clone();
            let action_id = action.id.clone();
            let action_clone = action.clone();
            tokio::spawn(async move {
                let result = crate::executor::execute(&action_clone).await;
                queue.set_execution_result(&action_id, result);
            });
            ApiResponse::ok(action).into_response()
        }
        None => (StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": "Action non trouvée" }))).into_response(),
    }
}

/// POST /api/actions/:id/reject — l'utilisateur rejette l'action
pub async fn post_action_reject(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.action_queue.reject(&id) {
        Some(action) => ApiResponse::ok(action).into_response(),
        None => (StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": "Action non trouvée" }))).into_response(),
    }
}

/// GET /api/actions/stream — flux SSE temps réel vers le dashboard
pub async fn get_actions_stream(
    State(state): State<AppState>,
) -> Sse<impl futures::Stream<Item = Result<Event, Infallible>>> {
    let rx = state.action_queue.subscribe();
    let stream = BroadcastStream::new(rx).filter_map(|result| {
        let event = result.ok().and_then(|ev| {
            serde_json::to_string(&ev).ok().map(|data| Ok(Event::default().data(data)))
        });
        async move { event }
    });
    Sse::new(stream).keep_alive(KeepAlive::default())
}

/// GET /api/actions/:id — statut d'une action spécifique (pour polling MCP)
pub async fn get_action_by_id(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    let all = state.action_queue.all();
    match all.into_iter().find(|a| a.id == id) {
        Some(action) => ApiResponse::ok(action).into_response(),
        None => (StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": "Action non trouvée" }))).into_response(),
    }
}

// ─── Permissions MCP ─────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct McpPermissions {
    pub jira:   bool,
    pub github: bool,
    pub linear: bool,
    pub notion: bool,
    pub email:  bool,
}

fn permissions_path() -> Option<std::path::PathBuf> {
    Some(dirs_next::home_dir()?.join(".osmozzz/permissions.toml"))
}

pub fn load_permissions() -> McpPermissions {
    let path = match permissions_path() { Some(p) => p, None => return McpPermissions::default() };
    let content = match std::fs::read_to_string(&path) { Ok(c) => c, Err(_) => return McpPermissions::default() };
    toml::from_str(&content).unwrap_or_default()
}

/// GET /api/permissions — permissions d'autorisation par connecteur MCP
pub async fn get_permissions() -> impl IntoResponse {
    ApiResponse::ok(load_permissions()).into_response()
}

#[derive(Deserialize)]
pub struct McpPermissionsBody {
    pub jira:   bool,
    pub github: bool,
    pub linear: bool,
    pub notion: bool,
    pub email:  bool,
}

/// POST /api/permissions — sauvegarde les permissions
pub async fn post_permissions(Json(body): Json<McpPermissionsBody>) -> impl IntoResponse {
    let content = format!(
        "jira = {}\ngithub = {}\nlinear = {}\nnotion = {}\nemail = {}\n",
        body.jira, body.github, body.linear, body.notion, body.email,
    );
    match write_config("permissions.toml", &content) {
        Ok(_)  => ApiResponse::ok("Permissions sauvegardées".to_string()).into_response(),
        Err(e) => ApiResponse::<String>::err(e).into_response(),
    }
}

// ─── Accès aux sources MCP ───────────────────────────────────────────────────

fn default_source_access() -> std::collections::HashMap<String, bool> {
    let mut m = std::collections::HashMap::new();
    for key in &["email","imessage","chrome","safari","notes","calendar","terminal","file",
                 "notion","github","linear","jira","slack","trello","todoist","gitlab",
                 "airtable","obsidian","supabase","sentry","cloudflare","vercel","railway",
                 "render","google","stripe","hubspot","posthog","resend","discord","twilio","figma"] {
        m.insert(key.to_string(), true);
    }
    m
}

pub fn load_source_access() -> std::collections::HashMap<String, bool> {
    let path = match dirs_next::home_dir() { Some(d) => d.join(".osmozzz/source_access.toml"), None => return default_source_access() };
    let content = match std::fs::read_to_string(&path) { Ok(c) => c, Err(_) => return default_source_access() };
    // Essaie d'abord le nouveau format HashMap, sinon parse l'ancien format struct
    if let Ok(map) = toml::from_str::<std::collections::HashMap<String, bool>>(&content) {
        // S'assure que toutes les clés par défaut sont présentes
        let mut defaults = default_source_access();
        defaults.extend(map);
        defaults
    } else {
        default_source_access()
    }
}

/// GET /api/source-access — accès aux sources par Claude via MCP
pub async fn get_source_access() -> impl IntoResponse {
    ApiResponse::ok(load_source_access()).into_response()
}

/// POST /api/source-access — met à jour les accès sources
pub async fn post_source_access(Json(body): Json<std::collections::HashMap<String, bool>>) -> impl IntoResponse {
    let mut lines = String::new();
    let mut keys: Vec<&String> = body.keys().collect();
    keys.sort();
    for k in keys {
        lines.push_str(&format!("{} = {}\n", k, body[k]));
    }
    match write_config("source_access.toml", &lines) {
        Ok(_)  => ApiResponse::ok("Accès sources sauvegardés".to_string()).into_response(),
        Err(e) => ApiResponse::<String>::err(e).into_response(),
    }
}

// ─── Audit log ───────────────────────────────────────────────────────────────

#[derive(Debug, Serialize, Deserialize)]
pub struct AuditEntry {
    pub ts:      i64,
    pub tool:    String,
    pub query:   String,
    pub results: usize,
    pub blocked: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data:    Option<serde_json::Value>,
}

fn audit_path() -> Option<std::path::PathBuf> {
    Some(dirs_next::home_dir()?.join(".osmozzz/audit.jsonl"))
}

/// GET /api/audit?limit=200 — retourne les dernières entrées d'audit (les plus récentes en premier)
pub async fn get_audit(
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    let limit: usize = params.get("limit").and_then(|v| v.parse().ok()).unwrap_or(200);
    let path = match audit_path() { Some(p) => p, None => return ApiResponse::ok(Vec::<AuditEntry>::new()).into_response() };
    let content = match std::fs::read_to_string(&path) { Ok(c) => c, Err(_) => return ApiResponse::ok(Vec::<AuditEntry>::new()).into_response() };
    let entries: Vec<AuditEntry> = content
        .lines()
        .rev()
        .take(limit)
        .filter_map(|line| serde_json::from_str(line).ok())
        .collect();
    ApiResponse::ok(entries).into_response()
}

// ─── POST /api/config/vercel ─────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct VercelConfigBody {
    pub token: String,
    #[serde(default)]
    pub team_id: String,
}

pub async fn post_config_vercel(Json(body): Json<VercelConfigBody>) -> impl IntoResponse {
    let mut content = format!("token = \"{}\"\n", esc(&body.token));
    if !body.team_id.is_empty() {
        content.push_str(&format!("team_id = \"{}\"\n", esc(&body.team_id)));
    }
    match write_config("vercel.toml", &content) {
        Ok(_)  => ApiResponse::ok("Vercel configuré".to_string()).into_response(),
        Err(e) => ApiResponse::<String>::err(e).into_response(),
    }
}

// ─── POST /api/config/railway ────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct RailwayConfigBody {
    pub token: String,
}

pub async fn post_config_railway(Json(body): Json<RailwayConfigBody>) -> impl IntoResponse {
    let content = format!("token = \"{}\"\n", esc(&body.token));
    match write_config("railway.toml", &content) {
        Ok(_)  => ApiResponse::ok("Railway configuré".to_string()).into_response(),
        Err(e) => ApiResponse::<String>::err(e).into_response(),
    }
}

// ─── POST /api/config/render ─────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct RenderConfigBody {
    pub token: String,
}

pub async fn post_config_render(Json(body): Json<RenderConfigBody>) -> impl IntoResponse {
    let content = format!("token = \"{}\"\n", esc(&body.token));
    match write_config("render.toml", &content) {
        Ok(_)  => ApiResponse::ok("Render configuré".to_string()).into_response(),
        Err(e) => ApiResponse::<String>::err(e).into_response(),
    }
}

// ─── POST /api/config/google (Google Calendar CalDAV) ────────────────────────

#[derive(Deserialize)]
pub struct GoogleConfigBody {
    pub username:     String,
    pub app_password: String,
}

pub async fn post_config_google(Json(body): Json<GoogleConfigBody>) -> impl IntoResponse {
    let content = format!(
        "username     = \"{}\"\napp_password = \"{}\"\n",
        esc(&body.username),
        esc(&body.app_password),
    );
    match write_config("google.toml", &content) {
        Ok(_)  => ApiResponse::ok("Google Calendar configuré".to_string()).into_response(),
        Err(e) => ApiResponse::<String>::err(e).into_response(),
    }
}

// ─── POST /api/config/stripe ─────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct StripeConfigBody {
    pub secret_key: String,
}

pub async fn post_config_stripe(Json(body): Json<StripeConfigBody>) -> impl IntoResponse {
    let content = format!("secret_key = \"{}\"\n", esc(&body.secret_key));
    match write_config("stripe.toml", &content) {
        Ok(_)  => ApiResponse::ok("Stripe configuré".to_string()).into_response(),
        Err(e) => ApiResponse::<String>::err(e).into_response(),
    }
}

// ─── POST /api/config/hubspot ────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct HubspotConfigBody { pub token: String }

pub async fn post_config_hubspot(Json(body): Json<HubspotConfigBody>) -> impl IntoResponse {
    let content = format!("token = \"{}\"\n", esc(&body.token));
    match write_config("hubspot.toml", &content) {
        Ok(_)  => ApiResponse::ok("HubSpot configuré".to_string()).into_response(),
        Err(e) => ApiResponse::<String>::err(e).into_response(),
    }
}

// ─── POST /api/config/posthog ────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct PosthogConfigBody {
    pub api_key: String,
    pub project_id: String,
    #[serde(default)]
    pub host: String,
}

pub async fn post_config_posthog(Json(body): Json<PosthogConfigBody>) -> impl IntoResponse {
    let mut content = format!("api_key    = \"{}\"\nproject_id = \"{}\"\n", esc(&body.api_key), esc(&body.project_id));
    if !body.host.is_empty() {
        content.push_str(&format!("host = \"{}\"\n", esc(&body.host)));
    }
    match write_config("posthog.toml", &content) {
        Ok(_)  => ApiResponse::ok("PostHog configuré".to_string()).into_response(),
        Err(e) => ApiResponse::<String>::err(e).into_response(),
    }
}

// ─── POST /api/config/resend ─────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct ResendConfigBody { pub api_key: String }

pub async fn post_config_resend(Json(body): Json<ResendConfigBody>) -> impl IntoResponse {
    let content = format!("api_key = \"{}\"\n", esc(&body.api_key));
    match write_config("resend.toml", &content) {
        Ok(_)  => ApiResponse::ok("Resend configuré".to_string()).into_response(),
        Err(e) => ApiResponse::<String>::err(e).into_response(),
    }
}

// ─── POST /api/config/discord ────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct DiscordConfigBody {
    pub bot_token: String,
    #[serde(default)]
    pub guild_id: String,
}

pub async fn post_config_discord(Json(body): Json<DiscordConfigBody>) -> impl IntoResponse {
    let mut content = format!("bot_token = \"{}\"\n", esc(&body.bot_token));
    if !body.guild_id.is_empty() {
        content.push_str(&format!("guild_id = \"{}\"\n", esc(&body.guild_id)));
    }
    match write_config("discord.toml", &content) {
        Ok(_)  => ApiResponse::ok("Discord configuré".to_string()).into_response(),
        Err(e) => ApiResponse::<String>::err(e).into_response(),
    }
}

// ─── POST /api/config/twilio ─────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct TwilioConfigBody {
    pub account_sid: String,
    pub auth_token: String,
    #[serde(default)]
    pub from_number: String,
}

pub async fn post_config_twilio(Json(body): Json<TwilioConfigBody>) -> impl IntoResponse {
    let mut content = format!("account_sid = \"{}\"\nauth_token  = \"{}\"\n", esc(&body.account_sid), esc(&body.auth_token));
    if !body.from_number.is_empty() {
        content.push_str(&format!("from_number = \"{}\"\n", esc(&body.from_number)));
    }
    match write_config("twilio.toml", &content) {
        Ok(_)  => ApiResponse::ok("Twilio configuré".to_string()).into_response(),
        Err(e) => ApiResponse::<String>::err(e).into_response(),
    }
}

// ─── POST /api/config/figma ──────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct FigmaConfigBody {
    pub token: String,
    #[serde(default)]
    pub team_id: String,
}

pub async fn post_config_figma(Json(body): Json<FigmaConfigBody>) -> impl IntoResponse {
    let mut content = format!("token = \"{}\"\n", esc(&body.token));
    if !body.team_id.is_empty() {
        content.push_str(&format!("team_id = \"{}\"\n", esc(&body.team_id)));
    }
    match write_config("figma.toml", &content) {
        Ok(_)  => ApiResponse::ok("Figma configuré".to_string()).into_response(),
        Err(e) => ApiResponse::<String>::err(e).into_response(),
    }
}

// ─── POST /api/config/reddit ─────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct RedditConfigBody {
    pub client_id:     String,
    pub client_secret: String,
    pub username:      String,
    pub password:      String,
}

pub async fn post_config_reddit(Json(body): Json<RedditConfigBody>) -> impl IntoResponse {
    let content = format!(
        "client_id     = \"{}\"\nclient_secret = \"{}\"\nusername      = \"{}\"\npassword      = \"{}\"\n",
        esc(&body.client_id),
        esc(&body.client_secret),
        esc(&body.username),
        esc(&body.password),
    );
    match write_config("reddit.toml", &content) {
        Ok(_)  => ApiResponse::ok("Reddit configuré".to_string()).into_response(),
        Err(e) => ApiResponse::<String>::err(e).into_response(),
    }
}

// ─── POST /api/config/calendly ────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct CalendlyConfigBody {
    pub token: String,
}

pub async fn post_config_calendly(Json(body): Json<CalendlyConfigBody>) -> impl IntoResponse {
    let content = format!("token = \"{}\"\n", esc(&body.token));
    match write_config("calendly.toml", &content) {
        Ok(_)  => ApiResponse::ok("Calendly configuré".to_string()).into_response(),
        Err(e) => ApiResponse::<String>::err(e).into_response(),
    }
}

// ─── POST /api/config/n8n ─────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct N8nConfigBody {
    pub api_url: String,
    pub api_key: String,
}

pub async fn post_config_n8n(Json(body): Json<N8nConfigBody>) -> impl IntoResponse {
    let content = format!(
        "api_url = \"{}\"\napi_key = \"{}\"\n",
        esc(&body.api_url),
        esc(&body.api_key),
    );
    match write_config("n8n.toml", &content) {
        Ok(_)  => ApiResponse::ok("n8n configuré".to_string()).into_response(),
        Err(e) => ApiResponse::<String>::err(e).into_response(),
    }
}

// ─── POST /api/config/shopify ─────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct ShopifyConfigBody {
    pub shop_domain: String,
    pub access_token: String,
}

pub async fn post_config_shopify(Json(body): Json<ShopifyConfigBody>) -> impl IntoResponse {
    let content = format!(
        "shop_domain = \"{}\"\naccess_token = \"{}\"\n",
        esc(&body.shop_domain),
        esc(&body.access_token),
    );
    match write_config("shopify.toml", &content) {
        Ok(_)  => ApiResponse::ok("Shopify configuré".to_string()).into_response(),
        Err(e) => ApiResponse::<String>::err(e).into_response(),
    }
}

// ─── P2P Flux d'approbation (séparé de la queue locale Claude) ───────────────

/// GET /api/network/p2p-pending — actions P2P en attente d'approbation
pub async fn get_network_p2p_pending(State(state): State<AppState>) -> impl IntoResponse {
    ApiResponse::ok(state.p2p_action_queue.pending()).into_response()
}

/// GET /api/network/p2p-stream — SSE temps réel pour le flux P2P
pub async fn get_network_p2p_stream(
    State(state): State<AppState>,
) -> Sse<impl futures::Stream<Item = Result<Event, Infallible>>> {
    let rx = state.p2p_action_queue.subscribe();
    let stream = BroadcastStream::new(rx).filter_map(|result| {
        let event = result.ok().and_then(|ev| {
            serde_json::to_string(&ev).ok().map(|data| Ok(Event::default().data(data)))
        });
        async move { event }
    });
    Sse::new(stream).keep_alive(KeepAlive::default())
}

/// POST /api/network/p2p-actions/:id/approve
pub async fn post_network_p2p_approve(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.p2p_action_queue.approve(&id) {
        Some(action) => ApiResponse::ok(action).into_response(),
        None => (StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": "Action P2P non trouvée" }))).into_response(),
    }
}

/// POST /api/network/p2p-actions/:id/reject
pub async fn post_network_p2p_reject(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> impl IntoResponse {
    match state.p2p_action_queue.reject(&id) {
        Some(action) => ApiResponse::ok(action).into_response(),
        None => (StatusCode::NOT_FOUND, Json(serde_json::json!({ "error": "Action P2P non trouvée" }))).into_response(),
    }
}

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    let mut end = max;
    while !s.is_char_boundary(end) { end -= 1; }
    format!("{}...", &s[..end])
}

