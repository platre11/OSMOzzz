use super::config::PrivacyConfig;
use super::rules::{FilterRule, ApiKeysRule, CreditCardRule, EmailRule, IbanRule, PhoneRule};

pub struct PrivacyFilter {
    rules: Vec<Box<dyn FilterRule>>,
}

impl PrivacyFilter {
    /// Construit le filtre à partir de la config — seules les règles activées sont chargées.
    pub fn from_config(config: &PrivacyConfig) -> Self {
        let mut rules: Vec<Box<dyn FilterRule>> = vec![];
        if config.credit_card { rules.push(Box::new(CreditCardRule::new())); }
        if config.iban        { rules.push(Box::new(IbanRule::new())); }
        if config.api_keys    { rules.push(Box::new(ApiKeysRule::new())); }
        if config.email       { rules.push(Box::new(EmailRule::new())); }
        if config.phone       { rules.push(Box::new(PhoneRule::new())); }
        Self { rules }
    }

    /// Applique toutes les règles actives dans l'ordre.
    pub fn apply(&self, text: &str) -> String {
        self.rules.iter().fold(text.to_string(), |acc, rule| rule.apply(&acc))
    }
}
