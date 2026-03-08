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
use osmozzz_harvester::GmailConfig;

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
    // Sources locales : toujours présentes
    let local_sources = ["email", "chrome", "file", "imessage", "safari", "notes", "terminal", "calendar"];
    // Sources cloud : présentes seulement si le .toml de config existe
    let cloud_sources = [
        ("notion",   "notion.toml"),
        ("github",   "github.toml"),
        ("linear",   "linear.toml"),
        ("jira",     "jira.toml"),
        ("slack",    "slack.toml"),
        ("trello",   "trello.toml"),
        ("todoist",  "todoist.toml"),
        ("gitlab",   "gitlab.toml"),
        ("airtable", "airtable.toml"),
        ("obsidian", "obsidian.toml"),
    ];

    let mut sources = HashMap::new();

    for src in &local_sources {
        let count = state.vault.count_source(src).await.unwrap_or(0);
        sources.insert(src.to_string(), SourceStatus { count, last_sync: None, error: None });
    }

    if let Some(dir) = osmozzz_dir() {
        for (src, toml_file) in &cloud_sources {
            if dir.join(toml_file).exists() {
                let count = state.vault.count_source(src).await.unwrap_or(0);
                sources.insert(src.to_string(), SourceStatus { count, last_sync: None, error: None });
            }
        }
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

    let grouped = match state.vault.search_grouped_by_keyword(q, 5).await {
        Ok(g) => g,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, ApiResponse::<GroupedSearchResponse>::err(e.to_string())).into_response(),
    };

    let source_order = ["email", "imessage", "chrome", "file", "safari", "notes", "terminal", "calendar",
                        "notion", "github", "linear", "jira", "slack", "trello", "todoist", "gitlab", "airtable", "obsidian"];

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
    pub obsidian: ConnectorStatus,
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
        notion:   connector_status("notion.toml",   "token"),
        github:   connector_status("github.toml",   "repos"),
        linear:   connector_status("linear.toml",   ""),
        jira:     connector_status("jira.toml",     "base_url"),
        slack:    connector_status("slack.toml",    "channels"),
        trello:   connector_status("trello.toml",   ""),
        todoist:  connector_status("todoist.toml",  ""),
        gitlab:   connector_status("gitlab.toml",   "base_url"),
        airtable: connector_status("airtable.toml", "bases"),
        obsidian: connector_status("obsidian.toml", "vault_path"),
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
    let content = format!(
        "base_url = \"{}\"\nemail = \"{}\"\ntoken = \"{}\"\n",
        esc(&body.base_url), esc(&body.email), esc(&body.token)
    );
    match write_config("jira.toml", &content) {
        Ok(_)  => ApiResponse::ok("Jira configuré".to_string()).into_response(),
        Err(e) => ApiResponse::<String>::err(e).into_response(),
    }
}

// ─── POST /api/config/slack ──────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct SlackConfigBody {
    pub token: String,
    pub channels: String, // "general, random, dev"
}

pub async fn post_config_slack(Json(body): Json<SlackConfigBody>) -> impl IntoResponse {
    let channels: Vec<String> = body.channels
        .split(',')
        .map(|s| format!("\"{}\"", esc(s.trim())))
        .filter(|s| s.len() > 2)
        .collect();
    let content = format!(
        "token = \"{}\"\nchannels = [{}]\n",
        esc(&body.token),
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
    let perms = osmozzz_p2p::PeerPermissions {
        allowed_sources: allowed,
        max_results_per_query: body.max_results_per_query.unwrap_or(10),
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

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    let mut end = max;
    while !s.is_char_boundary(end) { end -= 1; }
    format!("{}...", &s[..end])
}
