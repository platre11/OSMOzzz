use std::path::PathBuf;
use std::sync::Arc;

use osmozzz_core::{Embedder, OsmozzError, Result};
use osmozzz_embedder::Vault;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tracing::{error, info, warn};

use crate::protocol::{Request, Response};

/// Serveur de bridge local.
/// - Unix/macOS/Linux : Unix Domain Socket (UDS)
/// - Windows          : TCP loopback sur 127.0.0.1:47473
pub struct BridgeServer {
    socket_path: PathBuf,
    vault: Arc<Vault>,
}

impl BridgeServer {
    pub fn new(socket_path: impl Into<PathBuf>, vault: Arc<Vault>) -> Self {
        Self {
            socket_path: socket_path.into(),
            vault,
        }
    }

    pub async fn serve(&self) -> Result<()> {
        #[cfg(unix)]
        return self.serve_unix().await;

        #[cfg(windows)]
        return self.serve_tcp().await;

        #[cfg(not(any(unix, windows)))]
        return self.serve_tcp().await;
    }

    // ── Unix Domain Socket (macOS / Linux) ────────────────────────────────────

    #[cfg(unix)]
    async fn serve_unix(&self) -> Result<()> {
        use tokio::net::{UnixListener, UnixStream};

        let path = &self.socket_path;

        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                OsmozzError::Bridge(format!("Cannot create socket dir: {}", e))
            })?;
        }
        if path.exists() {
            std::fs::remove_file(path).map_err(|e| {
                OsmozzError::Bridge(format!("Cannot remove stale socket: {}", e))
            })?;
        }

        let listener = UnixListener::bind(path)
            .map_err(|e| OsmozzError::Bridge(format!("Bind UDS: {}", e)))?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
                .map_err(|e| OsmozzError::Bridge(format!("Set socket perms: {}", e)))?;
        }

        info!("OSMOzzz bridge (UDS) listening on: {}", path.display());

        loop {
            match listener.accept().await {
                Ok((stream, _)) => {
                    let vault = Arc::clone(&self.vault);
                    tokio::spawn(async move {
                        if let Err(e) = handle_unix_connection(stream, vault).await {
                            error!("Connection error: {}", e);
                        }
                    });
                }
                Err(e) => warn!("Accept error: {}", e),
            }
        }
    }

    // ── TCP loopback (Windows + fallback) ─────────────────────────────────────

    #[cfg(not(unix))]
    async fn serve_tcp(&self) -> Result<()> {
        use tokio::net::TcpListener;

        let addr = "127.0.0.1:47473";
        let listener = TcpListener::bind(addr)
            .await
            .map_err(|e| OsmozzError::Bridge(format!("Bind TCP bridge: {}", e)))?;

        info!("OSMOzzz bridge (TCP) listening on: {}", addr);

        loop {
            match listener.accept().await {
                Ok((stream, _)) => {
                    let vault = Arc::clone(&self.vault);
                    tokio::spawn(async move {
                        if let Err(e) = handle_tcp_connection(stream, vault).await {
                            error!("Connection error: {}", e);
                        }
                    });
                }
                Err(e) => warn!("Accept error: {}", e),
            }
        }
    }
}

// ── Handlers de connexion ─────────────────────────────────────────────────────

#[cfg(unix)]
async fn handle_unix_connection(
    stream: tokio::net::UnixStream,
    vault: Arc<Vault>,
) -> Result<()> {
    let (reader, writer) = stream.into_split();
    handle_lines(BufReader::new(reader), writer, vault).await
}

#[cfg(not(unix))]
async fn handle_tcp_connection(
    stream: tokio::net::TcpStream,
    vault: Arc<Vault>,
) -> Result<()> {
    let (reader, writer) = stream.into_split();
    handle_lines(BufReader::new(reader), writer, vault).await
}

async fn handle_lines<R, W>(
    mut lines: BufReader<R>,
    mut writer: W,
    vault: Arc<Vault>,
) -> Result<()>
where
    R: tokio::io::AsyncRead + Unpin,
    W: AsyncWriteExt + Unpin,
{
    let mut line = String::new();
    loop {
        line.clear();
        match lines.read_line(&mut line).await {
            Ok(0) => break,
            Ok(_) => {}
            Err(e) => { warn!("Read error: {}", e); break; }
        }

        let trimmed = line.trim();
        if trimmed.is_empty() { continue; }

        let response = match serde_json::from_str::<Request>(trimmed) {
            Ok(Request::Search { query, limit }) => {
                match vault.search(&query, limit).await {
                    Ok(results) => Response::Search { results },
                    Err(e) => Response::Error { error: e.to_string() },
                }
            }
            Ok(Request::Status) => match vault.count().await {
                Ok(count) => Response::Status { doc_count: count, status: "ok".to_string() },
                Err(e) => Response::Error { error: e.to_string() },
            },
            Ok(Request::Ping) => Response::Pong { pong: true },
            Err(e) => Response::Error { error: format!("Invalid request: {}", e) },
        };

        let mut json = serde_json::to_string(&response)
            .unwrap_or_else(|_| r#"{"error":"serialization failed"}"#.to_string());
        json.push('\n');

        if let Err(e) = writer.write_all(json.as_bytes()).await {
            warn!("Write error: {}", e);
            break;
        }
    }
    Ok(())
}
