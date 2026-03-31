use super::config::PrivacyConfig;
use super::rules::{FilterRule, ApiKeysRule, EmailRule, KnownSecretsRule, PhoneRule};

pub struct PrivacyFilter {
    rules: Vec<Box<dyn FilterRule>>,
}

impl PrivacyFilter {
    /// Construit le filtre à partir de la config.
    /// KnownSecretsRule et ApiKeysRule sont TOUJOURS actifs — non négociable.
    pub fn from_config(config: &PrivacyConfig) -> Self {
        let mut rules: Vec<Box<dyn FilterRule>> = vec![
            Box::new(KnownSecretsRule::load()),
            Box::new(ApiKeysRule::new()),
        ];
        if config.email { rules.push(Box::new(EmailRule::new())); }
        if config.phone { rules.push(Box::new(PhoneRule::new())); }
        Self { rules }
    }

    /// Applique toutes les règles actives dans l'ordre.
    pub fn apply(&self, text: &str) -> String {
        self.rules.iter().fold(text.to_string(), |acc, rule| rule.apply(&acc))
    }
}
