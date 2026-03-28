use super::config::PrivacyConfig;
use super::rules::{FilterRule, ApiKeysRule, CreditCardRule, EmailRule, IbanRule, KnownSecretsRule, PhoneRule};

pub struct PrivacyFilter {
    rules: Vec<Box<dyn FilterRule>>,
}

impl PrivacyFilter {
    /// Construit le filtre à partir de la config.
    /// KnownSecretsRule et ApiKeysRule sont TOUJOURS actifs — non négociable.
    pub fn from_config(config: &PrivacyConfig) -> Self {
        let mut rules: Vec<Box<dyn FilterRule>> = vec![
            Box::new(KnownSecretsRule::load()), // valeurs exactes des .toml — couvre tous les connecteurs automatiquement
            Box::new(ApiKeysRule::new()),        // patterns génériques (sk-, hf_, JWT) présents dans des docs indexés
        ];
        if config.credit_card { rules.push(Box::new(CreditCardRule::new())); }
        if config.iban        { rules.push(Box::new(IbanRule::new())); }
        if config.email       { rules.push(Box::new(EmailRule::new())); }
        if config.phone       { rules.push(Box::new(PhoneRule::new())); }
        Self { rules }
    }

    /// Applique toutes les règles actives dans l'ordre.
    pub fn apply(&self, text: &str) -> String {
        self.rules.iter().fold(text.to_string(), |acc, rule| rule.apply(&acc))
    }
}
