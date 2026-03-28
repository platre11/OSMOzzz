use super::FilterRule;

/// Masque TOUJOURS les valeurs exactes des tokens/clés stockées dans ~/.osmozzz/*.toml.
///
/// Contrairement à ConnectorTokensRule (regex par format), cette règle charge
/// les valeurs réelles depuis les fichiers de config et les masque à l'identique.
/// Ça garantit que tout nouveau connecteur est automatiquement couvert, quelle
/// que soit la forme de son token — sans avoir à modifier ConnectorTokensRule.
///
/// Clés considérées sensibles (nom de champ dans les .toml) :
///   token, api_key, access_token, app_password, client_secret, private_key, secret
pub struct KnownSecretsRule {
    secrets: Vec<String>,
}

/// Noms de clés TOML dont la valeur est considérée comme un secret.
const SENSITIVE_KEYS: &[&str] = &[
    "token",
    "api_key",
    "access_token",
    "app_password",
    "client_secret",
    "private_key",
    "secret",
];

impl KnownSecretsRule {
    /// Charge tous les fichiers ~/.osmozzz/*.toml et extrait les valeurs des clés sensibles.
    /// Appelé à chaque construction du filtre (au traitement de chaque requête MCP),
    /// ce qui garantit que les tokens révoqués sont immédiatement pris en compte.
    pub fn load() -> Self {
        let mut secrets = Vec::new();
        let dir = match dirs_next::home_dir() {
            Some(h) => h.join(".osmozzz"),
            None => return Self { secrets },
        };

        let entries = match std::fs::read_dir(&dir) {
            Ok(e) => e,
            Err(_) => return Self { secrets },
        };

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("toml") {
                continue;
            }
            let content = match std::fs::read_to_string(&path) {
                Ok(c) => c,
                Err(_) => continue,
            };
            for line in content.lines() {
                let line = line.trim();
                // Ignore les commentaires et les sections [header]
                if line.starts_with('#') || line.starts_with('[') {
                    continue;
                }
                if let Some((key, val)) = line.split_once('=') {
                    let key = key.trim().to_lowercase();
                    if SENSITIVE_KEYS.contains(&key.as_str()) {
                        let val = val.trim().trim_matches('"').trim();
                        // N'ajoute que les valeurs non triviales (longueur min 8)
                        if val.len() >= 8 {
                            secrets.push(val.to_string());
                        }
                    }
                }
            }
        }

        Self { secrets }
    }
}

impl FilterRule for KnownSecretsRule {
    fn apply(&self, text: &str) -> String {
        let mut result = text.to_string();
        for secret in &self.secrets {
            if result.contains(secret.as_str()) {
                result = result.replace(secret.as_str(), "[TOKEN masqué]");
            }
        }
        result
    }
}
