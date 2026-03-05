/// Jira Harvester — indexe les issues Jira via l'API REST v3.
///
/// Config : ~/.osmozzz/jira.toml
/// ```toml
/// base_url = "https://votre-domaine.atlassian.net"
/// email    = "votre@email.com"
/// token    = "ATATT3xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"
/// ```
///
/// Token : id.atlassian.com/manage-profile/security/api-tokens → Create API token
use chrono::DateTime;
use osmozzz_core::{Document, Result, SourceType};
use serde::Deserialize;
use tracing::{info, warn};

use crate::checksum;

const MAX_ISSUES: usize = 1000;

#[derive(Debug, Deserialize)]
struct JiraConfig {
    base_url: String,
    email: String,
    token: String,
}

impl JiraConfig {
    fn load() -> Option<Self> {
        let path = dirs_next::home_dir()?.join(".osmozzz/jira.toml");
        let content = std::fs::read_to_string(path).ok()?;
        toml::from_str(&content).ok()
    }
}

pub struct JiraHarvester;

impl JiraHarvester {
    pub fn new() -> Self {
        Self
    }
}

impl Default for JiraHarvester {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Réponse API Jira ─────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct JiraSearchResponse {
    issues: Vec<JiraIssue>,
    total: u64,
    #[serde(rename = "startAt")]
    start_at: u64,
    #[serde(rename = "maxResults")]
    max_results: u64,
}

#[derive(Debug, Deserialize)]
struct JiraIssue {
    key: String,
    #[serde(rename = "self")]
    self_url: String,
    fields: JiraFields,
}

#[derive(Debug, Deserialize)]
struct JiraFields {
    summary: String,
    #[serde(default)]
    description: Option<serde_json::Value>, // Jira Document format (ADF)
    status: Option<JiraStatus>,
    #[serde(rename = "issuetype")]
    issue_type: Option<JiraIssueType>,
    #[serde(default)]
    assignee: Option<JiraUser>,
    #[serde(default)]
    reporter: Option<JiraUser>,
    #[serde(default)]
    priority: Option<JiraPriority>,
    #[serde(rename = "created")]
    created: Option<String>,
    #[serde(default)]
    labels: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct JiraStatus {
    name: String,
}

#[derive(Debug, Deserialize)]
struct JiraIssueType {
    name: String,
}

#[derive(Debug, Deserialize)]
struct JiraUser {
    #[serde(rename = "displayName")]
    display_name: String,
}

#[derive(Debug, Deserialize)]
struct JiraPriority {
    name: String,
}

/// Extrait le texte brut d'un document ADF (Atlassian Document Format)
fn extract_adf_text(val: &serde_json::Value) -> String {
    let mut parts = Vec::new();
    extract_adf_recursive(val, &mut parts);
    parts.join(" ")
}

fn extract_adf_recursive(val: &serde_json::Value, parts: &mut Vec<String>) {
    if let Some(text) = val.get("text").and_then(|t| t.as_str()) {
        if !text.is_empty() {
            parts.push(text.to_string());
        }
    }
    if let Some(content) = val.get("content").and_then(|c| c.as_array()) {
        for child in content {
            extract_adf_recursive(child, parts);
        }
    }
}

impl osmozzz_core::Harvester for JiraHarvester {
    async fn harvest(&self) -> Result<Vec<Document>> {
        let cfg = match JiraConfig::load() {
            Some(c) => c,
            None => {
                warn!("Jira: ~/.osmozzz/jira.toml non trouvé — source ignorée");
                return Ok(vec![]);
            }
        };

        let base_url = cfg.base_url.trim_end_matches('/');
        let client = reqwest::Client::new();
        let mut documents = Vec::new();
        let mut start_at = 0u64;

        loop {
            let url = format!(
                "{}/rest/api/3/search?jql=ORDER+BY+created+DESC&startAt={}&maxResults=100&fields=summary,description,status,issuetype,assignee,reporter,priority,created,labels",
                base_url, start_at
            );

            let resp = match client
                .get(&url)
                .basic_auth(&cfg.email, Some(&cfg.token))
                .header("Accept", "application/json")
                .send()
                .await
            {
                Ok(r) => r,
                Err(e) => {
                    warn!("Jira API error: {}", e);
                    break;
                }
            };

            if resp.status() == 401 {
                warn!("Jira: authentification échouée (401) — vérifiez ~/.osmozzz/jira.toml");
                break;
            }

            if !resp.status().is_success() {
                warn!("Jira API status: {}", resp.status());
                break;
            }

            let search_resp: JiraSearchResponse = match resp.json().await {
                Ok(r) => r,
                Err(e) => {
                    warn!("Jira JSON parse error: {}", e);
                    break;
                }
            };

            for issue in &search_resp.issues {
                let status = issue
                    .fields
                    .status
                    .as_ref()
                    .map(|s| s.name.as_str())
                    .unwrap_or("Unknown");
                let issue_type = issue
                    .fields
                    .issue_type
                    .as_ref()
                    .map(|t| t.name.as_str())
                    .unwrap_or("Issue");
                let assignee = issue
                    .fields
                    .assignee
                    .as_ref()
                    .map(|a| a.display_name.as_str())
                    .unwrap_or("Non assigné");
                let priority = issue
                    .fields
                    .priority
                    .as_ref()
                    .map(|p| p.name.as_str())
                    .unwrap_or("None");

                let desc_text = issue
                    .fields
                    .description
                    .as_ref()
                    .map(|d| extract_adf_text(d))
                    .unwrap_or_default();

                let labels = issue.fields.labels.join(", ");

                let jira_url = format!(
                    "{}/browse/{}",
                    base_url, issue.key
                );

                let content = format!(
                    "[{}] {} — {}\nStatut: {} | Priorité: {} | Assigné: {}\nLabels: {}\n\n{}",
                    issue_type,
                    issue.key,
                    issue.fields.summary,
                    status,
                    priority,
                    assignee,
                    labels,
                    desc_text
                );

                let checksum = checksum::compute(&content);
                let mut doc =
                    Document::new(SourceType::Jira, &jira_url, &content, &checksum)
                        .with_title(&issue.fields.summary);

                if let Some(ref created) = issue.fields.created {
                    if let Ok(ts) = DateTime::parse_from_rfc3339(created) {
                        doc = doc.with_source_ts(ts.with_timezone(&chrono::Utc));
                    }
                }

                documents.push(doc);
            }

            start_at += search_resp.max_results;
            if start_at >= search_resp.total || documents.len() >= MAX_ISSUES {
                break;
            }
        }

        info!("Jira harvester found {} issues", documents.len());
        Ok(documents)
    }
}
