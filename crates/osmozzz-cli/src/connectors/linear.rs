/// Connecteur Linear — API GraphQL officielle.
/// Toutes les fonctions retournent Result<String, String> sans jamais appeler send().
/// La couche sécurité (sanitize_proxy_response + audit_log) est appliquée par le dispatcher (mcp.rs).
use serde_json::{json, Value};

// ─── Config ──────────────────────────────────────────────────────────────────

#[derive(serde::Deserialize)]
struct LinearConfig { api_key: String }

impl LinearConfig {
    fn load() -> Option<Self> {
        let path = dirs_next::home_dir()?.join(".osmozzz/linear.toml");
        toml::from_str(&std::fs::read_to_string(path).ok()?).ok()
    }
}

// ─── HTTP helper ─────────────────────────────────────────────────────────────

async fn gql(api_key: &str, query: &str, variables: Value) -> Result<Value, String> {
    let resp = reqwest::Client::new()
        .post("https://api.linear.app/graphql")
        .header("Authorization", api_key)
        .json(&json!({"query": query, "variables": variables}))
        .send().await.map_err(|e| e.to_string())?
        .json::<Value>().await.map_err(|e| e.to_string())?;
    if let Some(errs) = resp.get("errors") { return Err(errs.to_string()); }
    Ok(resp["data"].clone())
}

// ─── Formatters ──────────────────────────────────────────────────────────────

fn fmt_issues(nodes: &[Value]) -> String {
    if nodes.is_empty() { return "Aucune issue trouvée.".to_string(); }
    nodes.iter().map(|n| {
        let id    = n["identifier"].as_str().unwrap_or("?");
        let uuid  = n["id"].as_str().unwrap_or("?");
        let title = n["title"].as_str().unwrap_or("?");
        let state = n["state"]["name"].as_str().unwrap_or("?");
        let asgn  = n["assignee"]["name"].as_str().unwrap_or("—");
        let team  = n["team"]["name"].as_str().unwrap_or("?");
        format!("[{id}] {title}\n  id={uuid} · état={state} · équipe={team} · assigné={asgn}")
    }).collect::<Vec<_>>().join("\n\n")
}

fn fmt_issue(n: &Value) -> String {
    let id    = n["identifier"].as_str().unwrap_or("?");
    let title = n["title"].as_str().unwrap_or("?");
    let desc  = n["description"].as_str().unwrap_or("—");
    let state = n["state"]["name"].as_str().unwrap_or("?");
    let asgn  = n["assignee"]["name"].as_str().unwrap_or("—");
    let team  = n["team"]["name"].as_str().unwrap_or("?");
    let prio  = n["priority"].as_u64().unwrap_or(0);
    let labels: Vec<&str> = n["labels"]["nodes"].as_array().map(|a|
        a.iter().filter_map(|l| l["name"].as_str()).collect()
    ).unwrap_or_default();
    let comments_str = n["comments"]["nodes"].as_array().map(|a| {
        a.iter().map(|c| {
            let user = c["user"]["name"].as_str().unwrap_or("?");
            let body = c["body"].as_str().unwrap_or("");
            let date = c["createdAt"].as_str().unwrap_or("?");
            format!("  [{date}] {user}: {body}")
        }).collect::<Vec<_>>().join("\n")
    }).unwrap_or_default();
    let prio_str = match prio { 1 => "Urgente", 2 => "Haute", 3 => "Moyenne", 4 => "Basse", _ => "Aucune" };
    let mut out = format!(
        "[{id}] {title}\nÉtat: {state} · Équipe: {team} · Assigné: {asgn} · Priorité: {prio_str}\n"
    );
    if !labels.is_empty() { out.push_str(&format!("Labels: {}\n", labels.join(", "))); }
    out.push_str(&format!("\n{desc}"));
    if !comments_str.is_empty() { out.push_str(&format!("\n\nCommentaires:\n{comments_str}")); }
    out
}

// ─── Tool definitions ────────────────────────────────────────────────────────

pub fn tools() -> Vec<Value> {
    vec![
        json!({"name":"linear_search_issues","description":"LINEAR — Recherche des issues par mot-clé (titre, description). Retourne liste compacte avec identifiants (ex: ENG-42). Enchaîner avec linear_get_issue pour le détail complet.","inputSchema":{"type":"object","properties":{"query":{"type":"string"},"limit":{"type":"integer","default":10,"minimum":1,"maximum":50}},"required":["query"]}}),
        json!({"name":"linear_get_issue","description":"LINEAR — Récupère le détail complet d'une issue (description, commentaires, labels, état). Utilise l'UUID retourné par linear_search_issues.","inputSchema":{"type":"object","properties":{"id":{"type":"string","description":"UUID de l'issue (retourné par linear_search_issues)"}},"required":["id"]}}),
        json!({"name":"linear_create_issue","description":"LINEAR — Crée une nouvelle issue. Utiliser linear_list_teams pour obtenir le teamId requis.","inputSchema":{"type":"object","properties":{"title":{"type":"string"},"team_id":{"type":"string","description":"UUID de l'équipe (obtenu via linear_list_teams)"},"description":{"type":"string"},"assignee_id":{"type":"string"},"label_ids":{"type":"array","items":{"type":"string"}},"priority":{"type":"integer","description":"1=Urgente 2=Haute 3=Moyenne 4=Basse","minimum":0,"maximum":4},"state_id":{"type":"string","description":"UUID du statut (obtenu via linear_list_workflow_states)"}},"required":["title","team_id"]}}),
        json!({"name":"linear_update_issue","description":"LINEAR — Met à jour une issue existante (titre, description, état, priorité, assigné).","inputSchema":{"type":"object","properties":{"id":{"type":"string","description":"UUID de l'issue"},"title":{"type":"string"},"description":{"type":"string"},"state_id":{"type":"string"},"assignee_id":{"type":"string"},"priority":{"type":"integer","minimum":0,"maximum":4}},"required":["id"]}}),
        json!({"name":"linear_add_comment","description":"LINEAR — Ajoute un commentaire à une issue.","inputSchema":{"type":"object","properties":{"issue_id":{"type":"string","description":"UUID de l'issue"},"body":{"type":"string","description":"Texte du commentaire (Markdown)"}},"required":["issue_id","body"]}}),
        json!({"name":"linear_list_teams","description":"LINEAR — Liste toutes les équipes du workspace avec leurs IDs. Nécessaire pour créer des issues.","inputSchema":{"type":"object","properties":{}}}),
        json!({"name":"linear_list_issues","description":"LINEAR — Liste les issues d'une équipe (optionnel: filtrer par état). Retourne les 25 issues les plus récentes par défaut.","inputSchema":{"type":"object","properties":{"team_id":{"type":"string","description":"UUID de l'équipe (laisser vide pour toutes les équipes)"},"limit":{"type":"integer","default":25,"maximum":50}}}}),
        json!({"name":"linear_list_projects","description":"LINEAR — Liste tous les projets du workspace.","inputSchema":{"type":"object","properties":{}}}),
        json!({"name":"linear_list_workflow_states","description":"LINEAR — Liste les états/statuts disponibles pour une équipe (Ex: Todo, In Progress, Done). Nécessaire pour créer ou mettre à jour une issue.","inputSchema":{"type":"object","properties":{"team_id":{"type":"string","description":"UUID de l'équipe"}},"required":["team_id"]}}),
        json!({"name":"linear_list_labels","description":"LINEAR — Liste tous les labels disponibles dans le workspace.","inputSchema":{"type":"object","properties":{}}}),
        json!({"name":"linear_list_members","description":"LINEAR — Liste les membres du workspace avec leurs UUIDs. Nécessaire pour assigner des issues.","inputSchema":{"type":"object","properties":{}}}),
        json!({"name":"linear_archive_issue","description":"LINEAR — Archive (ferme) une issue.","inputSchema":{"type":"object","properties":{"id":{"type":"string","description":"UUID de l'issue"}},"required":["id"]}}),
        json!({"name":"linear_get_viewer","description":"LINEAR — Retourne les informations de l'utilisateur authentifié (nom, email, UUID).","inputSchema":{"type":"object","properties":{}}}),
        json!({"name":"linear_create_project","description":"LINEAR — Crée un nouveau projet dans le workspace.","inputSchema":{"type":"object","properties":{"name":{"type":"string"},"team_ids":{"type":"array","items":{"type":"string"},"description":"UUIDs des équipes associées (obtenu via linear_list_teams)"},"description":{"type":"string"},"state":{"type":"string","description":"État initial (ex: planned, started, paused, completed, cancelled)","default":"planned"}},"required":["name","team_ids"]}}),
        json!({"name":"linear_list_cycles","description":"LINEAR — Liste les cycles (sprints) d'une équipe.","inputSchema":{"type":"object","properties":{"team_id":{"type":"string","description":"UUID de l'équipe (obtenu via linear_list_teams)"}},"required":["team_id"]}}),
        json!({"name":"linear_get_cycle","description":"LINEAR — Récupère le détail d'un cycle (sprint) avec ses issues.","inputSchema":{"type":"object","properties":{"id":{"type":"string","description":"UUID du cycle (obtenu via linear_list_cycles)"}},"required":["id"]}}),
        json!({"name":"linear_delete_comment","description":"LINEAR — Supprime un commentaire d'une issue.","inputSchema":{"type":"object","properties":{"id":{"type":"string","description":"UUID du commentaire"}},"required":["id"]}}),
    ]
}

// ─── Handler ─────────────────────────────────────────────────────────────────

pub async fn handle(name: &str, args: &Value) -> Result<String, String> {
    let cfg = match LinearConfig::load() {
        Some(c) => c,
        None => return Ok("Linear non configuré (linear.toml manquant)".to_string()),
    };

    match name {
        "linear_search_issues" => {
            let query = args["query"].as_str().unwrap_or("").to_string();
            let limit = args["limit"].as_u64().unwrap_or(10) as i64;
            let q = "query($q:String!,$n:Int){searchIssues(query:$q,first:$n){nodes{id identifier title state{name}priority assignee{name}team{name}updatedAt}}}";
            let data = gql(&cfg.api_key, q, json!({"q":query,"n":limit})).await?;
            let nodes = data["searchIssues"]["nodes"].as_array().cloned().unwrap_or_default();
            Ok(fmt_issues(&nodes))
        }

        "linear_get_issue" => {
            let issue_id = args["id"].as_str().ok_or("Missing param: id")?.to_string();
            let q = "query($id:String!){issue(id:$id){id identifier title description state{name}priority assignee{name}team{name}labels{nodes{name}}createdAt updatedAt url comments{nodes{id body user{name}createdAt}}}}";
            let data = gql(&cfg.api_key, q, json!({"id":issue_id})).await?;
            Ok(fmt_issue(&data["issue"]))
        }

        "linear_create_issue" => {
            let title   = args["title"].as_str().ok_or("Missing param: title")?.to_string();
            let team_id = args["team_id"].as_str().ok_or("Missing param: team_id")?.to_string();
            let mut vars = json!({"title":title,"teamId":team_id});
            if let Some(d) = args["description"].as_str() { vars["description"] = json!(d); }
            if let Some(a) = args["assignee_id"].as_str() { vars["assigneeId"]  = json!(a); }
            if let Some(s) = args["state_id"].as_str()    { vars["stateId"]     = json!(s); }
            if let Some(p) = args["priority"].as_u64()    { vars["priority"]    = json!(p); }
            if let Some(l) = args["label_ids"].as_array() { vars["labelIds"]    = json!(l); }
            let q = "mutation($title:String!,$teamId:String!,$description:String,$assigneeId:String,$stateId:String,$priority:Int,$labelIds:[String!]){issueCreate(input:{title:$title,teamId:$teamId,description:$description,assigneeId:$assigneeId,stateId:$stateId,priority:$priority,labelIds:$labelIds}){success issue{id identifier title url}}}";
            let data  = gql(&cfg.api_key, q, vars).await?;
            let issue = &data["issueCreate"]["issue"];
            let id_str = issue["identifier"].as_str().unwrap_or("?");
            let url    = issue["url"].as_str().unwrap_or("");
            Ok(format!("✅ Issue créée : [{id_str}] {url}"))
        }

        "linear_update_issue" => {
            let issue_id = args["id"].as_str().ok_or("Missing param: id")?.to_string();
            let mut input = json!({});
            if let Some(v) = args["title"].as_str()       { input["title"]       = json!(v); }
            if let Some(v) = args["description"].as_str() { input["description"] = json!(v); }
            if let Some(v) = args["state_id"].as_str()    { input["stateId"]     = json!(v); }
            if let Some(v) = args["assignee_id"].as_str() { input["assigneeId"]  = json!(v); }
            if let Some(v) = args["priority"].as_u64()    { input["priority"]    = json!(v); }
            let q = "mutation($id:String!,$input:IssueUpdateInput!){issueUpdate(id:$id,input:$input){success issue{identifier title state{name}}}}";
            let data  = gql(&cfg.api_key, q, json!({"id":issue_id,"input":input})).await?;
            let issue = &data["issueUpdate"]["issue"];
            let iid = issue["identifier"].as_str().unwrap_or("?");
            let t   = issue["title"].as_str().unwrap_or("?");
            let s   = issue["state"]["name"].as_str().unwrap_or("?");
            Ok(format!("✅ [{iid}] {t} — état: {s}"))
        }

        "linear_add_comment" => {
            let issue_id = args["issue_id"].as_str().ok_or("Missing param: issue_id")?.to_string();
            let body     = args["body"].as_str().ok_or("Missing param: body")?.to_string();
            let q = "mutation($issueId:String!,$body:String!){commentCreate(input:{issueId:$issueId,body:$body}){success}}";
            gql(&cfg.api_key, q, json!({"issueId":issue_id,"body":body})).await?;
            Ok("✅ Commentaire ajouté.".to_string())
        }

        "linear_list_teams" => {
            let q = "query{teams{nodes{id name key description}}}";
            let data  = gql(&cfg.api_key, q, json!({})).await?;
            let teams = data["teams"]["nodes"].as_array().cloned().unwrap_or_default();
            if teams.is_empty() { return Ok("Aucune équipe.".to_string()); }
            Ok(teams.iter().map(|t| {
                let name = t["name"].as_str().unwrap_or("?");
                let key  = t["key"].as_str().unwrap_or("?");
                let tid  = t["id"].as_str().unwrap_or("?");
                format!("[{key}] {name}\n  id={tid}")
            }).collect::<Vec<_>>().join("\n\n"))
        }

        "linear_list_issues" => {
            let limit = args["limit"].as_u64().unwrap_or(25) as i64;
            let (q, vars) = if let Some(team_id) = args["team_id"].as_str() {
                ("query($teamId:String!,$n:Int){team(id:$teamId){issues(first:$n,orderBy:updatedAt){nodes{id identifier title state{name}priority assignee{name}updatedAt}}}}", json!({"teamId":team_id,"n":limit}))
            } else {
                ("query($n:Int){issues(first:$n,orderBy:updatedAt){nodes{id identifier title state{name}priority assignee{name}team{name}updatedAt}}}", json!({"n":limit}))
            };
            let data  = gql(&cfg.api_key, q, vars).await?;
            let nodes = if args["team_id"].as_str().is_some() {
                data["team"]["issues"]["nodes"].as_array().cloned().unwrap_or_default()
            } else {
                data["issues"]["nodes"].as_array().cloned().unwrap_or_default()
            };
            Ok(fmt_issues(&nodes))
        }

        "linear_list_projects" => {
            let q        = "query{projects{nodes{id name description state url}}}";
            let data     = gql(&cfg.api_key, q, json!({})).await?;
            let projects = data["projects"]["nodes"].as_array().cloned().unwrap_or_default();
            if projects.is_empty() { return Ok("Aucun projet.".to_string()); }
            Ok(projects.iter().map(|p| {
                let name  = p["name"].as_str().unwrap_or("?");
                let state = p["state"].as_str().unwrap_or("?");
                let pid   = p["id"].as_str().unwrap_or("?");
                format!("{name} [{state}]\n  id={pid}")
            }).collect::<Vec<_>>().join("\n\n"))
        }

        "linear_list_workflow_states" => {
            let team_id = args["team_id"].as_str().ok_or("Missing param: team_id")?.to_string();
            let q      = "query($teamId:String!){team(id:$teamId){states{nodes{id name type color}}}}";
            let data   = gql(&cfg.api_key, q, json!({"teamId":team_id})).await?;
            let states = data["team"]["states"]["nodes"].as_array().cloned().unwrap_or_default();
            Ok(states.iter().map(|s| {
                let name = s["name"].as_str().unwrap_or("?");
                let typ  = s["type"].as_str().unwrap_or("?");
                let sid  = s["id"].as_str().unwrap_or("?");
                format!("{name} [{typ}]\n  id={sid}")
            }).collect::<Vec<_>>().join("\n\n"))
        }

        "linear_list_labels" => {
            let q      = "query{issueLabels{nodes{id name color}}}";
            let data   = gql(&cfg.api_key, q, json!({})).await?;
            let labels = data["issueLabels"]["nodes"].as_array().cloned().unwrap_or_default();
            Ok(labels.iter().map(|l| {
                let name = l["name"].as_str().unwrap_or("?");
                let lid  = l["id"].as_str().unwrap_or("?");
                format!("{name}  id={lid}")
            }).collect::<Vec<_>>().join("\n"))
        }

        "linear_list_members" => {
            let q     = "query{users{nodes{id name email displayName}}}";
            let data  = gql(&cfg.api_key, q, json!({})).await?;
            let users = data["users"]["nodes"].as_array().cloned().unwrap_or_default();
            Ok(users.iter().map(|u| {
                let name  = u["name"].as_str().unwrap_or("?");
                let email = u["email"].as_str().unwrap_or("?");
                let uid   = u["id"].as_str().unwrap_or("?");
                format!("{name} <{email}>\n  id={uid}")
            }).collect::<Vec<_>>().join("\n\n"))
        }

        "linear_archive_issue" => {
            let issue_id = args["id"].as_str().ok_or("Missing param: id")?.to_string();
            let q        = "mutation($id:String!){issueArchive(id:$id){success}}";
            gql(&cfg.api_key, q, json!({"id":issue_id})).await?;
            Ok("✅ Issue archivée.".to_string())
        }

        "linear_get_viewer" => {
            let q    = "query{viewer{id name email displayName}}";
            let data = gql(&cfg.api_key, q, json!({})).await?;
            let v    = &data["viewer"];
            let name  = v["name"].as_str().unwrap_or("?");
            let email = v["email"].as_str().unwrap_or("?");
            let uid   = v["id"].as_str().unwrap_or("?");
            Ok(format!("{name} <{email}>\nid={uid}"))
        }

        "linear_create_project" => {
            let name     = args["name"].as_str().ok_or("Missing param: name")?.to_string();
            let team_ids = args["team_ids"].as_array().ok_or("Missing param: team_ids")?
                .iter().filter_map(|v| v.as_str().map(String::from)).collect::<Vec<_>>();
            let mut vars = json!({"name": name, "teamIds": team_ids});
            if let Some(d) = args["description"].as_str() { vars["description"] = json!(d); }
            if let Some(s) = args["state"].as_str()       { vars["state"]       = json!(s); }
            let q = "mutation($name:String!,$teamIds:[String!]!,$description:String,$state:String){projectCreate(input:{name:$name,teamIds:$teamIds,description:$description,state:$state}){success project{id name url}}}";
            let data    = gql(&cfg.api_key, q, vars).await?;
            let project = &data["projectCreate"]["project"];
            let pid  = project["id"].as_str().unwrap_or("?");
            let url  = project["url"].as_str().unwrap_or("");
            Ok(format!("✅ Projet créé : {name}\n  id={pid}\n  {url}"))
        }

        "linear_list_cycles" => {
            let team_id = args["team_id"].as_str().ok_or("Missing param: team_id")?.to_string();
            let q      = "query($teamId:String!){team(id:$teamId){cycles{nodes{id number name startsAt endsAt completedAt progress}}}}";
            let data   = gql(&cfg.api_key, q, json!({"teamId":team_id})).await?;
            let cycles = data["team"]["cycles"]["nodes"].as_array().cloned().unwrap_or_default();
            if cycles.is_empty() { return Ok("Aucun cycle trouvé.".to_string()); }
            Ok(cycles.iter().map(|c| {
                let num   = c["number"].as_u64().unwrap_or(0);
                let name  = c["name"].as_str().unwrap_or("");
                let start = c["startsAt"].as_str().unwrap_or("?");
                let end   = c["endsAt"].as_str().unwrap_or("?");
                let prog  = c["progress"].as_f64().unwrap_or(0.0);
                let cid   = c["id"].as_str().unwrap_or("?");
                let label = if name.is_empty() { format!("Cycle {num}") } else { format!("Cycle {num} — {name}") };
                format!("{label}\n  {start} → {end} · progrès={:.0}%\n  id={cid}", prog * 100.0)
            }).collect::<Vec<_>>().join("\n\n"))
        }

        "linear_get_cycle" => {
            let cycle_id = args["id"].as_str().ok_or("Missing param: id")?.to_string();
            let q = "query($id:String!){cycle(id:$id){id number name startsAt endsAt progress issues{nodes{id identifier title state{name}assignee{name}}}}}";
            let data  = gql(&cfg.api_key, q, json!({"id":cycle_id})).await?;
            let c     = &data["cycle"];
            let num   = c["number"].as_u64().unwrap_or(0);
            let name  = c["name"].as_str().unwrap_or("");
            let start = c["startsAt"].as_str().unwrap_or("?");
            let end   = c["endsAt"].as_str().unwrap_or("?");
            let prog  = c["progress"].as_f64().unwrap_or(0.0);
            let issues = c["issues"]["nodes"].as_array().cloned().unwrap_or_default();
            let label  = if name.is_empty() { format!("Cycle {num}") } else { format!("Cycle {num} — {name}") };
            let mut out = format!("{label}\n{start} → {end} · progrès={:.0}%\n\nIssues ({}) :\n", prog * 100.0, issues.len());
            for i in &issues {
                let iid   = i["identifier"].as_str().unwrap_or("?");
                let title = i["title"].as_str().unwrap_or("?");
                let state = i["state"]["name"].as_str().unwrap_or("?");
                let asgn  = i["assignee"]["name"].as_str().unwrap_or("—");
                out.push_str(&format!("  [{iid}] {title} · {state} · {asgn}\n"));
            }
            Ok(out)
        }

        "linear_delete_comment" => {
            let comment_id = args["id"].as_str().ok_or("Missing param: id")?.to_string();
            let q          = "mutation($id:String!){commentDelete(id:$id){success}}";
            gql(&cfg.api_key, q, json!({"id":comment_id})).await?;
            Ok("✅ Commentaire supprimé.".to_string())
        }

        other => Err(format!("Unknown linear tool: {other}")),
    }
}
