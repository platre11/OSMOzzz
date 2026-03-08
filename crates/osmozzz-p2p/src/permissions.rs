use serde::{Deserialize, Serialize};

/// Sources de données que l'on accepte de partager avec un peer.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum SharedSource {
    Chrome,
    Safari,
    Email,
    IMessage,
    Notes,
    Calendar,
    Terminal,
    File,
    Notion,
    Github,
    Linear,
    Jira,
    Slack,
    Trello,
    Todoist,
    Gitlab,
    Airtable,
    Obsidian,
}

impl SharedSource {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Chrome   => "chrome",
            Self::Safari   => "safari",
            Self::Email    => "email",
            Self::IMessage => "imessage",
            Self::Notes    => "notes",
            Self::Calendar => "calendar",
            Self::Terminal => "terminal",
            Self::File     => "file",
            Self::Notion   => "notion",
            Self::Github   => "github",
            Self::Linear   => "linear",
            Self::Jira     => "jira",
            Self::Slack    => "slack",
            Self::Trello   => "trello",
            Self::Todoist  => "todoist",
            Self::Gitlab   => "gitlab",
            Self::Airtable => "airtable",
            Self::Obsidian => "obsidian",
        }
    }

    pub fn all() -> Vec<Self> {
        vec![
            Self::File, Self::Notion, Self::Github, Self::Linear,
            Self::Jira, Self::Slack, Self::Trello, Self::Todoist,
            Self::Gitlab, Self::Airtable, Self::Obsidian,
            // Sources sensibles désactivées par défaut
            // Self::Email, Self::IMessage, Self::Terminal,
            // Self::Chrome, Self::Safari, Self::Notes, Self::Calendar,
        ]
    }
}

/// Permissions accordées à UN peer spécifique.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PeerPermissions {
    /// Sources que ce peer peut interroger sur notre machine
    pub allowed_sources: Vec<SharedSource>,
    /// Limite de résultats par requête (protection contre l'abus)
    pub max_results_per_query: usize,
}

impl Default for PeerPermissions {
    fn default() -> Self {
        Self {
            allowed_sources: SharedSource::all(),
            max_results_per_query: 10,
        }
    }
}

impl PeerPermissions {
    pub fn allows(&self, source: &str) -> bool {
        self.allowed_sources.iter().any(|s| s.as_str() == source)
    }

    pub fn allowed_source_names(&self) -> Vec<String> {
        self.allowed_sources.iter().map(|s| s.as_str().to_string()).collect()
    }
}
