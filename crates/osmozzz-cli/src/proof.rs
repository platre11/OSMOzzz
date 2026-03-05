/// Proof of Context — signature cryptographique locale.
///
/// Chaque snippet retourné par osmozzz est signé avec une clé HMAC-SHA256
/// stockée uniquement sur ce Mac (~/.osmozzz/proof.key).
/// La signature prouve que le snippet provient de la DB locale et n'a pas été modifié.
///
/// Usage : osmozzz verify --sig <hex> --source <source> --url <url> --content <content> --ts <ts>

use hmac::{Hmac, Mac};
use sha2::Sha256;
use std::io::Read;
use std::path::PathBuf;

type HmacSha256 = Hmac<Sha256>;

fn key_path() -> PathBuf {
    dirs_next::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".osmozzz/proof.key")
}

/// Charge la clé depuis ~/.osmozzz/proof.key, ou la génère si elle n'existe pas.
pub fn load_or_create_key() -> [u8; 32] {
    let path = key_path();

    if path.exists() {
        if let Ok(bytes) = std::fs::read(&path) {
            if bytes.len() == 32 {
                let mut key = [0u8; 32];
                key.copy_from_slice(&bytes);
                return key;
            }
        }
    }

    let key = generate_key();

    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(&path, &key);
    eprintln!("[OSMOzzz Proof] Clé générée : {}", path.display());

    key
}

fn generate_key() -> [u8; 32] {
    let mut key = [0u8; 32];
    if let Ok(mut f) = std::fs::File::open("/dev/urandom") {
        let _ = f.read_exact(&mut key);
    }
    key
}

/// Signe un snippet : HMAC-SHA256(source + url + content_100 + ts, clé_locale).
/// Retourne une string hex de 64 caractères.
pub fn sign(key: &[u8; 32], source: &str, url: &str, content: &str, ts: i64) -> String {
    let snippet = first_chars(content, 100);
    let payload = format!("{}\n{}\n{}\n{}", source, url, snippet, ts);

    let mut mac = HmacSha256::new_from_slice(key).expect("HMAC key size valide");
    mac.update(payload.as_bytes());
    hex::encode(mac.finalize().into_bytes())
}

/// Vérifie une signature (comparaison en temps constant).
pub fn verify_sig(sig_hex: &str, key: &[u8; 32], source: &str, url: &str, content: &str, ts: i64) -> bool {
    let expected = sign(key, source, url, content, ts);
    if expected.len() != sig_hex.len() {
        return false;
    }
    expected
        .bytes()
        .zip(sig_hex.bytes())
        .fold(0u8, |acc, (a, b)| acc | (a ^ b))
        == 0
}

fn first_chars(s: &str, n: usize) -> &str {
    let mut end = n.min(s.len());
    while end > 0 && !s.is_char_boundary(end) {
        end -= 1;
    }
    &s[..end]
}
