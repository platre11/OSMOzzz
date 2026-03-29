use anyhow::Result;
use rusqlite::{Connection, params};
use dirs_next::home_dir;
use rand::Rng;

/// Local SQLite vault: maps stable tokens to real values
/// Token format: tok_{type_prefix}_{8 random alphanumeric chars}
/// e.g. tok_em_3KmN9xDj for an email, tok_nm_7Qp2vLwR for a name
pub struct TokenVault {
    conn: Connection,
}

impl TokenVault {
    pub fn open() -> Result<Self> {
        let path = home_dir()
            .ok_or_else(|| anyhow::anyhow!("home dir not found"))?
            .join(".osmozzz/token_vault.db");

        let conn = Connection::open(&path)?;
        conn.execute_batch("
            CREATE TABLE IF NOT EXISTS tokens (
                token      TEXT PRIMARY KEY,
                real_value TEXT NOT NULL,
                col_type   TEXT NOT NULL,
                created_at INTEGER NOT NULL DEFAULT (strftime('%s','now'))
            );
            CREATE INDEX IF NOT EXISTS idx_real_value ON tokens(real_value, col_type);
        ")?;
        Ok(Self { conn })
    }

    /// Get or create a stable token for a given real value + column type
    pub fn get_or_create(&self, real_value: &str, col_type: &str) -> Result<String> {
        // Check if already exists
        let existing: Option<String> = self.conn.query_row(
            "SELECT token FROM tokens WHERE real_value = ?1 AND col_type = ?2",
            params![real_value, col_type],
            |row| row.get(0),
        ).ok();

        if let Some(token) = existing {
            return Ok(token);
        }

        // Generate new token
        let token = Self::generate_token(col_type);
        self.conn.execute(
            "INSERT INTO tokens (token, real_value, col_type) VALUES (?1, ?2, ?3)",
            params![token, real_value, col_type],
        )?;
        Ok(token)
    }

    /// Resolve a token back to its real value (for action executor)
    pub fn resolve(&self, token: &str) -> Option<String> {
        self.conn.query_row(
            "SELECT real_value FROM tokens WHERE token = ?1",
            params![token],
            |row| row.get(0),
        ).ok()
    }

    fn generate_token(col_type: &str) -> String {
        let prefix = match col_type {
            "email"    => "em",
            "name"     => "nm",
            "phone"    => "ph",
            "id"       => "id",
            "address"  => "ad",
            _          => "tk",
        };
        let suffix: String = rand::thread_rng()
            .sample_iter(&rand::distributions::Alphanumeric)
            .take(8)
            .map(char::from)
            .collect();
        format!("tok_{}_{}", prefix, suffix)
    }
}
