use anyhow::{Context, Result};
use ed25519_dalek::{SigningKey, VerifyingKey};
use rand::rngs::OsRng;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Identité cryptographique d'un daemon OSMOzzz.
/// Persistée dans ~/.osmozzz/identity.toml — créée une seule fois.
#[derive(Clone)]
pub struct PeerIdentity {
    pub id: String,           // hex de la clé publique (32 bytes = 64 chars)
    pub signing_key: SigningKey,
}

#[derive(Serialize, Deserialize)]
struct IdentityFile {
    peer_id: String,
    private_key_hex: String,
    display_name: String,
}

impl PeerIdentity {
    /// Charge l'identité depuis le disque, ou en crée une nouvelle.
    pub fn load_or_create(display_name: &str) -> Result<Self> {
        let path = identity_path()?;

        if path.exists() {
            let content = std::fs::read_to_string(&path)
                .context("Lecture identity.toml")?;
            let file: IdentityFile = toml::from_str(&content)
                .context("Parse identity.toml")?;

            let key_bytes = hex::decode(&file.private_key_hex)
                .context("Décodage clé privée")?;
            let key_arr: [u8; 32] = key_bytes.try_into()
                .map_err(|_| anyhow::anyhow!("Longueur clé invalide"))?;
            let signing_key = SigningKey::from_bytes(&key_arr);

            return Ok(Self {
                id: file.peer_id,
                signing_key,
            });
        }

        // Génère une nouvelle identité
        let signing_key = SigningKey::generate(&mut OsRng);
        let verifying_key: VerifyingKey = signing_key.verifying_key();
        let peer_id = hex::encode(verifying_key.as_bytes());

        let file = IdentityFile {
            peer_id: peer_id.clone(),
            private_key_hex: hex::encode(signing_key.as_bytes()),
            display_name: display_name.to_string(),
        };

        let dir = path.parent().unwrap();
        std::fs::create_dir_all(dir)?;
        std::fs::write(&path, toml::to_string(&file)?)?;

        Ok(Self { id: peer_id, signing_key })
    }

    pub fn public_key_bytes(&self) -> [u8; 32] {
        *self.signing_key.verifying_key().as_bytes()
    }
}

fn identity_path() -> Result<PathBuf> {
    dirs_next::home_dir()
        .map(|h| h.join(".osmozzz/identity.toml"))
        .context("Home introuvable")
}
