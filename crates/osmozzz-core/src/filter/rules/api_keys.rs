use regex::Regex;
use super::FilterRule;

pub struct ApiKeysRule {
    re: Regex,
}

impl ApiKeysRule {
    pub fn new() -> Self {
        Self {
            // Clés API génériques non liées aux connecteurs OSMOzzz (optionnel utilisateur)
            // Les tokens connecteurs sont dans ConnectorTokensRule (toujours actif)
            re: Regex::new(concat!(
                r"(?x)",
                // OpenAI / Stripe / Anthropic (sk- générique)
                r"sk-[a-zA-Z0-9\-_]{20,}",
                r"|",
                // HuggingFace
                r"hf_[a-zA-Z0-9]{30,}",
                r"|",
                // JWT (Bearer tokens)
                r"eyJ[a-zA-Z0-9\-_]+\.[a-zA-Z0-9\-_]+\.[a-zA-Z0-9\-_]+",
            )).unwrap(),
        }
    }
}

impl FilterRule for ApiKeysRule {
    fn apply(&self, text: &str) -> String {
        self.re.replace_all(text, "[CLÉ API masquée]").to_string()
    }
}
