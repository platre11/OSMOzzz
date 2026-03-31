pub mod api_keys;
pub mod connector_tokens;
pub mod email;
pub mod known_secrets;
pub mod phone;

pub use api_keys::ApiKeysRule;
pub use connector_tokens::ConnectorTokensRule;
pub use email::EmailRule;
pub use known_secrets::KnownSecretsRule;
pub use phone::PhoneRule;

/// Trait que chaque règle de filtrage doit implémenter.
pub trait FilterRule: Send + Sync {
    fn apply(&self, text: &str) -> String;
}
