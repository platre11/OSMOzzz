use regex::Regex;
use super::FilterRule;

pub struct ApiKeysRule {
    re: Regex,
}

impl ApiKeysRule {
    pub fn new() -> Self {
        Self {
            re: Regex::new(concat!(
                r"(?x)",
                // OpenAI / Stripe
                r"sk-[a-zA-Z0-9]{20,}",
                r"|",
                // GitHub tokens
                r"gh[psobur]_[a-zA-Z0-9]{36}",
                r"|",
                // Linear
                r"lin_api_[a-zA-Z0-9]{40}",
                r"|",
                // Slack
                r"xox[pboa]-[a-zA-Z0-9\-]{10,}",
                r"|",
                // Google API
                r"AIza[a-zA-Z0-9\-_]{35}",
                r"|",
                // GitLab
                r"glpat-[a-zA-Z0-9\-_]{20}",
                r"|",
                // AWS Access Key
                r"AKIA[A-Z0-9]{16}",
                r"|",
                // Airtable Personal Access Token
                r"pat[a-zA-Z0-9]{14}\.[a-zA-Z0-9]{64}",
            )).unwrap(),
        }
    }
}

impl FilterRule for ApiKeysRule {
    fn apply(&self, text: &str) -> String {
        self.re.replace_all(text, "[CLÉ API masquée]").to_string()
    }
}
