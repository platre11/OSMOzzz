pub mod api_keys;
pub mod connector_tokens;
pub mod credit_card;
pub mod email;
pub mod iban;
pub mod known_secrets;
pub mod phone;

pub use api_keys::ApiKeysRule;
pub use connector_tokens::ConnectorTokensRule;
pub use credit_card::CreditCardRule;
pub use email::EmailRule;
pub use iban::IbanRule;
pub use known_secrets::KnownSecretsRule;
pub use phone::PhoneRule;

/// Trait que chaque règle de filtrage doit implémenter.
/// Pour ajouter une nouvelle règle : créer un fichier dans rules/, implémenter ce trait,
/// exporter depuis mod.rs, et l'enregistrer dans engine.rs.
pub trait FilterRule: Send + Sync {
    fn apply(&self, text: &str) -> String;
}
