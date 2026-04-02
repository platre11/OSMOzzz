/// Connecteur GitHub — REST API v3 officielle.
/// Toutes les fonctions retournent Result<String, String> sans jamais appeler send().
/// La couche sécurité (sanitize_proxy_response + audit_log) est appliquée par le dispatcher (mcp.rs).
use serde_json::{json, Value};

// ─── Config ──────────────────────────────────────────────────────────────────

#[derive(serde::Deserialize)]
struct GitHubConfig {
    token: String,
}

impl GitHubConfig {
    fn load() -> Option<Self> {
        let path = dirs_next::home_dir()?.join(".osmozzz/github.toml");
        toml::from_str(&std::fs::read_to_string(path).ok()?).ok()
    }

    fn api(&self, path: &str) -> String {
        format!("https://api.github.com/{}", path.trim_start_matches('/'))
    }
}

// ─── HTTP helpers ─────────────────────────────────────────────────────────────

async fn get(cfg: &GitHubConfig, url: &str) -> Result<Value, String> {
    reqwest::Client::new()
        .get(url)
        .header("Authorization", format!("Bearer {}", cfg.token))
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .header("User-Agent", "osmozzz")
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<Value>()
        .await
        .map_err(|e| e.to_string())
}

async fn post_json(cfg: &GitHubConfig, url: &str, body: &Value) -> Result<Value, String> {
    reqwest::Client::new()
        .post(url)
        .header("Authorization", format!("Bearer {}", cfg.token))
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .header("Content-Type", "application/json")
        .header("User-Agent", "osmozzz")
        .json(body)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<Value>()
        .await
        .map_err(|e| e.to_string())
}

async fn patch_json(cfg: &GitHubConfig, url: &str, body: &Value) -> Result<Value, String> {
    reqwest::Client::new()
        .patch(url)
        .header("Authorization", format!("Bearer {}", cfg.token))
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .header("Content-Type", "application/json")
        .header("User-Agent", "osmozzz")
        .json(body)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<Value>()
        .await
        .map_err(|e| e.to_string())
}

async fn put_json(cfg: &GitHubConfig, url: &str, body: &Value) -> Result<Value, String> {
    reqwest::Client::new()
        .put(url)
        .header("Authorization", format!("Bearer {}", cfg.token))
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .header("Content-Type", "application/json")
        .header("User-Agent", "osmozzz")
        .json(body)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<Value>()
        .await
        .map_err(|e| e.to_string())
}

async fn post_json_raw(cfg: &GitHubConfig, url: &str, body: &Value) -> Result<reqwest::Response, String> {
    reqwest::Client::new()
        .post(url)
        .header("Authorization", format!("Bearer {}", cfg.token))
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .header("Content-Type", "application/json")
        .header("User-Agent", "osmozzz")
        .json(body)
        .send()
        .await
        .map_err(|e| e.to_string())
}

// ─── Tool definitions ─────────────────────────────────────────────────────────

pub fn tools() -> Vec<Value> {
    vec![
        // ── Repositories ─────────────────────────────────────────────────────
        json!({
            "name": "github_search_repositories",
            "description": "GITHUB — Recherche des dépôts GitHub par mots-clés (nom, description, topics). Retourne la liste avec étoiles, langue, description et URL.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query":    { "type": "string",  "description": "Termes de recherche (ex: 'rust async' ou 'org:myorg')" },
                    "page":     { "type": "integer", "description": "Numéro de page (défaut: 1)", "default": 1 },
                    "per_page": { "type": "integer", "description": "Résultats par page (défaut: 30, max: 100)", "default": 30, "maximum": 100 }
                },
                "required": ["query"]
            }
        }),
        json!({
            "name": "github_create_repository",
            "description": "GITHUB — Crée un nouveau dépôt GitHub dans le compte de l'utilisateur authentifié. Retourne l'URL et l'ID du dépôt créé.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name":        { "type": "string",  "description": "Nom du dépôt" },
                    "description": { "type": "string",  "description": "Description (optionnel)" },
                    "private":     { "type": "boolean", "description": "Dépôt privé (défaut: false)", "default": false },
                    "auto_init":   { "type": "boolean", "description": "Initialiser avec un README (défaut: false)", "default": false }
                },
                "required": ["name"]
            }
        }),
        // ── Files ─────────────────────────────────────────────────────────────
        json!({
            "name": "github_get_file_contents",
            "description": "GITHUB — Récupère le contenu d'un fichier (ou liste d'un dossier) dans un dépôt GitHub. Le contenu est décodé depuis base64.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "owner":  { "type": "string", "description": "Propriétaire du dépôt (user ou org)" },
                    "repo":   { "type": "string", "description": "Nom du dépôt" },
                    "path":   { "type": "string", "description": "Chemin du fichier ou dossier dans le dépôt (ex: 'src/main.rs')" },
                    "branch": { "type": "string", "description": "Branche (défaut: branche par défaut du dépôt)" }
                },
                "required": ["owner", "repo", "path"]
            }
        }),
        json!({
            "name": "github_create_or_update_file",
            "description": "GITHUB — Crée ou met à jour un fichier dans un dépôt GitHub via un commit. Le contenu doit être encodé en base64. Pour une mise à jour, fournir le sha du fichier existant.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "owner":           { "type": "string", "description": "Propriétaire du dépôt" },
                    "repo":            { "type": "string", "description": "Nom du dépôt" },
                    "path":            { "type": "string", "description": "Chemin du fichier dans le dépôt" },
                    "message":         { "type": "string", "description": "Message du commit" },
                    "content_base64":  { "type": "string", "description": "Contenu du fichier encodé en base64" },
                    "sha":             { "type": "string", "description": "SHA du fichier existant (requis pour mise à jour)" },
                    "branch":          { "type": "string", "description": "Branche cible (optionnel)" }
                },
                "required": ["owner", "repo", "path", "message", "content_base64"]
            }
        }),
        json!({
            "name": "github_push_files",
            "description": "GITHUB — Pousse plusieurs fichiers en un seul commit via l'API Git trees. Permet de committer plusieurs fichiers atomiquement sur une branche.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "owner":   { "type": "string", "description": "Propriétaire du dépôt" },
                    "repo":    { "type": "string", "description": "Nom du dépôt" },
                    "branch":  { "type": "string", "description": "Branche cible" },
                    "message": { "type": "string", "description": "Message du commit" },
                    "files":   {
                        "type": "array",
                        "description": "Liste de fichiers à pousser",
                        "items": {
                            "type": "object",
                            "properties": {
                                "path":    { "type": "string", "description": "Chemin du fichier dans le dépôt" },
                                "content": { "type": "string", "description": "Contenu textuel du fichier" }
                            },
                            "required": ["path", "content"]
                        }
                    }
                },
                "required": ["owner", "repo", "branch", "message", "files"]
            }
        }),
        // ── Issues ────────────────────────────────────────────────────────────
        json!({
            "name": "github_create_issue",
            "description": "GITHUB — Crée une nouvelle issue dans un dépôt GitHub. Retourne le numéro et l'URL de l'issue créée.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "owner":     { "type": "string", "description": "Propriétaire du dépôt" },
                    "repo":      { "type": "string", "description": "Nom du dépôt" },
                    "title":     { "type": "string", "description": "Titre de l'issue" },
                    "body":      { "type": "string", "description": "Description de l'issue en Markdown (optionnel)" },
                    "assignees": { "type": "array",  "items": { "type": "string" }, "description": "Logins des assignés (optionnel)" },
                    "labels":    { "type": "array",  "items": { "type": "string" }, "description": "Labels à appliquer (optionnel)" },
                    "milestone": { "type": "integer", "description": "Numéro du milestone (optionnel)" }
                },
                "required": ["owner", "repo", "title"]
            }
        }),
        json!({
            "name": "github_list_issues",
            "description": "GITHUB — Liste les issues d'un dépôt GitHub. Filtrable par état, labels, assigné, date. Retourne numéro, titre, état et date.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "owner":     { "type": "string", "description": "Propriétaire du dépôt" },
                    "repo":      { "type": "string", "description": "Nom du dépôt" },
                    "state":     { "type": "string", "description": "État : open (défaut), closed, all", "enum": ["open", "closed", "all"], "default": "open" },
                    "labels":    { "type": "string", "description": "Labels séparés par virgule (optionnel)" },
                    "sort":      { "type": "string", "description": "Tri : created (défaut), updated, comments", "enum": ["created", "updated", "comments"] },
                    "direction": { "type": "string", "description": "Ordre : desc (défaut), asc", "enum": ["asc", "desc"] },
                    "since":     { "type": "string", "description": "Date ISO 8601 minimum (optionnel)" },
                    "page":      { "type": "integer", "description": "Numéro de page (défaut: 1)", "default": 1 },
                    "per_page":  { "type": "integer", "description": "Résultats par page (défaut: 30, max: 100)", "default": 30, "maximum": 100 }
                },
                "required": ["owner", "repo"]
            }
        }),
        json!({
            "name": "github_update_issue",
            "description": "GITHUB — Met à jour une issue GitHub : titre, description, état, labels, assignés ou milestone.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "owner":        { "type": "string",  "description": "Propriétaire du dépôt" },
                    "repo":         { "type": "string",  "description": "Nom du dépôt" },
                    "issue_number": { "type": "integer", "description": "Numéro de l'issue" },
                    "title":        { "type": "string",  "description": "Nouveau titre (optionnel)" },
                    "body":         { "type": "string",  "description": "Nouvelle description (optionnel)" },
                    "state":        { "type": "string",  "description": "Nouvel état : open ou closed", "enum": ["open", "closed"] },
                    "labels":       { "type": "array",   "items": { "type": "string" }, "description": "Labels (remplace tout, optionnel)" },
                    "assignees":    { "type": "array",   "items": { "type": "string" }, "description": "Assignés (remplace tout, optionnel)" },
                    "milestone":    { "type": "integer", "description": "Numéro du milestone (optionnel)" }
                },
                "required": ["owner", "repo", "issue_number"]
            }
        }),
        json!({
            "name": "github_add_issue_comment",
            "description": "GITHUB — Ajoute un commentaire à une issue GitHub. Retourne l'ID et l'URL du commentaire créé.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "owner":        { "type": "string",  "description": "Propriétaire du dépôt" },
                    "repo":         { "type": "string",  "description": "Nom du dépôt" },
                    "issue_number": { "type": "integer", "description": "Numéro de l'issue" },
                    "body":         { "type": "string",  "description": "Contenu du commentaire en Markdown" }
                },
                "required": ["owner", "repo", "issue_number", "body"]
            }
        }),
        json!({
            "name": "github_get_issue",
            "description": "GITHUB — Récupère les détails complets d'une issue GitHub : titre, état, description, assignés, labels, milestone et dates.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "owner":        { "type": "string",  "description": "Propriétaire du dépôt" },
                    "repo":         { "type": "string",  "description": "Nom du dépôt" },
                    "issue_number": { "type": "integer", "description": "Numéro de l'issue" }
                },
                "required": ["owner", "repo", "issue_number"]
            }
        }),
        json!({
            "name": "github_search_issues",
            "description": "GITHUB — Recherche des issues et pull requests GitHub avec la syntaxe de recherche avancée (ex: 'is:issue is:open repo:owner/repo label:bug').",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query":    { "type": "string",  "description": "Requête de recherche GitHub (ex: 'is:issue is:open author:me')" },
                    "sort":     { "type": "string",  "description": "Tri : comments, created, updated, reactions", "enum": ["comments", "created", "updated", "reactions"] },
                    "order":    { "type": "string",  "description": "Ordre : desc (défaut), asc", "enum": ["asc", "desc"] },
                    "per_page": { "type": "integer", "description": "Résultats par page (défaut: 30, max: 100)", "default": 30 },
                    "page":     { "type": "integer", "description": "Numéro de page (défaut: 1)", "default": 1 }
                },
                "required": ["query"]
            }
        }),
        // ── Pull Requests ─────────────────────────────────────────────────────
        json!({
            "name": "github_create_pull_request",
            "description": "GITHUB — Crée une pull request GitHub. Retourne le numéro et l'URL de la PR créée.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "owner": { "type": "string",  "description": "Propriétaire du dépôt" },
                    "repo":  { "type": "string",  "description": "Nom du dépôt" },
                    "title": { "type": "string",  "description": "Titre de la PR" },
                    "body":  { "type": "string",  "description": "Description de la PR en Markdown (optionnel)" },
                    "head":  { "type": "string",  "description": "Branche source (ex: 'feature/my-feature' ou 'user:branch')" },
                    "base":  { "type": "string",  "description": "Branche cible (ex: 'main')" },
                    "draft": { "type": "boolean", "description": "Créer comme brouillon (défaut: false)", "default": false }
                },
                "required": ["owner", "repo", "title", "head", "base"]
            }
        }),
        json!({
            "name": "github_get_pull_request",
            "description": "GITHUB — Récupère les détails d'une pull request GitHub : titre, état, branches, assignés, reviewers et statistiques de code.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "owner":       { "type": "string",  "description": "Propriétaire du dépôt" },
                    "repo":        { "type": "string",  "description": "Nom du dépôt" },
                    "pull_number": { "type": "integer", "description": "Numéro de la pull request" }
                },
                "required": ["owner", "repo", "pull_number"]
            }
        }),
        json!({
            "name": "github_list_pull_requests",
            "description": "GITHUB — Liste les pull requests d'un dépôt GitHub. Filtrable par état, branche source/cible, tri et ordre.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "owner":     { "type": "string",  "description": "Propriétaire du dépôt" },
                    "repo":      { "type": "string",  "description": "Nom du dépôt" },
                    "state":     { "type": "string",  "description": "État : open (défaut), closed, all", "enum": ["open", "closed", "all"], "default": "open" },
                    "head":      { "type": "string",  "description": "Filtre par branche source (optionnel)" },
                    "base":      { "type": "string",  "description": "Filtre par branche cible (optionnel)" },
                    "sort":      { "type": "string",  "description": "Tri : created (défaut), updated, popularity, long-running" },
                    "direction": { "type": "string",  "description": "Ordre : desc (défaut), asc", "enum": ["asc", "desc"] },
                    "per_page":  { "type": "integer", "description": "Résultats par page (défaut: 30, max: 100)", "default": 30 },
                    "page":      { "type": "integer", "description": "Numéro de page (défaut: 1)", "default": 1 }
                },
                "required": ["owner", "repo"]
            }
        }),
        json!({
            "name": "github_merge_pull_request",
            "description": "GITHUB — Fusionne une pull request GitHub. Supporte les méthodes merge, squash et rebase.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "owner":          { "type": "string",  "description": "Propriétaire du dépôt" },
                    "repo":           { "type": "string",  "description": "Nom du dépôt" },
                    "pull_number":    { "type": "integer", "description": "Numéro de la pull request" },
                    "commit_title":   { "type": "string",  "description": "Titre du commit de merge (optionnel)" },
                    "commit_message": { "type": "string",  "description": "Corps du commit de merge (optionnel)" },
                    "merge_method":   { "type": "string",  "description": "Méthode : merge (défaut), squash, rebase", "enum": ["merge", "squash", "rebase"], "default": "merge" }
                },
                "required": ["owner", "repo", "pull_number"]
            }
        }),
        json!({
            "name": "github_get_pull_request_files",
            "description": "GITHUB — Liste les fichiers modifiés dans une pull request GitHub avec les statistiques de changements (additions, suppressions, patches).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "owner":       { "type": "string",  "description": "Propriétaire du dépôt" },
                    "repo":        { "type": "string",  "description": "Nom du dépôt" },
                    "pull_number": { "type": "integer", "description": "Numéro de la pull request" }
                },
                "required": ["owner", "repo", "pull_number"]
            }
        }),
        json!({
            "name": "github_get_pull_request_status",
            "description": "GITHUB — Récupère le statut CI/CD combiné de la dernière révision d'une pull request (checks, status contexts). Utile pour vérifier si les tests passent.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "owner":       { "type": "string",  "description": "Propriétaire du dépôt" },
                    "repo":        { "type": "string",  "description": "Nom du dépôt" },
                    "pull_number": { "type": "integer", "description": "Numéro de la pull request" }
                },
                "required": ["owner", "repo", "pull_number"]
            }
        }),
        json!({
            "name": "github_update_pull_request_branch",
            "description": "GITHUB — Met à jour la branche d'une pull request avec les derniers commits de la branche de base (équivalent à 'Update branch').",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "owner":             { "type": "string",  "description": "Propriétaire du dépôt" },
                    "repo":              { "type": "string",  "description": "Nom du dépôt" },
                    "pull_number":       { "type": "integer", "description": "Numéro de la pull request" },
                    "expected_head_sha": { "type": "string",  "description": "SHA attendu de la tête de la PR (optionnel)" }
                },
                "required": ["owner", "repo", "pull_number"]
            }
        }),
        json!({
            "name": "github_get_pull_request_comments",
            "description": "GITHUB — Récupère les commentaires de révision de code (inline) d'une pull request GitHub.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "owner":       { "type": "string",  "description": "Propriétaire du dépôt" },
                    "repo":        { "type": "string",  "description": "Nom du dépôt" },
                    "pull_number": { "type": "integer", "description": "Numéro de la pull request" }
                },
                "required": ["owner", "repo", "pull_number"]
            }
        }),
        json!({
            "name": "github_get_pull_request_reviews",
            "description": "GITHUB — Récupère les revues (reviews) d'une pull request GitHub : approbations, demandes de modifications, commentaires de revue.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "owner":       { "type": "string",  "description": "Propriétaire du dépôt" },
                    "repo":        { "type": "string",  "description": "Nom du dépôt" },
                    "pull_number": { "type": "integer", "description": "Numéro de la pull request" }
                },
                "required": ["owner", "repo", "pull_number"]
            }
        }),
        json!({
            "name": "github_create_pull_request_review",
            "description": "GITHUB — Soumet une revue sur une pull request GitHub. Événements possibles : APPROVE, REQUEST_CHANGES, COMMENT.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "owner":       { "type": "string", "description": "Propriétaire du dépôt" },
                    "repo":        { "type": "string", "description": "Nom du dépôt" },
                    "pull_number": { "type": "integer", "description": "Numéro de la pull request" },
                    "body":        { "type": "string", "description": "Corps du commentaire de revue" },
                    "event":       { "type": "string", "description": "Action : APPROVE, REQUEST_CHANGES ou COMMENT", "enum": ["APPROVE", "REQUEST_CHANGES", "COMMENT"] },
                    "comments":    {
                        "type": "array",
                        "description": "Commentaires inline (optionnel)",
                        "items": {
                            "type": "object",
                            "properties": {
                                "path":     { "type": "string",  "description": "Chemin du fichier" },
                                "position": { "type": "integer", "description": "Position dans le diff" },
                                "body":     { "type": "string",  "description": "Contenu du commentaire" }
                            }
                        }
                    }
                },
                "required": ["owner", "repo", "pull_number", "body", "event"]
            }
        }),
        // ── Repository operations ─────────────────────────────────────────────
        json!({
            "name": "github_fork_repository",
            "description": "GITHUB — Forke un dépôt GitHub dans le compte de l'utilisateur ou dans une organisation. Retourne l'URL du fork créé.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "owner":        { "type": "string", "description": "Propriétaire du dépôt source" },
                    "repo":         { "type": "string", "description": "Nom du dépôt source" },
                    "organization": { "type": "string", "description": "Organisation dans laquelle forker (optionnel — défaut: compte personnel)" }
                },
                "required": ["owner", "repo"]
            }
        }),
        json!({
            "name": "github_create_branch",
            "description": "GITHUB — Crée une nouvelle branche dans un dépôt GitHub à partir d'une branche existante (défaut: branche par défaut).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "owner":       { "type": "string", "description": "Propriétaire du dépôt" },
                    "repo":        { "type": "string", "description": "Nom du dépôt" },
                    "branch":      { "type": "string", "description": "Nom de la nouvelle branche" },
                    "from_branch": { "type": "string", "description": "Branche source (optionnel — défaut: branche par défaut)" }
                },
                "required": ["owner", "repo", "branch"]
            }
        }),
        json!({
            "name": "github_list_commits",
            "description": "GITHUB — Liste les commits d'un dépôt GitHub. Filtrable par branche, fichier, auteur et période.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "owner":    { "type": "string",  "description": "Propriétaire du dépôt" },
                    "repo":     { "type": "string",  "description": "Nom du dépôt" },
                    "sha":      { "type": "string",  "description": "Branche, tag ou SHA (optionnel — défaut: branche par défaut)" },
                    "path":     { "type": "string",  "description": "Filtre par fichier/dossier (optionnel)" },
                    "author":   { "type": "string",  "description": "Filtre par login ou email de l'auteur (optionnel)" },
                    "since":    { "type": "string",  "description": "Date ISO 8601 minimum (optionnel)" },
                    "until":    { "type": "string",  "description": "Date ISO 8601 maximum (optionnel)" },
                    "per_page": { "type": "integer", "description": "Résultats par page (défaut: 30, max: 100)", "default": 30 },
                    "page":     { "type": "integer", "description": "Numéro de page (défaut: 1)", "default": 1 }
                },
                "required": ["owner", "repo"]
            }
        }),
        // ── Search ────────────────────────────────────────────────────────────
        json!({
            "name": "github_search_code",
            "description": "GITHUB — Recherche du code dans les dépôts GitHub avec la syntaxe de recherche avancée (ex: 'MyClass repo:owner/repo language:rust').",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query":    { "type": "string",  "description": "Requête de recherche de code (ex: 'fn main repo:owner/repo')" },
                    "sort":     { "type": "string",  "description": "Tri : indexed (seule option disponible)" },
                    "order":    { "type": "string",  "description": "Ordre : desc (défaut), asc", "enum": ["asc", "desc"] },
                    "per_page": { "type": "integer", "description": "Résultats par page (défaut: 30, max: 100)", "default": 30 },
                    "page":     { "type": "integer", "description": "Numéro de page (défaut: 1)", "default": 1 }
                },
                "required": ["query"]
            }
        }),
        json!({
            "name": "github_search_users",
            "description": "GITHUB — Recherche des utilisateurs ou organisations GitHub (login, nom, email, localisation).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query":    { "type": "string",  "description": "Requête de recherche (ex: 'john location:Paris')" },
                    "sort":     { "type": "string",  "description": "Tri : followers, repositories, joined" },
                    "order":    { "type": "string",  "description": "Ordre : desc (défaut), asc", "enum": ["asc", "desc"] },
                    "per_page": { "type": "integer", "description": "Résultats par page (défaut: 30, max: 100)", "default": 30 },
                    "page":     { "type": "integer", "description": "Numéro de page (défaut: 1)", "default": 1 }
                },
                "required": ["query"]
            }
        }),
    ]
}

// ─── Handler ──────────────────────────────────────────────────────────────────

pub async fn handle(name: &str, args: &Value) -> Result<String, String> {
    let cfg = GitHubConfig::load()
        .ok_or_else(|| "GitHub non configuré — créer ~/.osmozzz/github.toml avec token".to_string())?;

    match name {
        // ── Repositories ─────────────────────────────────────────────────────

        "github_search_repositories" => {
            let query    = args["query"].as_str().ok_or("Paramètre 'query' requis")?;
            let page     = args["page"].as_u64().unwrap_or(1);
            let per_page = args["per_page"].as_u64().unwrap_or(30).min(100);

            let url  = cfg.api(&format!(
                "search/repositories?q={}&page={page}&per_page={per_page}",
                urlencoding_simple(query)
            ));
            let resp = get(&cfg, &url).await?;

            let total = resp["total_count"].as_u64().unwrap_or(0);
            let items = resp["items"].as_array().cloned().unwrap_or_default();
            if items.is_empty() {
                return Ok(format!("Aucun dépôt trouvé pour la recherche : {query}"));
            }

            let mut out = format!("{total} résultat(s) — page {page} :\n");
            for r in &items {
                let full_name    = r["full_name"].as_str().unwrap_or("—");
                let description  = r["description"].as_str().unwrap_or("");
                let stars        = r["stargazers_count"].as_u64().unwrap_or(0);
                let language     = r["language"].as_str().unwrap_or("—");
                let html_url     = r["html_url"].as_str().unwrap_or("—");
                let private      = r["private"].as_bool().unwrap_or(false);
                let visibility   = if private { "privé" } else { "public" };
                if description.is_empty() {
                    out.push_str(&format!("• {full_name} ({visibility}) — {language} — ★{stars}\n  {html_url}\n"));
                } else {
                    let desc_preview = if description.len() > 100 { &description[..100] } else { description };
                    out.push_str(&format!("• {full_name} ({visibility}) — {language} — ★{stars}\n  {desc_preview}\n  {html_url}\n"));
                }
            }
            Ok(out.trim_end().to_string())
        }

        "github_create_repository" => {
            let repo_name   = args["name"].as_str().ok_or("Paramètre 'name' requis")?;
            let mut body    = json!({ "name": repo_name });

            if let Some(desc) = args["description"].as_str() { body["description"] = json!(desc); }
            if let Some(priv_) = args["private"].as_bool()   { body["private"]     = json!(priv_); }
            if let Some(init)  = args["auto_init"].as_bool() { body["auto_init"]   = json!(init); }

            let url  = cfg.api("user/repos");
            let resp = post_json(&cfg, &url, &body).await?;

            let id        = resp["id"].as_u64().unwrap_or(0);
            let full_name = resp["full_name"].as_str().unwrap_or("—");
            let html_url  = resp["html_url"].as_str().unwrap_or("—");
            let private   = resp["private"].as_bool().unwrap_or(false);
            Ok(format!(
                "Dépôt créé.\nID         : {id}\nNom complet: {full_name}\nVisibilité : {}\nURL        : {html_url}",
                if private { "privé" } else { "public" }
            ))
        }

        // ── Files ─────────────────────────────────────────────────────────────

        "github_get_file_contents" => {
            let owner = args["owner"].as_str().ok_or("Paramètre 'owner' requis")?;
            let repo  = args["repo"].as_str().ok_or("Paramètre 'repo' requis")?;
            let path  = args["path"].as_str().ok_or("Paramètre 'path' requis")?;

            let mut url = cfg.api(&format!("repos/{owner}/{repo}/contents/{}", path.trim_start_matches('/')));
            if let Some(branch) = args["branch"].as_str() {
                url.push_str(&format!("?ref={branch}"));
            }
            let resp = get(&cfg, &url).await?;

            // Si c'est un tableau, c'est un dossier
            if let Some(items) = resp.as_array() {
                let mut out = format!("Contenu du dossier {path} ({} entrées) :\n", items.len());
                for item in items {
                    let iname = item["name"].as_str().unwrap_or("—");
                    let itype = item["type"].as_str().unwrap_or("—");
                    let isize = item["size"].as_u64().unwrap_or(0);
                    out.push_str(&format!("• [{itype}] {iname} ({isize} octets)\n"));
                }
                return Ok(out.trim_end().to_string());
            }

            // Fichier individuel
            let fname   = resp["name"].as_str().unwrap_or(path);
            let fsize   = resp["size"].as_u64().unwrap_or(0);
            let sha     = resp["sha"].as_str().unwrap_or("—");
            let content_b64 = resp["content"].as_str().unwrap_or("");
            // Décoder base64 (GitHub insère des \n dans le base64)
            let clean_b64: String = content_b64.chars().filter(|c| !c.is_whitespace()).collect();
            let decoded = base64_decode(&clean_b64);

            Ok(format!(
                "Fichier : {fname}\nSHA     : {sha}\nTaille  : {fsize} octets\n\n---\n{decoded}"
            ))
        }

        "github_create_or_update_file" => {
            let owner          = args["owner"].as_str().ok_or("Paramètre 'owner' requis")?;
            let repo           = args["repo"].as_str().ok_or("Paramètre 'repo' requis")?;
            let path           = args["path"].as_str().ok_or("Paramètre 'path' requis")?;
            let message        = args["message"].as_str().ok_or("Paramètre 'message' requis")?;
            let content_base64 = args["content_base64"].as_str().ok_or("Paramètre 'content_base64' requis")?;

            let mut body = json!({
                "message": message,
                "content": content_base64
            });
            if let Some(sha)    = args["sha"].as_str()    { body["sha"]    = json!(sha); }
            if let Some(branch) = args["branch"].as_str() { body["branch"] = json!(branch); }

            let url  = cfg.api(&format!("repos/{owner}/{repo}/contents/{}", path.trim_start_matches('/')));
            let resp = put_json(&cfg, &url, &body).await?;

            let commit_sha = resp["commit"]["sha"].as_str().unwrap_or("—");
            let html_url   = resp["content"]["html_url"].as_str().unwrap_or("—");
            let action     = if args["sha"].is_null() { "créé" } else { "mis à jour" };
            Ok(format!(
                "Fichier {action}.\nChemin     : {path}\nCommit SHA : {commit_sha}\nURL        : {html_url}"
            ))
        }

        "github_push_files" => {
            let owner   = args["owner"].as_str().ok_or("Paramètre 'owner' requis")?;
            let repo    = args["repo"].as_str().ok_or("Paramètre 'repo' requis")?;
            let branch  = args["branch"].as_str().ok_or("Paramètre 'branch' requis")?;
            let message = args["message"].as_str().ok_or("Paramètre 'message' requis")?;
            let files   = args["files"].as_array().ok_or("Paramètre 'files' requis (tableau)")?;

            if files.is_empty() {
                return Err("La liste 'files' est vide".to_string());
            }

            // 1. Récupérer le SHA de la ref de branche
            let ref_url  = cfg.api(&format!("repos/{owner}/{repo}/git/ref/heads/{branch}"));
            let ref_resp = get(&cfg, &ref_url).await?;
            let branch_sha = ref_resp["object"]["sha"].as_str()
                .ok_or_else(|| format!("Impossible de récupérer le SHA de la branche '{branch}'"))?
                .to_string();

            // 2. Récupérer le SHA du tree du dernier commit
            let commit_url  = cfg.api(&format!("repos/{owner}/{repo}/git/commits/{branch_sha}"));
            let commit_resp = get(&cfg, &commit_url).await?;
            let base_tree_sha = commit_resp["tree"]["sha"].as_str()
                .ok_or("Impossible de récupérer le SHA du tree de base")?
                .to_string();

            // 3. Créer le nouveau tree
            let tree_items: Vec<Value> = files.iter().map(|f| {
                let fpath    = f["path"].as_str().unwrap_or("");
                let fcontent = f["content"].as_str().unwrap_or("");
                json!({
                    "path":    fpath,
                    "mode":    "100644",
                    "type":    "blob",
                    "content": fcontent
                })
            }).collect();

            let tree_body = json!({
                "base_tree": base_tree_sha,
                "tree": tree_items
            });
            let tree_url  = cfg.api(&format!("repos/{owner}/{repo}/git/trees"));
            let tree_resp = post_json(&cfg, &tree_url, &tree_body).await?;
            let new_tree_sha = tree_resp["sha"].as_str()
                .ok_or("Impossible de récupérer le SHA du nouveau tree")?
                .to_string();

            // 4. Créer le commit
            let commit_body = json!({
                "message": message,
                "tree":    new_tree_sha,
                "parents": [branch_sha]
            });
            let new_commit_url  = cfg.api(&format!("repos/{owner}/{repo}/git/commits"));
            let new_commit_resp = post_json(&cfg, &new_commit_url, &commit_body).await?;
            let new_commit_sha = new_commit_resp["sha"].as_str()
                .ok_or("Impossible de récupérer le SHA du nouveau commit")?
                .to_string();

            // 5. Mettre à jour la référence de branche
            let update_ref_url  = cfg.api(&format!("repos/{owner}/{repo}/git/refs/heads/{branch}"));
            let update_ref_body = json!({ "sha": new_commit_sha });
            patch_json(&cfg, &update_ref_url, &update_ref_body).await?;

            Ok(format!(
                "{} fichier(s) poussé(s) sur '{branch}'.\nCommit SHA : {new_commit_sha}\nMessage    : {message}",
                files.len()
            ))
        }

        // ── Issues ────────────────────────────────────────────────────────────

        "github_create_issue" => {
            let owner = args["owner"].as_str().ok_or("Paramètre 'owner' requis")?;
            let repo  = args["repo"].as_str().ok_or("Paramètre 'repo' requis")?;
            let title = args["title"].as_str().ok_or("Paramètre 'title' requis")?;

            let mut body = json!({ "title": title });
            if let Some(b)  = args["body"].as_str()           { body["body"]      = json!(b); }
            if let Some(a)  = args["assignees"].as_array()    { body["assignees"] = json!(a); }
            if let Some(l)  = args["labels"].as_array()       { body["labels"]    = json!(l); }
            if let Some(m)  = args["milestone"].as_u64()      { body["milestone"] = json!(m); }

            let url  = cfg.api(&format!("repos/{owner}/{repo}/issues"));
            let resp = post_json(&cfg, &url, &body).await?;

            let number   = resp["number"].as_u64().unwrap_or(0);
            let html_url = resp["html_url"].as_str().unwrap_or("—");
            let state    = resp["state"].as_str().unwrap_or("—");
            Ok(format!(
                "Issue créée.\nNuméro : #{number}\nTitre  : {title}\nÉtat   : {state}\nURL    : {html_url}"
            ))
        }

        "github_list_issues" => {
            let owner    = args["owner"].as_str().ok_or("Paramètre 'owner' requis")?;
            let repo     = args["repo"].as_str().ok_or("Paramètre 'repo' requis")?;
            let state    = args["state"].as_str().unwrap_or("open");
            let per_page = args["per_page"].as_u64().unwrap_or(30).min(100);
            let page     = args["page"].as_u64().unwrap_or(1);

            let mut url = cfg.api(&format!(
                "repos/{owner}/{repo}/issues?state={state}&per_page={per_page}&page={page}"
            ));
            if let Some(labels) = args["labels"].as_str()    { url.push_str(&format!("&labels={labels}")); }
            if let Some(sort)   = args["sort"].as_str()      { url.push_str(&format!("&sort={sort}")); }
            if let Some(dir)    = args["direction"].as_str() { url.push_str(&format!("&direction={dir}")); }
            if let Some(since)  = args["since"].as_str()     { url.push_str(&format!("&since={since}")); }

            let resp   = get(&cfg, &url).await?;
            let issues = resp.as_array().cloned().unwrap_or_default();

            if issues.is_empty() {
                return Ok(format!("Aucune issue ({state}) dans {owner}/{repo}."));
            }

            let mut out = format!("{} issue(s) ({state}) dans {owner}/{repo} — page {page} :\n", issues.len());
            for issue in &issues {
                let number    = issue["number"].as_u64().unwrap_or(0);
                let ititle    = issue["title"].as_str().unwrap_or("—");
                let istate    = issue["state"].as_str().unwrap_or("—");
                let author    = issue["user"]["login"].as_str().unwrap_or("—");
                let created   = issue["created_at"].as_str().unwrap_or("—");
                let comments  = issue["comments"].as_u64().unwrap_or(0);
                // Ignorer les PR dans la liste des issues
                if !issue["pull_request"].is_null() { continue; }
                let labels: Vec<&str> = issue["labels"].as_array()
                    .map(|a| a.iter().filter_map(|l| l["name"].as_str()).collect())
                    .unwrap_or_default();
                let label_str = if labels.is_empty() { String::new() } else { format!(" [{}]", labels.join(", ")) };
                out.push_str(&format!("• #{number} ({istate}){label_str} — {ititle}\n  par {author} le {created} — {comments} commentaire(s)\n"));
            }
            Ok(out.trim_end().to_string())
        }

        "github_update_issue" => {
            let owner        = args["owner"].as_str().ok_or("Paramètre 'owner' requis")?;
            let repo         = args["repo"].as_str().ok_or("Paramètre 'repo' requis")?;
            let issue_number = args["issue_number"].as_u64().ok_or("Paramètre 'issue_number' requis")?;

            let mut body = json!({});
            if let Some(t) = args["title"].as_str()        { body["title"]     = json!(t); }
            if let Some(b) = args["body"].as_str()         { body["body"]      = json!(b); }
            if let Some(s) = args["state"].as_str()        { body["state"]     = json!(s); }
            if let Some(l) = args["labels"].as_array()     { body["labels"]    = json!(l); }
            if let Some(a) = args["assignees"].as_array()  { body["assignees"] = json!(a); }
            if let Some(m) = args["milestone"].as_u64()    { body["milestone"] = json!(m); }

            if body.as_object().map(|o| o.is_empty()).unwrap_or(true) {
                return Err("Au moins un paramètre de mise à jour est requis".to_string());
            }

            let url  = cfg.api(&format!("repos/{owner}/{repo}/issues/{issue_number}"));
            let resp = patch_json(&cfg, &url, &body).await?;

            let ititle   = resp["title"].as_str().unwrap_or("—");
            let istate   = resp["state"].as_str().unwrap_or("—");
            let html_url = resp["html_url"].as_str().unwrap_or("—");
            Ok(format!(
                "Issue #{issue_number} mise à jour.\nTitre : {ititle}\nÉtat  : {istate}\nURL   : {html_url}"
            ))
        }

        "github_add_issue_comment" => {
            let owner        = args["owner"].as_str().ok_or("Paramètre 'owner' requis")?;
            let repo         = args["repo"].as_str().ok_or("Paramètre 'repo' requis")?;
            let issue_number = args["issue_number"].as_u64().ok_or("Paramètre 'issue_number' requis")?;
            let body_text    = args["body"].as_str().ok_or("Paramètre 'body' requis")?;

            let body = json!({ "body": body_text });
            let url  = cfg.api(&format!("repos/{owner}/{repo}/issues/{issue_number}/comments"));
            let resp = post_json(&cfg, &url, &body).await?;

            let id       = resp["id"].as_u64().unwrap_or(0);
            let html_url = resp["html_url"].as_str().unwrap_or("—");
            let author   = resp["user"]["login"].as_str().unwrap_or("—");
            Ok(format!(
                "Commentaire ajouté à #{issue_number}.\nID     : {id}\nAuteur : {author}\nURL    : {html_url}"
            ))
        }

        "github_get_issue" => {
            let owner        = args["owner"].as_str().ok_or("Paramètre 'owner' requis")?;
            let repo         = args["repo"].as_str().ok_or("Paramètre 'repo' requis")?;
            let issue_number = args["issue_number"].as_u64().ok_or("Paramètre 'issue_number' requis")?;

            let url  = cfg.api(&format!("repos/{owner}/{repo}/issues/{issue_number}"));
            let resp = get(&cfg, &url).await?;

            let ititle    = resp["title"].as_str().unwrap_or("—");
            let istate    = resp["state"].as_str().unwrap_or("—");
            let author    = resp["user"]["login"].as_str().unwrap_or("—");
            let created   = resp["created_at"].as_str().unwrap_or("—");
            let updated   = resp["updated_at"].as_str().unwrap_or("—");
            let comments  = resp["comments"].as_u64().unwrap_or(0);
            let html_url  = resp["html_url"].as_str().unwrap_or("—");
            let body_text = resp["body"].as_str().unwrap_or("(vide)");

            let labels: Vec<&str> = resp["labels"].as_array()
                .map(|a| a.iter().filter_map(|l| l["name"].as_str()).collect())
                .unwrap_or_default();
            let assignees: Vec<&str> = resp["assignees"].as_array()
                .map(|a| a.iter().filter_map(|u| u["login"].as_str()).collect())
                .unwrap_or_default();
            let milestone = resp["milestone"]["title"].as_str().unwrap_or("aucun");

            let body_preview = if body_text.len() > 500 { &body_text[..500] } else { body_text };

            Ok(format!(
                "Issue #{issue_number} — {owner}/{repo}\nTitre      : {ititle}\nÉtat       : {istate}\nAuteur     : {author}\nCréée le   : {created}\nMise à jour: {updated}\nLabels     : {labels}\nAssignés   : {assignees}\nMilestone  : {milestone}\nCommentaires: {comments}\nURL        : {html_url}\n\n{body_preview}",
                labels    = if labels.is_empty()    { "aucun".to_string()    } else { labels.join(", ")    },
                assignees = if assignees.is_empty() { "aucun".to_string()    } else { assignees.join(", ") },
            ))
        }

        "github_search_issues" => {
            let query    = args["query"].as_str().ok_or("Paramètre 'query' requis")?;
            let per_page = args["per_page"].as_u64().unwrap_or(30).min(100);
            let page     = args["page"].as_u64().unwrap_or(1);

            let mut url = cfg.api(&format!(
                "search/issues?q={}&per_page={per_page}&page={page}",
                urlencoding_simple(query)
            ));
            if let Some(sort)  = args["sort"].as_str()  { url.push_str(&format!("&sort={sort}")); }
            if let Some(order) = args["order"].as_str() { url.push_str(&format!("&order={order}")); }

            let resp  = get(&cfg, &url).await?;
            let total = resp["total_count"].as_u64().unwrap_or(0);
            let items = resp["items"].as_array().cloned().unwrap_or_default();

            if items.is_empty() {
                return Ok(format!("Aucun résultat pour : {query}"));
            }

            let mut out = format!("{total} résultat(s) — page {page} :\n");
            for item in &items {
                let number   = item["number"].as_u64().unwrap_or(0);
                let ititle   = item["title"].as_str().unwrap_or("—");
                let istate   = item["state"].as_str().unwrap_or("—");
                let repo_url = item["repository_url"].as_str().unwrap_or("—");
                let is_pr    = !item["pull_request"].is_null();
                let kind     = if is_pr { "PR" } else { "Issue" };
                let author   = item["user"]["login"].as_str().unwrap_or("—");
                // Extraire owner/repo depuis repository_url
                let repo_name = repo_url.rsplit('/').take(2).collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>().join("/");
                out.push_str(&format!("• [{kind}] {repo_name}#{number} ({istate}) — {ititle} — par {author}\n"));
            }
            Ok(out.trim_end().to_string())
        }

        // ── Pull Requests ─────────────────────────────────────────────────────

        "github_create_pull_request" => {
            let owner = args["owner"].as_str().ok_or("Paramètre 'owner' requis")?;
            let repo  = args["repo"].as_str().ok_or("Paramètre 'repo' requis")?;
            let title = args["title"].as_str().ok_or("Paramètre 'title' requis")?;
            let head  = args["head"].as_str().ok_or("Paramètre 'head' requis")?;
            let base  = args["base"].as_str().ok_or("Paramètre 'base' requis")?;

            let mut body = json!({ "title": title, "head": head, "base": base });
            if let Some(b) = args["body"].as_str()   { body["body"]  = json!(b); }
            if let Some(d) = args["draft"].as_bool() { body["draft"] = json!(d); }

            let url  = cfg.api(&format!("repos/{owner}/{repo}/pulls"));
            let resp = post_json(&cfg, &url, &body).await?;

            let number   = resp["number"].as_u64().unwrap_or(0);
            let html_url = resp["html_url"].as_str().unwrap_or("—");
            let state    = resp["state"].as_str().unwrap_or("—");
            let draft    = resp["draft"].as_bool().unwrap_or(false);
            Ok(format!(
                "Pull Request créée.\nNuméro  : #{number}\nTitre   : {title}\nÉtat    : {state}{draft_info}\n{head} → {base}\nURL     : {html_url}",
                draft_info = if draft { " (brouillon)" } else { "" }
            ))
        }

        "github_get_pull_request" => {
            let owner       = args["owner"].as_str().ok_or("Paramètre 'owner' requis")?;
            let repo        = args["repo"].as_str().ok_or("Paramètre 'repo' requis")?;
            let pull_number = args["pull_number"].as_u64().ok_or("Paramètre 'pull_number' requis")?;

            let url  = cfg.api(&format!("repos/{owner}/{repo}/pulls/{pull_number}"));
            let resp = get(&cfg, &url).await?;

            let ptitle    = resp["title"].as_str().unwrap_or("—");
            let pstate    = resp["state"].as_str().unwrap_or("—");
            let author    = resp["user"]["login"].as_str().unwrap_or("—");
            let created   = resp["created_at"].as_str().unwrap_or("—");
            let updated   = resp["updated_at"].as_str().unwrap_or("—");
            let head_ref  = resp["head"]["ref"].as_str().unwrap_or("—");
            let base_ref  = resp["base"]["ref"].as_str().unwrap_or("—");
            let html_url  = resp["html_url"].as_str().unwrap_or("—");
            let merged    = resp["merged"].as_bool().unwrap_or(false);
            let draft     = resp["draft"].as_bool().unwrap_or(false);
            let commits   = resp["commits"].as_u64().unwrap_or(0);
            let additions = resp["additions"].as_u64().unwrap_or(0);
            let deletions = resp["deletions"].as_u64().unwrap_or(0);
            let changed   = resp["changed_files"].as_u64().unwrap_or(0);

            let reviewers: Vec<&str> = resp["requested_reviewers"].as_array()
                .map(|a| a.iter().filter_map(|u| u["login"].as_str()).collect())
                .unwrap_or_default();

            Ok(format!(
                "PR #{pull_number} — {owner}/{repo}\nTitre      : {ptitle}\nÉtat       : {pstate}{merged_info}{draft_info}\nAuteur     : {author}\nCréée le   : {created}\nMise à jour: {updated}\nBranches   : {head_ref} → {base_ref}\nCommits    : {commits}\nChangements: +{additions}/-{deletions} sur {changed} fichier(s)\nReviewers  : {reviewers}\nURL        : {html_url}",
                merged_info  = if merged { " (fusionnée)" } else { "" },
                draft_info   = if draft  { " (brouillon)" } else { "" },
                reviewers    = if reviewers.is_empty() { "aucun".to_string() } else { reviewers.join(", ") },
            ))
        }

        "github_list_pull_requests" => {
            let owner    = args["owner"].as_str().ok_or("Paramètre 'owner' requis")?;
            let repo     = args["repo"].as_str().ok_or("Paramètre 'repo' requis")?;
            let state    = args["state"].as_str().unwrap_or("open");
            let per_page = args["per_page"].as_u64().unwrap_or(30).min(100);
            let page     = args["page"].as_u64().unwrap_or(1);

            let mut url = cfg.api(&format!(
                "repos/{owner}/{repo}/pulls?state={state}&per_page={per_page}&page={page}"
            ));
            if let Some(head) = args["head"].as_str()      { url.push_str(&format!("&head={head}")); }
            if let Some(base) = args["base"].as_str()      { url.push_str(&format!("&base={base}")); }
            if let Some(sort) = args["sort"].as_str()      { url.push_str(&format!("&sort={sort}")); }
            if let Some(dir)  = args["direction"].as_str() { url.push_str(&format!("&direction={dir}")); }

            let resp = get(&cfg, &url).await?;
            let prs  = resp.as_array().cloned().unwrap_or_default();

            if prs.is_empty() {
                return Ok(format!("Aucune PR ({state}) dans {owner}/{repo}."));
            }

            let mut out = format!("{} PR ({state}) dans {owner}/{repo} — page {page} :\n", prs.len());
            for pr in &prs {
                let number   = pr["number"].as_u64().unwrap_or(0);
                let ptitle   = pr["title"].as_str().unwrap_or("—");
                let pstate   = pr["state"].as_str().unwrap_or("—");
                let author   = pr["user"]["login"].as_str().unwrap_or("—");
                let head_ref = pr["head"]["ref"].as_str().unwrap_or("—");
                let base_ref = pr["base"]["ref"].as_str().unwrap_or("—");
                let draft    = pr["draft"].as_bool().unwrap_or(false);
                let draft_s  = if draft { " [brouillon]" } else { "" };
                out.push_str(&format!("• #{number} ({pstate}){draft_s} — {ptitle}\n  par {author} — {head_ref} → {base_ref}\n"));
            }
            Ok(out.trim_end().to_string())
        }

        "github_merge_pull_request" => {
            let owner       = args["owner"].as_str().ok_or("Paramètre 'owner' requis")?;
            let repo        = args["repo"].as_str().ok_or("Paramètre 'repo' requis")?;
            let pull_number = args["pull_number"].as_u64().ok_or("Paramètre 'pull_number' requis")?;
            let method      = args["merge_method"].as_str().unwrap_or("merge");

            let mut body = json!({ "merge_method": method });
            if let Some(t) = args["commit_title"].as_str()   { body["commit_title"]   = json!(t); }
            if let Some(m) = args["commit_message"].as_str() { body["commit_message"] = json!(m); }

            let url  = cfg.api(&format!("repos/{owner}/{repo}/pulls/{pull_number}/merge"));
            let resp = put_json(&cfg, &url, &body).await?;

            let sha     = resp["sha"].as_str().unwrap_or("—");
            let msg     = resp["message"].as_str().unwrap_or("—");
            let merged  = resp["merged"].as_bool().unwrap_or(false);
            if !merged {
                return Err(format!("Fusion échouée : {msg}"));
            }
            Ok(format!(
                "PR #{pull_number} fusionnée ({method}).\nCommit SHA : {sha}\nMessage    : {msg}"
            ))
        }

        "github_get_pull_request_files" => {
            let owner       = args["owner"].as_str().ok_or("Paramètre 'owner' requis")?;
            let repo        = args["repo"].as_str().ok_or("Paramètre 'repo' requis")?;
            let pull_number = args["pull_number"].as_u64().ok_or("Paramètre 'pull_number' requis")?;

            let url   = cfg.api(&format!("repos/{owner}/{repo}/pulls/{pull_number}/files"));
            let resp  = get(&cfg, &url).await?;
            let files = resp.as_array().cloned().unwrap_or_default();

            if files.is_empty() {
                return Ok(format!("Aucun fichier modifié dans la PR #{pull_number}."));
            }

            let mut out = format!("{} fichier(s) modifié(s) dans la PR #{pull_number} :\n", files.len());
            for f in &files {
                let fname     = f["filename"].as_str().unwrap_or("—");
                let fstatus   = f["status"].as_str().unwrap_or("—");
                let additions = f["additions"].as_u64().unwrap_or(0);
                let deletions = f["deletions"].as_u64().unwrap_or(0);
                out.push_str(&format!("• [{fstatus}] {fname} (+{additions}/-{deletions})\n"));
            }
            Ok(out.trim_end().to_string())
        }

        "github_get_pull_request_status" => {
            let owner       = args["owner"].as_str().ok_or("Paramètre 'owner' requis")?;
            let repo        = args["repo"].as_str().ok_or("Paramètre 'repo' requis")?;
            let pull_number = args["pull_number"].as_u64().ok_or("Paramètre 'pull_number' requis")?;

            // Récupérer d'abord le SHA de la tête de la PR
            let pr_url  = cfg.api(&format!("repos/{owner}/{repo}/pulls/{pull_number}"));
            let pr_resp = get(&cfg, &pr_url).await?;
            let head_sha = pr_resp["head"]["sha"].as_str()
                .ok_or("Impossible de récupérer le SHA de la tête de la PR")?
                .to_string();

            // Récupérer le statut combiné
            let status_url  = cfg.api(&format!("repos/{owner}/{repo}/commits/{head_sha}/status"));
            let status_resp = get(&cfg, &status_url).await?;

            let state      = status_resp["state"].as_str().unwrap_or("unknown");
            let total      = status_resp["total_count"].as_u64().unwrap_or(0);
            let statuses   = status_resp["statuses"].as_array().cloned().unwrap_or_default();
            let commit_sha = status_resp["sha"].as_str().unwrap_or(&head_sha);

            let mut out = format!(
                "Statut CI/CD de la PR #{pull_number}\nCommit SHA : {commit_sha}\nÉtat global: {state} ({total} contexte(s))\n"
            );
            for s in &statuses {
                let sstate   = s["state"].as_str().unwrap_or("—");
                let scontext = s["context"].as_str().unwrap_or("—");
                let sdesc    = s["description"].as_str().unwrap_or("");
                out.push_str(&format!("• [{sstate}] {scontext}{}\n",
                    if sdesc.is_empty() { String::new() } else { format!(" — {sdesc}") }
                ));
            }
            Ok(out.trim_end().to_string())
        }

        "github_update_pull_request_branch" => {
            let owner       = args["owner"].as_str().ok_or("Paramètre 'owner' requis")?;
            let repo        = args["repo"].as_str().ok_or("Paramètre 'repo' requis")?;
            let pull_number = args["pull_number"].as_u64().ok_or("Paramètre 'pull_number' requis")?;

            let mut body = json!({});
            if let Some(sha) = args["expected_head_sha"].as_str() { body["expected_head_sha"] = json!(sha); }

            let url  = cfg.api(&format!("repos/{owner}/{repo}/pulls/{pull_number}/update-branch"));
            let resp = post_json_raw(&cfg, &url, &body).await?;
            let status = resp.status().as_u16();
            let resp_json: Value = resp.json().await.unwrap_or(json!({}));

            if status == 202 {
                let message = resp_json["message"].as_str().unwrap_or("Branche mise à jour.");
                Ok(format!("PR #{pull_number} — branche mise à jour.\n{message}"))
            } else {
                let message = resp_json["message"].as_str().unwrap_or("Erreur inconnue");
                Err(format!("Échec de la mise à jour de la branche (HTTP {status}) : {message}"))
            }
        }

        "github_get_pull_request_comments" => {
            let owner       = args["owner"].as_str().ok_or("Paramètre 'owner' requis")?;
            let repo        = args["repo"].as_str().ok_or("Paramètre 'repo' requis")?;
            let pull_number = args["pull_number"].as_u64().ok_or("Paramètre 'pull_number' requis")?;

            let url      = cfg.api(&format!("repos/{owner}/{repo}/pulls/{pull_number}/comments"));
            let resp     = get(&cfg, &url).await?;
            let comments = resp.as_array().cloned().unwrap_or_default();

            if comments.is_empty() {
                return Ok(format!("Aucun commentaire de révision dans la PR #{pull_number}."));
            }

            let mut out = format!("{} commentaire(s) de révision dans la PR #{pull_number} :\n", comments.len());
            for c in &comments {
                let id       = c["id"].as_u64().unwrap_or(0);
                let author   = c["user"]["login"].as_str().unwrap_or("—");
                let path     = c["path"].as_str().unwrap_or("—");
                let body     = c["body"].as_str().unwrap_or("(vide)");
                let created  = c["created_at"].as_str().unwrap_or("—");
                let preview  = if body.len() > 150 { &body[..150] } else { body };
                out.push_str(&format!("• [{id}] {author} sur {path} ({created}) :\n  {preview}\n"));
            }
            Ok(out.trim_end().to_string())
        }

        "github_get_pull_request_reviews" => {
            let owner       = args["owner"].as_str().ok_or("Paramètre 'owner' requis")?;
            let repo        = args["repo"].as_str().ok_or("Paramètre 'repo' requis")?;
            let pull_number = args["pull_number"].as_u64().ok_or("Paramètre 'pull_number' requis")?;

            let url     = cfg.api(&format!("repos/{owner}/{repo}/pulls/{pull_number}/reviews"));
            let resp    = get(&cfg, &url).await?;
            let reviews = resp.as_array().cloned().unwrap_or_default();

            if reviews.is_empty() {
                return Ok(format!("Aucune revue pour la PR #{pull_number}."));
            }

            let mut out = format!("{} revue(s) pour la PR #{pull_number} :\n", reviews.len());
            for r in &reviews {
                let id       = r["id"].as_u64().unwrap_or(0);
                let author   = r["user"]["login"].as_str().unwrap_or("—");
                let state    = r["state"].as_str().unwrap_or("—");
                let body     = r["body"].as_str().unwrap_or("(sans commentaire)");
                let submitted = r["submitted_at"].as_str().unwrap_or("—");
                let preview  = if body.len() > 200 { &body[..200] } else { body };
                out.push_str(&format!("• [{id}] {author} — {state} ({submitted})\n  {preview}\n"));
            }
            Ok(out.trim_end().to_string())
        }

        "github_create_pull_request_review" => {
            let owner       = args["owner"].as_str().ok_or("Paramètre 'owner' requis")?;
            let repo        = args["repo"].as_str().ok_or("Paramètre 'repo' requis")?;
            let pull_number = args["pull_number"].as_u64().ok_or("Paramètre 'pull_number' requis")?;
            let body_text   = args["body"].as_str().ok_or("Paramètre 'body' requis")?;
            let event       = args["event"].as_str().ok_or("Paramètre 'event' requis (APPROVE, REQUEST_CHANGES, COMMENT)")?;

            let mut body = json!({ "body": body_text, "event": event });
            if let Some(comments) = args["comments"].as_array() {
                body["comments"] = json!(comments);
            }

            let url  = cfg.api(&format!("repos/{owner}/{repo}/pulls/{pull_number}/reviews"));
            let resp = post_json(&cfg, &url, &body).await?;

            let id       = resp["id"].as_u64().unwrap_or(0);
            let rstate   = resp["state"].as_str().unwrap_or("—");
            let author   = resp["user"]["login"].as_str().unwrap_or("—");
            let html_url = resp["html_url"].as_str().unwrap_or("—");
            Ok(format!(
                "Revue soumise sur PR #{pull_number}.\nID     : {id}\nÉtat   : {rstate}\nAuteur : {author}\nURL    : {html_url}"
            ))
        }

        // ── Repository operations ─────────────────────────────────────────────

        "github_fork_repository" => {
            let owner = args["owner"].as_str().ok_or("Paramètre 'owner' requis")?;
            let repo  = args["repo"].as_str().ok_or("Paramètre 'repo' requis")?;

            let mut body = json!({});
            if let Some(org) = args["organization"].as_str() { body["organization"] = json!(org); }

            let url  = cfg.api(&format!("repos/{owner}/{repo}/forks"));
            let resp = post_json(&cfg, &url, &body).await?;

            let full_name = resp["full_name"].as_str().unwrap_or("—");
            let html_url  = resp["html_url"].as_str().unwrap_or("—");
            let private   = resp["private"].as_bool().unwrap_or(false);
            Ok(format!(
                "Fork créé.\nNom complet: {full_name}\nVisibilité : {}\nURL        : {html_url}",
                if private { "privé" } else { "public" }
            ))
        }

        "github_create_branch" => {
            let owner  = args["owner"].as_str().ok_or("Paramètre 'owner' requis")?;
            let repo   = args["repo"].as_str().ok_or("Paramètre 'repo' requis")?;
            let branch = args["branch"].as_str().ok_or("Paramètre 'branch' requis")?;

            // Récupérer le SHA de la branche source
            let from_branch = args["from_branch"].as_str();
            let sha = if let Some(src) = from_branch {
                let ref_url  = cfg.api(&format!("repos/{owner}/{repo}/git/ref/heads/{src}"));
                let ref_resp = get(&cfg, &ref_url).await?;
                ref_resp["object"]["sha"].as_str()
                    .ok_or_else(|| format!("Impossible de récupérer le SHA de la branche '{src}'"))?
                    .to_string()
            } else {
                // Récupérer la branche par défaut
                let repo_url  = cfg.api(&format!("repos/{owner}/{repo}"));
                let repo_resp = get(&cfg, &repo_url).await?;
                let default_branch = repo_resp["default_branch"].as_str().unwrap_or("main").to_string();
                let ref_url  = cfg.api(&format!("repos/{owner}/{repo}/git/ref/heads/{default_branch}"));
                let ref_resp = get(&cfg, &ref_url).await?;
                ref_resp["object"]["sha"].as_str()
                    .ok_or("Impossible de récupérer le SHA de la branche par défaut")?
                    .to_string()
            };

            let body = json!({ "ref": format!("refs/heads/{branch}"), "sha": sha });
            let url  = cfg.api(&format!("repos/{owner}/{repo}/git/refs"));
            let resp = post_json(&cfg, &url, &body).await?;

            let created_ref = resp["ref"].as_str().unwrap_or("—");
            let created_sha = resp["object"]["sha"].as_str().unwrap_or("—");
            let source      = from_branch.unwrap_or("branche par défaut");
            Ok(format!(
                "Branche créée.\nNom    : {branch}\nDepuis : {source}\nRef    : {created_ref}\nSHA    : {created_sha}"
            ))
        }

        "github_list_commits" => {
            let owner    = args["owner"].as_str().ok_or("Paramètre 'owner' requis")?;
            let repo     = args["repo"].as_str().ok_or("Paramètre 'repo' requis")?;
            let per_page = args["per_page"].as_u64().unwrap_or(30).min(100);
            let page     = args["page"].as_u64().unwrap_or(1);

            let mut url = cfg.api(&format!(
                "repos/{owner}/{repo}/commits?per_page={per_page}&page={page}"
            ));
            if let Some(sha)    = args["sha"].as_str()    { url.push_str(&format!("&sha={sha}")); }
            if let Some(path)   = args["path"].as_str()   { url.push_str(&format!("&path={path}")); }
            if let Some(author) = args["author"].as_str() { url.push_str(&format!("&author={author}")); }
            if let Some(since)  = args["since"].as_str()  { url.push_str(&format!("&since={since}")); }
            if let Some(until)  = args["until"].as_str()  { url.push_str(&format!("&until={until}")); }

            let resp    = get(&cfg, &url).await?;
            let commits = resp.as_array().cloned().unwrap_or_default();

            if commits.is_empty() {
                return Ok(format!("Aucun commit trouvé dans {owner}/{repo}."));
            }

            let mut out = format!("{} commit(s) dans {owner}/{repo} — page {page} :\n", commits.len());
            for c in &commits {
                let sha     = c["sha"].as_str().unwrap_or("—");
                let short   = if sha.len() >= 7 { &sha[..7] } else { sha };
                let message = c["commit"]["message"].as_str().unwrap_or("—");
                let first_line = message.lines().next().unwrap_or("—");
                let author  = c["commit"]["author"]["name"].as_str().unwrap_or("—");
                let date    = c["commit"]["author"]["date"].as_str().unwrap_or("—");
                out.push_str(&format!("• [{short}] {first_line}\n  par {author} le {date}\n"));
            }
            Ok(out.trim_end().to_string())
        }

        // ── Search ────────────────────────────────────────────────────────────

        "github_search_code" => {
            let query    = args["query"].as_str().ok_or("Paramètre 'query' requis")?;
            let per_page = args["per_page"].as_u64().unwrap_or(30).min(100);
            let page     = args["page"].as_u64().unwrap_or(1);

            let mut url = cfg.api(&format!(
                "search/code?q={}&per_page={per_page}&page={page}",
                urlencoding_simple(query)
            ));
            if let Some(sort)  = args["sort"].as_str()  { url.push_str(&format!("&sort={sort}")); }
            if let Some(order) = args["order"].as_str() { url.push_str(&format!("&order={order}")); }

            let resp  = get(&cfg, &url).await?;
            let total = resp["total_count"].as_u64().unwrap_or(0);
            let items = resp["items"].as_array().cloned().unwrap_or_default();

            if items.is_empty() {
                return Ok(format!("Aucun résultat de code pour : {query}"));
            }

            let mut out = format!("{total} résultat(s) — page {page} :\n");
            for item in &items {
                let fname    = item["name"].as_str().unwrap_or("—");
                let fpath    = item["path"].as_str().unwrap_or("—");
                let repo     = item["repository"]["full_name"].as_str().unwrap_or("—");
                let html_url = item["html_url"].as_str().unwrap_or("—");
                out.push_str(&format!("• {repo} — {fpath}/{fname}\n  {html_url}\n"));
            }
            Ok(out.trim_end().to_string())
        }

        "github_search_users" => {
            let query    = args["query"].as_str().ok_or("Paramètre 'query' requis")?;
            let per_page = args["per_page"].as_u64().unwrap_or(30).min(100);
            let page     = args["page"].as_u64().unwrap_or(1);

            let mut url = cfg.api(&format!(
                "search/users?q={}&per_page={per_page}&page={page}",
                urlencoding_simple(query)
            ));
            if let Some(sort)  = args["sort"].as_str()  { url.push_str(&format!("&sort={sort}")); }
            if let Some(order) = args["order"].as_str() { url.push_str(&format!("&order={order}")); }

            let resp  = get(&cfg, &url).await?;
            let total = resp["total_count"].as_u64().unwrap_or(0);
            let items = resp["items"].as_array().cloned().unwrap_or_default();

            if items.is_empty() {
                return Ok(format!("Aucun utilisateur trouvé pour : {query}"));
            }

            let mut out = format!("{total} résultat(s) — page {page} :\n");
            for item in &items {
                let login    = item["login"].as_str().unwrap_or("—");
                let utype    = item["type"].as_str().unwrap_or("User");
                let html_url = item["html_url"].as_str().unwrap_or("—");
                out.push_str(&format!("• [{utype}] {login} — {html_url}\n"));
            }
            Ok(out.trim_end().to_string())
        }

        _ => Err(format!("Tool GitHub inconnu : {name}")),
    }
}

// ─── Helpers internes ─────────────────────────────────────────────────────────

/// Encode minimalement une chaîne pour une URL (espaces → %20, & → %26, etc.)
fn urlencoding_simple(s: &str) -> String {
    s.chars().map(|c| match c {
        ' '  => "%20".to_string(),
        '&'  => "%26".to_string(),
        '+'  => "%2B".to_string(),
        '#'  => "%23".to_string(),
        '?'  => "%3F".to_string(),
        '='  => "%3D".to_string(),
        '/'  => "%2F".to_string(),
        ':'  => "%3A".to_string(),
        '@'  => "%40".to_string(),
        '"'  => "%22".to_string(),
        _    => c.to_string(),
    }).collect()
}

/// Décode le base64 standard (avec padding) vers une String UTF-8.
/// En cas d'échec, retourne le contenu brut.
fn base64_decode(input: &str) -> String {
    // Implémentation manuelle légère — évite d'ajouter une dépendance.
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut table = [255u8; 256];
    for (i, &c) in CHARS.iter().enumerate() {
        table[c as usize] = i as u8;
    }

    let bytes = input.as_bytes();
    let mut out: Vec<u8> = Vec::with_capacity(bytes.len() * 3 / 4);
    let mut i = 0;
    while i + 3 < bytes.len() {
        let a = table[bytes[i] as usize];
        let b = table[bytes[i + 1] as usize];
        let c = table[bytes[i + 2] as usize];
        let d = table[bytes[i + 3] as usize];
        if a == 255 || b == 255 { break; }
        out.push((a << 2) | (b >> 4));
        if bytes[i + 2] != b'=' && c != 255 {
            out.push((b << 4) | (c >> 2));
            if bytes[i + 3] != b'=' && d != 255 {
                out.push((c << 6) | d);
            }
        }
        i += 4;
    }
    String::from_utf8(out).unwrap_or_else(|_| "(contenu binaire non affichable)".to_string())
}
