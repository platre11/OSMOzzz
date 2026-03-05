/// GitLab Harvester — indexe les issues et merge requests GitLab.
/// Supporte gitlab.com et les instances auto-hébergées.
///
/// Config : ~/.osmozzz/gitlab.toml
/// ```toml
/// token    = "glpat-xxxxxxxxxxxxxxxxxxxx"
/// base_url = "https://gitlab.com"    # ou votre instance
/// groups   = ["mon-groupe"]          # optionnel : limiter à certains groupes
/// ```
///
/// Token : gitlab.com/-/user_settings/personal_access_tokens
/// Scopes nécessaires : read_api
use chrono::DateTime;
use osmozzz_core::{Document, Result, SourceType};
use serde::Deserialize;
use tracing::{info, warn};

use crate::checksum;

const MAX_ITEMS_PER_PROJECT: usize = 500;

#[derive(Debug, Deserialize)]
struct GitlabConfig {
    token: String,
    #[serde(default = "default_gitlab_url")]
    base_url: String,
    #[serde(default)]
    groups: Vec<String>,
}

fn default_gitlab_url() -> String {
    "https://gitlab.com".to_string()
}

impl GitlabConfig {
    fn load() -> Option<Self> {
        let path = dirs_next::home_dir()?.join(".osmozzz/gitlab.toml");
        let content = std::fs::read_to_string(path).ok()?;
        toml::from_str(&content).ok()
    }
}

pub struct GitlabHarvester;

impl GitlabHarvester {
    pub fn new() -> Self {
        Self
    }
}

impl Default for GitlabHarvester {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Deserialize)]
struct GitlabProject {
    id: u64,
    name: String,
    #[serde(rename = "path_with_namespace")]
    path: String,
}

#[derive(Debug, Deserialize)]
struct GitlabIssue {
    iid: u64,
    title: String,
    #[serde(default)]
    description: Option<String>,
    state: String,
    #[serde(rename = "web_url")]
    web_url: String,
    #[serde(rename = "created_at")]
    created_at: String,
    #[serde(default)]
    labels: Vec<String>,
    #[serde(default)]
    assignees: Vec<GitlabUser>,
}

#[derive(Debug, Deserialize)]
struct GitlabMR {
    iid: u64,
    title: String,
    #[serde(default)]
    description: Option<String>,
    state: String,
    #[serde(rename = "web_url")]
    web_url: String,
    #[serde(rename = "created_at")]
    created_at: String,
    #[serde(default)]
    labels: Vec<String>,
    #[serde(default)]
    assignees: Vec<GitlabUser>,
}

#[derive(Debug, Deserialize)]
struct GitlabUser {
    username: String,
}

async fn fetch_gitlab_paginated<T: for<'de> Deserialize<'de>>(
    client: &reqwest::Client,
    token: &str,
    url: &str,
    max: usize,
) -> Vec<T> {
    let mut all = Vec::new();
    let mut page = 1u32;

    loop {
        let full_url = format!("{}&page={}&per_page=100", url, page);
        let resp = match client
            .get(&full_url)
            .header("PRIVATE-TOKEN", token)
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                warn!("GitLab API error: {}", e);
                break;
            }
        };

        if !resp.status().is_success() {
            warn!("GitLab status: {}", resp.status());
            break;
        }

        let items: Vec<T> = match resp.json().await {
            Ok(v) => v,
            Err(_) => break,
        };

        if items.is_empty() {
            break;
        }

        let fetched = items.len();
        all.extend(items);

        if all.len() >= max || fetched < 100 {
            break;
        }
        page += 1;
    }

    all
}

impl osmozzz_core::Harvester for GitlabHarvester {
    async fn harvest(&self) -> Result<Vec<Document>> {
        let cfg = match GitlabConfig::load() {
            Some(c) => c,
            None => {
                warn!("GitLab: ~/.osmozzz/gitlab.toml non trouvé — source ignorée");
                return Ok(vec![]);
            }
        };

        let api = format!("{}/api/v4", cfg.base_url.trim_end_matches('/'));
        let client = reqwest::Client::new();
        let mut documents = Vec::new();

        // Récupérer les projets (depuis les groupes ou directement l'utilisateur)
        let projects: Vec<GitlabProject> = if cfg.groups.is_empty() {
            fetch_gitlab_paginated(
                &client,
                &cfg.token,
                &format!("{}/projects?membership=true&archived=false", api),
                200,
            )
            .await
        } else {
            let mut all_projects = Vec::new();
            for group in &cfg.groups {
                let url = format!(
                    "{}/groups/{}/projects?archived=false&include_subgroups=true",
                    api,
                    urlencoding_simple(group)
                );
                let projects: Vec<GitlabProject> =
                    fetch_gitlab_paginated(&client, &cfg.token, &url, 200).await;
                all_projects.extend(projects);
            }
            all_projects
        };

        for project in &projects {
            // Issues
            let issues_url = format!(
                "{}/projects/{}/issues?state=all",
                api, project.id
            );
            let issues: Vec<GitlabIssue> =
                fetch_gitlab_paginated(&client, &cfg.token, &issues_url, MAX_ITEMS_PER_PROJECT)
                    .await;

            for issue in issues {
                let assignees: Vec<&str> =
                    issue.assignees.iter().map(|a| a.username.as_str()).collect();
                let desc = issue.description.as_deref().unwrap_or("").trim();

                let content = format!(
                    "[Issue] !{} — {}\nProjet: {} | Statut: {} | Labels: {} | Assignés: {}\n\n{}",
                    issue.iid,
                    issue.title,
                    project.path,
                    issue.state,
                    issue.labels.join(", "),
                    assignees.join(", "),
                    desc
                );

                let checksum = checksum::compute(&content);
                let mut doc = Document::new(
                    SourceType::Gitlab,
                    &issue.web_url,
                    &content,
                    &checksum,
                )
                .with_title(&issue.title);

                if let Ok(ts) = DateTime::parse_from_rfc3339(&issue.created_at) {
                    doc = doc.with_source_ts(ts.with_timezone(&chrono::Utc));
                }

                documents.push(doc);
            }

            // Merge Requests
            let mrs_url = format!(
                "{}/projects/{}/merge_requests?state=all",
                api, project.id
            );
            let mrs: Vec<GitlabMR> =
                fetch_gitlab_paginated(&client, &cfg.token, &mrs_url, MAX_ITEMS_PER_PROJECT).await;

            for mr in mrs {
                let assignees: Vec<&str> =
                    mr.assignees.iter().map(|a| a.username.as_str()).collect();
                let desc = mr.description.as_deref().unwrap_or("").trim();

                let content = format!(
                    "[MR] !{} — {}\nProjet: {} | Statut: {} | Labels: {} | Assignés: {}\n\n{}",
                    mr.iid,
                    mr.title,
                    project.path,
                    mr.state,
                    mr.labels.join(", "),
                    assignees.join(", "),
                    desc
                );

                let checksum = checksum::compute(&content);
                let mut doc = Document::new(
                    SourceType::Gitlab,
                    &mr.web_url,
                    &content,
                    &checksum,
                )
                .with_title(&mr.title);

                if let Ok(ts) = DateTime::parse_from_rfc3339(&mr.created_at) {
                    doc = doc.with_source_ts(ts.with_timezone(&chrono::Utc));
                }

                documents.push(doc);
            }
        }

        info!("GitLab harvester found {} items across {} projects", documents.len(), projects.len());
        Ok(documents)
    }
}

fn urlencoding_simple(s: &str) -> String {
    s.replace('/', "%2F").replace(' ', "%20")
}
