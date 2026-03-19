use std::sync::Arc;
use anyhow::Result;
use axum::{
    Router,
    routing::{delete, get, post},
    http::{HeaderValue, Method},
};
use include_dir::{include_dir, Dir};
use tower_http::cors::{Any, CorsLayer};

use osmozzz_embedder::Vault;
use osmozzz_p2p::P2pNode;
use crate::action_queue::ActionQueue;
use crate::routes;
use crate::state::AppState;

// En mode release, le build React est embarqué dans le binaire
static DASHBOARD_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/../../dashboard/dist");

pub async fn start_server(
    vault: Arc<Vault>,
    p2p: Option<Arc<P2pNode>>,
    action_queue: Arc<ActionQueue>,
    port: u16,
) -> Result<()> {
    let state = AppState {
        vault,
        p2p,
        index_progress: Arc::new(std::sync::Mutex::new(Default::default())),
        action_queue,
    };

    let cors = CorsLayer::new()
        .allow_origin("http://localhost:5173".parse::<HeaderValue>()?)
        .allow_methods([Method::GET, Method::POST])
        .allow_headers(Any);

    let api_router = Router::new()
        .route("/status", get(routes::get_status))
        .route("/search", get(routes::get_search))
        .route("/recent", get(routes::get_recent))
        .route("/config", get(routes::get_config))
        .route("/config/gmail",     post(routes::post_config_gmail))
        .route("/config/notion",    post(routes::post_config_notion))
        .route("/config/github",    post(routes::post_config_github))
        .route("/config/linear",    post(routes::post_config_linear))
        .route("/config/jira",      post(routes::post_config_jira))
        .route("/config/slack",     post(routes::post_config_slack))
        .route("/config/trello",    post(routes::post_config_trello))
        .route("/config/todoist",   post(routes::post_config_todoist))
        .route("/config/gitlab",    post(routes::post_config_gitlab))
        .route("/config/airtable",  post(routes::post_config_airtable))
        .route("/config/obsidian",  post(routes::post_config_obsidian))
        .route("/open", get(routes::get_open))
        .route("/messages/contacts", get(routes::get_imessage_contacts))
        .route("/messages/conversation", get(routes::get_imessage_conversation))
        .route("/ban", post(routes::post_ban))
        .route("/unban", post(routes::post_unban))
        .route("/blacklist", get(routes::get_blacklist))
        .route("/compact", post(routes::post_compact))
        .route("/reindex/imessage", post(routes::post_reindex_imessage))
        .route("/files/search", get(routes::get_files_search))
        .route("/index/preview", get(routes::get_index_preview))
        .route("/index/progress", get(routes::get_index_progress))
        .route("/index", post(routes::post_index))
        .route("/privacy", get(routes::get_privacy).post(routes::post_privacy))
        .route("/network/peers", get(routes::get_network_peers))
        .route("/network/invite", post(routes::post_network_invite))
        .route("/network/connect", post(routes::post_network_connect))
        .route("/network/peers/:peer_id", delete(routes::delete_network_peer))
        .route("/network/permissions/:peer_id", get(routes::get_network_permissions).post(routes::post_network_permissions))
        .route("/network/history", get(routes::get_network_history))
        // ── Actions orchestrateur ──────────────────────────────────────────
        .route("/actions",              get(routes::get_actions_all).post(routes::post_action))
        .route("/actions/pending",      get(routes::get_actions_pending))
        .route("/actions/stream",       get(routes::get_actions_stream))
        .route("/actions/:id",          get(routes::get_action_by_id))
        .route("/actions/:id/approve",  post(routes::post_action_approve))
        .route("/actions/:id/reject",   post(routes::post_action_reject))
        .route("/permissions",          get(routes::get_permissions).post(routes::post_permissions))
        .route("/source-access",        get(routes::get_source_access).post(routes::post_source_access))
        .with_state(state);

    let app = Router::new()
        .nest("/api", api_router)
        .fallback(serve_static)
        .layer(cors);

    let addr = format!("127.0.0.1:{}", port);
    eprintln!("[OSMOzzz Dashboard] http://localhost:{}", port);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn serve_static(uri: axum::http::Uri) -> impl axum::response::IntoResponse {
    let path = uri.path().trim_start_matches('/');
    let path = if path.is_empty() { "index.html" } else { path };

    if let Some(file) = DASHBOARD_DIR.get_file(path) {
        let mime = mime_for(path);
        // Les assets JS/CSS ont un hash dans leur nom → cache longue durée
        // index.html ne doit jamais être mis en cache
        let cache = if path.ends_with(".html") {
            "no-cache, no-store, must-revalidate"
        } else {
            "public, max-age=31536000, immutable"
        };
        axum::response::Response::builder()
            .header("Content-Type", mime)
            .header("Cache-Control", cache)
            .body(axum::body::Body::from(file.contents().to_vec()))
            .unwrap()
    } else {
        // SPA fallback → index.html
        if let Some(index) = DASHBOARD_DIR.get_file("index.html") {
            axum::response::Response::builder()
                .header("Content-Type", "text/html")
                .header("Cache-Control", "no-cache, no-store, must-revalidate")
                .body(axum::body::Body::from(index.contents().to_vec()))
                .unwrap()
        } else {
            axum::response::Response::builder()
                .status(404)
                .body(axum::body::Body::from("Not found"))
                .unwrap()
        }
    }
}

fn mime_for(path: &str) -> &'static str {
    if path.ends_with(".html") { "text/html; charset=utf-8" }
    else if path.ends_with(".js") || path.ends_with(".mjs") { "application/javascript" }
    else if path.ends_with(".css") { "text/css" }
    else if path.ends_with(".svg") { "image/svg+xml" }
    else if path.ends_with(".png") { "image/png" }
    else if path.ends_with(".ico") { "image/x-icon" }
    else { "application/octet-stream" }
}
