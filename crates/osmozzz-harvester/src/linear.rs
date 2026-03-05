/// Linear Harvester — indexe les issues Linear via l'API GraphQL.
///
/// Config : ~/.osmozzz/linear.toml
/// ```toml
/// api_key = "lin_api_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"
/// ```
///
/// Obtenir la clé : linear.app/settings/api → Personal API keys → Create key
use chrono::DateTime;
use osmozzz_core::{Document, Result, SourceType};
use serde::Deserialize;
use tracing::{info, warn};

use crate::checksum;

const LINEAR_API: &str = "https://api.linear.app/graphql";
const MAX_ISSUES: usize = 1000;

#[derive(Debug, Deserialize)]
struct LinearConfig {
    api_key: String,
}

impl LinearConfig {
    fn load() -> Option<Self> {
        let path = dirs_next::home_dir()?.join(".osmozzz/linear.toml");
        let content = std::fs::read_to_string(path).ok()?;
        toml::from_str(&content).ok()
    }
}

pub struct LinearHarvester;

impl LinearHarvester {
    pub fn new() -> Self {
        Self
    }
}

impl Default for LinearHarvester {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Réponse API Linear (GraphQL) ────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct GraphQLResponse {
    data: Option<LinearData>,
}

#[derive(Debug, Deserialize)]
struct LinearData {
    issues: IssueConnection,
}

#[derive(Debug, Deserialize)]
struct IssueConnection {
    nodes: Vec<LinearIssue>,
    #[serde(rename = "pageInfo")]
    page_info: PageInfo,
}

#[derive(Debug, Deserialize)]
struct PageInfo {
    #[serde(rename = "hasNextPage")]
    has_next_page: bool,
    #[serde(rename = "endCursor")]
    end_cursor: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LinearIssue {
    id: String,
    title: String,
    #[serde(default)]
    description: Option<String>,
    #[serde(rename = "state")]
    state: Option<LinearState>,
    #[serde(default)]
    team: Option<LinearTeam>,
    #[serde(default)]
    assignee: Option<LinearUser>,
    #[serde(default)]
    priority: Option<f64>,
    url: String,
    #[serde(rename = "createdAt")]
    created_at: String,
}

#[derive(Debug, Deserialize)]
struct LinearState {
    name: String,
}

#[derive(Debug, Deserialize)]
struct LinearTeam {
    name: String,
}

#[derive(Debug, Deserialize)]
struct LinearUser {
    name: String,
}

const ISSUES_QUERY: &str = r#"
query($cursor: String) {
  issues(first: 100, after: $cursor, orderBy: updatedAt) {
    nodes {
      id
      title
      description
      state { name }
      team { name }
      assignee { name }
      priority
      url
      createdAt
    }
    pageInfo {
      hasNextPage
      endCursor
    }
  }
}
"#;

impl osmozzz_core::Harvester for LinearHarvester {
    async fn harvest(&self) -> Result<Vec<Document>> {
        let cfg = match LinearConfig::load() {
            Some(c) => c,
            None => {
                warn!("Linear: ~/.osmozzz/linear.toml non trouvé — source ignorée");
                return Ok(vec![]);
            }
        };

        let client = reqwest::Client::new();
        let mut documents = Vec::new();
        let mut cursor: Option<String> = None;

        loop {
            let variables = if let Some(ref c) = cursor {
                serde_json::json!({ "cursor": c })
            } else {
                serde_json::json!({})
            };

            let body = serde_json::json!({
                "query": ISSUES_QUERY,
                "variables": variables,
            });

            let resp = match client
                .post(LINEAR_API)
                .header("Authorization", &cfg.api_key)
                .header("Content-Type", "application/json")
                .json(&body)
                .send()
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    warn!("Linear API error: {}", e);
                    break;
                }
            };

            if resp.status() == 401 {
                warn!("Linear: api_key invalide (401) — vérifiez ~/.osmozzz/linear.toml");
                break;
            }

            if !resp.status().is_success() {
                warn!("Linear API status: {}", resp.status());
                break;
            }

            let gql_resp: GraphQLResponse = match resp.json().await {
                Ok(r) => r,
                Err(e) => {
                    warn!("Linear JSON parse error: {}", e);
                    break;
                }
            };

            let data = match gql_resp.data {
                Some(d) => d,
                None => break,
            };

            for issue in &data.issues.nodes {
                let state = issue
                    .state
                    .as_ref()
                    .map(|s| s.name.as_str())
                    .unwrap_or("Unknown");
                let team = issue
                    .team
                    .as_ref()
                    .map(|t| t.name.as_str())
                    .unwrap_or("Unknown");
                let assignee = issue
                    .assignee
                    .as_ref()
                    .map(|a| a.name.as_str())
                    .unwrap_or("Non assigné");
                let priority = issue.priority.unwrap_or(0.0);
                let desc = issue.description.as_deref().unwrap_or("").trim();

                let priority_label = match priority as u32 {
                    0 => "Aucune",
                    1 => "Urgente",
                    2 => "Haute",
                    3 => "Moyenne",
                    4 => "Basse",
                    _ => "Inconnue",
                };

                let content = format!(
                    "{}\nÉquipe: {} | Statut: {} | Priorité: {} | Assigné: {}\n\n{}",
                    issue.title, team, state, priority_label, assignee, desc
                );

                let checksum = checksum::compute(&content);
                let mut doc =
                    Document::new(SourceType::Linear, &issue.url, &content, &checksum)
                        .with_title(&issue.title);

                if let Ok(ts) = DateTime::parse_from_rfc3339(&issue.created_at) {
                    doc = doc.with_source_ts(ts.with_timezone(&chrono::Utc));
                }

                documents.push(doc);
            }

            if !data.issues.page_info.has_next_page || documents.len() >= MAX_ISSUES {
                break;
            }
            cursor = data.issues.page_info.end_cursor;
        }

        info!("Linear harvester found {} issues", documents.len());
        Ok(documents)
    }
}
