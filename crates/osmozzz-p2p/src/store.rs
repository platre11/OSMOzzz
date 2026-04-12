use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::PathBuf;

use crate::permissions::{PeerPermissions, ToolAccessMode};

/// Un peer connu — persisté dans ~/.osmozzz/peers.toml
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct KnownPeer {
    pub peer_id: String,
    pub display_name: String,
    pub addresses: Vec<String>,   // tickets iroh base64 (incluent relay + adresses directes)
    pub public_key_hex: String,
    /// Ce que MOI j'autorise ce peer à faire sur MA machine
    pub permissions: PeerPermissions,
    /// Ce que CE PEER m'autorise à faire sur SA machine (reçu via PermissionsSync)
    #[serde(default)]
    pub peer_granted_to_me: Option<PeerPermissions>,
    pub connected: bool,
    pub last_seen: Option<i64>,   // timestamp unix
}

#[derive(Debug, Serialize, Deserialize, Default)]
struct PeersFile {
    peers: HashMap<String, KnownPeer>,
}

#[derive(Debug, Clone)]
pub struct PeerStore {
    path: PathBuf,
}

impl PeerStore {
    pub fn new() -> Result<Self> {
        let path = dirs_next::home_dir()
            .context("Home introuvable")?
            .join(".osmozzz/peers.toml");
        Ok(Self { path })
    }

    fn load_file(&self) -> PeersFile {
        std::fs::read_to_string(&self.path)
            .ok()
            .and_then(|s| toml::from_str(&s).ok())
            .unwrap_or_default()
    }

    fn save_file(&self, file: &PeersFile) -> Result<()> {
        if let Some(dir) = self.path.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(&self.path, toml::to_string(file)?)?;
        Ok(())
    }

    pub fn all(&self) -> Vec<KnownPeer> {
        self.load_file().peers.into_values().collect()
    }

    pub fn get(&self, peer_id: &str) -> Option<KnownPeer> {
        self.load_file().peers.remove(peer_id)
    }

    pub fn upsert(&self, peer: KnownPeer) -> Result<()> {
        let mut file = self.load_file();
        file.peers.insert(peer.peer_id.clone(), peer);
        self.save_file(&file)
    }

    pub fn remove(&self, peer_id: &str) -> Result<()> {
        let mut file = self.load_file();
        file.peers.remove(peer_id);
        self.save_file(&file)
    }

    pub fn set_connected(&self, peer_id: &str, connected: bool) -> Result<()> {
        let mut file = self.load_file();
        if let Some(peer) = file.peers.get_mut(peer_id) {
            peer.connected = connected;
            if connected {
                peer.last_seen = Some(chrono::Utc::now().timestamp());
            }
        }
        self.save_file(&file)
    }

    pub fn update_permissions(&self, peer_id: &str, perms: PeerPermissions) -> Result<()> {
        let mut file = self.load_file();
        if let Some(peer) = file.peers.get_mut(peer_id) {
            peer.permissions = perms;
        }
        self.save_file(&file)
    }

    /// Stocke les permissions que le peer nous a accordées (reçu via PermissionsSync).
    pub fn update_peer_granted(&self, peer_id: &str, granted: PeerPermissions) -> Result<()> {
        let mut file = self.load_file();
        if let Some(peer) = file.peers.get_mut(peer_id) {
            peer.peer_granted_to_me = Some(granted);
        }
        self.save_file(&file)
    }

    /// Met à jour uniquement les permissions de tools (connecteurs) d'un peer.
    pub fn update_tool_permissions(
        &self,
        peer_id: &str,
        tool_perms: HashMap<String, ToolAccessMode>,
    ) -> Result<()> {
        let mut file = self.load_file();
        if let Some(peer) = file.peers.get_mut(peer_id) {
            peer.permissions.tool_permissions = tool_perms;
        }
        self.save_file(&file)
    }
}
