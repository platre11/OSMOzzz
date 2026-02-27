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
pub struct StatusResponse {
    pub daemon_status: String,
    pub sources: HashMap<String, SourceStatus>,
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
}

#[derive(Deserialize)]
pub struct OpenQuery {
    pub url: String,
}

#[derive(Deserialize)]
pub struct RecentQuery {
    pub source: Option<String>,
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
    let sources_list = ["email", "chrome", "file", "imessage", "safari", "notes", "terminal", "calendar"];
    let mut sources = HashMap::new();

    for src in &sources_list {
        let count = state.vault.count_source(src).await.unwrap_or(0);
        sources.insert(src.to_string(), SourceStatus {
            count,
            last_sync: None,
            error: None,
        });
    }

    ApiResponse::ok(StatusResponse {
        daemon_status: "running".to_string(),
        sources,
    })
}

// ─── GET /api/search ─────────────────────────────────────────────────────────

pub async fn get_search(
    State(state): State<AppState>,
    Query(params): Query<SearchQuery>,
) -> impl IntoResponse {
    let q = &params.q;

    let grouped = match state.vault.search_grouped_by_keyword(q, 5).await {
        Ok(g) => g,
        Err(e) => return (StatusCode::INTERNAL_SERVER_ERROR, ApiResponse::<GroupedSearchResponse>::err(e.to_string())).into_response(),
    };

    // Ordre d'affichage fixe — sources vides filtrées automatiquement
    let source_order = ["email", "imessage", "chrome", "file", "safari", "notes", "terminal", "calendar"];
    let groups: Vec<SourceGroup> = source_order.iter()
        .filter_map(|src| {
            grouped.get(*src).map(|results| SourceGroup {
                source: src.to_string(),
                results: results.iter().map(|(ts, title, url, content)| SearchDoc {
                    url: url.clone(),
                    title: title.clone(),
                    content: truncate(content, 300),
                    date: if *ts > 0 {
                        chrono::DateTime::from_timestamp(*ts, 0)
                            .map(|dt: chrono::DateTime<chrono::Utc>| dt.format("%d/%m/%Y").to_string())
                    } else { None },
                }).collect(),
            })
        })
        .collect();

    ApiResponse::ok(GroupedSearchResponse { groups }).into_response()
}

// ─── GET /api/recent ─────────────────────────────────────────────────────────

pub async fn get_recent(
    State(state): State<AppState>,
    Query(params): Query<RecentQuery>,
) -> impl IntoResponse {
    let source = params.source.as_deref().unwrap_or("email");
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
        })
        .collect();

    ApiResponse::ok(docs).into_response()
}

// ─── GET /api/config ─────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct ConfigResponse {
    pub gmail_configured: bool,
    pub gmail_username: Option<String>,
}

pub async fn get_config() -> impl IntoResponse {
    let config = GmailConfig::load();
    ApiResponse::ok(ConfigResponse {
        gmail_configured: config.is_some(),
        gmail_username: config.map(|c| c.username),
    })
}

// ─── POST /api/config/gmail ──────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct GmailConfigBody {
    pub username: String,
    pub app_password: String,
}

pub async fn post_config_gmail(
    Json(body): Json<GmailConfigBody>,
) -> impl IntoResponse {
    let home: std::path::PathBuf = match dirs_next::home_dir() {
        Some(h) => h,
        None => return ApiResponse::<String>::err("Cannot find home directory").into_response(),
    };

    let config_dir = home.join(".osmozzz");
    if let Err(e) = std::fs::create_dir_all(&config_dir) {
        return ApiResponse::<String>::err(format!("Cannot create config dir: {}", e)).into_response();
    }

    let content = format!(
        "username = \"{}\"\napp_password = \"{}\"\n",
        body.username.replace('"', "\\\""),
        body.app_password.replace('"', "\\\"")
    );

    let path = config_dir.join("gmail.toml");
    if let Err(e) = std::fs::write(&path, content) {
        return ApiResponse::<String>::err(format!("Cannot write gmail.toml: {}", e)).into_response();
    }

    ApiResponse::ok("Gmail configuré avec succès".to_string()).into_response()
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

// ─── Helpers ─────────────────────────────────────────────────────────────────

fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    let mut end = max;
    while !s.is_char_boundary(end) { end -= 1; }
    format!("{}...", &s[..end])
}
