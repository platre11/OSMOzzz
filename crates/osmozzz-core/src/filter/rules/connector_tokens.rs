use regex::Regex;
use super::FilterRule;

/// Masque TOUJOURS les tokens des connecteurs OSMOzzz, peu importe la config utilisateur.
/// Claude ne doit jamais voir ces tokens — sinon il pourrait accéder aux services externes
/// sans passer par OSMOzzz.
pub struct ConnectorTokensRule {
    re: Regex,
}

impl ConnectorTokensRule {
    pub fn new() -> Self {
        Self {
            re: Regex::new(concat!(
                r"(?x)",
                // GitHub
                r"gh[psobur]_[a-zA-Z0-9]{36}",
                r"|",
                // Linear
                r"lin_api_[a-zA-Z0-9]{40}",
                r"|",
                // Slack
                r"xox[pboa]-[a-zA-Z0-9\-]{10,}",
                r"|",
                // GitLab
                r"glpat-[a-zA-Z0-9\-_]{20}",
                r"|",
                // Airtable
                r"pat[a-zA-Z0-9]{14}\.[a-zA-Z0-9]{64}",
                r"|",
                // Notion
                r"secret_[a-zA-Z0-9]{43}",
                r"|",
                // Supabase
                r"sbp_[a-zA-Z0-9]{40}",
                r"|",
                // Google API
                r"AIza[a-zA-Z0-9\-_]{35}",
                r"|",
                // AWS
                r"AKIA[A-Z0-9]{16}",
            )).unwrap(),
        }
    }
}

impl FilterRule for ConnectorTokensRule {
    fn apply(&self, text: &str) -> String {
        self.re.replace_all(text, "[TOKEN masqué]").to_string()
    }
}
