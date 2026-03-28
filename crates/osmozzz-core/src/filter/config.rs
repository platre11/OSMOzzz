use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct PrivacyConfig {
    #[serde(default = "default_true")]
    pub credit_card: bool,
    #[serde(default = "default_true")]
    pub iban: bool,
    #[serde(default)]
    pub email: bool,
    #[serde(default)]
    pub phone: bool,
}

fn default_true() -> bool { true }

impl Default for PrivacyConfig {
    fn default() -> Self {
        Self {
            credit_card: true,
            iban: true,
            email: false,
            phone: false,
        }
    }
}

impl PrivacyConfig {
    pub fn load() -> Self {
        let path = match dirs_next::home_dir() {
            Some(h) => h.join(".osmozzz/privacy.toml"),
            None => return Self::default(),
        };
        match std::fs::read_to_string(&path) {
            Ok(content) => toml::from_str(&content).unwrap_or_default(),
            Err(_) => Self::default(),
        }
    }

    pub fn save(&self) -> Result<(), String> {
        let dir = dirs_next::home_dir()
            .map(|h| h.join(".osmozzz"))
            .ok_or("Cannot find home directory")?;
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let content = toml::to_string(self).map_err(|e| e.to_string())?;
        std::fs::write(dir.join("privacy.toml"), content).map_err(|e| e.to_string())
    }

    pub fn is_any_active(&self) -> bool {
        self.credit_card || self.iban || self.email || self.phone
    }
}
