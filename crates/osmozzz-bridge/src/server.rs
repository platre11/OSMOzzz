use std::path::PathBuf;
use std::sync::Arc;

use osmozzz_core::{Embedder, OsmozzError, Result};
use osmozzz_embedder::Vault;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{UnixListener, UnixStream};
use tracing::{error, info, warn};

use crate::protocol::{Request, Response};

/// Unix Domain Socket server that exposes the Vault to local clients.
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

    /// Start listening on the UDS socket.
    pub async fn serve(&self) -> Result<()> {
        let path = &self.socket_path;

        // Create parent directory if needed
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| {
                OsmozzError::Bridge(format!("Cannot create socket dir: {}", e))
            })?;
        }

        // Remove stale socket file
        if path.exists() {
            std::fs::remove_file(path).map_err(|e| {
                OsmozzError::Bridge(format!("Cannot remove stale socket: {}", e))
            })?;
        }

        let listener = UnixListener::bind(path)
            .map_err(|e| OsmozzError::Bridge(format!("Bind UDS: {}", e)))?;

        // Set socket permissions to 600 (owner read/write only)
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
                .map_err(|e| OsmozzError::Bridge(format!("Set socket perms: {}", e)))?;
        }

        info!("OSMOzzz bridge listening on: {}", path.display());

        loop {
            match listener.accept().await {
                Ok((stream, _addr)) => {
                    let vault = Arc::clone(&self.vault);
                    tokio::spawn(async move {
                        if let Err(e) = handle_connection(stream, vault).await {
                            error!("Connection error: {}", e);
                        }
                    });
                }
                Err(e) => {
                    warn!("Accept error: {}", e);
                }
            }
        }
    }
}

async fn handle_connection(stream: UnixStream, vault: Arc<Vault>) -> Result<()> {
    let (reader, mut writer) = stream.into_split();
    let mut lines = BufReader::new(reader).lines();

    while let Ok(Some(line)) = lines.next_line().await {
        let line = line.trim().to_string();
        if line.is_empty() {
            continue;
        }

        let response = match serde_json::from_str::<Request>(&line) {
            Ok(Request::Search { query, limit }) => {
                match vault.search(&query, limit).await {
                    Ok(results) => Response::Search { results },
                    Err(e) => Response::Error {
                        error: e.to_string(),
                    },
                }
            }
            Ok(Request::Status) => match vault.count().await {
                Ok(count) => Response::Status {
                    doc_count: count,
                    status: "ok".to_string(),
                },
                Err(e) => Response::Error {
                    error: e.to_string(),
                },
            },
            Ok(Request::Ping) => Response::Pong { pong: true },
            Err(e) => Response::Error {
                error: format!("Invalid request: {}", e),
            },
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
