/// GitHub Harvester — indexe les issues et pull requests de vos repos.
///
/// Config : ~/.osmozzz/github.toml
/// ```toml
/// token = "ghp_xxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx"
/// repos = ["owner/repo1", "owner/repo2"]
/// ```
///
/// Token : github.com/settings/tokens → Fine-grained token ou classic token
/// Scopes nécessaires : repo (read)
use chrono::DateTime;
use osmozzz_core::{Document, Result, SourceType};
use serde::Deserialize;
use tracing::{info, warn};

use crate::checksum;

const GITHUB_API: &str = "https://api.github.com";
const MAX_ITEMS_PER_REPO: usize = 500;

#[derive(Debug, Deserialize)]
struct GithubConfig {
    token: String,
    #[serde(default)]
    repos: Vec<String>,
}

impl GithubConfig {
    fn load() -> Option<Self> {
        let path = dirs_next::home_dir()?.join(".osmozzz/github.toml");
        let content = std::fs::read_to_string(path).ok()?;
        toml::from_str(&content).ok()
    }
}

pub struct GithubHarvester;

impl GithubHarvester {
    pub fn new() -> Self {
        Self
    }
}

impl Default for GithubHarvester {
    fn default() -> Self {
        Self::new()
    }
}

// ─── Réponse API GitHub ───────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct GithubIssue {
    number: u64,
    title: String,
    #[serde(default)]
    body: Option<String>,
    state: String,
    html_url: String,
    created_at: String,
    #[serde(default)]
    pull_request: Option<serde_json::Value>, // présent si c'est une PR
    #[serde(default)]
    labels: Vec<GithubLabel>,
    #[serde(default)]
    user: Option<GithubUser>,
}

#[derive(Debug, Deserialize)]
struct GithubLabel {
    name: String,
}

#[derive(Debug, Deserialize)]
struct GithubUser {
    login: String,
}

async fn fetch_issues_for_repo(
    client: &reqwest::Client,
    token: &str,
    repo: &str,
    is_pr: bool,
) -> Vec<GithubIssue> {
    let endpoint = if is_pr { "pulls" } else { "issues" };
    let mut all = Vec::new();
    let mut page = 1u32;

    loop {
        let url = format!(
            "{}/repos/{}/{}?state=all&per_page=100&page={}",
            GITHUB_API, repo, endpoint, page
        );

        let resp = match client
            .get(&url)
            .bearer_auth(token)
            .header("User-Agent", "OSMOzzz/1.0")
            .header("Accept", "application/vnd.github+json")
            .header("X-GitHub-Api-Version", "2022-11-28")
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                warn!("GitHub API error for {}/{}: {}", repo, endpoint, e);
                break;
            }
        };

        if resp.status() == 401 || resp.status() == 403 {
            warn!("GitHub: accès refusé pour {} ({})", repo, resp.status());
            break;
        }

        if resp.status() == 404 {
            warn!("GitHub: repo '{}' introuvable", repo);
            break;
        }

        if !resp.status().is_success() {
            warn!("GitHub status {} pour {}", resp.status(), repo);
            break;
        }

        let items: Vec<GithubIssue> = match resp.json().await {
            Ok(v) => v,
            Err(e) => {
                warn!("GitHub JSON parse error: {}", e);
                break;
            }
        };

        if items.is_empty() {
            break;
        }

        let fetched = items.len();
        all.extend(items);

        if all.len() >= MAX_ITEMS_PER_REPO || fetched < 100 {
            break;
        }
        page += 1;
    }

    all
}

impl osmozzz_core::Harvester for GithubHarvester {
    async fn harvest(&self) -> Result<Vec<Document>> {
        let cfg = match GithubConfig::load() {
            Some(c) => c,
            None => {
                warn!("GitHub: ~/.osmozzz/github.toml non trouvé — source ignorée");
                return Ok(vec![]);
            }
        };

        if cfg.repos.is_empty() {
            warn!("GitHub: aucun repo configuré dans github.toml (repos = [...])");
            return Ok(vec![]);
        }

        let client = reqwest::Client::new();
        let mut documents = Vec::new();

        for repo in &cfg.repos {
            // Issues (exclut les PRs car GitHub les retourne aussi dans /issues)
            let issues = fetch_issues_for_repo(&client, &cfg.token, repo, false).await;
            for issue in issues {
                // Exclure les PRs du endpoint issues (elles ont le champ pull_request)
                if issue.pull_request.is_some() {
                    continue;
                }

                let labels: Vec<&str> = issue.labels.iter().map(|l| l.name.as_str()).collect();
                let author = issue
                    .user
                    .as_ref()
                    .map(|u| u.login.as_str())
                    .unwrap_or("unknown");
                let body = issue.body.as_deref().unwrap_or("").trim().to_string();

                let content = format!(
                    "[{}] #{} — {}\nRepo: {}\nAuteur: {}\nStatut: {}\nLabels: {}\n\n{}",
                    "Issue",
                    issue.number,
                    issue.title,
                    repo,
                    author,
                    issue.state,
                    labels.join(", "),
                    body
                );

                let checksum = checksum::compute(&content);
                let mut doc = Document::new(
                    SourceType::Github,
                    &issue.html_url,
                    &content,
                    &checksum,
                )
                .with_title(&issue.title);

                if let Ok(ts) = DateTime::parse_from_rfc3339(&issue.created_at) {
                    doc = doc.with_source_ts(ts.with_timezone(&chrono::Utc));
                }

                documents.push(doc);
            }

            // Pull Requests
            let prs = fetch_issues_for_repo(&client, &cfg.token, repo, true).await;
            for pr in prs {
                let labels: Vec<&str> = pr.labels.iter().map(|l| l.name.as_str()).collect();
                let author = pr
                    .user
                    .as_ref()
                    .map(|u| u.login.as_str())
                    .unwrap_or("unknown");
                let body = pr.body.as_deref().unwrap_or("").trim().to_string();

                let content = format!(
                    "[PR] #{} — {}\nRepo: {}\nAuteur: {}\nStatut: {}\nLabels: {}\n\n{}",
                    pr.number,
                    pr.title,
                    repo,
                    author,
                    pr.state,
                    labels.join(", "),
                    body
                );

                let checksum = checksum::compute(&content);
                let mut doc = Document::new(
                    SourceType::Github,
                    &pr.html_url,
                    &content,
                    &checksum,
                )
                .with_title(&pr.title);

                if let Ok(ts) = DateTime::parse_from_rfc3339(&pr.created_at) {
                    doc = doc.with_source_ts(ts.with_timezone(&chrono::Utc));
                }

                documents.push(doc);
            }
        }

        info!("GitHub harvester found {} items across {} repos", documents.len(), cfg.repos.len());
        Ok(documents)
    }
}
