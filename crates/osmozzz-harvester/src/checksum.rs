use sha2::{Digest, Sha256};

/// Compute SHA-256 checksum of text content.
pub fn compute(content: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    hex::encode(hasher.finalize())
}
