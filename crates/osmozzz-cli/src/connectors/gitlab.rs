/// Connecteur GitLab — API REST v4 officielle.
/// Remplace le subprocess npm @zereight/mcp-gitlab (dev solo, CVE patché).
/// Toutes les fonctions retournent Result<String, String> sans jamais appeler send().
/// La couche sécurité (sanitize_proxy_response + audit_log) est appliquée par le dispatcher (mcp.rs).
use serde_json::{json, Value};

// ─── Config ──────────────────────────────────────────────────────────────────

#[derive(serde::Deserialize)]
struct GitlabConfig {
    token:    String,
    #[serde(default = "default_base_url")]
    base_url: String,
    #[serde(default)]
    groups:   Vec<String>,
}

fn default_base_url() -> String { "https://gitlab.com".to_string() }

impl GitlabConfig {
    fn load() -> Option<Self> {
        let path = dirs_next::home_dir()?.join(".osmozzz/gitlab.toml");
        toml::from_str(&std::fs::read_to_string(path).ok()?).ok()
    }
    fn api(&self, path: &str) -> String {
        format!("{}/api/v4/{}", self.base_url.trim_end_matches('/'), path)
    }
}

// ─── HTTP helpers ─────────────────────────────────────────────────────────────

async fn get(token: &str, url: &str) -> Result<Value, String> {
    reqwest::Client::new().get(url)
        .header("PRIVATE-TOKEN", token)
        .header("Accept", "application/json")
        .send().await.map_err(|e| e.to_string())?
        .json::<Value>().await.map_err(|e| e.to_string())
}

async fn post(token: &str, url: &str, body: Value) -> Result<Value, String> {
    reqwest::Client::new().post(url)
        .header("PRIVATE-TOKEN", token)
        .header("Content-Type", "application/json")
        .json(&body)
        .send().await.map_err(|e| e.to_string())?
        .json::<Value>().await.map_err(|e| e.to_string())
}

async fn put(token: &str, url: &str, body: Value) -> Result<Value, String> {
    reqwest::Client::new().put(url)
        .header("PRIVATE-TOKEN", token)
        .header("Content-Type", "application/json")
        .json(&body)
        .send().await.map_err(|e| e.to_string())?
        .json::<Value>().await.map_err(|e| e.to_string())
}

async fn delete(token: &str, url: &str) -> Result<(), String> {
    reqwest::Client::new().delete(url)
        .header("PRIVATE-TOKEN", token)
        .send().await.map_err(|e| e.to_string())?;
    Ok(())
}

async fn get_text(token: &str, url: &str) -> Result<String, String> {
    reqwest::Client::new().get(url)
        .header("PRIVATE-TOKEN", token)
        .send().await.map_err(|e| e.to_string())?
        .text().await.map_err(|e| e.to_string())
}

// ─── Formatters ──────────────────────────────────────────────────────────────

fn fmt_issues(issues: &[Value]) -> String {
    if issues.is_empty() { return "Aucune issue trouvée.".to_string(); }
    issues.iter().map(|i| {
        let iid    = i["iid"].as_u64().unwrap_or(0);
        let title  = i["title"].as_str().unwrap_or("?");
        let state  = i["state"].as_str().unwrap_or("?");
        let asgn   = i["assignee"]["name"].as_str().unwrap_or("—");
        let labels = i["labels"].as_array()
            .map(|a| a.iter().filter_map(|l| l.as_str()).collect::<Vec<_>>().join(", "))
            .unwrap_or_default();
        let id = i["id"].as_u64().unwrap_or(0);
        let mut line = format!("[#{iid}] {title}\n  état={state} · assigné={asgn} · id={id}");
        if !labels.is_empty() { line.push_str(&format!(" · labels={labels}")); }
        line
    }).collect::<Vec<_>>().join("\n\n")
}

fn fmt_issue(i: &Value) -> String {
    let iid    = i["iid"].as_u64().unwrap_or(0);
    let title  = i["title"].as_str().unwrap_or("?");
    let state  = i["state"].as_str().unwrap_or("?");
    let asgn   = i["assignee"]["name"].as_str().unwrap_or("—");
    let desc   = i["description"].as_str().unwrap_or("—");
    let labels = i["labels"].as_array()
        .map(|a| a.iter().filter_map(|l| l.as_str()).collect::<Vec<_>>().join(", "))
        .unwrap_or_default();
    let url    = i["web_url"].as_str().unwrap_or("");
    format!("[#{iid}] {title}\nÉtat: {state} · Assigné: {asgn}\nLabels: {labels}\n{url}\n\n{desc}")
}

fn fmt_mrs(mrs: &[Value]) -> String {
    if mrs.is_empty() { return "Aucune merge request.".to_string(); }
    mrs.iter().map(|m| {
        let iid    = m["iid"].as_u64().unwrap_or(0);
        let title  = m["title"].as_str().unwrap_or("?");
        let state  = m["state"].as_str().unwrap_or("?");
        let author = m["author"]["name"].as_str().unwrap_or("?");
        let source = m["source_branch"].as_str().unwrap_or("?");
        let target = m["target_branch"].as_str().unwrap_or("?");
        let id     = m["id"].as_u64().unwrap_or(0);
        format!("[!{iid}] {title}\n  état={state} · auteur={author} · {source}→{target} · id={id}")
    }).collect::<Vec<_>>().join("\n\n")
}

// ─── Tool definitions ────────────────────────────────────────────────────────

pub fn tools() -> Vec<Value> {
    vec![
        // Issues
        json!({"name":"gitlab_list_issues","description":"GITLAB — Liste les issues d'un projet (filtres: state, label, assignee). Retourne liste compacte avec IIDs. Enchaîner avec gitlab_get_issue pour le détail.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string","description":"ID ou chemin du projet (ex: 42 ou 'group/repo')"},"state":{"type":"string","description":"open, closed, all","default":"open"},"labels":{"type":"string","description":"Labels séparés par virgule (ex: bug,urgent)"},"assignee_id":{"type":"integer","description":"ID de l'assigné"},"limit":{"type":"integer","default":20,"maximum":100}},"required":["project_id"]}}),
        json!({"name":"gitlab_get_issue","description":"GITLAB — Récupère le détail complet d'une issue (description, labels, assigné, URL).","inputSchema":{"type":"object","properties":{"project_id":{"type":"string","description":"ID ou chemin du projet"},"issue_iid":{"type":"integer","description":"IID de l'issue (numéro affiché dans GitLab, ex: 42)"}},"required":["project_id","issue_iid"]}}),
        json!({"name":"gitlab_create_issue","description":"GITLAB — Crée une nouvelle issue dans un projet.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"title":{"type":"string"},"description":{"type":"string"},"labels":{"type":"string","description":"Labels séparés par virgule"},"assignee_ids":{"type":"array","items":{"type":"integer"}},"milestone_id":{"type":"integer"}},"required":["project_id","title"]}}),
        json!({"name":"gitlab_update_issue","description":"GITLAB — Met à jour une issue (titre, description, état, labels, assigné).","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"issue_iid":{"type":"integer"},"title":{"type":"string"},"description":{"type":"string"},"state_event":{"type":"string","description":"close ou reopen"},"labels":{"type":"string"},"assignee_ids":{"type":"array","items":{"type":"integer"}}},"required":["project_id","issue_iid"]}}),
        json!({"name":"gitlab_close_issue","description":"GITLAB — Ferme une issue.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"issue_iid":{"type":"integer"}},"required":["project_id","issue_iid"]}}),
        json!({"name":"gitlab_add_comment","description":"GITLAB — Ajoute un commentaire (note) à une issue.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"issue_iid":{"type":"integer"},"body":{"type":"string"}},"required":["project_id","issue_iid","body"]}}),
        json!({"name":"gitlab_get_comments","description":"GITLAB — Récupère les commentaires d'une issue.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"issue_iid":{"type":"integer"}},"required":["project_id","issue_iid"]}}),
        json!({"name":"gitlab_assign_issue","description":"GITLAB — Assigne une issue à un ou plusieurs utilisateurs. Utiliser gitlab_list_members pour obtenir les IDs.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"issue_iid":{"type":"integer"},"assignee_ids":{"type":"array","items":{"type":"integer"},"description":"IDs des utilisateurs (obtenu via gitlab_list_members)"}},"required":["project_id","issue_iid","assignee_ids"]}}),
        json!({"name":"gitlab_list_labels","description":"GITLAB — Liste les labels d'un projet.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"}},"required":["project_id"]}}),
        // Merge Requests
        json!({"name":"gitlab_list_mrs","description":"GITLAB — Liste les merge requests d'un projet (filtres: state, source_branch). Retourne liste compacte avec IIDs.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"state":{"type":"string","description":"opened, closed, merged, all","default":"opened"},"source_branch":{"type":"string"},"limit":{"type":"integer","default":20,"maximum":100}},"required":["project_id"]}}),
        json!({"name":"gitlab_get_mr","description":"GITLAB — Récupère le détail complet d'une merge request.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"mr_iid":{"type":"integer","description":"IID de la MR (numéro affiché dans GitLab)"}},"required":["project_id","mr_iid"]}}),
        json!({"name":"gitlab_create_mr","description":"GITLAB — Crée une merge request.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"title":{"type":"string"},"source_branch":{"type":"string"},"target_branch":{"type":"string","default":"main"},"description":{"type":"string"},"assignee_id":{"type":"integer"},"remove_source_branch":{"type":"boolean","default":false}},"required":["project_id","title","source_branch"]}}),
        json!({"name":"gitlab_merge_mr","description":"GITLAB — Fusionne une merge request (si les conditions sont remplies).","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"mr_iid":{"type":"integer"},"merge_commit_message":{"type":"string"},"should_remove_source_branch":{"type":"boolean","default":false}},"required":["project_id","mr_iid"]}}),
        json!({"name":"gitlab_add_mr_comment","description":"GITLAB — Ajoute un commentaire à une merge request.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"mr_iid":{"type":"integer"},"body":{"type":"string"}},"required":["project_id","mr_iid","body"]}}),
        // Pipelines
        json!({"name":"gitlab_list_pipelines","description":"GITLAB — Liste les pipelines CI/CD d'un projet (filtres: status, branch).","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"status":{"type":"string","description":"running, pending, success, failed, canceled, skipped"},"ref":{"type":"string","description":"Branche ou tag"},"limit":{"type":"integer","default":20,"maximum":100}},"required":["project_id"]}}),
        json!({"name":"gitlab_get_pipeline","description":"GITLAB — Récupère le statut détaillé d'un pipeline (jobs, durée, statut).","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"pipeline_id":{"type":"integer"}},"required":["project_id","pipeline_id"]}}),
        json!({"name":"gitlab_retry_pipeline","description":"GITLAB — Relance un pipeline échoué.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"pipeline_id":{"type":"integer"}},"required":["project_id","pipeline_id"]}}),
        json!({"name":"gitlab_cancel_pipeline","description":"GITLAB — Annule un pipeline en cours d'exécution.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"pipeline_id":{"type":"integer"}},"required":["project_id","pipeline_id"]}}),
        // Projet & Utilisateurs
        json!({"name":"gitlab_list_projects","description":"GITLAB — Liste les projets accessibles (les groupes configurés dans gitlab.toml sont utilisés si présents).","inputSchema":{"type":"object","properties":{"search":{"type":"string","description":"Filtrer par nom de projet"},"limit":{"type":"integer","default":20,"maximum":100}}}}),
        json!({"name":"gitlab_get_project","description":"GITLAB — Récupère les détails d'un projet (description, URL, stats).","inputSchema":{"type":"object","properties":{"project_id":{"type":"string","description":"ID ou chemin du projet (ex: 42 ou 'group/repo')"}},"required":["project_id"]}}),
        json!({"name":"gitlab_list_members","description":"GITLAB — Liste les membres d'un projet avec leurs IDs. Nécessaire pour les assignations.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"}},"required":["project_id"]}}),
        json!({"name":"gitlab_get_current_user","description":"GITLAB — Retourne les informations de l'utilisateur authentifié (nom, email, ID). Utile pour filtrer les issues assignées à soi-même.","inputSchema":{"type":"object","properties":{}}}),
        json!({"name":"gitlab_list_branches","description":"GITLAB — Liste les branches d'un projet.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"search":{"type":"string","description":"Filtrer par nom de branche"}},"required":["project_id"]}}),
        // Wikis
        json!({"name":"gitlab_list_wiki_pages","description":"GITLAB — Liste les pages wiki d'un projet.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string","description":"ID ou chemin du projet"}},"required":["project_id"]}}),
        json!({"name":"gitlab_get_wiki_page","description":"GITLAB — Récupère le contenu d'une page wiki par son slug.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"slug":{"type":"string","description":"Slug de la page wiki (ex: home)"}},"required":["project_id","slug"]}}),
        json!({"name":"gitlab_create_wiki_page","description":"GITLAB — Crée une nouvelle page wiki dans un projet.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"title":{"type":"string"},"content":{"type":"string"},"format":{"type":"string","description":"markdown (défaut), rdoc, asciidoc, org","default":"markdown"}},"required":["project_id","title","content"]}}),
        json!({"name":"gitlab_update_wiki_page","description":"GITLAB — Met à jour une page wiki existante.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"slug":{"type":"string"},"title":{"type":"string"},"content":{"type":"string"},"format":{"type":"string"}},"required":["project_id","slug"]}}),
        json!({"name":"gitlab_delete_wiki_page","description":"GITLAB — Supprime une page wiki.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"slug":{"type":"string"}},"required":["project_id","slug"]}}),
        // Milestones
        json!({"name":"gitlab_list_milestones","description":"GITLAB — Liste les milestones d'un projet (filtres: state, search).","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"state":{"type":"string","description":"active ou closed"},"search":{"type":"string"}},"required":["project_id"]}}),
        json!({"name":"gitlab_get_milestone","description":"GITLAB — Récupère le détail d'un milestone.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"milestone_id":{"type":"integer"}},"required":["project_id","milestone_id"]}}),
        json!({"name":"gitlab_create_milestone","description":"GITLAB — Crée un milestone dans un projet.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"title":{"type":"string"},"description":{"type":"string"},"due_date":{"type":"string","description":"Date d'échéance (YYYY-MM-DD)"},"start_date":{"type":"string","description":"Date de début (YYYY-MM-DD)"}},"required":["project_id","title"]}}),
        json!({"name":"gitlab_update_milestone","description":"GITLAB — Met à jour un milestone (titre, description, date, état).","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"milestone_id":{"type":"integer"},"title":{"type":"string"},"description":{"type":"string"},"due_date":{"type":"string"},"state_event":{"type":"string","description":"activate ou close"}},"required":["project_id","milestone_id"]}}),
        // Releases
        json!({"name":"gitlab_list_releases","description":"GITLAB — Liste les releases d'un projet.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"}},"required":["project_id"]}}),
        json!({"name":"gitlab_get_release","description":"GITLAB — Récupère le détail d'une release par son tag.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"tag_name":{"type":"string","description":"Nom du tag de la release (ex: v1.0.0)"}},"required":["project_id","tag_name"]}}),
        json!({"name":"gitlab_create_release","description":"GITLAB — Crée une release associée à un tag existant.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"tag_name":{"type":"string"},"name":{"type":"string","description":"Titre de la release"},"description":{"type":"string","description":"Notes de la release (markdown)"},"ref":{"type":"string","description":"Branche ou commit sur lequel créer le tag si inexistant"}},"required":["project_id","tag_name","name","description"]}}),
        // Deployments
        json!({"name":"gitlab_list_deployments","description":"GITLAB — Liste les déploiements d'un projet (filtres: environment, status).","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"environment":{"type":"string","description":"Nom de l'environnement"},"status":{"type":"string","description":"created, running, success, failed, canceled, blocked, skipped"},"order_by":{"type":"string","description":"id, iid, created_at, updated_at, ref","default":"created_at"},"sort":{"type":"string","description":"asc ou desc","default":"desc"}},"required":["project_id"]}}),
        json!({"name":"gitlab_get_deployment","description":"GITLAB — Récupère le détail d'un déploiement.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"deployment_id":{"type":"integer"}},"required":["project_id","deployment_id"]}}),
        // Environments
        json!({"name":"gitlab_list_environments","description":"GITLAB — Liste les environnements d'un projet (filtres: name, search, states).","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"name":{"type":"string"},"search":{"type":"string"},"states":{"type":"string","description":"available ou stopped"}},"required":["project_id"]}}),
        json!({"name":"gitlab_get_environment","description":"GITLAB — Récupère le détail d'un environnement.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"environment_id":{"type":"integer"}},"required":["project_id","environment_id"]}}),
        // Jobs
        json!({"name":"gitlab_list_pipeline_jobs","description":"GITLAB — Liste les jobs d'un pipeline (filtre: scope par statut).","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"pipeline_id":{"type":"integer"},"scope":{"type":"string","description":"created, pending, running, failed, success, canceled, skipped, waiting_for_resource, manual"}},"required":["project_id","pipeline_id"]}}),
        json!({"name":"gitlab_get_pipeline_job_output","description":"GITLAB — Récupère la trace (log) d'un job de pipeline.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"job_id":{"type":"integer"}},"required":["project_id","job_id"]}}),
        json!({"name":"gitlab_play_pipeline_job","description":"GITLAB — Déclenche manuellement un job (trigger un job manuel).","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"job_id":{"type":"integer"}},"required":["project_id","job_id"]}}),
        // Groups
        json!({"name":"gitlab_list_group_projects","description":"GITLAB — Liste les projets d'un groupe GitLab.","inputSchema":{"type":"object","properties":{"group_id":{"type":"string","description":"ID ou chemin du groupe (ex: 42 ou 'my-group')"},"search":{"type":"string"},"order_by":{"type":"string","description":"id, name, path, created_at, updated_at, last_activity_at","default":"last_activity_at"}},"required":["group_id"]}}),
        json!({"name":"gitlab_get_namespace","description":"GITLAB — Récupère un namespace GitLab (utilisateur ou groupe) par son ID.","inputSchema":{"type":"object","properties":{"namespace_id":{"type":"string","description":"ID ou chemin du namespace"}},"required":["namespace_id"]}}),
        // MR discussions
        json!({"name":"gitlab_create_merge_request_thread","description":"GITLAB — Crée un nouveau thread de discussion sur une merge request (peut inclure une position de code).","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"mr_iid":{"type":"integer"},"body":{"type":"string","description":"Contenu du commentaire"},"position":{"type":"object","description":"Position de code optionnelle (pour les inline comments)"}},"required":["project_id","mr_iid","body"]}}),
        json!({"name":"gitlab_resolve_merge_request_thread","description":"GITLAB — Résout ou ré-ouvre un thread de discussion sur une merge request.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"mr_iid":{"type":"integer"},"discussion_id":{"type":"string","description":"ID du thread de discussion"},"resolved":{"type":"boolean","description":"true pour résoudre, false pour ré-ouvrir"}},"required":["project_id","mr_iid","discussion_id","resolved"]}}),
        json!({"name":"gitlab_list_issue_discussions","description":"GITLAB — Liste les discussions (threads) d'une issue.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"issue_iid":{"type":"integer"}},"required":["project_id","issue_iid"]}}),
        // Commits
        json!({"name":"gitlab_get_commit","description":"GITLAB — Récupère le détail d'un commit (auteur, message, stats).","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"sha":{"type":"string","description":"SHA complet ou abrégé du commit"}},"required":["project_id","sha"]}}),
        json!({"name":"gitlab_get_commit_diff","description":"GITLAB — Récupère le diff complet d'un commit.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"sha":{"type":"string","description":"SHA du commit"}},"required":["project_id","sha"]}}),
        // Repository
        json!({"name":"gitlab_get_repository_tree","description":"GITLAB — Liste les fichiers et dossiers d'un répertoire du dépôt (arbre de fichiers).","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"path":{"type":"string","description":"Chemin du répertoire (défaut: racine)","default":""},"ref":{"type":"string","description":"Branche ou tag (défaut: branche par défaut)"},"recursive":{"type":"boolean","description":"Lister récursivement","default":false}},"required":["project_id"]}}),
        json!({"name":"gitlab_get_file_contents","description":"GITLAB — Récupère le contenu d'un fichier du dépôt (décodé base64).","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"file_path":{"type":"string","description":"Chemin du fichier dans le dépôt (ex: src/main.rs)"},"ref":{"type":"string","description":"Branche ou tag (défaut: branche par défaut)"}},"required":["project_id","file_path"]}}),
        json!({"name":"gitlab_create_or_update_file","description":"GITLAB — Crée ou met à jour un fichier dans le dépôt GitLab avec un commit.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"file_path":{"type":"string","description":"Chemin du fichier dans le dépôt"},"branch":{"type":"string","description":"Branche cible"},"content":{"type":"string","description":"Contenu du fichier (texte brut)"},"commit_message":{"type":"string"},"previous_path":{"type":"string","description":"Ancien chemin du fichier si renommage"}},"required":["project_id","file_path","branch","content","commit_message"]}}),

        // MR — Approbation & Diffs
        json!({"name":"gitlab_approve_merge_request","description":"GITLAB — Approuve une merge request.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"mr_iid":{"type":"integer"}},"required":["project_id","mr_iid"]}}),
        json!({"name":"gitlab_unapprove_merge_request","description":"GITLAB — Retire l'approbation d'une merge request.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"mr_iid":{"type":"integer"}},"required":["project_id","mr_iid"]}}),
        json!({"name":"gitlab_get_merge_request_approval_state","description":"GITLAB — Retourne l'état d'approbation d'une merge request (qui a approuvé, règles, etc.).","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"mr_iid":{"type":"integer"}},"required":["project_id","mr_iid"]}}),
        json!({"name":"gitlab_get_merge_request_diffs","description":"GITLAB — Retourne les diffs d'une merge request (liste des fichiers modifiés avec contenu diff).","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"mr_iid":{"type":"integer"}},"required":["project_id","mr_iid"]}}),
        json!({"name":"gitlab_list_merge_request_diffs","description":"GITLAB — Liste les versions de diffs d'une merge request (alias de get_merge_request_diffs avec paramètres supplémentaires).","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"mr_iid":{"type":"integer"},"unidiff":{"type":"boolean","description":"Retourner le diff au format unidiff","default":false}},"required":["project_id","mr_iid"]}}),
        json!({"name":"gitlab_get_merge_request_conflicts","description":"GITLAB — Retourne les conflits d'une merge request.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"mr_iid":{"type":"integer"}},"required":["project_id","mr_iid"]}}),
        json!({"name":"gitlab_list_merge_request_changed_files","description":"GITLAB — Liste les fichiers modifiés dans une merge request (avec diffs).","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"mr_iid":{"type":"integer"}},"required":["project_id","mr_iid"]}}),
        json!({"name":"gitlab_update_merge_request","description":"GITLAB — Met à jour une merge request (titre, description, état, labels, assignés, milestone).","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"mr_iid":{"type":"integer"},"title":{"type":"string"},"description":{"type":"string"},"state_event":{"type":"string","description":"close ou reopen"},"labels":{"type":"string"},"assignee_ids":{"type":"array","items":{"type":"integer"}},"milestone_id":{"type":"integer"}},"required":["project_id","mr_iid"]}}),
        json!({"name":"gitlab_merge_merge_request","description":"GITLAB — Fusionne une merge request (alias de merge_mr avec le même format).","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"mr_iid":{"type":"integer"},"merge_commit_message":{"type":"string"},"should_remove_source_branch":{"type":"boolean","default":false}},"required":["project_id","mr_iid"]}}),
        json!({"name":"gitlab_get_merge_request","description":"GITLAB — Récupère le détail complet d'une merge request (alias de get_mr).","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"mr_iid":{"type":"integer"}},"required":["project_id","mr_iid"]}}),
        json!({"name":"gitlab_list_merge_requests","description":"GITLAB — Liste les merge requests d'un projet (alias de list_mrs).","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"state":{"type":"string","description":"opened, closed, merged, all","default":"opened"},"source_branch":{"type":"string"},"limit":{"type":"integer","default":20,"maximum":100}},"required":["project_id"]}}),
        json!({"name":"gitlab_create_merge_request","description":"GITLAB — Crée une merge request (alias de create_mr).","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"title":{"type":"string"},"source_branch":{"type":"string"},"target_branch":{"type":"string","default":"main"},"description":{"type":"string"},"assignee_id":{"type":"integer"},"remove_source_branch":{"type":"boolean","default":false}},"required":["project_id","title","source_branch"]}}),

        // MR — Notes
        json!({"name":"gitlab_get_merge_request_notes","description":"GITLAB — Retourne tous les commentaires (notes) d'une merge request.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"mr_iid":{"type":"integer"}},"required":["project_id","mr_iid"]}}),
        json!({"name":"gitlab_get_merge_request_note","description":"GITLAB — Retourne une note spécifique d'une merge request.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"mr_iid":{"type":"integer"},"note_id":{"type":"integer"}},"required":["project_id","mr_iid","note_id"]}}),
        json!({"name":"gitlab_create_merge_request_note","description":"GITLAB — Crée une note (commentaire) sur une merge request.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"mr_iid":{"type":"integer"},"body":{"type":"string"}},"required":["project_id","mr_iid","body"]}}),
        json!({"name":"gitlab_update_merge_request_note","description":"GITLAB — Met à jour une note sur une merge request.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"mr_iid":{"type":"integer"},"note_id":{"type":"integer"},"body":{"type":"string"}},"required":["project_id","mr_iid","note_id","body"]}}),
        json!({"name":"gitlab_delete_merge_request_note","description":"GITLAB — Supprime une note sur une merge request.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"mr_iid":{"type":"integer"},"note_id":{"type":"integer"}},"required":["project_id","mr_iid","note_id"]}}),

        // MR — Discussion notes
        json!({"name":"gitlab_create_merge_request_discussion_note","description":"GITLAB — Ajoute une note à un thread de discussion existant sur une merge request.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"mr_iid":{"type":"integer"},"discussion_id":{"type":"string"},"body":{"type":"string"}},"required":["project_id","mr_iid","discussion_id","body"]}}),
        json!({"name":"gitlab_update_merge_request_discussion_note","description":"GITLAB — Met à jour une note dans un thread de discussion d'une merge request.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"mr_iid":{"type":"integer"},"discussion_id":{"type":"string"},"note_id":{"type":"integer"},"body":{"type":"string"},"resolved":{"type":"boolean"}},"required":["project_id","mr_iid","discussion_id","note_id"]}}),
        json!({"name":"gitlab_delete_merge_request_discussion_note","description":"GITLAB — Supprime une note dans un thread de discussion d'une merge request.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"mr_iid":{"type":"integer"},"discussion_id":{"type":"string"},"note_id":{"type":"integer"}},"required":["project_id","mr_iid","discussion_id","note_id"]}}),

        // MR — Versions
        json!({"name":"gitlab_list_merge_request_versions","description":"GITLAB — Liste les versions de diff d'une merge request (chaque push crée une nouvelle version).","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"mr_iid":{"type":"integer"}},"required":["project_id","mr_iid"]}}),
        json!({"name":"gitlab_get_merge_request_version","description":"GITLAB — Retourne le détail d'une version de diff d'une merge request.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"mr_iid":{"type":"integer"},"version_id":{"type":"integer"}},"required":["project_id","mr_iid","version_id"]}}),
        json!({"name":"gitlab_get_merge_request_file_diff","description":"GITLAB — Retourne le diff d'un fichier spécifique dans une version de MR.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"mr_iid":{"type":"integer"},"version_id":{"type":"integer"},"file_path":{"type":"string","description":"Chemin du fichier à inspecter"}},"required":["project_id","mr_iid","version_id","file_path"]}}),
        json!({"name":"gitlab_mr_discussions","description":"GITLAB — Liste les discussions d'une merge request (alias compact).","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"mr_iid":{"type":"integer"}},"required":["project_id","mr_iid"]}}),

        // Draft notes
        json!({"name":"gitlab_create_draft_note","description":"GITLAB — Crée une note brouillon sur une merge request (non publiée).","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"mr_iid":{"type":"integer"},"note":{"type":"string","description":"Contenu de la note brouillon"},"position":{"type":"object","description":"Position de code optionnelle"}},"required":["project_id","mr_iid","note"]}}),
        json!({"name":"gitlab_list_draft_notes","description":"GITLAB — Liste les notes brouillon d'une merge request.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"mr_iid":{"type":"integer"}},"required":["project_id","mr_iid"]}}),
        json!({"name":"gitlab_get_draft_note","description":"GITLAB — Retourne une note brouillon spécifique.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"mr_iid":{"type":"integer"},"draft_note_id":{"type":"integer"}},"required":["project_id","mr_iid","draft_note_id"]}}),
        json!({"name":"gitlab_update_draft_note","description":"GITLAB — Met à jour une note brouillon sur une merge request.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"mr_iid":{"type":"integer"},"draft_note_id":{"type":"integer"},"note":{"type":"string"}},"required":["project_id","mr_iid","draft_note_id","note"]}}),
        json!({"name":"gitlab_delete_draft_note","description":"GITLAB — Supprime une note brouillon sur une merge request.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"mr_iid":{"type":"integer"},"draft_note_id":{"type":"integer"}},"required":["project_id","mr_iid","draft_note_id"]}}),
        json!({"name":"gitlab_publish_draft_note","description":"GITLAB — Publie une note brouillon (la rend visible).","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"mr_iid":{"type":"integer"},"draft_note_id":{"type":"integer"}},"required":["project_id","mr_iid","draft_note_id"]}}),
        json!({"name":"gitlab_bulk_publish_draft_notes","description":"GITLAB — Publie toutes les notes brouillon d'une merge request en une seule opération.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"mr_iid":{"type":"integer"}},"required":["project_id","mr_iid"]}}),

        // Repository — Branches & Commits
        json!({"name":"gitlab_create_branch","description":"GITLAB — Crée une nouvelle branche dans un projet.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"branch":{"type":"string","description":"Nom de la nouvelle branche"},"ref":{"type":"string","description":"Branche, tag ou commit SHA de référence"}},"required":["project_id","branch","ref"]}}),
        json!({"name":"gitlab_get_branch_diffs","description":"GITLAB — Compare deux branches ou commits et retourne les diffs (équivalent git diff A...B).","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"from":{"type":"string","description":"Branche ou SHA de départ"},"to":{"type":"string","description":"Branche ou SHA de destination"},"straight":{"type":"boolean","description":"Utiliser la comparaison directe au lieu de merge-base","default":false}},"required":["project_id","from","to"]}}),
        json!({"name":"gitlab_list_commits","description":"GITLAB — Liste les commits d'une branche ou d'un tag.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"ref_name":{"type":"string","description":"Branche ou tag (défaut: branche par défaut)"},"since":{"type":"string","description":"Filtrer les commits après cette date (ISO 8601)"},"until":{"type":"string","description":"Filtrer les commits avant cette date (ISO 8601)"},"limit":{"type":"integer","default":20,"maximum":100}},"required":["project_id"]}}),
        json!({"name":"gitlab_push_files","description":"GITLAB — Pousse plusieurs fichiers en un seul commit (create/update/delete/move). Utilise l'API commits avec actions.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"branch":{"type":"string","description":"Branche cible"},"commit_message":{"type":"string"},"actions":{"type":"array","description":"Liste d'actions de fichiers","items":{"type":"object","properties":{"action":{"type":"string","description":"create, update, delete, move, chmod"},"file_path":{"type":"string","description":"Chemin du fichier"},"content":{"type":"string","description":"Contenu du fichier (pour create/update)"},"previous_path":{"type":"string","description":"Ancien chemin (pour move)"},"encoding":{"type":"string","description":"text ou base64","default":"text"}},"required":["action","file_path"]}}},"required":["project_id","branch","commit_message","actions"]}}),

        // Repository — Projects
        json!({"name":"gitlab_fork_repository","description":"GITLAB — Fork un projet GitLab dans un namespace (utilisateur ou groupe).","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"namespace":{"type":"string","description":"Namespace de destination (chemin ou ID). Défaut: namespace personnel."},"name":{"type":"string","description":"Nom du projet forké (défaut: même nom)"},"path":{"type":"string","description":"Chemin du projet forké (défaut: même chemin)"}},"required":["project_id"]}}),
        json!({"name":"gitlab_create_repository","description":"GITLAB — Crée un nouveau projet (dépôt) GitLab.","inputSchema":{"type":"object","properties":{"name":{"type":"string","description":"Nom du projet"},"path":{"type":"string","description":"Chemin URL du projet (défaut: slug du nom)"},"namespace_id":{"type":"integer","description":"ID du namespace (groupe ou utilisateur)"},"description":{"type":"string"},"visibility":{"type":"string","description":"private, internal, public","default":"private"},"initialize_with_readme":{"type":"boolean","default":true}},"required":["name"]}}),
        json!({"name":"gitlab_search_repositories","description":"GITLAB — Recherche des projets GitLab par nom ou chemin.","inputSchema":{"type":"object","properties":{"search":{"type":"string","description":"Terme de recherche"},"limit":{"type":"integer","default":20,"maximum":100}},"required":["search"]}}),

        // Jobs
        json!({"name":"gitlab_get_pipeline_job","description":"GITLAB — Retourne le détail d'un job spécifique (statut, durée, artefacts).","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"job_id":{"type":"integer"}},"required":["project_id","job_id"]}}),
        json!({"name":"gitlab_cancel_pipeline_job","description":"GITLAB — Annule un job en cours d'exécution.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"job_id":{"type":"integer"}},"required":["project_id","job_id"]}}),
        json!({"name":"gitlab_retry_pipeline_job","description":"GITLAB — Relance un job échoué ou annulé.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"job_id":{"type":"integer"}},"required":["project_id","job_id"]}}),
        json!({"name":"gitlab_list_job_artifacts","description":"GITLAB — Liste les artefacts d'un job (nom, taille, type).","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"job_id":{"type":"integer"}},"required":["project_id","job_id"]}}),
        json!({"name":"gitlab_get_job_artifact_file","description":"GITLAB — Récupère le contenu d'un fichier artefact d'un job.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"job_id":{"type":"integer"},"artifact_path":{"type":"string","description":"Chemin du fichier dans l'archive des artefacts"}},"required":["project_id","job_id","artifact_path"]}}),
        json!({"name":"gitlab_download_job_artifacts","description":"GITLAB — Retourne l'URL de téléchargement des artefacts d'un job.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"job_id":{"type":"integer"},"ref_name":{"type":"string","description":"Branche ou tag (pour récupérer les artefacts du dernier job réussi)"},"job_name":{"type":"string","description":"Nom du job (pour récupérer les artefacts du dernier job réussi)"}},"required":["project_id","job_id"]}}),
        json!({"name":"gitlab_list_pipeline_trigger_jobs","description":"GITLAB — Liste les déclencheurs (triggers) d'un projet.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"}},"required":["project_id"]}}),
        json!({"name":"gitlab_create_pipeline","description":"GITLAB — Déclenche un nouveau pipeline sur une branche ou un tag avec des variables optionnelles.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"ref":{"type":"string","description":"Branche ou tag sur lequel déclencher le pipeline"},"variables":{"type":"array","description":"Variables CI/CD","items":{"type":"object","properties":{"key":{"type":"string"},"value":{"type":"string"},"variable_type":{"type":"string","description":"env_var ou file","default":"env_var"}},"required":["key","value"]}}},"required":["project_id","ref"]}}),

        // Issues — Extras
        json!({"name":"gitlab_delete_issue","description":"GITLAB — Supprime définitivement une issue (nécessite droits owner ou admin).","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"issue_iid":{"type":"integer"}},"required":["project_id","issue_iid"]}}),
        json!({"name":"gitlab_create_issue_note","description":"GITLAB — Crée une note sur une issue (alias de add_comment).","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"issue_iid":{"type":"integer"},"body":{"type":"string"}},"required":["project_id","issue_iid","body"]}}),
        json!({"name":"gitlab_update_issue_note","description":"GITLAB — Met à jour une note existante sur une issue.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"issue_iid":{"type":"integer"},"note_id":{"type":"integer"},"body":{"type":"string"}},"required":["project_id","issue_iid","note_id","body"]}}),
        json!({"name":"gitlab_create_issue_link","description":"GITLAB — Crée un lien entre deux issues (blocks, is_blocked_by, relates_to).","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"issue_iid":{"type":"integer"},"target_project_id":{"type":"string","description":"ID du projet de l'issue cible"},"target_issue_iid":{"type":"integer","description":"IID de l'issue cible"},"link_type":{"type":"string","description":"relates_to, blocks, is_blocked_by","default":"relates_to"}},"required":["project_id","issue_iid","target_project_id","target_issue_iid"]}}),
        json!({"name":"gitlab_list_issue_links","description":"GITLAB — Liste les liens d'une issue avec d'autres issues.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"issue_iid":{"type":"integer"}},"required":["project_id","issue_iid"]}}),
        json!({"name":"gitlab_get_issue_link","description":"GITLAB — Retourne le détail d'un lien entre issues.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"issue_iid":{"type":"integer"},"link_id":{"type":"integer"}},"required":["project_id","issue_iid","link_id"]}}),
        json!({"name":"gitlab_delete_issue_link","description":"GITLAB — Supprime un lien entre issues.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"issue_iid":{"type":"integer"},"link_id":{"type":"integer"}},"required":["project_id","issue_iid","link_id"]}}),
        json!({"name":"gitlab_my_issues","description":"GITLAB — Liste les issues assignées à l'utilisateur courant.","inputSchema":{"type":"object","properties":{"state":{"type":"string","description":"opened, closed, all","default":"opened"},"limit":{"type":"integer","default":20,"maximum":100}}}}),
        json!({"name":"gitlab_create_note","description":"GITLAB — Crée une note sur une issue (alias générique).","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"issue_iid":{"type":"integer"},"body":{"type":"string"}},"required":["project_id","issue_iid","body"]}}),

        // Users & Namespaces
        json!({"name":"gitlab_get_users","description":"GITLAB — Recherche des utilisateurs GitLab par nom ou username.","inputSchema":{"type":"object","properties":{"search":{"type":"string","description":"Terme de recherche (nom ou username)"},"limit":{"type":"integer","default":20,"maximum":100}},"required":["search"]}}),
        json!({"name":"gitlab_list_project_members","description":"GITLAB — Liste les membres d'un projet (alias de list_members).","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"}},"required":["project_id"]}}),
        json!({"name":"gitlab_list_namespaces","description":"GITLAB — Liste tous les namespaces accessibles (utilisateurs et groupes).","inputSchema":{"type":"object","properties":{"search":{"type":"string","description":"Filtrer par nom"},"limit":{"type":"integer","default":50}}}}),
        json!({"name":"gitlab_verify_namespace","description":"GITLAB — Vérifie si un namespace existe et retourne ses informations.","inputSchema":{"type":"object","properties":{"namespace":{"type":"string","description":"Chemin ou ID du namespace à vérifier"}},"required":["namespace"]}}),

        // Labels — extras
        json!({"name":"gitlab_create_label","description":"GITLAB — Crée un label dans un projet.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"name":{"type":"string"},"color":{"type":"string","description":"Couleur hexadécimale (ex: #FF0000)"},"description":{"type":"string"}},"required":["project_id","name","color"]}}),
        json!({"name":"gitlab_get_label","description":"GITLAB — Retourne le détail d'un label par son ID.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"label_id":{"type":"integer"}},"required":["project_id","label_id"]}}),
        json!({"name":"gitlab_update_label","description":"GITLAB — Met à jour un label (nom, couleur, description).","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"label_id":{"type":"integer"},"name":{"type":"string"},"color":{"type":"string"},"description":{"type":"string"}},"required":["project_id","label_id"]}}),
        json!({"name":"gitlab_delete_label","description":"GITLAB — Supprime un label d'un projet.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"label_id":{"type":"integer"}},"required":["project_id","label_id"]}}),

        // Milestones — extras
        json!({"name":"gitlab_delete_milestone","description":"GITLAB — Supprime un milestone d'un projet.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"milestone_id":{"type":"integer"}},"required":["project_id","milestone_id"]}}),
        json!({"name":"gitlab_promote_milestone","description":"GITLAB — Promeut un milestone de projet en milestone de groupe.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"milestone_id":{"type":"integer"}},"required":["project_id","milestone_id"]}}),
        json!({"name":"gitlab_edit_milestone","description":"GITLAB — Met à jour un milestone (alias de update_milestone).","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"milestone_id":{"type":"integer"},"title":{"type":"string"},"description":{"type":"string"},"due_date":{"type":"string"},"state_event":{"type":"string","description":"activate ou close"}},"required":["project_id","milestone_id"]}}),
        json!({"name":"gitlab_get_milestone_burndown_events","description":"GITLAB — Retourne les événements burndown d'un milestone.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"milestone_id":{"type":"integer"}},"required":["project_id","milestone_id"]}}),
        json!({"name":"gitlab_get_milestone_issue","description":"GITLAB — Liste les issues associées à un milestone.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"milestone_id":{"type":"integer"}},"required":["project_id","milestone_id"]}}),
        json!({"name":"gitlab_get_milestone_merge_requests","description":"GITLAB — Liste les merge requests associées à un milestone.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"milestone_id":{"type":"integer"}},"required":["project_id","milestone_id"]}}),

        // Releases — extras
        json!({"name":"gitlab_delete_release","description":"GITLAB — Supprime une release (le tag n'est pas supprimé).","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"tag_name":{"type":"string"}},"required":["project_id","tag_name"]}}),
        json!({"name":"gitlab_update_release","description":"GITLAB — Met à jour le nom et/ou la description d'une release.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"tag_name":{"type":"string"},"name":{"type":"string"},"description":{"type":"string"}},"required":["project_id","tag_name"]}}),
        json!({"name":"gitlab_create_release_evidence","description":"GITLAB — Crée une preuve (evidence) pour une release.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"tag_name":{"type":"string"}},"required":["project_id","tag_name"]}}),
        json!({"name":"gitlab_download_release_asset","description":"GITLAB — Retourne l'URL de téléchargement d'un asset de release.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"tag_name":{"type":"string"},"direct_asset_path":{"type":"string","description":"Chemin de l'asset dans la release"}},"required":["project_id","tag_name","direct_asset_path"]}}),

        // Group wikis
        json!({"name":"gitlab_list_group_wiki_pages","description":"GITLAB — Liste les pages wiki d'un groupe.","inputSchema":{"type":"object","properties":{"group_id":{"type":"string"}},"required":["group_id"]}}),
        json!({"name":"gitlab_get_group_wiki_page","description":"GITLAB — Récupère le contenu d'une page wiki de groupe par son slug.","inputSchema":{"type":"object","properties":{"group_id":{"type":"string"},"slug":{"type":"string"}},"required":["group_id","slug"]}}),
        json!({"name":"gitlab_create_group_wiki_page","description":"GITLAB — Crée une nouvelle page wiki dans un groupe.","inputSchema":{"type":"object","properties":{"group_id":{"type":"string"},"title":{"type":"string"},"content":{"type":"string"},"format":{"type":"string","default":"markdown"}},"required":["group_id","title","content"]}}),
        json!({"name":"gitlab_update_group_wiki_page","description":"GITLAB — Met à jour une page wiki de groupe.","inputSchema":{"type":"object","properties":{"group_id":{"type":"string"},"slug":{"type":"string"},"title":{"type":"string"},"content":{"type":"string"},"format":{"type":"string"}},"required":["group_id","slug"]}}),
        json!({"name":"gitlab_delete_group_wiki_page","description":"GITLAB — Supprime une page wiki de groupe.","inputSchema":{"type":"object","properties":{"group_id":{"type":"string"},"slug":{"type":"string"}},"required":["group_id","slug"]}}),

        // Group iterations
        json!({"name":"gitlab_list_group_iterations","description":"GITLAB — Liste les itérations (sprints) d'un groupe GitLab.","inputSchema":{"type":"object","properties":{"group_id":{"type":"string"},"state":{"type":"string","description":"opened, closed, all","default":"opened"},"search":{"type":"string"}},"required":["group_id"]}}),

        // Events
        json!({"name":"gitlab_list_events","description":"GITLAB — Liste les événements d'activité d'un projet ou d'un utilisateur.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string","description":"ID du projet (optionnel — si absent, liste les événements de l'utilisateur courant)"},"action":{"type":"string","description":"Type d'action (created, updated, closed, reopened, pushed, commented, merged, etc.)"},"target_type":{"type":"string","description":"Type de cible (issue, merge_request, note, snippet, project, etc.)"},"limit":{"type":"integer","default":20,"maximum":100}}}}),
        json!({"name":"gitlab_get_project_events","description":"GITLAB — Liste les événements d'activité d'un projet spécifique.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"action":{"type":"string","description":"Type d'action à filtrer"},"limit":{"type":"integer","default":20,"maximum":100}},"required":["project_id"]}}),

        // Upload & attachments
        json!({"name":"gitlab_upload_markdown","description":"GITLAB — Upload un fichier dans un projet GitLab et retourne l'URL markdown pour l'intégrer dans des commentaires ou descriptions.","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"filename":{"type":"string","description":"Nom du fichier"},"content":{"type":"string","description":"Contenu du fichier en base64 ou texte"},"content_type":{"type":"string","description":"Type MIME du fichier (ex: image/png, text/plain)","default":"text/plain"}},"required":["project_id","filename","content"]}}),
        json!({"name":"gitlab_download_attachment","description":"GITLAB — Télécharge le contenu d'une pièce jointe depuis une URL GitLab (attachments uploadés via gitlab_upload_markdown).","inputSchema":{"type":"object","properties":{"project_id":{"type":"string"},"attachment_url":{"type":"string","description":"URL complète de la pièce jointe GitLab"}},"required":["project_id","attachment_url"]}}),
    ]
}

// ─── Handler ─────────────────────────────────────────────────────────────────

pub async fn handle(name: &str, args: &Value) -> Result<String, String> {
    let cfg = match GitlabConfig::load() {
        Some(c) => c,
        None => return Ok("GitLab non configuré (gitlab.toml manquant)".to_string()),
    };
    let token = cfg.token.clone();

    // Encode le project_id pour les URLs (ex: "group/repo" → "group%2Frepo")
    let encode_pid = |s: &str| urlencoding::encode(s).into_owned();

    match name {

        // ── Issues ────────────────────────────────────────────────────────────

        "gitlab_list_issues" => {
            let pid   = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let state = args["state"].as_str().unwrap_or("open");
            let limit = args["limit"].as_u64().unwrap_or(20);
            let mut url = cfg.api(&format!("projects/{}/issues?state={}&per_page={}", encode_pid(pid), state, limit));
            if let Some(l) = args["labels"].as_str()     { url.push_str(&format!("&labels={}", urlencoding::encode(l))); }
            if let Some(a) = args["assignee_id"].as_u64() { url.push_str(&format!("&assignee_id={a}")); }
            let data   = get(&token, &url).await?;
            let issues = data.as_array().cloned().unwrap_or_default();
            Ok(fmt_issues(&issues))
        }

        "gitlab_get_issue" => {
            let pid = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let iid = args["issue_iid"].as_u64().ok_or("Missing param: issue_iid")?;
            let url = cfg.api(&format!("projects/{}/issues/{}", encode_pid(pid), iid));
            let data = get(&token, &url).await?;
            Ok(fmt_issue(&data))
        }

        "gitlab_create_issue" => {
            let pid   = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let title = args["title"].as_str().ok_or("Missing param: title")?;
            let url   = cfg.api(&format!("projects/{}/issues", encode_pid(pid)));
            let mut body = json!({"title": title});
            if let Some(d) = args["description"].as_str()     { body["description"]  = json!(d); }
            if let Some(l) = args["labels"].as_str()          { body["labels"]        = json!(l); }
            if let Some(a) = args["assignee_ids"].as_array()  { body["assignee_ids"]  = json!(a); }
            if let Some(m) = args["milestone_id"].as_u64()    { body["milestone_id"]  = json!(m); }
            let data = post(&token, &url, body).await?;
            let iid  = data["iid"].as_u64().unwrap_or(0);
            let wurl = data["web_url"].as_str().unwrap_or("");
            Ok(format!("✅ Issue créée : #{iid}\n{wurl}"))
        }

        "gitlab_update_issue" => {
            let pid = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let iid = args["issue_iid"].as_u64().ok_or("Missing param: issue_iid")?;
            let url = cfg.api(&format!("projects/{}/issues/{}", encode_pid(pid), iid));
            let mut body = json!({});
            if let Some(v) = args["title"].as_str()            { body["title"]        = json!(v); }
            if let Some(v) = args["description"].as_str()      { body["description"]  = json!(v); }
            if let Some(v) = args["state_event"].as_str()      { body["state_event"]  = json!(v); }
            if let Some(v) = args["labels"].as_str()           { body["labels"]       = json!(v); }
            if let Some(v) = args["assignee_ids"].as_array()   { body["assignee_ids"] = json!(v); }
            put(&token, &url, body).await?;
            Ok(format!("✅ Issue #{iid} mise à jour."))
        }

        "gitlab_close_issue" => {
            let pid = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let iid = args["issue_iid"].as_u64().ok_or("Missing param: issue_iid")?;
            let url = cfg.api(&format!("projects/{}/issues/{}", encode_pid(pid), iid));
            put(&token, &url, json!({"state_event": "close"})).await?;
            Ok(format!("✅ Issue #{iid} fermée."))
        }

        "gitlab_add_comment" => {
            let pid  = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let iid  = args["issue_iid"].as_u64().ok_or("Missing param: issue_iid")?;
            let body = args["body"].as_str().ok_or("Missing param: body")?;
            let url  = cfg.api(&format!("projects/{}/issues/{}/notes", encode_pid(pid), iid));
            post(&token, &url, json!({"body": body})).await?;
            Ok("✅ Commentaire ajouté.".to_string())
        }

        "gitlab_get_comments" => {
            let pid  = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let iid  = args["issue_iid"].as_u64().ok_or("Missing param: issue_iid")?;
            let url  = cfg.api(&format!("projects/{}/issues/{}/notes?sort=asc", encode_pid(pid), iid));
            let data = get(&token, &url).await?;
            let notes = data.as_array().cloned().unwrap_or_default();
            if notes.is_empty() { return Ok("Aucun commentaire.".to_string()); }
            Ok(notes.iter().filter(|n| !n["system"].as_bool().unwrap_or(false)).map(|n| {
                let author = n["author"]["name"].as_str().unwrap_or("?");
                let date   = n["created_at"].as_str().unwrap_or("?");
                let body   = n["body"].as_str().unwrap_or("");
                format!("[{date}] {author}:\n{body}")
            }).collect::<Vec<_>>().join("\n\n---\n\n"))
        }

        "gitlab_assign_issue" => {
            let pid         = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let iid         = args["issue_iid"].as_u64().ok_or("Missing param: issue_iid")?;
            let assignee_ids = args["assignee_ids"].as_array().ok_or("Missing param: assignee_ids")?;
            let url  = cfg.api(&format!("projects/{}/issues/{}", encode_pid(pid), iid));
            put(&token, &url, json!({"assignee_ids": assignee_ids})).await?;
            Ok(format!("✅ Issue #{iid} assignée."))
        }

        "gitlab_list_labels" => {
            let pid    = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let url    = cfg.api(&format!("projects/{}/labels?per_page=100", encode_pid(pid)));
            let data   = get(&token, &url).await?;
            let labels = data.as_array().cloned().unwrap_or_default();
            if labels.is_empty() { return Ok("Aucun label.".to_string()); }
            Ok(labels.iter().map(|l| {
                let name  = l["name"].as_str().unwrap_or("?");
                let color = l["color"].as_str().unwrap_or("?");
                format!("{name}  couleur={color}")
            }).collect::<Vec<_>>().join("\n"))
        }

        // ── Merge Requests ────────────────────────────────────────────────────

        "gitlab_list_mrs" => {
            let pid   = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let state = args["state"].as_str().unwrap_or("opened");
            let limit = args["limit"].as_u64().unwrap_or(20);
            let mut url = cfg.api(&format!("projects/{}/merge_requests?state={}&per_page={}", encode_pid(pid), state, limit));
            if let Some(b) = args["source_branch"].as_str() { url.push_str(&format!("&source_branch={}", urlencoding::encode(b))); }
            let data = get(&token, &url).await?;
            let mrs  = data.as_array().cloned().unwrap_or_default();
            Ok(fmt_mrs(&mrs))
        }

        "gitlab_get_mr" => {
            let pid    = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let mr_iid = args["mr_iid"].as_u64().ok_or("Missing param: mr_iid")?;
            let url    = cfg.api(&format!("projects/{}/merge_requests/{}", encode_pid(pid), mr_iid));
            let m      = get(&token, &url).await?;
            let title  = m["title"].as_str().unwrap_or("?");
            let state  = m["state"].as_str().unwrap_or("?");
            let author = m["author"]["name"].as_str().unwrap_or("?");
            let source = m["source_branch"].as_str().unwrap_or("?");
            let target = m["target_branch"].as_str().unwrap_or("?");
            let desc   = m["description"].as_str().unwrap_or("—");
            let wurl   = m["web_url"].as_str().unwrap_or("");
            Ok(format!("[!{mr_iid}] {title}\nÉtat: {state} · Auteur: {author}\n{source} → {target}\n{wurl}\n\n{desc}"))
        }

        "gitlab_create_mr" => {
            let pid    = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let title  = args["title"].as_str().ok_or("Missing param: title")?;
            let source = args["source_branch"].as_str().ok_or("Missing param: source_branch")?;
            let target = args["target_branch"].as_str().unwrap_or("main");
            let url    = cfg.api(&format!("projects/{}/merge_requests", encode_pid(pid)));
            let mut body = json!({"title": title, "source_branch": source, "target_branch": target});
            if let Some(d) = args["description"].as_str()          { body["description"]          = json!(d); }
            if let Some(a) = args["assignee_id"].as_u64()          { body["assignee_id"]           = json!(a); }
            if let Some(r) = args["remove_source_branch"].as_bool() { body["remove_source_branch"] = json!(r); }
            let data  = post(&token, &url, body).await?;
            let iid   = data["iid"].as_u64().unwrap_or(0);
            let wurl  = data["web_url"].as_str().unwrap_or("");
            Ok(format!("✅ MR créée : !{iid}\n{wurl}"))
        }

        "gitlab_merge_mr" => {
            let pid    = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let mr_iid = args["mr_iid"].as_u64().ok_or("Missing param: mr_iid")?;
            let url    = cfg.api(&format!("projects/{}/merge_requests/{}/merge", encode_pid(pid), mr_iid));
            let mut body = json!({});
            if let Some(m) = args["merge_commit_message"].as_str()       { body["merge_commit_message"]        = json!(m); }
            if let Some(r) = args["should_remove_source_branch"].as_bool() { body["should_remove_source_branch"] = json!(r); }
            post(&token, &url, body).await?;
            Ok(format!("✅ MR !{mr_iid} fusionnée."))
        }

        "gitlab_add_mr_comment" => {
            let pid    = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let mr_iid = args["mr_iid"].as_u64().ok_or("Missing param: mr_iid")?;
            let body   = args["body"].as_str().ok_or("Missing param: body")?;
            let url    = cfg.api(&format!("projects/{}/merge_requests/{}/notes", encode_pid(pid), mr_iid));
            post(&token, &url, json!({"body": body})).await?;
            Ok("✅ Commentaire ajouté sur la MR.".to_string())
        }

        // ── Pipelines ─────────────────────────────────────────────────────────

        "gitlab_list_pipelines" => {
            let pid   = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let limit = args["limit"].as_u64().unwrap_or(20);
            let mut url = cfg.api(&format!("projects/{}/pipelines?per_page={}", encode_pid(pid), limit));
            if let Some(s) = args["status"].as_str() { url.push_str(&format!("&status={s}")); }
            if let Some(r) = args["ref"].as_str()    { url.push_str(&format!("&ref={}", urlencoding::encode(r))); }
            let data      = get(&token, &url).await?;
            let pipelines = data.as_array().cloned().unwrap_or_default();
            if pipelines.is_empty() { return Ok("Aucun pipeline.".to_string()); }
            Ok(pipelines.iter().map(|p| {
                let id     = p["id"].as_u64().unwrap_or(0);
                let status = p["status"].as_str().unwrap_or("?");
                let ref_   = p["ref"].as_str().unwrap_or("?");
                let sha    = p["sha"].as_str().unwrap_or("?").get(..8).unwrap_or("?");
                format!("[{id}] {status} · branche={ref_} · sha={sha}")
            }).collect::<Vec<_>>().join("\n"))
        }

        "gitlab_get_pipeline" => {
            let pid         = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let pipeline_id = args["pipeline_id"].as_u64().ok_or("Missing param: pipeline_id")?;
            let url         = cfg.api(&format!("projects/{}/pipelines/{}", encode_pid(pid), pipeline_id));
            let p           = get(&token, &url).await?;
            let status   = p["status"].as_str().unwrap_or("?");
            let ref_     = p["ref"].as_str().unwrap_or("?");
            let sha      = p["sha"].as_str().unwrap_or("?").get(..8).unwrap_or("?");
            let created  = p["created_at"].as_str().unwrap_or("?");
            let duration = p["duration"].as_u64().unwrap_or(0);
            let wurl     = p["web_url"].as_str().unwrap_or("");
            Ok(format!("Pipeline #{pipeline_id}\nStatut: {status} · Branche: {ref_} · SHA: {sha}\nCréé: {created} · Durée: {duration}s\n{wurl}"))
        }

        "gitlab_retry_pipeline" => {
            let pid         = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let pipeline_id = args["pipeline_id"].as_u64().ok_or("Missing param: pipeline_id")?;
            let url         = cfg.api(&format!("projects/{}/pipelines/{}/retry", encode_pid(pid), pipeline_id));
            let data        = post(&token, &url, json!({})).await?;
            let new_id      = data["id"].as_u64().unwrap_or(0);
            Ok(format!("✅ Pipeline relancé — nouveau ID: {new_id}"))
        }

        "gitlab_cancel_pipeline" => {
            let pid         = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let pipeline_id = args["pipeline_id"].as_u64().ok_or("Missing param: pipeline_id")?;
            let url         = cfg.api(&format!("projects/{}/pipelines/{}/cancel", encode_pid(pid), pipeline_id));
            post(&token, &url, json!({})).await?;
            Ok(format!("✅ Pipeline #{pipeline_id} annulé."))
        }

        // ── Projet & Utilisateurs ─────────────────────────────────────────────

        "gitlab_list_projects" => {
            let limit  = args["limit"].as_u64().unwrap_or(20);
            let mut url = if !cfg.groups.is_empty() {
                let gid = urlencoding::encode(&cfg.groups[0]).into_owned();
                cfg.api(&format!("groups/{}/projects?per_page={}&order_by=last_activity_at", gid, limit))
            } else {
                cfg.api(&format!("projects?membership=true&per_page={}&order_by=last_activity_at", limit))
            };
            if let Some(s) = args["search"].as_str() { url.push_str(&format!("&search={}", urlencoding::encode(s))); }
            let data     = get(&token, &url).await?;
            let projects = data.as_array().cloned().unwrap_or_default();
            if projects.is_empty() { return Ok("Aucun projet trouvé.".to_string()); }
            Ok(projects.iter().map(|p| {
                let name = p["path_with_namespace"].as_str().unwrap_or("?");
                let id   = p["id"].as_u64().unwrap_or(0);
                let desc = p["description"].as_str().unwrap_or("—");
                format!("{name}  id={id}\n  {desc}")
            }).collect::<Vec<_>>().join("\n\n"))
        }

        "gitlab_get_project" => {
            let pid  = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let url  = cfg.api(&format!("projects/{}", encode_pid(pid)));
            let p    = get(&token, &url).await?;
            let name = p["path_with_namespace"].as_str().unwrap_or("?");
            let desc = p["description"].as_str().unwrap_or("—");
            let wurl = p["web_url"].as_str().unwrap_or("");
            let stars = p["star_count"].as_u64().unwrap_or(0);
            let forks = p["forks_count"].as_u64().unwrap_or(0);
            let branch = p["default_branch"].as_str().unwrap_or("main");
            Ok(format!("{name}\n{wurl}\nBranche défaut: {branch} · ⭐{stars} · 🍴{forks}\n\n{desc}"))
        }

        "gitlab_list_members" => {
            let pid     = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let url     = cfg.api(&format!("projects/{}/members/all?per_page=100", encode_pid(pid)));
            let data    = get(&token, &url).await?;
            let members = data.as_array().cloned().unwrap_or_default();
            if members.is_empty() { return Ok("Aucun membre.".to_string()); }
            Ok(members.iter().map(|m| {
                let name     = m["name"].as_str().unwrap_or("?");
                let username = m["username"].as_str().unwrap_or("?");
                let id       = m["id"].as_u64().unwrap_or(0);
                format!("{name} (@{username})  id={id}")
            }).collect::<Vec<_>>().join("\n"))
        }

        "gitlab_get_current_user" => {
            let url  = cfg.api("user");
            let data = get(&token, &url).await?;
            let name     = data["name"].as_str().unwrap_or("?");
            let username = data["username"].as_str().unwrap_or("?");
            let email    = data["email"].as_str().unwrap_or("?");
            let id       = data["id"].as_u64().unwrap_or(0);
            Ok(format!("{name} (@{username}) <{email}>\nid={id}"))
        }

        "gitlab_list_branches" => {
            let pid   = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let mut url = cfg.api(&format!("projects/{}/repository/branches?per_page=50", encode_pid(pid)));
            if let Some(s) = args["search"].as_str() { url.push_str(&format!("&search={}", urlencoding::encode(s))); }
            let data     = get(&token, &url).await?;
            let branches = data.as_array().cloned().unwrap_or_default();
            if branches.is_empty() { return Ok("Aucune branche.".to_string()); }
            Ok(branches.iter().map(|b| {
                let name      = b["name"].as_str().unwrap_or("?");
                let protected = b["protected"].as_bool().unwrap_or(false);
                let sha       = b["commit"]["id"].as_str().unwrap_or("?").get(..8).unwrap_or("?");
                let flag      = if protected { " 🔒" } else { "" };
                format!("{name}{flag}  sha={sha}")
            }).collect::<Vec<_>>().join("\n"))
        }

        // ── Wikis ─────────────────────────────────────────────────────────────

        "gitlab_list_wiki_pages" => {
            let pid  = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let url  = cfg.api(&format!("projects/{}/wikis?per_page=100", encode_pid(pid)));
            let data = get(&token, &url).await?;
            let pages = data.as_array().cloned().unwrap_or_default();
            if pages.is_empty() { return Ok("Aucune page wiki.".to_string()); }
            Ok(pages.iter().map(|p| {
                let title = p["title"].as_str().unwrap_or("?");
                let slug  = p["slug"].as_str().unwrap_or("?");
                format!("{title}  slug={slug}")
            }).collect::<Vec<_>>().join("\n"))
        }

        "gitlab_get_wiki_page" => {
            let pid  = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let slug = args["slug"].as_str().ok_or("Missing param: slug")?;
            let url  = cfg.api(&format!("projects/{}/wikis/{}", encode_pid(pid), urlencoding::encode(slug)));
            let data = get(&token, &url).await?;
            let title   = data["title"].as_str().unwrap_or("?");
            let content = data["content"].as_str().unwrap_or("—");
            let format  = data["format"].as_str().unwrap_or("?");
            Ok(format!("# {title}  (format={format})\n\n{content}"))
        }

        "gitlab_create_wiki_page" => {
            let pid     = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let title   = args["title"].as_str().ok_or("Missing param: title")?;
            let content = args["content"].as_str().ok_or("Missing param: content")?;
            let fmt     = args["format"].as_str().unwrap_or("markdown");
            let url     = cfg.api(&format!("projects/{}/wikis", encode_pid(pid)));
            let data    = post(&token, &url, json!({"title": title, "content": content, "format": fmt})).await?;
            let slug    = data["slug"].as_str().unwrap_or("?");
            Ok(format!("✅ Page wiki créée : slug={slug}"))
        }

        "gitlab_update_wiki_page" => {
            let pid  = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let slug = args["slug"].as_str().ok_or("Missing param: slug")?;
            let url  = cfg.api(&format!("projects/{}/wikis/{}", encode_pid(pid), urlencoding::encode(slug)));
            let mut body = json!({});
            if let Some(v) = args["title"].as_str()   { body["title"]   = json!(v); }
            if let Some(v) = args["content"].as_str() { body["content"] = json!(v); }
            if let Some(v) = args["format"].as_str()  { body["format"]  = json!(v); }
            put(&token, &url, body).await?;
            Ok(format!("✅ Page wiki '{slug}' mise à jour."))
        }

        "gitlab_delete_wiki_page" => {
            let pid  = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let slug = args["slug"].as_str().ok_or("Missing param: slug")?;
            let url  = cfg.api(&format!("projects/{}/wikis/{}", encode_pid(pid), urlencoding::encode(slug)));
            delete(&token, &url).await?;
            Ok(format!("✅ Page wiki '{slug}' supprimée."))
        }

        // ── Milestones ────────────────────────────────────────────────────────

        "gitlab_list_milestones" => {
            let pid  = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let mut url = cfg.api(&format!("projects/{}/milestones?per_page=50", encode_pid(pid)));
            if let Some(s) = args["state"].as_str()  { url.push_str(&format!("&state={s}")); }
            if let Some(s) = args["search"].as_str() { url.push_str(&format!("&search={}", urlencoding::encode(s))); }
            let data       = get(&token, &url).await?;
            let milestones = data.as_array().cloned().unwrap_or_default();
            if milestones.is_empty() { return Ok("Aucun milestone.".to_string()); }
            Ok(milestones.iter().map(|m| {
                let id    = m["id"].as_u64().unwrap_or(0);
                let title = m["title"].as_str().unwrap_or("?");
                let state = m["state"].as_str().unwrap_or("?");
                let due   = m["due_date"].as_str().unwrap_or("—");
                format!("[{id}] {title}  état={state} · échéance={due}")
            }).collect::<Vec<_>>().join("\n"))
        }

        "gitlab_get_milestone" => {
            let pid          = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let milestone_id = args["milestone_id"].as_u64().ok_or("Missing param: milestone_id")?;
            let url          = cfg.api(&format!("projects/{}/milestones/{}", encode_pid(pid), milestone_id));
            let m            = get(&token, &url).await?;
            let title = m["title"].as_str().unwrap_or("?");
            let state = m["state"].as_str().unwrap_or("?");
            let desc  = m["description"].as_str().unwrap_or("—");
            let due   = m["due_date"].as_str().unwrap_or("—");
            let start = m["start_date"].as_str().unwrap_or("—");
            Ok(format!("[{milestone_id}] {title}\nÉtat: {state} · Début: {start} · Échéance: {due}\n\n{desc}"))
        }

        "gitlab_create_milestone" => {
            let pid   = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let title = args["title"].as_str().ok_or("Missing param: title")?;
            let url   = cfg.api(&format!("projects/{}/milestones", encode_pid(pid)));
            let mut body = json!({"title": title});
            if let Some(v) = args["description"].as_str() { body["description"] = json!(v); }
            if let Some(v) = args["due_date"].as_str()    { body["due_date"]    = json!(v); }
            if let Some(v) = args["start_date"].as_str()  { body["start_date"]  = json!(v); }
            let data = post(&token, &url, body).await?;
            let id   = data["id"].as_u64().unwrap_or(0);
            Ok(format!("✅ Milestone créé : id={id}"))
        }

        "gitlab_update_milestone" => {
            let pid          = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let milestone_id = args["milestone_id"].as_u64().ok_or("Missing param: milestone_id")?;
            let url          = cfg.api(&format!("projects/{}/milestones/{}", encode_pid(pid), milestone_id));
            let mut body = json!({});
            if let Some(v) = args["title"].as_str()       { body["title"]       = json!(v); }
            if let Some(v) = args["description"].as_str() { body["description"] = json!(v); }
            if let Some(v) = args["due_date"].as_str()    { body["due_date"]    = json!(v); }
            if let Some(v) = args["state_event"].as_str() { body["state_event"] = json!(v); }
            put(&token, &url, body).await?;
            Ok(format!("✅ Milestone #{milestone_id} mis à jour."))
        }

        // ── Releases ──────────────────────────────────────────────────────────

        "gitlab_list_releases" => {
            let pid  = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let url  = cfg.api(&format!("projects/{}/releases?per_page=20", encode_pid(pid)));
            let data = get(&token, &url).await?;
            let rels = data.as_array().cloned().unwrap_or_default();
            if rels.is_empty() { return Ok("Aucune release.".to_string()); }
            Ok(rels.iter().map(|r| {
                let tag  = r["tag_name"].as_str().unwrap_or("?");
                let name = r["name"].as_str().unwrap_or("?");
                let date = r["released_at"].as_str().unwrap_or("?");
                format!("{tag}  {name}  date={date}")
            }).collect::<Vec<_>>().join("\n"))
        }

        "gitlab_get_release" => {
            let pid      = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let tag_name = args["tag_name"].as_str().ok_or("Missing param: tag_name")?;
            let url      = cfg.api(&format!("projects/{}/releases/{}", encode_pid(pid), urlencoding::encode(tag_name)));
            let r        = get(&token, &url).await?;
            let name     = r["name"].as_str().unwrap_or("?");
            let desc     = r["description"].as_str().unwrap_or("—");
            let date     = r["released_at"].as_str().unwrap_or("?");
            let wurl     = r["_links"]["self"].as_str().unwrap_or("");
            Ok(format!("{tag_name}  {name}\nDate: {date}\n{wurl}\n\n{desc}"))
        }

        "gitlab_create_release" => {
            let pid      = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let tag_name = args["tag_name"].as_str().ok_or("Missing param: tag_name")?;
            let name     = args["name"].as_str().ok_or("Missing param: name")?;
            let desc     = args["description"].as_str().ok_or("Missing param: description")?;
            let url      = cfg.api(&format!("projects/{}/releases", encode_pid(pid)));
            let mut body = json!({"tag_name": tag_name, "name": name, "description": desc});
            if let Some(v) = args["ref"].as_str() { body["ref"] = json!(v); }
            let data     = post(&token, &url, body).await?;
            let wurl     = data["_links"]["self"].as_str().unwrap_or("");
            Ok(format!("✅ Release '{tag_name}' créée.\n{wurl}"))
        }

        // ── Deployments ───────────────────────────────────────────────────────

        "gitlab_list_deployments" => {
            let pid  = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let mut url = cfg.api(&format!("projects/{}/deployments?per_page=20", encode_pid(pid)));
            if let Some(v) = args["environment"].as_str() { url.push_str(&format!("&environment={}", urlencoding::encode(v))); }
            if let Some(v) = args["status"].as_str()      { url.push_str(&format!("&status={v}")); }
            if let Some(v) = args["order_by"].as_str()    { url.push_str(&format!("&order_by={v}")); }
            if let Some(v) = args["sort"].as_str()        { url.push_str(&format!("&sort={v}")); }
            let data  = get(&token, &url).await?;
            let deps  = data.as_array().cloned().unwrap_or_default();
            if deps.is_empty() { return Ok("Aucun déploiement.".to_string()); }
            Ok(deps.iter().map(|d| {
                let id  = d["id"].as_u64().unwrap_or(0);
                let env = d["environment"]["name"].as_str().unwrap_or("?");
                let st  = d["status"].as_str().unwrap_or("?");
                let sha = d["sha"].as_str().unwrap_or("?").get(..8).unwrap_or("?");
                let ref_ = d["ref"].as_str().unwrap_or("?");
                format!("[{id}] {env}  status={st} · ref={ref_} · sha={sha}")
            }).collect::<Vec<_>>().join("\n"))
        }

        "gitlab_get_deployment" => {
            let pid           = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let deployment_id = args["deployment_id"].as_u64().ok_or("Missing param: deployment_id")?;
            let url           = cfg.api(&format!("projects/{}/deployments/{}", encode_pid(pid), deployment_id));
            let d             = get(&token, &url).await?;
            let env    = d["environment"]["name"].as_str().unwrap_or("?");
            let status = d["status"].as_str().unwrap_or("?");
            let ref_   = d["ref"].as_str().unwrap_or("?");
            let sha    = d["sha"].as_str().unwrap_or("?").get(..8).unwrap_or("?");
            let date   = d["created_at"].as_str().unwrap_or("?");
            Ok(format!("Déploiement #{deployment_id}\nEnviron: {env} · Statut: {status}\nRef: {ref_} · SHA: {sha}\nDate: {date}"))
        }

        // ── Environments ──────────────────────────────────────────────────────

        "gitlab_list_environments" => {
            let pid  = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let mut url = cfg.api(&format!("projects/{}/environments?per_page=50", encode_pid(pid)));
            if let Some(v) = args["name"].as_str()   { url.push_str(&format!("&name={}", urlencoding::encode(v))); }
            if let Some(v) = args["search"].as_str() { url.push_str(&format!("&search={}", urlencoding::encode(v))); }
            if let Some(v) = args["states"].as_str() { url.push_str(&format!("&states={v}")); }
            let data  = get(&token, &url).await?;
            let envs  = data.as_array().cloned().unwrap_or_default();
            if envs.is_empty() { return Ok("Aucun environnement.".to_string()); }
            Ok(envs.iter().map(|e| {
                let id    = e["id"].as_u64().unwrap_or(0);
                let name  = e["name"].as_str().unwrap_or("?");
                let state = e["state"].as_str().unwrap_or("?");
                let url   = e["external_url"].as_str().unwrap_or("");
                format!("[{id}] {name}  état={state}  {url}")
            }).collect::<Vec<_>>().join("\n"))
        }

        "gitlab_get_environment" => {
            let pid            = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let environment_id = args["environment_id"].as_u64().ok_or("Missing param: environment_id")?;
            let url            = cfg.api(&format!("projects/{}/environments/{}", encode_pid(pid), environment_id));
            let e              = get(&token, &url).await?;
            let name  = e["name"].as_str().unwrap_or("?");
            let state = e["state"].as_str().unwrap_or("?");
            let eurl  = e["external_url"].as_str().unwrap_or("—");
            Ok(format!("Environnement #{environment_id}  {name}\nÉtat: {state}\nURL externe: {eurl}"))
        }

        // ── Jobs ──────────────────────────────────────────────────────────────

        "gitlab_list_pipeline_jobs" => {
            let pid         = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let pipeline_id = args["pipeline_id"].as_u64().ok_or("Missing param: pipeline_id")?;
            let mut url     = cfg.api(&format!("projects/{}/pipelines/{}/jobs?per_page=100", encode_pid(pid), pipeline_id));
            if let Some(v) = args["scope"].as_str() { url.push_str(&format!("&scope[]={v}")); }
            let data = get(&token, &url).await?;
            let jobs = data.as_array().cloned().unwrap_or_default();
            if jobs.is_empty() { return Ok("Aucun job.".to_string()); }
            Ok(jobs.iter().map(|j| {
                let id       = j["id"].as_u64().unwrap_or(0);
                let name     = j["name"].as_str().unwrap_or("?");
                let status   = j["status"].as_str().unwrap_or("?");
                let stage    = j["stage"].as_str().unwrap_or("?");
                let duration = j["duration"].as_f64().unwrap_or(0.0) as u64;
                format!("[{id}] {stage}/{name}  status={status}  durée={duration}s")
            }).collect::<Vec<_>>().join("\n"))
        }

        "gitlab_get_pipeline_job_output" => {
            let pid    = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let job_id = args["job_id"].as_u64().ok_or("Missing param: job_id")?;
            let url    = cfg.api(&format!("projects/{}/jobs/{}/trace", encode_pid(pid), job_id));
            let trace  = get_text(&token, &url).await?;
            // Tronquer si trop long
            let out = if trace.len() > 8000 {
                format!("...[tronqué, affichage des 8000 derniers caractères]\n{}", &trace[trace.len()-8000..])
            } else {
                trace
            };
            Ok(out)
        }

        "gitlab_play_pipeline_job" => {
            let pid    = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let job_id = args["job_id"].as_u64().ok_or("Missing param: job_id")?;
            let url    = cfg.api(&format!("projects/{}/jobs/{}/play", encode_pid(pid), job_id));
            let data   = post(&token, &url, json!({})).await?;
            let new_id = data["id"].as_u64().unwrap_or(0);
            Ok(format!("✅ Job déclenché — id={new_id}"))
        }

        // ── Groups ────────────────────────────────────────────────────────────

        "gitlab_list_group_projects" => {
            let gid  = args["group_id"].as_str().ok_or("Missing param: group_id")?;
            let mut url = cfg.api(&format!("groups/{}/projects?per_page=50", urlencoding::encode(gid)));
            if let Some(v) = args["search"].as_str()   { url.push_str(&format!("&search={}", urlencoding::encode(v))); }
            if let Some(v) = args["order_by"].as_str() { url.push_str(&format!("&order_by={v}")); }
            let data     = get(&token, &url).await?;
            let projects = data.as_array().cloned().unwrap_or_default();
            if projects.is_empty() { return Ok("Aucun projet dans ce groupe.".to_string()); }
            Ok(projects.iter().map(|p| {
                let name = p["path_with_namespace"].as_str().unwrap_or("?");
                let id   = p["id"].as_u64().unwrap_or(0);
                let desc = p["description"].as_str().unwrap_or("—");
                format!("{name}  id={id}\n  {desc}")
            }).collect::<Vec<_>>().join("\n\n"))
        }

        "gitlab_get_namespace" => {
            let nid  = args["namespace_id"].as_str().ok_or("Missing param: namespace_id")?;
            let url  = cfg.api(&format!("namespaces/{}", urlencoding::encode(nid)));
            let data = get(&token, &url).await?;
            let name = data["name"].as_str().unwrap_or("?");
            let kind = data["kind"].as_str().unwrap_or("?");
            let path = data["full_path"].as_str().unwrap_or("?");
            let id   = data["id"].as_u64().unwrap_or(0);
            Ok(format!("{name}  ({kind})\nChemin: {path}  id={id}"))
        }

        // ── MR Discussions ────────────────────────────────────────────────────

        "gitlab_create_merge_request_thread" => {
            let pid    = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let mr_iid = args["mr_iid"].as_u64().ok_or("Missing param: mr_iid")?;
            let body   = args["body"].as_str().ok_or("Missing param: body")?;
            let url    = cfg.api(&format!("projects/{}/merge_requests/{}/discussions", encode_pid(pid), mr_iid));
            let mut payload = json!({"body": body});
            if let Some(pos) = args.get("position") { payload["position"] = pos.clone(); }
            let data   = post(&token, &url, payload).await?;
            let did    = data["id"].as_str().unwrap_or("?");
            Ok(format!("✅ Thread créé sur MR !{mr_iid}  discussion_id={did}"))
        }

        "gitlab_resolve_merge_request_thread" => {
            let pid           = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let mr_iid        = args["mr_iid"].as_u64().ok_or("Missing param: mr_iid")?;
            let discussion_id = args["discussion_id"].as_str().ok_or("Missing param: discussion_id")?;
            let resolved      = args["resolved"].as_bool().ok_or("Missing param: resolved")?;
            let url = cfg.api(&format!("projects/{}/merge_requests/{}/discussions/{}", encode_pid(pid), mr_iid, discussion_id));
            put(&token, &url, json!({"resolved": resolved})).await?;
            let action = if resolved { "résolu" } else { "ré-ouvert" };
            Ok(format!("✅ Thread {discussion_id} {action}."))
        }

        "gitlab_list_issue_discussions" => {
            let pid       = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let issue_iid = args["issue_iid"].as_u64().ok_or("Missing param: issue_iid")?;
            let url       = cfg.api(&format!("projects/{}/issues/{}/discussions?per_page=50", encode_pid(pid), issue_iid));
            let data      = get(&token, &url).await?;
            let threads   = data.as_array().cloned().unwrap_or_default();
            if threads.is_empty() { return Ok("Aucune discussion.".to_string()); }
            Ok(threads.iter().map(|t| {
                let did  = t["id"].as_str().unwrap_or("?");
                let notes = t["notes"].as_array().cloned().unwrap_or_default();
                let first = notes.first().map(|n| {
                    let author = n["author"]["name"].as_str().unwrap_or("?");
                    let body   = n["body"].as_str().unwrap_or("").chars().take(200).collect::<String>();
                    format!("{author}: {body}")
                }).unwrap_or_default();
                format!("[{did}] {first}")
            }).collect::<Vec<_>>().join("\n\n"))
        }

        // ── Commits ───────────────────────────────────────────────────────────

        "gitlab_get_commit" => {
            let pid = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let sha = args["sha"].as_str().ok_or("Missing param: sha")?;
            let url = cfg.api(&format!("projects/{}/repository/commits/{}", encode_pid(pid), sha));
            let c   = get(&token, &url).await?;
            let short_sha = c["short_id"].as_str().unwrap_or(sha);
            let author    = c["author_name"].as_str().unwrap_or("?");
            let email     = c["author_email"].as_str().unwrap_or("?");
            let date      = c["authored_date"].as_str().unwrap_or("?");
            let message   = c["message"].as_str().unwrap_or("?");
            let additions = c["stats"]["additions"].as_u64().unwrap_or(0);
            let deletions = c["stats"]["deletions"].as_u64().unwrap_or(0);
            Ok(format!("Commit {short_sha}\nAuteur: {author} <{email}>\nDate: {date}\n+{additions} -{deletions}\n\n{message}"))
        }

        "gitlab_get_commit_diff" => {
            let pid  = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let sha  = args["sha"].as_str().ok_or("Missing param: sha")?;
            let url  = cfg.api(&format!("projects/{}/repository/commits/{}/diff", encode_pid(pid), sha));
            let data = get(&token, &url).await?;
            let diffs = data.as_array().cloned().unwrap_or_default();
            if diffs.is_empty() { return Ok("Aucun diff.".to_string()); }
            Ok(diffs.iter().map(|d| {
                let path  = d["new_path"].as_str().unwrap_or(d["old_path"].as_str().unwrap_or("?"));
                let diff  = d["diff"].as_str().unwrap_or("");
                format!("--- {path} ---\n{diff}")
            }).collect::<Vec<_>>().join("\n\n"))
        }

        // ── Repository ────────────────────────────────────────────────────────

        "gitlab_get_repository_tree" => {
            let pid  = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let path = args["path"].as_str().unwrap_or("");
            let recursive = args["recursive"].as_bool().unwrap_or(false);
            let mut url = cfg.api(&format!("projects/{}/repository/tree?per_page=100&recursive={}", encode_pid(pid), recursive));
            if !path.is_empty() { url.push_str(&format!("&path={}", urlencoding::encode(path))); }
            if let Some(v) = args["ref"].as_str() { url.push_str(&format!("&ref={}", urlencoding::encode(v))); }
            let data  = get(&token, &url).await?;
            let items = data.as_array().cloned().unwrap_or_default();
            if items.is_empty() { return Ok("Dossier vide.".to_string()); }
            Ok(items.iter().map(|item| {
                let name = item["name"].as_str().unwrap_or("?");
                let kind = item["type"].as_str().unwrap_or("?");
                let path = item["path"].as_str().unwrap_or("?");
                let prefix = if kind == "tree" { "📁" } else { "📄" };
                format!("{prefix} {path}  ({name})")
            }).collect::<Vec<_>>().join("\n"))
        }

        "gitlab_get_file_contents" => {
            let pid       = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let file_path = args["file_path"].as_str().ok_or("Missing param: file_path")?;
            let mut url   = cfg.api(&format!("projects/{}/repository/files/{}", encode_pid(pid), urlencoding::encode(file_path)));
            if let Some(v) = args["ref"].as_str() {
                url.push_str(&format!("?ref={}", urlencoding::encode(v)));
            } else {
                url.push_str("?ref=HEAD");
            }
            let data      = get(&token, &url).await?;
            let encoding  = data["encoding"].as_str().unwrap_or("base64");
            let content_b64 = data["content"].as_str().unwrap_or("");
            if encoding == "base64" {
                // Décoder base64 (GitLab retourne avec des newlines)
                let clean: String = content_b64.chars().filter(|c| !c.is_whitespace()).collect();
                let decoded = base64_decode(&clean);
                Ok(decoded)
            } else {
                Ok(content_b64.to_string())
            }
        }

        "gitlab_create_or_update_file" => {
            let pid            = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let file_path      = args["file_path"].as_str().ok_or("Missing param: file_path")?;
            let branch         = args["branch"].as_str().ok_or("Missing param: branch")?;
            let content        = args["content"].as_str().ok_or("Missing param: content")?;
            let commit_message = args["commit_message"].as_str().ok_or("Missing param: commit_message")?;
            let url            = cfg.api(&format!("projects/{}/repository/files/{}", encode_pid(pid), urlencoding::encode(file_path)));
            let mut body       = json!({
                "branch":         branch,
                "content":        content,
                "commit_message": commit_message,
                "encoding":       "text"
            });
            if let Some(v) = args["previous_path"].as_str() { body["previous_path"] = json!(v); }
            // Essayer PUT (update) d'abord, sinon POST (create)
            let client = reqwest::Client::new();
            let put_resp = client.put(&url)
                .header("PRIVATE-TOKEN", &token)
                .header("Content-Type", "application/json")
                .json(&body)
                .send().await.map_err(|e| e.to_string())?;
            if put_resp.status().is_success() {
                Ok(format!("✅ Fichier '{file_path}' mis à jour sur '{branch}'."))
            } else {
                // Fichier n'existe pas → créer
                let post_resp = client.post(&url)
                    .header("PRIVATE-TOKEN", &token)
                    .header("Content-Type", "application/json")
                    .json(&body)
                    .send().await.map_err(|e| e.to_string())?;
                if post_resp.status().is_success() {
                    Ok(format!("✅ Fichier '{file_path}' créé sur '{branch}'."))
                } else {
                    let err = post_resp.text().await.unwrap_or_default();
                    Err(format!("Erreur création/mise à jour fichier: {err}"))
                }
            }
        }

        // ── MR — Approbations ─────────────────────────────────────────────────

        "gitlab_approve_merge_request" => {
            let pid    = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let mr_iid = args["mr_iid"].as_u64().ok_or("Missing param: mr_iid")?;
            let url    = cfg.api(&format!("projects/{}/merge_requests/{}/approve", encode_pid(pid), mr_iid));
            post(&token, &url, json!({})).await?;
            Ok(format!("✅ MR !{mr_iid} approuvée."))
        }

        "gitlab_unapprove_merge_request" => {
            let pid    = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let mr_iid = args["mr_iid"].as_u64().ok_or("Missing param: mr_iid")?;
            let url    = cfg.api(&format!("projects/{}/merge_requests/{}/unapprove", encode_pid(pid), mr_iid));
            post(&token, &url, json!({})).await?;
            Ok(format!("✅ Approbation retirée de la MR !{mr_iid}."))
        }

        "gitlab_get_merge_request_approval_state" => {
            let pid    = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let mr_iid = args["mr_iid"].as_u64().ok_or("Missing param: mr_iid")?;
            let url    = cfg.api(&format!("projects/{}/merge_requests/{}/approval_state", encode_pid(pid), mr_iid));
            let data   = get(&token, &url).await?;
            Ok(serde_json::to_string_pretty(&data).unwrap_or_default())
        }

        "gitlab_get_merge_request_diffs" => {
            let pid    = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let mr_iid = args["mr_iid"].as_u64().ok_or("Missing param: mr_iid")?;
            let url    = cfg.api(&format!("projects/{}/merge_requests/{}/diffs", encode_pid(pid), mr_iid));
            let data   = get(&token, &url).await?;
            let diffs  = data.as_array().cloned().unwrap_or_default();
            if diffs.is_empty() { return Ok("Aucun diff.".to_string()); }
            Ok(diffs.iter().map(|d| {
                let path = d["new_path"].as_str().unwrap_or(d["old_path"].as_str().unwrap_or("?"));
                let diff = d["diff"].as_str().unwrap_or("");
                format!("--- {path} ---\n{diff}")
            }).collect::<Vec<_>>().join("\n\n"))
        }

        "gitlab_list_merge_request_diffs" => {
            let pid    = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let mr_iid = args["mr_iid"].as_u64().ok_or("Missing param: mr_iid")?;
            let mut url = cfg.api(&format!("projects/{}/merge_requests/{}/diffs", encode_pid(pid), mr_iid));
            if args["unidiff"].as_bool().unwrap_or(false) { url.push_str("?unidiff=true"); }
            let data   = get(&token, &url).await?;
            let diffs  = data.as_array().cloned().unwrap_or_default();
            if diffs.is_empty() { return Ok("Aucun diff.".to_string()); }
            Ok(diffs.iter().map(|d| {
                let path    = d["new_path"].as_str().unwrap_or(d["old_path"].as_str().unwrap_or("?"));
                let new_    = d["new_file"].as_bool().unwrap_or(false);
                let deleted = d["deleted_file"].as_bool().unwrap_or(false);
                let renamed = d["renamed_file"].as_bool().unwrap_or(false);
                let mut flags = vec![];
                if new_ { flags.push("new"); }
                if deleted { flags.push("deleted"); }
                if renamed { flags.push("renamed"); }
                let flag_str = if flags.is_empty() { String::new() } else { format!(" [{}]", flags.join(",")) };
                format!("{path}{flag_str}")
            }).collect::<Vec<_>>().join("\n"))
        }

        "gitlab_get_merge_request_conflicts" => {
            let pid    = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let mr_iid = args["mr_iid"].as_u64().ok_or("Missing param: mr_iid")?;
            let url    = cfg.api(&format!("projects/{}/merge_requests/{}/conflicts", encode_pid(pid), mr_iid));
            let data   = get(&token, &url).await?;
            Ok(serde_json::to_string_pretty(&data).unwrap_or_default())
        }

        "gitlab_list_merge_request_changed_files" => {
            let pid    = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let mr_iid = args["mr_iid"].as_u64().ok_or("Missing param: mr_iid")?;
            let url    = cfg.api(&format!("projects/{}/merge_requests/{}/changes", encode_pid(pid), mr_iid));
            let data   = get(&token, &url).await?;
            let changes = data["changes"].as_array().cloned().unwrap_or_default();
            if changes.is_empty() { return Ok("Aucun fichier modifié.".to_string()); }
            Ok(changes.iter().map(|d| {
                let path    = d["new_path"].as_str().unwrap_or(d["old_path"].as_str().unwrap_or("?"));
                let new_    = d["new_file"].as_bool().unwrap_or(false);
                let deleted = d["deleted_file"].as_bool().unwrap_or(false);
                let renamed = d["renamed_file"].as_bool().unwrap_or(false);
                let mut flags = vec![];
                if new_ { flags.push("new"); }
                if deleted { flags.push("deleted"); }
                if renamed { flags.push("renamed"); }
                let flag_str = if flags.is_empty() { String::new() } else { format!(" [{}]", flags.join(",")) };
                format!("{path}{flag_str}")
            }).collect::<Vec<_>>().join("\n"))
        }

        "gitlab_update_merge_request" => {
            let pid    = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let mr_iid = args["mr_iid"].as_u64().ok_or("Missing param: mr_iid")?;
            let url    = cfg.api(&format!("projects/{}/merge_requests/{}", encode_pid(pid), mr_iid));
            let mut body = json!({});
            if let Some(v) = args["title"].as_str()           { body["title"]        = json!(v); }
            if let Some(v) = args["description"].as_str()     { body["description"]  = json!(v); }
            if let Some(v) = args["state_event"].as_str()     { body["state_event"]  = json!(v); }
            if let Some(v) = args["labels"].as_str()          { body["labels"]       = json!(v); }
            if let Some(v) = args["assignee_ids"].as_array()  { body["assignee_ids"] = json!(v); }
            if let Some(v) = args["milestone_id"].as_u64()    { body["milestone_id"] = json!(v); }
            put(&token, &url, body).await?;
            Ok(format!("✅ MR !{mr_iid} mise à jour."))
        }

        "gitlab_merge_merge_request" => {
            let pid    = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let mr_iid = args["mr_iid"].as_u64().ok_or("Missing param: mr_iid")?;
            let url    = cfg.api(&format!("projects/{}/merge_requests/{}/merge", encode_pid(pid), mr_iid));
            let mut body = json!({});
            if let Some(m) = args["merge_commit_message"].as_str()        { body["merge_commit_message"]        = json!(m); }
            if let Some(r) = args["should_remove_source_branch"].as_bool() { body["should_remove_source_branch"] = json!(r); }
            post(&token, &url, body).await?;
            Ok(format!("✅ MR !{mr_iid} fusionnée."))
        }

        "gitlab_get_merge_request" => {
            let pid    = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let mr_iid = args["mr_iid"].as_u64().ok_or("Missing param: mr_iid")?;
            let url    = cfg.api(&format!("projects/{}/merge_requests/{}", encode_pid(pid), mr_iid));
            let m      = get(&token, &url).await?;
            let title  = m["title"].as_str().unwrap_or("?");
            let state  = m["state"].as_str().unwrap_or("?");
            let author = m["author"]["name"].as_str().unwrap_or("?");
            let source = m["source_branch"].as_str().unwrap_or("?");
            let target = m["target_branch"].as_str().unwrap_or("?");
            let desc   = m["description"].as_str().unwrap_or("—");
            let wurl   = m["web_url"].as_str().unwrap_or("");
            Ok(format!("[!{mr_iid}] {title}\nÉtat: {state} · Auteur: {author}\n{source} → {target}\n{wurl}\n\n{desc}"))
        }

        "gitlab_list_merge_requests" => {
            let pid   = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let state = args["state"].as_str().unwrap_or("opened");
            let limit = args["limit"].as_u64().unwrap_or(20);
            let mut url = cfg.api(&format!("projects/{}/merge_requests?state={}&per_page={}", encode_pid(pid), state, limit));
            if let Some(b) = args["source_branch"].as_str() { url.push_str(&format!("&source_branch={}", urlencoding::encode(b))); }
            let data = get(&token, &url).await?;
            let mrs  = data.as_array().cloned().unwrap_or_default();
            Ok(fmt_mrs(&mrs))
        }

        "gitlab_create_merge_request" => {
            let pid    = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let title  = args["title"].as_str().ok_or("Missing param: title")?;
            let source = args["source_branch"].as_str().ok_or("Missing param: source_branch")?;
            let target = args["target_branch"].as_str().unwrap_or("main");
            let url    = cfg.api(&format!("projects/{}/merge_requests", encode_pid(pid)));
            let mut body = json!({"title": title, "source_branch": source, "target_branch": target});
            if let Some(d) = args["description"].as_str()           { body["description"]          = json!(d); }
            if let Some(a) = args["assignee_id"].as_u64()           { body["assignee_id"]           = json!(a); }
            if let Some(r) = args["remove_source_branch"].as_bool() { body["remove_source_branch"] = json!(r); }
            let data  = post(&token, &url, body).await?;
            let iid   = data["iid"].as_u64().unwrap_or(0);
            let wurl  = data["web_url"].as_str().unwrap_or("");
            Ok(format!("✅ MR créée : !{iid}\n{wurl}"))
        }

        // ── MR — Notes ────────────────────────────────────────────────────────

        "gitlab_get_merge_request_notes" => {
            let pid    = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let mr_iid = args["mr_iid"].as_u64().ok_or("Missing param: mr_iid")?;
            let url    = cfg.api(&format!("projects/{}/merge_requests/{}/notes?sort=asc&per_page=100", encode_pid(pid), mr_iid));
            let data   = get(&token, &url).await?;
            let notes  = data.as_array().cloned().unwrap_or_default();
            if notes.is_empty() { return Ok("Aucune note.".to_string()); }
            Ok(notes.iter().filter(|n| !n["system"].as_bool().unwrap_or(false)).map(|n| {
                let id     = n["id"].as_u64().unwrap_or(0);
                let author = n["author"]["name"].as_str().unwrap_or("?");
                let date   = n["created_at"].as_str().unwrap_or("?");
                let body   = n["body"].as_str().unwrap_or("");
                format!("[{id}] [{date}] {author}:\n{body}")
            }).collect::<Vec<_>>().join("\n\n---\n\n"))
        }

        "gitlab_get_merge_request_note" => {
            let pid    = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let mr_iid = args["mr_iid"].as_u64().ok_or("Missing param: mr_iid")?;
            let note_id = args["note_id"].as_u64().ok_or("Missing param: note_id")?;
            let url    = cfg.api(&format!("projects/{}/merge_requests/{}/notes/{}", encode_pid(pid), mr_iid, note_id));
            let n      = get(&token, &url).await?;
            let author = n["author"]["name"].as_str().unwrap_or("?");
            let date   = n["created_at"].as_str().unwrap_or("?");
            let body   = n["body"].as_str().unwrap_or("");
            Ok(format!("[{note_id}] [{date}] {author}:\n{body}"))
        }

        "gitlab_create_merge_request_note" => {
            let pid    = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let mr_iid = args["mr_iid"].as_u64().ok_or("Missing param: mr_iid")?;
            let body   = args["body"].as_str().ok_or("Missing param: body")?;
            let url    = cfg.api(&format!("projects/{}/merge_requests/{}/notes", encode_pid(pid), mr_iid));
            let data   = post(&token, &url, json!({"body": body})).await?;
            let id     = data["id"].as_u64().unwrap_or(0);
            Ok(format!("✅ Note créée : id={id}"))
        }

        "gitlab_update_merge_request_note" => {
            let pid     = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let mr_iid  = args["mr_iid"].as_u64().ok_or("Missing param: mr_iid")?;
            let note_id = args["note_id"].as_u64().ok_or("Missing param: note_id")?;
            let body    = args["body"].as_str().ok_or("Missing param: body")?;
            let url     = cfg.api(&format!("projects/{}/merge_requests/{}/notes/{}", encode_pid(pid), mr_iid, note_id));
            put(&token, &url, json!({"body": body})).await?;
            Ok(format!("✅ Note {note_id} mise à jour."))
        }

        "gitlab_delete_merge_request_note" => {
            let pid     = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let mr_iid  = args["mr_iid"].as_u64().ok_or("Missing param: mr_iid")?;
            let note_id = args["note_id"].as_u64().ok_or("Missing param: note_id")?;
            let url     = cfg.api(&format!("projects/{}/merge_requests/{}/notes/{}", encode_pid(pid), mr_iid, note_id));
            delete(&token, &url).await?;
            Ok(format!("✅ Note {note_id} supprimée."))
        }

        // ── MR — Discussion notes ─────────────────────────────────────────────

        "gitlab_create_merge_request_discussion_note" => {
            let pid           = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let mr_iid        = args["mr_iid"].as_u64().ok_or("Missing param: mr_iid")?;
            let discussion_id = args["discussion_id"].as_str().ok_or("Missing param: discussion_id")?;
            let body          = args["body"].as_str().ok_or("Missing param: body")?;
            let url           = cfg.api(&format!("projects/{}/merge_requests/{}/discussions/{}/notes", encode_pid(pid), mr_iid, discussion_id));
            let data          = post(&token, &url, json!({"body": body})).await?;
            let id            = data["id"].as_u64().unwrap_or(0);
            Ok(format!("✅ Note ajoutée au thread : id={id}"))
        }

        "gitlab_update_merge_request_discussion_note" => {
            let pid           = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let mr_iid        = args["mr_iid"].as_u64().ok_or("Missing param: mr_iid")?;
            let discussion_id = args["discussion_id"].as_str().ok_or("Missing param: discussion_id")?;
            let note_id       = args["note_id"].as_u64().ok_or("Missing param: note_id")?;
            let url           = cfg.api(&format!("projects/{}/merge_requests/{}/discussions/{}/notes/{}", encode_pid(pid), mr_iid, discussion_id, note_id));
            let mut payload   = json!({});
            if let Some(v) = args["body"].as_str()      { payload["body"]     = json!(v); }
            if let Some(v) = args["resolved"].as_bool() { payload["resolved"] = json!(v); }
            put(&token, &url, payload).await?;
            Ok(format!("✅ Note {note_id} du thread mise à jour."))
        }

        "gitlab_delete_merge_request_discussion_note" => {
            let pid           = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let mr_iid        = args["mr_iid"].as_u64().ok_or("Missing param: mr_iid")?;
            let discussion_id = args["discussion_id"].as_str().ok_or("Missing param: discussion_id")?;
            let note_id       = args["note_id"].as_u64().ok_or("Missing param: note_id")?;
            let url           = cfg.api(&format!("projects/{}/merge_requests/{}/discussions/{}/notes/{}", encode_pid(pid), mr_iid, discussion_id, note_id));
            delete(&token, &url).await?;
            Ok(format!("✅ Note {note_id} du thread supprimée."))
        }

        // ── MR — Versions ─────────────────────────────────────────────────────

        "gitlab_list_merge_request_versions" => {
            let pid    = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let mr_iid = args["mr_iid"].as_u64().ok_or("Missing param: mr_iid")?;
            let url    = cfg.api(&format!("projects/{}/merge_requests/{}/versions", encode_pid(pid), mr_iid));
            let data   = get(&token, &url).await?;
            let vers   = data.as_array().cloned().unwrap_or_default();
            if vers.is_empty() { return Ok("Aucune version de diff.".to_string()); }
            Ok(vers.iter().map(|v| {
                let id      = v["id"].as_u64().unwrap_or(0);
                let head    = v["head_commit_sha"].as_str().unwrap_or("?").get(..8).unwrap_or("?");
                let base    = v["base_commit_sha"].as_str().unwrap_or("?").get(..8).unwrap_or("?");
                let created = v["created_at"].as_str().unwrap_or("?");
                format!("[{id}] head={head} base={base}  créé={created}")
            }).collect::<Vec<_>>().join("\n"))
        }

        "gitlab_get_merge_request_version" => {
            let pid        = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let mr_iid     = args["mr_iid"].as_u64().ok_or("Missing param: mr_iid")?;
            let version_id = args["version_id"].as_u64().ok_or("Missing param: version_id")?;
            let url        = cfg.api(&format!("projects/{}/merge_requests/{}/versions/{}", encode_pid(pid), mr_iid, version_id));
            let data       = get(&token, &url).await?;
            Ok(serde_json::to_string_pretty(&data).unwrap_or_default())
        }

        "gitlab_get_merge_request_file_diff" => {
            let pid        = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let mr_iid     = args["mr_iid"].as_u64().ok_or("Missing param: mr_iid")?;
            let version_id = args["version_id"].as_u64().ok_or("Missing param: version_id")?;
            let file_path  = args["file_path"].as_str().ok_or("Missing param: file_path")?;
            let url        = cfg.api(&format!("projects/{}/merge_requests/{}/versions/{}", encode_pid(pid), mr_iid, version_id));
            let data       = get(&token, &url).await?;
            let diffs      = data["diffs"].as_array().cloned().unwrap_or_default();
            if let Some(d) = diffs.iter().find(|d| {
                d["new_path"].as_str().unwrap_or("") == file_path ||
                d["old_path"].as_str().unwrap_or("") == file_path
            }) {
                let diff = d["diff"].as_str().unwrap_or("(aucun diff)");
                Ok(format!("--- {file_path} ---\n{diff}"))
            } else {
                Ok(format!("Fichier '{file_path}' non trouvé dans cette version de diff."))
            }
        }

        "gitlab_mr_discussions" => {
            let pid    = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let mr_iid = args["mr_iid"].as_u64().ok_or("Missing param: mr_iid")?;
            let url    = cfg.api(&format!("projects/{}/merge_requests/{}/discussions?per_page=50", encode_pid(pid), mr_iid));
            let data   = get(&token, &url).await?;
            let threads = data.as_array().cloned().unwrap_or_default();
            if threads.is_empty() { return Ok("Aucune discussion.".to_string()); }
            Ok(threads.iter().map(|t| {
                let did   = t["id"].as_str().unwrap_or("?");
                let notes = t["notes"].as_array().cloned().unwrap_or_default();
                let first = notes.first().map(|n| {
                    let author = n["author"]["name"].as_str().unwrap_or("?");
                    let body   = n["body"].as_str().unwrap_or("").chars().take(200).collect::<String>();
                    format!("{author}: {body}")
                }).unwrap_or_default();
                let resolved = t["resolved"].as_bool().unwrap_or(false);
                let flag = if resolved { " [résolu]" } else { "" };
                format!("[{did}]{flag} {first}")
            }).collect::<Vec<_>>().join("\n\n"))
        }

        // ── Draft notes ───────────────────────────────────────────────────────

        "gitlab_create_draft_note" => {
            let pid    = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let mr_iid = args["mr_iid"].as_u64().ok_or("Missing param: mr_iid")?;
            let note   = args["note"].as_str().ok_or("Missing param: note")?;
            let url    = cfg.api(&format!("projects/{}/merge_requests/{}/draft_notes", encode_pid(pid), mr_iid));
            let mut payload = json!({"note": note});
            if let Some(pos) = args.get("position") { payload["position"] = pos.clone(); }
            let data   = post(&token, &url, payload).await?;
            let id     = data["id"].as_u64().unwrap_or(0);
            Ok(format!("✅ Note brouillon créée : id={id}"))
        }

        "gitlab_list_draft_notes" => {
            let pid    = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let mr_iid = args["mr_iid"].as_u64().ok_or("Missing param: mr_iid")?;
            let url    = cfg.api(&format!("projects/{}/merge_requests/{}/draft_notes", encode_pid(pid), mr_iid));
            let data   = get(&token, &url).await?;
            let notes  = data.as_array().cloned().unwrap_or_default();
            if notes.is_empty() { return Ok("Aucune note brouillon.".to_string()); }
            Ok(notes.iter().map(|n| {
                let id   = n["id"].as_u64().unwrap_or(0);
                let note = n["note"].as_str().unwrap_or("").chars().take(200).collect::<String>();
                format!("[{id}] {note}")
            }).collect::<Vec<_>>().join("\n\n"))
        }

        "gitlab_get_draft_note" => {
            let pid          = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let mr_iid       = args["mr_iid"].as_u64().ok_or("Missing param: mr_iid")?;
            let draft_note_id = args["draft_note_id"].as_u64().ok_or("Missing param: draft_note_id")?;
            let url           = cfg.api(&format!("projects/{}/merge_requests/{}/draft_notes/{}", encode_pid(pid), mr_iid, draft_note_id));
            let n             = get(&token, &url).await?;
            let note          = n["note"].as_str().unwrap_or("—");
            Ok(format!("[{draft_note_id}] {note}"))
        }

        "gitlab_update_draft_note" => {
            let pid           = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let mr_iid        = args["mr_iid"].as_u64().ok_or("Missing param: mr_iid")?;
            let draft_note_id = args["draft_note_id"].as_u64().ok_or("Missing param: draft_note_id")?;
            let note          = args["note"].as_str().ok_or("Missing param: note")?;
            let url           = cfg.api(&format!("projects/{}/merge_requests/{}/draft_notes/{}", encode_pid(pid), mr_iid, draft_note_id));
            put(&token, &url, json!({"note": note})).await?;
            Ok(format!("✅ Note brouillon {draft_note_id} mise à jour."))
        }

        "gitlab_delete_draft_note" => {
            let pid           = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let mr_iid        = args["mr_iid"].as_u64().ok_or("Missing param: mr_iid")?;
            let draft_note_id = args["draft_note_id"].as_u64().ok_or("Missing param: draft_note_id")?;
            let url           = cfg.api(&format!("projects/{}/merge_requests/{}/draft_notes/{}", encode_pid(pid), mr_iid, draft_note_id));
            delete(&token, &url).await?;
            Ok(format!("✅ Note brouillon {draft_note_id} supprimée."))
        }

        "gitlab_publish_draft_note" => {
            let pid           = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let mr_iid        = args["mr_iid"].as_u64().ok_or("Missing param: mr_iid")?;
            let draft_note_id = args["draft_note_id"].as_u64().ok_or("Missing param: draft_note_id")?;
            let url           = cfg.api(&format!("projects/{}/merge_requests/{}/draft_notes/{}/publish", encode_pid(pid), mr_iid, draft_note_id));
            put(&token, &url, json!({})).await?;
            Ok(format!("✅ Note brouillon {draft_note_id} publiée."))
        }

        "gitlab_bulk_publish_draft_notes" => {
            let pid    = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let mr_iid = args["mr_iid"].as_u64().ok_or("Missing param: mr_iid")?;
            let url    = cfg.api(&format!("projects/{}/merge_requests/{}/draft_notes/bulk_publish", encode_pid(pid), mr_iid));
            post(&token, &url, json!({})).await?;
            Ok("✅ Toutes les notes brouillon ont été publiées.".to_string())
        }

        // ── Repository — Branches & Commits ───────────────────────────────────

        "gitlab_create_branch" => {
            let pid    = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let branch = args["branch"].as_str().ok_or("Missing param: branch")?;
            let ref_   = args["ref"].as_str().ok_or("Missing param: ref")?;
            let url    = cfg.api(&format!("projects/{}/repository/branches", encode_pid(pid)));
            let data   = post(&token, &url, json!({"branch": branch, "ref": ref_})).await?;
            let name   = data["name"].as_str().unwrap_or(branch);
            let sha    = data["commit"]["id"].as_str().unwrap_or("?").get(..8).unwrap_or("?");
            Ok(format!("✅ Branche '{name}' créée depuis '{ref_}' (sha={sha})"))
        }

        "gitlab_get_branch_diffs" => {
            let pid      = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let from     = args["from"].as_str().ok_or("Missing param: from")?;
            let to       = args["to"].as_str().ok_or("Missing param: to")?;
            let straight = args["straight"].as_bool().unwrap_or(false);
            let url      = cfg.api(&format!(
                "projects/{}/repository/compare?from={}&to={}&straight={}",
                encode_pid(pid), urlencoding::encode(from), urlencoding::encode(to), straight
            ));
            let data  = get(&token, &url).await?;
            let diffs = data["diffs"].as_array().cloned().unwrap_or_default();
            let commits_count = data["commits"].as_array().map(|a| a.len()).unwrap_or(0);
            if diffs.is_empty() { return Ok(format!("Aucun diff entre '{from}' et '{to}'.  ({commits_count} commits)")); }
            let diff_summary = diffs.iter().map(|d| {
                let path    = d["new_path"].as_str().unwrap_or(d["old_path"].as_str().unwrap_or("?"));
                let new_    = d["new_file"].as_bool().unwrap_or(false);
                let deleted = d["deleted_file"].as_bool().unwrap_or(false);
                let renamed = d["renamed_file"].as_bool().unwrap_or(false);
                let mut flags = vec![];
                if new_ { flags.push("new"); }
                if deleted { flags.push("deleted"); }
                if renamed { flags.push("renamed"); }
                let flag_str = if flags.is_empty() { String::new() } else { format!(" [{}]", flags.join(",")) };
                format!("{path}{flag_str}")
            }).collect::<Vec<_>>().join("\n");
            Ok(format!("Comparaison {from}...{to} ({commits_count} commits, {} fichiers modifiés)\n\n{diff_summary}", diffs.len()))
        }

        "gitlab_list_commits" => {
            let pid   = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let limit = args["limit"].as_u64().unwrap_or(20);
            let mut url = cfg.api(&format!("projects/{}/repository/commits?per_page={}", encode_pid(pid), limit));
            if let Some(v) = args["ref_name"].as_str() { url.push_str(&format!("&ref_name={}", urlencoding::encode(v))); }
            if let Some(v) = args["since"].as_str()    { url.push_str(&format!("&since={}", urlencoding::encode(v))); }
            if let Some(v) = args["until"].as_str()    { url.push_str(&format!("&until={}", urlencoding::encode(v))); }
            let data    = get(&token, &url).await?;
            let commits = data.as_array().cloned().unwrap_or_default();
            if commits.is_empty() { return Ok("Aucun commit.".to_string()); }
            Ok(commits.iter().map(|c| {
                let sha     = c["short_id"].as_str().unwrap_or("?");
                let author  = c["author_name"].as_str().unwrap_or("?");
                let date    = c["authored_date"].as_str().unwrap_or("?").get(..10).unwrap_or("?");
                let message = c["title"].as_str().unwrap_or("?");
                format!("[{sha}] {date} {author}: {message}")
            }).collect::<Vec<_>>().join("\n"))
        }

        "gitlab_push_files" => {
            let pid            = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let branch         = args["branch"].as_str().ok_or("Missing param: branch")?;
            let commit_message = args["commit_message"].as_str().ok_or("Missing param: commit_message")?;
            let actions        = args["actions"].as_array().ok_or("Missing param: actions")?;
            let url            = cfg.api(&format!("projects/{}/repository/commits", encode_pid(pid)));
            let body           = json!({
                "branch":         branch,
                "commit_message": commit_message,
                "actions":        actions
            });
            let data = post(&token, &url, body).await?;
            let sha  = data["id"].as_str().unwrap_or("?").get(..8).unwrap_or("?");
            let msg  = data["title"].as_str().unwrap_or(commit_message);
            Ok(format!("✅ Commit créé : {sha} — {msg}"))
        }

        // ── Repository — Projects ─────────────────────────────────────────────

        "gitlab_fork_repository" => {
            let pid  = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let url  = cfg.api(&format!("projects/{}/fork", encode_pid(pid)));
            let mut body = json!({});
            if let Some(v) = args["namespace"].as_str() { body["namespace_path"] = json!(v); }
            if let Some(v) = args["name"].as_str()      { body["name"]           = json!(v); }
            if let Some(v) = args["path"].as_str()      { body["path"]           = json!(v); }
            let data = post(&token, &url, body).await?;
            let name = data["path_with_namespace"].as_str().unwrap_or("?");
            let wurl = data["web_url"].as_str().unwrap_or("");
            Ok(format!("✅ Fork créé : {name}\n{wurl}"))
        }

        "gitlab_create_repository" => {
            let name = args["name"].as_str().ok_or("Missing param: name")?;
            let url  = cfg.api("projects");
            let mut body = json!({"name": name});
            if let Some(v) = args["path"].as_str()           { body["path"]                    = json!(v); }
            if let Some(v) = args["namespace_id"].as_u64()   { body["namespace_id"]             = json!(v); }
            if let Some(v) = args["description"].as_str()    { body["description"]              = json!(v); }
            if let Some(v) = args["visibility"].as_str()     { body["visibility"]               = json!(v); }
            if let Some(v) = args["initialize_with_readme"].as_bool() { body["initialize_with_readme"] = json!(v); }
            let data = post(&token, &url, body).await?;
            let pname = data["path_with_namespace"].as_str().unwrap_or("?");
            let wurl  = data["web_url"].as_str().unwrap_or("");
            Ok(format!("✅ Projet créé : {pname}\n{wurl}"))
        }

        "gitlab_search_repositories" => {
            let search = args["search"].as_str().ok_or("Missing param: search")?;
            let limit  = args["limit"].as_u64().unwrap_or(20);
            let url    = cfg.api(&format!("projects?search={}&per_page={}&membership=false", urlencoding::encode(search), limit));
            let data   = get(&token, &url).await?;
            let projects = data.as_array().cloned().unwrap_or_default();
            if projects.is_empty() { return Ok("Aucun projet trouvé.".to_string()); }
            Ok(projects.iter().map(|p| {
                let name = p["path_with_namespace"].as_str().unwrap_or("?");
                let id   = p["id"].as_u64().unwrap_or(0);
                let vis  = p["visibility"].as_str().unwrap_or("?");
                format!("{name}  id={id}  visibility={vis}")
            }).collect::<Vec<_>>().join("\n"))
        }

        // ── Jobs — extras ─────────────────────────────────────────────────────

        "gitlab_get_pipeline_job" => {
            let pid    = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let job_id = args["job_id"].as_u64().ok_or("Missing param: job_id")?;
            let url    = cfg.api(&format!("projects/{}/jobs/{}", encode_pid(pid), job_id));
            let j      = get(&token, &url).await?;
            let name     = j["name"].as_str().unwrap_or("?");
            let status   = j["status"].as_str().unwrap_or("?");
            let stage    = j["stage"].as_str().unwrap_or("?");
            let duration = j["duration"].as_f64().unwrap_or(0.0) as u64;
            let created  = j["created_at"].as_str().unwrap_or("?");
            let ref_     = j["ref"].as_str().unwrap_or("?");
            let wurl     = j["web_url"].as_str().unwrap_or("");
            Ok(format!("Job [{job_id}] {stage}/{name}\nStatut: {status} · Durée: {duration}s\nRef: {ref_} · Créé: {created}\n{wurl}"))
        }

        "gitlab_cancel_pipeline_job" => {
            let pid    = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let job_id = args["job_id"].as_u64().ok_or("Missing param: job_id")?;
            let url    = cfg.api(&format!("projects/{}/jobs/{}/cancel", encode_pid(pid), job_id));
            post(&token, &url, json!({})).await?;
            Ok(format!("✅ Job {job_id} annulé."))
        }

        "gitlab_retry_pipeline_job" => {
            let pid    = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let job_id = args["job_id"].as_u64().ok_or("Missing param: job_id")?;
            let url    = cfg.api(&format!("projects/{}/jobs/{}/retry", encode_pid(pid), job_id));
            let data   = post(&token, &url, json!({})).await?;
            let new_id = data["id"].as_u64().unwrap_or(0);
            Ok(format!("✅ Job relancé — nouveau id={new_id}"))
        }

        "gitlab_list_job_artifacts" => {
            let pid    = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let job_id = args["job_id"].as_u64().ok_or("Missing param: job_id")?;
            let url    = cfg.api(&format!("projects/{}/jobs/{}", encode_pid(pid), job_id));
            let j      = get(&token, &url).await?;
            let artifacts = j["artifacts"].as_array().cloned().unwrap_or_default();
            if artifacts.is_empty() { return Ok("Aucun artefact pour ce job.".to_string()); }
            Ok(artifacts.iter().map(|a| {
                let file_type = a["file_type"].as_str().unwrap_or("?");
                let size      = a["size"].as_u64().unwrap_or(0);
                let filename  = a["filename"].as_str().unwrap_or("?");
                format!("{filename}  type={file_type}  taille={size}o")
            }).collect::<Vec<_>>().join("\n"))
        }

        "gitlab_get_job_artifact_file" => {
            let pid           = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let job_id        = args["job_id"].as_u64().ok_or("Missing param: job_id")?;
            let artifact_path = args["artifact_path"].as_str().ok_or("Missing param: artifact_path")?;
            let url           = cfg.api(&format!("projects/{}/jobs/{}/artifacts/{}", encode_pid(pid), job_id, urlencoding::encode(artifact_path)));
            let content       = get_text(&token, &url).await?;
            let out = if content.len() > 8000 {
                format!("...[tronqué]\n{}", &content[content.len()-8000..])
            } else {
                content
            };
            Ok(out)
        }

        "gitlab_download_job_artifacts" => {
            let pid    = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let job_id = args["job_id"].as_u64().ok_or("Missing param: job_id")?;
            let url    = cfg.api(&format!("projects/{}/jobs/{}/artifacts", encode_pid(pid), job_id));
            Ok(format!("URL de téléchargement des artefacts (job {job_id}): {url}"))
        }

        "gitlab_list_pipeline_trigger_jobs" => {
            let pid  = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let url  = cfg.api(&format!("projects/{}/triggers?per_page=50", encode_pid(pid)));
            let data = get(&token, &url).await?;
            let triggers = data.as_array().cloned().unwrap_or_default();
            if triggers.is_empty() { return Ok("Aucun déclencheur configuré.".to_string()); }
            Ok(triggers.iter().map(|t| {
                let id          = t["id"].as_u64().unwrap_or(0);
                let description = t["description"].as_str().unwrap_or("—");
                let owner       = t["owner"]["name"].as_str().unwrap_or("?");
                let token_str   = t["token"].as_str().unwrap_or("?").get(..8).unwrap_or("?");
                format!("[{id}] {description}  owner={owner}  token={token_str}...")
            }).collect::<Vec<_>>().join("\n"))
        }

        "gitlab_create_pipeline" => {
            let pid  = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let ref_ = args["ref"].as_str().ok_or("Missing param: ref")?;
            let url  = cfg.api(&format!("projects/{}/pipeline", encode_pid(pid)));
            let mut body = json!({"ref": ref_});
            if let Some(vars) = args["variables"].as_array() { body["variables"] = json!(vars); }
            let data   = post(&token, &url, body).await?;
            let id     = data["id"].as_u64().unwrap_or(0);
            let status = data["status"].as_str().unwrap_or("?");
            let wurl   = data["web_url"].as_str().unwrap_or("");
            Ok(format!("✅ Pipeline {id} déclenché sur '{ref_}'\nStatut: {status}\n{wurl}"))
        }

        // ── Issues — extras ───────────────────────────────────────────────────

        "gitlab_delete_issue" => {
            let pid = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let iid = args["issue_iid"].as_u64().ok_or("Missing param: issue_iid")?;
            let url = cfg.api(&format!("projects/{}/issues/{}", encode_pid(pid), iid));
            delete(&token, &url).await?;
            Ok(format!("✅ Issue #{iid} supprimée."))
        }

        "gitlab_create_issue_note" => {
            let pid  = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let iid  = args["issue_iid"].as_u64().ok_or("Missing param: issue_iid")?;
            let body = args["body"].as_str().ok_or("Missing param: body")?;
            let url  = cfg.api(&format!("projects/{}/issues/{}/notes", encode_pid(pid), iid));
            let data = post(&token, &url, json!({"body": body})).await?;
            let id   = data["id"].as_u64().unwrap_or(0);
            Ok(format!("✅ Note créée : id={id}"))
        }

        "gitlab_update_issue_note" => {
            let pid     = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let iid     = args["issue_iid"].as_u64().ok_or("Missing param: issue_iid")?;
            let note_id = args["note_id"].as_u64().ok_or("Missing param: note_id")?;
            let body    = args["body"].as_str().ok_or("Missing param: body")?;
            let url     = cfg.api(&format!("projects/{}/issues/{}/notes/{}", encode_pid(pid), iid, note_id));
            put(&token, &url, json!({"body": body})).await?;
            Ok(format!("✅ Note {note_id} mise à jour."))
        }

        "gitlab_create_issue_link" => {
            let pid               = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let iid               = args["issue_iid"].as_u64().ok_or("Missing param: issue_iid")?;
            let target_project_id = args["target_project_id"].as_str().ok_or("Missing param: target_project_id")?;
            let target_issue_iid  = args["target_issue_iid"].as_u64().ok_or("Missing param: target_issue_iid")?;
            let link_type         = args["link_type"].as_str().unwrap_or("relates_to");
            let url               = cfg.api(&format!("projects/{}/issues/{}/links", encode_pid(pid), iid));
            let body              = json!({
                "target_project_id": target_project_id,
                "target_issue_iid":  target_issue_iid,
                "link_type":         link_type
            });
            post(&token, &url, body).await?;
            Ok(format!("✅ Lien '{link_type}' créé entre #{iid} et #{target_issue_iid}"))
        }

        "gitlab_list_issue_links" => {
            let pid  = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let iid  = args["issue_iid"].as_u64().ok_or("Missing param: issue_iid")?;
            let url  = cfg.api(&format!("projects/{}/issues/{}/links", encode_pid(pid), iid));
            let data = get(&token, &url).await?;
            let links = data.as_array().cloned().unwrap_or_default();
            if links.is_empty() { return Ok("Aucun lien d'issue.".to_string()); }
            Ok(links.iter().map(|l| {
                let link_id   = l["id"].as_u64().unwrap_or(0);
                let target_id = l["iid"].as_u64().unwrap_or(0);
                let title     = l["title"].as_str().unwrap_or("?");
                let link_type = l["link_type"].as_str().unwrap_or("?");
                format!("[{link_id}] #{target_id} {title}  type={link_type}")
            }).collect::<Vec<_>>().join("\n"))
        }

        "gitlab_get_issue_link" => {
            let pid     = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let iid     = args["issue_iid"].as_u64().ok_or("Missing param: issue_iid")?;
            let link_id = args["link_id"].as_u64().ok_or("Missing param: link_id")?;
            let url     = cfg.api(&format!("projects/{}/issues/{}/links/{}", encode_pid(pid), iid, link_id));
            let data    = get(&token, &url).await?;
            Ok(serde_json::to_string_pretty(&data).unwrap_or_default())
        }

        "gitlab_delete_issue_link" => {
            let pid     = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let iid     = args["issue_iid"].as_u64().ok_or("Missing param: issue_iid")?;
            let link_id = args["link_id"].as_u64().ok_or("Missing param: link_id")?;
            let url     = cfg.api(&format!("projects/{}/issues/{}/links/{}", encode_pid(pid), iid, link_id));
            delete(&token, &url).await?;
            Ok(format!("✅ Lien {link_id} supprimé."))
        }

        "gitlab_my_issues" => {
            let state = args["state"].as_str().unwrap_or("opened");
            let limit = args["limit"].as_u64().unwrap_or(20);
            let url   = cfg.api(&format!("issues?scope=assigned_to_me&state={}&per_page={}", state, limit));
            let data  = get(&token, &url).await?;
            let issues = data.as_array().cloned().unwrap_or_default();
            Ok(fmt_issues(&issues))
        }

        "gitlab_create_note" => {
            let pid  = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let iid  = args["issue_iid"].as_u64().ok_or("Missing param: issue_iid")?;
            let body = args["body"].as_str().ok_or("Missing param: body")?;
            let url  = cfg.api(&format!("projects/{}/issues/{}/notes", encode_pid(pid), iid));
            let data = post(&token, &url, json!({"body": body})).await?;
            let id   = data["id"].as_u64().unwrap_or(0);
            Ok(format!("✅ Note créée : id={id}"))
        }

        // ── Users & Namespaces ────────────────────────────────────────────────

        "gitlab_get_users" => {
            let search = args["search"].as_str().ok_or("Missing param: search")?;
            let limit  = args["limit"].as_u64().unwrap_or(20);
            let url    = cfg.api(&format!("users?search={}&per_page={}", urlencoding::encode(search), limit));
            let data   = get(&token, &url).await?;
            let users  = data.as_array().cloned().unwrap_or_default();
            if users.is_empty() { return Ok("Aucun utilisateur trouvé.".to_string()); }
            Ok(users.iter().map(|u| {
                let name     = u["name"].as_str().unwrap_or("?");
                let username = u["username"].as_str().unwrap_or("?");
                let id       = u["id"].as_u64().unwrap_or(0);
                let state    = u["state"].as_str().unwrap_or("?");
                format!("{name} (@{username})  id={id}  state={state}")
            }).collect::<Vec<_>>().join("\n"))
        }

        "gitlab_list_project_members" => {
            let pid     = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let url     = cfg.api(&format!("projects/{}/members/all?per_page=100", encode_pid(pid)));
            let data    = get(&token, &url).await?;
            let members = data.as_array().cloned().unwrap_or_default();
            if members.is_empty() { return Ok("Aucun membre.".to_string()); }
            Ok(members.iter().map(|m| {
                let name     = m["name"].as_str().unwrap_or("?");
                let username = m["username"].as_str().unwrap_or("?");
                let id       = m["id"].as_u64().unwrap_or(0);
                format!("{name} (@{username})  id={id}")
            }).collect::<Vec<_>>().join("\n"))
        }

        "gitlab_list_namespaces" => {
            let limit = args["limit"].as_u64().unwrap_or(50);
            let mut url = cfg.api(&format!("namespaces?per_page={}", limit));
            if let Some(s) = args["search"].as_str() { url.push_str(&format!("&search={}", urlencoding::encode(s))); }
            let data       = get(&token, &url).await?;
            let namespaces = data.as_array().cloned().unwrap_or_default();
            if namespaces.is_empty() { return Ok("Aucun namespace.".to_string()); }
            Ok(namespaces.iter().map(|n| {
                let id   = n["id"].as_u64().unwrap_or(0);
                let name = n["name"].as_str().unwrap_or("?");
                let path = n["full_path"].as_str().unwrap_or("?");
                let kind = n["kind"].as_str().unwrap_or("?");
                format!("[{id}] {name}  path={path}  kind={kind}")
            }).collect::<Vec<_>>().join("\n"))
        }

        "gitlab_verify_namespace" => {
            let ns   = args["namespace"].as_str().ok_or("Missing param: namespace")?;
            let url  = cfg.api(&format!("namespaces/{}", urlencoding::encode(ns)));
            let data = get(&token, &url).await?;
            let name = data["name"].as_str().unwrap_or("?");
            let kind = data["kind"].as_str().unwrap_or("?");
            let path = data["full_path"].as_str().unwrap_or("?");
            let id   = data["id"].as_u64().unwrap_or(0);
            Ok(format!("Namespace '{ns}' existe.\n{name} ({kind}) · chemin={path} · id={id}"))
        }

        // ── Labels — extras ───────────────────────────────────────────────────

        "gitlab_create_label" => {
            let pid   = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let name  = args["name"].as_str().ok_or("Missing param: name")?;
            let color = args["color"].as_str().ok_or("Missing param: color")?;
            let url   = cfg.api(&format!("projects/{}/labels", encode_pid(pid)));
            let mut body = json!({"name": name, "color": color});
            if let Some(v) = args["description"].as_str() { body["description"] = json!(v); }
            let data = post(&token, &url, body).await?;
            let id   = data["id"].as_u64().unwrap_or(0);
            Ok(format!("✅ Label '{name}' créé : id={id}"))
        }

        "gitlab_get_label" => {
            let pid      = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let label_id = args["label_id"].as_u64().ok_or("Missing param: label_id")?;
            let url      = cfg.api(&format!("projects/{}/labels/{}", encode_pid(pid), label_id));
            let l        = get(&token, &url).await?;
            let name     = l["name"].as_str().unwrap_or("?");
            let color    = l["color"].as_str().unwrap_or("?");
            let desc     = l["description"].as_str().unwrap_or("—");
            Ok(format!("[{label_id}] {name}  couleur={color}\n{desc}"))
        }

        "gitlab_update_label" => {
            let pid      = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let label_id = args["label_id"].as_u64().ok_or("Missing param: label_id")?;
            let url      = cfg.api(&format!("projects/{}/labels/{}", encode_pid(pid), label_id));
            let mut body = json!({});
            if let Some(v) = args["name"].as_str()        { body["new_name"]    = json!(v); }
            if let Some(v) = args["color"].as_str()       { body["color"]       = json!(v); }
            if let Some(v) = args["description"].as_str() { body["description"] = json!(v); }
            put(&token, &url, body).await?;
            Ok(format!("✅ Label {label_id} mis à jour."))
        }

        "gitlab_delete_label" => {
            let pid      = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let label_id = args["label_id"].as_u64().ok_or("Missing param: label_id")?;
            let url      = cfg.api(&format!("projects/{}/labels/{}", encode_pid(pid), label_id));
            delete(&token, &url).await?;
            Ok(format!("✅ Label {label_id} supprimé."))
        }

        // ── Milestones — extras ───────────────────────────────────────────────

        "gitlab_delete_milestone" => {
            let pid          = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let milestone_id = args["milestone_id"].as_u64().ok_or("Missing param: milestone_id")?;
            let url          = cfg.api(&format!("projects/{}/milestones/{}", encode_pid(pid), milestone_id));
            delete(&token, &url).await?;
            Ok(format!("✅ Milestone {milestone_id} supprimé."))
        }

        "gitlab_promote_milestone" => {
            let pid          = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let milestone_id = args["milestone_id"].as_u64().ok_or("Missing param: milestone_id")?;
            let url          = cfg.api(&format!("projects/{}/milestones/{}/promote", encode_pid(pid), milestone_id));
            post(&token, &url, json!({})).await?;
            Ok(format!("✅ Milestone {milestone_id} promu en milestone de groupe."))
        }

        "gitlab_edit_milestone" => {
            let pid          = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let milestone_id = args["milestone_id"].as_u64().ok_or("Missing param: milestone_id")?;
            let url          = cfg.api(&format!("projects/{}/milestones/{}", encode_pid(pid), milestone_id));
            let mut body = json!({});
            if let Some(v) = args["title"].as_str()       { body["title"]       = json!(v); }
            if let Some(v) = args["description"].as_str() { body["description"] = json!(v); }
            if let Some(v) = args["due_date"].as_str()    { body["due_date"]    = json!(v); }
            if let Some(v) = args["state_event"].as_str() { body["state_event"] = json!(v); }
            put(&token, &url, body).await?;
            Ok(format!("✅ Milestone {milestone_id} mis à jour."))
        }

        "gitlab_get_milestone_burndown_events" => {
            let pid          = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let milestone_id = args["milestone_id"].as_u64().ok_or("Missing param: milestone_id")?;
            let url          = cfg.api(&format!("projects/{}/milestones/{}/burndown_events", encode_pid(pid), milestone_id));
            let data         = get(&token, &url).await?;
            Ok(serde_json::to_string_pretty(&data).unwrap_or_default())
        }

        "gitlab_get_milestone_issue" => {
            let pid          = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let milestone_id = args["milestone_id"].as_u64().ok_or("Missing param: milestone_id")?;
            let url          = cfg.api(&format!("projects/{}/milestones/{}/issues?per_page=50", encode_pid(pid), milestone_id));
            let data         = get(&token, &url).await?;
            let issues       = data.as_array().cloned().unwrap_or_default();
            Ok(fmt_issues(&issues))
        }

        "gitlab_get_milestone_merge_requests" => {
            let pid          = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let milestone_id = args["milestone_id"].as_u64().ok_or("Missing param: milestone_id")?;
            let url          = cfg.api(&format!("projects/{}/milestones/{}/merge_requests?per_page=50", encode_pid(pid), milestone_id));
            let data         = get(&token, &url).await?;
            let mrs          = data.as_array().cloned().unwrap_or_default();
            Ok(fmt_mrs(&mrs))
        }

        // ── Releases — extras ─────────────────────────────────────────────────

        "gitlab_delete_release" => {
            let pid      = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let tag_name = args["tag_name"].as_str().ok_or("Missing param: tag_name")?;
            let url      = cfg.api(&format!("projects/{}/releases/{}", encode_pid(pid), urlencoding::encode(tag_name)));
            delete(&token, &url).await?;
            Ok(format!("✅ Release '{tag_name}' supprimée (le tag est conservé)."))
        }

        "gitlab_update_release" => {
            let pid      = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let tag_name = args["tag_name"].as_str().ok_or("Missing param: tag_name")?;
            let url      = cfg.api(&format!("projects/{}/releases/{}", encode_pid(pid), urlencoding::encode(tag_name)));
            let mut body = json!({});
            if let Some(v) = args["name"].as_str()        { body["name"]        = json!(v); }
            if let Some(v) = args["description"].as_str() { body["description"] = json!(v); }
            put(&token, &url, body).await?;
            Ok(format!("✅ Release '{tag_name}' mise à jour."))
        }

        "gitlab_create_release_evidence" => {
            let pid      = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let tag_name = args["tag_name"].as_str().ok_or("Missing param: tag_name")?;
            let url      = cfg.api(&format!("projects/{}/releases/{}/evidence", encode_pid(pid), urlencoding::encode(tag_name)));
            post(&token, &url, json!({})).await?;
            Ok(format!("✅ Preuve créée pour la release '{tag_name}'."))
        }

        "gitlab_download_release_asset" => {
            let pid                = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let tag_name           = args["tag_name"].as_str().ok_or("Missing param: tag_name")?;
            let direct_asset_path  = args["direct_asset_path"].as_str().ok_or("Missing param: direct_asset_path")?;
            let url                = cfg.api(&format!("projects/{}/releases/{}/downloads/{}", encode_pid(pid), urlencoding::encode(tag_name), direct_asset_path));
            Ok(format!("URL de téléchargement de l'asset '{direct_asset_path}' (release {tag_name}):\n{url}"))
        }

        // ── Group wikis ───────────────────────────────────────────────────────

        "gitlab_list_group_wiki_pages" => {
            let gid  = args["group_id"].as_str().ok_or("Missing param: group_id")?;
            let url  = cfg.api(&format!("groups/{}/wikis?per_page=100", urlencoding::encode(gid)));
            let data = get(&token, &url).await?;
            let pages = data.as_array().cloned().unwrap_or_default();
            if pages.is_empty() { return Ok("Aucune page wiki de groupe.".to_string()); }
            Ok(pages.iter().map(|p| {
                let title = p["title"].as_str().unwrap_or("?");
                let slug  = p["slug"].as_str().unwrap_or("?");
                format!("{title}  slug={slug}")
            }).collect::<Vec<_>>().join("\n"))
        }

        "gitlab_get_group_wiki_page" => {
            let gid  = args["group_id"].as_str().ok_or("Missing param: group_id")?;
            let slug = args["slug"].as_str().ok_or("Missing param: slug")?;
            let url  = cfg.api(&format!("groups/{}/wikis/{}", urlencoding::encode(gid), urlencoding::encode(slug)));
            let data = get(&token, &url).await?;
            let title   = data["title"].as_str().unwrap_or("?");
            let content = data["content"].as_str().unwrap_or("—");
            let format  = data["format"].as_str().unwrap_or("?");
            Ok(format!("# {title}  (format={format})\n\n{content}"))
        }

        "gitlab_create_group_wiki_page" => {
            let gid     = args["group_id"].as_str().ok_or("Missing param: group_id")?;
            let title   = args["title"].as_str().ok_or("Missing param: title")?;
            let content = args["content"].as_str().ok_or("Missing param: content")?;
            let fmt     = args["format"].as_str().unwrap_or("markdown");
            let url     = cfg.api(&format!("groups/{}/wikis", urlencoding::encode(gid)));
            let data    = post(&token, &url, json!({"title": title, "content": content, "format": fmt})).await?;
            let slug    = data["slug"].as_str().unwrap_or("?");
            Ok(format!("✅ Page wiki de groupe créée : slug={slug}"))
        }

        "gitlab_update_group_wiki_page" => {
            let gid  = args["group_id"].as_str().ok_or("Missing param: group_id")?;
            let slug = args["slug"].as_str().ok_or("Missing param: slug")?;
            let url  = cfg.api(&format!("groups/{}/wikis/{}", urlencoding::encode(gid), urlencoding::encode(slug)));
            let mut body = json!({});
            if let Some(v) = args["title"].as_str()   { body["title"]   = json!(v); }
            if let Some(v) = args["content"].as_str() { body["content"] = json!(v); }
            if let Some(v) = args["format"].as_str()  { body["format"]  = json!(v); }
            put(&token, &url, body).await?;
            Ok(format!("✅ Page wiki de groupe '{slug}' mise à jour."))
        }

        "gitlab_delete_group_wiki_page" => {
            let gid  = args["group_id"].as_str().ok_or("Missing param: group_id")?;
            let slug = args["slug"].as_str().ok_or("Missing param: slug")?;
            let url  = cfg.api(&format!("groups/{}/wikis/{}", urlencoding::encode(gid), urlencoding::encode(slug)));
            delete(&token, &url).await?;
            Ok(format!("✅ Page wiki de groupe '{slug}' supprimée."))
        }

        // ── Group iterations ──────────────────────────────────────────────────

        "gitlab_list_group_iterations" => {
            let gid   = args["group_id"].as_str().ok_or("Missing param: group_id")?;
            let mut url = cfg.api(&format!("groups/{}/iterations?per_page=50", urlencoding::encode(gid)));
            if let Some(s) = args["state"].as_str()  { url.push_str(&format!("&state={s}")); }
            if let Some(s) = args["search"].as_str() { url.push_str(&format!("&search={}", urlencoding::encode(s))); }
            let data  = get(&token, &url).await?;
            let iters = data.as_array().cloned().unwrap_or_default();
            if iters.is_empty() { return Ok("Aucune itération.".to_string()); }
            Ok(iters.iter().map(|i| {
                let id    = i["id"].as_u64().unwrap_or(0);
                let title = i["title"].as_str().unwrap_or("?");
                let state = i["state"].as_u64().map(|s| match s { 1 => "upcoming", 2 => "current", 3 => "closed", _ => "?" }).unwrap_or("?");
                let due   = i["due_date"].as_str().unwrap_or("—");
                format!("[{id}] {title}  état={state} · échéance={due}")
            }).collect::<Vec<_>>().join("\n"))
        }

        // ── Events ────────────────────────────────────────────────────────────

        "gitlab_list_events" => {
            let limit = args["limit"].as_u64().unwrap_or(20);
            let url = if let Some(pid) = args["project_id"].as_str() {
                let mut u = cfg.api(&format!("projects/{}/events?per_page={}", encode_pid(pid), limit));
                if let Some(a) = args["action"].as_str()      { u.push_str(&format!("&action={a}")); }
                if let Some(t) = args["target_type"].as_str() { u.push_str(&format!("&target_type={t}")); }
                u
            } else {
                let mut u = cfg.api(&format!("events?per_page={}", limit));
                if let Some(a) = args["action"].as_str()      { u.push_str(&format!("&action={a}")); }
                if let Some(t) = args["target_type"].as_str() { u.push_str(&format!("&target_type={t}")); }
                u
            };
            let data   = get(&token, &url).await?;
            let events = data.as_array().cloned().unwrap_or_default();
            if events.is_empty() { return Ok("Aucun événement.".to_string()); }
            Ok(events.iter().map(|e| {
                let author  = e["author"]["name"].as_str().unwrap_or("?");
                let action  = e["action_name"].as_str().unwrap_or("?");
                let target  = e["target_type"].as_str().unwrap_or("?");
                let title   = e["target_title"].as_str().unwrap_or("");
                let date    = e["created_at"].as_str().unwrap_or("?").get(..10).unwrap_or("?");
                format!("[{date}] {author} {action} {target}: {title}")
            }).collect::<Vec<_>>().join("\n"))
        }

        "gitlab_get_project_events" => {
            let pid   = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let limit = args["limit"].as_u64().unwrap_or(20);
            let mut url = cfg.api(&format!("projects/{}/events?per_page={}", encode_pid(pid), limit));
            if let Some(a) = args["action"].as_str() { url.push_str(&format!("&action={a}")); }
            let data   = get(&token, &url).await?;
            let events = data.as_array().cloned().unwrap_or_default();
            if events.is_empty() { return Ok("Aucun événement.".to_string()); }
            Ok(events.iter().map(|e| {
                let author = e["author"]["name"].as_str().unwrap_or("?");
                let action = e["action_name"].as_str().unwrap_or("?");
                let target = e["target_type"].as_str().unwrap_or("?");
                let title  = e["target_title"].as_str().unwrap_or("");
                let date   = e["created_at"].as_str().unwrap_or("?").get(..10).unwrap_or("?");
                format!("[{date}] {author} {action} {target}: {title}")
            }).collect::<Vec<_>>().join("\n"))
        }

        // ── Upload & attachments ──────────────────────────────────────────────

        "gitlab_upload_markdown" => {
            let pid          = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let filename     = args["filename"].as_str().ok_or("Missing param: filename")?;
            let content      = args["content"].as_str().ok_or("Missing param: content")?;
            let content_type = args["content_type"].as_str().unwrap_or("text/plain");
            let url          = cfg.api(&format!("projects/{}/uploads", encode_pid(pid)));
            // Build multipart body manually (reqwest multipart feature not enabled)
            let boundary = "------------------------osmozzzboundary42";
            let body = format!(
                "--{boundary}\r\nContent-Disposition: form-data; name=\"file\"; filename=\"{filename}\"\r\nContent-Type: {content_type}\r\n\r\n{content}\r\n--{boundary}--\r\n"
            );
            let resp = reqwest::Client::new()
                .post(&url)
                .header("PRIVATE-TOKEN", &token)
                .header("Content-Type", format!("multipart/form-data; boundary={boundary}"))
                .body(body)
                .send().await.map_err(|e| e.to_string())?;
            let data      = resp.json::<Value>().await.map_err(|e| e.to_string())?;
            let markdown  = data["markdown"].as_str().unwrap_or("?");
            let full_path = data["full_path"].as_str().unwrap_or("?");
            Ok(format!("✅ Fichier uploadé.\nURL: {full_path}\nMarkdown: {markdown}"))
        }

        "gitlab_download_attachment" => {
            let _pid            = args["project_id"].as_str().ok_or("Missing param: project_id")?;
            let attachment_url  = args["attachment_url"].as_str().ok_or("Missing param: attachment_url")?;
            let content = get_text(&token, attachment_url).await?;
            let out = if content.len() > 8000 {
                format!("...[tronqué]\n{}", &content[content.len()-8000..])
            } else {
                content
            };
            Ok(out)
        }

        other => Err(format!("Unknown gitlab tool: {other}")),
    }
}

// ─── Helpers internes ─────────────────────────────────────────────────────────

fn base64_decode(s: &str) -> String {
    // Décodeur base64 simple sans dépendance externe
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut table = [0u8; 256];
    for (i, &c) in ALPHABET.iter().enumerate() { table[c as usize] = i as u8; }
    let bytes: Vec<u8> = s.bytes().collect();
    let mut out = Vec::with_capacity(bytes.len() * 3 / 4);
    let mut i = 0;
    while i + 3 < bytes.len() {
        let b0 = table[bytes[i] as usize] as u32;
        let b1 = table[bytes[i+1] as usize] as u32;
        let b2 = table[bytes[i+2] as usize] as u32;
        let b3 = table[bytes[i+3] as usize] as u32;
        let n = (b0 << 18) | (b1 << 12) | (b2 << 6) | b3;
        out.push(((n >> 16) & 0xFF) as u8);
        if bytes[i+2] != b'=' { out.push(((n >> 8) & 0xFF) as u8); }
        if bytes[i+3] != b'=' { out.push((n & 0xFF) as u8); }
        i += 4;
    }
    String::from_utf8_lossy(&out).into_owned()
}
