/// Todoist Harvester — indexe les tâches et projets Todoist.
///
/// Config : ~/.osmozzz/todoist.toml
/// ```toml
/// token = "xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"
/// ```
///
/// Token : todoist.com/app/settings/integrations/developer → API token
use chrono::DateTime;
use osmozzz_core::{Document, Result, SourceType};
use serde::Deserialize;
use tracing::{info, warn};

use crate::checksum;

const TODOIST_API: &str = "https://api.todoist.com/rest/v2";

#[derive(Debug, Deserialize)]
struct TodoistConfig {
    token: String,
}

impl TodoistConfig {
    fn load() -> Option<Self> {
        let path = dirs_next::home_dir()?.join(".osmozzz/todoist.toml");
        let content = std::fs::read_to_string(path).ok()?;
        toml::from_str(&content).ok()
    }
}

pub struct TodoistHarvester;

impl TodoistHarvester {
    pub fn new() -> Self {
        Self
    }
}

impl Default for TodoistHarvester {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Deserialize)]
struct TodoistTask {
    id: String,
    content: String,
    #[serde(default)]
    description: String,
    #[serde(rename = "projectId")]
    #[serde(default)]
    project_id: Option<String>,
    #[serde(default)]
    priority: u8,
    #[serde(rename = "isCompleted")]
    #[serde(default)]
    is_completed: bool,
    #[serde(rename = "createdAt")]
    created_at: Option<String>,
    #[serde(default)]
    labels: Vec<String>,
    #[serde(default)]
    due: Option<TodoistDue>,
    url: String,
}

#[derive(Debug, Deserialize)]
struct TodoistDue {
    #[serde(default)]
    date: String,
    #[serde(default)]
    string: String,
}

#[derive(Debug, Deserialize)]
struct TodoistProject {
    id: String,
    name: String,
}

impl osmozzz_core::Harvester for TodoistHarvester {
    async fn harvest(&self) -> Result<Vec<Document>> {
        let cfg = match TodoistConfig::load() {
            Some(c) => c,
            None => {
                warn!("Todoist: ~/.osmozzz/todoist.toml non trouvé — source ignorée");
                return Ok(vec![]);
            }
        };

        let client = reqwest::Client::new();

        // Récupérer les projets pour les noms
        let projects: Vec<TodoistProject> = match client
            .get(format!("{}/projects", TODOIST_API))
            .bearer_auth(&cfg.token)
            .send()
            .await
        {
            Ok(r) => {
                if r.status() == 401 {
                    warn!("Todoist: token invalide (401) — vérifiez ~/.osmozzz/todoist.toml");
                    return Ok(vec![]);
                }
                r.json().await.unwrap_or_default()
            }
            Err(e) => {
                warn!("Todoist API error: {}", e);
                return Ok(vec![]);
            }
        };

        let project_map: std::collections::HashMap<String, String> = projects
            .into_iter()
            .map(|p| (p.id, p.name))
            .collect();

        // Récupérer toutes les tâches actives
        let tasks: Vec<TodoistTask> = match client
            .get(format!("{}/tasks", TODOIST_API))
            .bearer_auth(&cfg.token)
            .send()
            .await
        {
            Ok(r) => r.json().await.unwrap_or_default(),
            Err(e) => {
                warn!("Todoist tasks API error: {}", e);
                return Ok(vec![]);
            }
        };

        let mut documents = Vec::new();

        for task in &tasks {
            let project_name = task
                .project_id
                .as_ref()
                .and_then(|id| project_map.get(id))
                .map(|s| s.as_str())
                .unwrap_or("Inbox");

            let priority_label = match task.priority {
                4 => "P1 (Urgente)",
                3 => "P2 (Haute)",
                2 => "P3 (Moyenne)",
                _ => "P4 (Normale)",
            };

            let due_str = task
                .due
                .as_ref()
                .map(|d| {
                    if d.string.is_empty() {
                        d.date.clone()
                    } else {
                        d.string.clone()
                    }
                })
                .unwrap_or_default();

            let labels = task.labels.join(", ");

            let content = format!(
                "{}\nProjet: {} | Priorité: {} | Labels: {}{}\n\n{}",
                task.content,
                project_name,
                priority_label,
                labels,
                if due_str.is_empty() {
                    String::new()
                } else {
                    format!(" | Échéance: {}", due_str)
                },
                task.description.trim()
            );

            let checksum = checksum::compute(&content);
            let mut doc =
                Document::new(SourceType::Todoist, &task.url, &content, &checksum)
                    .with_title(&task.content);

            if let Some(ref created) = task.created_at {
                if let Ok(ts) = DateTime::parse_from_rfc3339(created) {
                    doc = doc.with_source_ts(ts.with_timezone(&chrono::Utc));
                }
            }

            documents.push(doc);
        }

        info!("Todoist harvester found {} tasks", documents.len());
        Ok(documents)
    }
}
