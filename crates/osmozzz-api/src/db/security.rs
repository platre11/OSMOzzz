use std::collections::HashMap;
use serde::{Deserialize, Serialize};
use dirs_next::home_dir;

/// Rule applied to a column before sending data to Claude
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ColumnRule {
    Free,      // real value sent as-is
    Tokenize,  // replaced with a stable token (tok_xxx_yyy)
    Block,     // column removed entirely from result
}

impl Default for ColumnRule {
    fn default() -> Self { ColumnRule::Free }
}

/// Config for one table: column_name → rule
pub type TableConfig = HashMap<String, ColumnRule>;

/// Config saved per project
#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct ProjectSecurityConfig {
    #[serde(default)]
    pub supabase: HashMap<String, TableConfig>,
    #[serde(default)]
    pub column_order: HashMap<String, Vec<String>>,
}

/// Full config — stores one config per project_id
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct DbSecurityConfig {
    #[serde(default)]
    pub active_project_id: Option<String>,
    /// Current project's config (backward compat — mirrors projects[active_project_id])
    #[serde(default)]
    pub supabase: HashMap<String, TableConfig>,
    #[serde(default)]
    pub column_order: HashMap<String, Vec<String>>,
    /// Per-project storage: project_id → config
    #[serde(default)]
    pub projects: HashMap<String, ProjectSecurityConfig>,
}

impl DbSecurityConfig {
    /// Load from ~/.osmozzz/db_security.toml — returns empty config if file absent
    pub fn load() -> Self {
        let path = match home_dir() {
            Some(h) => h.join(".osmozzz/db_security.toml"),
            None => return Self::default(),
        };
        let content = match std::fs::read_to_string(&path) {
            Ok(c) => c,
            Err(_) => return Self::default(),
        };
        toml::from_str(&content).unwrap_or_default()
    }

    /// Save to ~/.osmozzz/db_security.toml
    pub fn save(&self) -> anyhow::Result<()> {
        let path = home_dir()
            .ok_or_else(|| anyhow::anyhow!("home dir not found"))?
            .join(".osmozzz/db_security.toml");
        let content = toml::to_string_pretty(self)?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    /// Get the rule for a specific column in a table (default: Free)
    pub fn rule(&self, connector: &str, table: &str, column: &str) -> &ColumnRule {
        match connector {
            "supabase" => {
                // First try per-project config
                if let Some(proj_id) = &self.active_project_id {
                    if let Some(proj) = self.projects.get(proj_id) {
                        if let Some(rule) = proj.supabase.get(table).and_then(|t| t.get(column)) {
                            return rule;
                        }
                    }
                }
                // Fallback to flat supabase (backward compat with old TOML format)
                self.supabase
                    .get(table)
                    .and_then(|t| t.get(column))
                    .unwrap_or(&ColumnRule::Free)
            }
            _ => &ColumnRule::Free,
        }
    }
}
