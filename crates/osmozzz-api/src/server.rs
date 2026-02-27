use std::sync::Arc;
use anyhow::Result;
use axum::{
    Router,
    routing::{get, post},
    // post already imported
    http::{HeaderValue, Method},
};
use include_dir::{include_dir, Dir};
use tower_http::cors::{Any, CorsLayer};

use osmozzz_embedder::Vault;
use crate::routes;
use crate::state::AppState;

// En mode release, le build React est embarqué dans le binaire
static DASHBOARD_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/../../dashboard/dist");

pub async fn start_server(vault: Arc<Vault>, port: u16) -> Result<()> {
    let state = AppState { vault };

    let cors = CorsLayer::new()
        .allow_origin("http://localhost:5173".parse::<HeaderValue>()?)
        .allow_methods([Method::GET, Method::POST])
        .allow_headers(Any);

    let api_router = Router::new()
        .route("/status", get(routes::get_status))
        .route("/search", get(routes::get_search))
        .route("/recent", get(routes::get_recent))
        .route("/config", get(routes::get_config))
        .route("/config/gmail", post(routes::post_config_gmail))
        .route("/open", get(routes::get_open))
        .route("/messages/contacts", get(routes::get_imessage_contacts))
        .route("/messages/conversation", get(routes::get_imessage_conversation))
        .route("/ban", post(routes::post_ban))
        .route("/unban", post(routes::post_unban))
        .route("/blacklist", get(routes::get_blacklist))
        .route("/compact", post(routes::post_compact))
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
