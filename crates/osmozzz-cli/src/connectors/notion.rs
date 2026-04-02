/// Connecteur Notion — REST API officielle v1.
/// Toutes les fonctions retournent Result<String, String> sans jamais appeler send().
/// La couche sécurité (sanitize_proxy_response + audit_log) est appliquée par le dispatcher (mcp.rs).
use serde_json::{json, Value};

// ─── Config ──────────────────────────────────────────────────────────────────

#[derive(serde::Deserialize)]
struct NotionConfig {
    token: String,
}

impl NotionConfig {
    fn load() -> Option<Self> {
        let path = dirs_next::home_dir()?.join(".osmozzz/notion.toml");
        toml::from_str(&std::fs::read_to_string(path).ok()?).ok()
    }

    fn api(&self, path: &str) -> String {
        format!("https://api.notion.com/v1/{}", path.trim_start_matches('/'))
    }
}

// ─── HTTP helpers ─────────────────────────────────────────────────────────────

async fn get(cfg: &NotionConfig, url: &str) -> Result<Value, String> {
    reqwest::Client::new()
        .get(url)
        .header("Authorization", format!("Bearer {}", cfg.token))
        .header("Notion-Version", "2022-06-28")
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<Value>()
        .await
        .map_err(|e| e.to_string())
}

async fn post_json(cfg: &NotionConfig, url: &str, body: &Value) -> Result<Value, String> {
    reqwest::Client::new()
        .post(url)
        .header("Authorization", format!("Bearer {}", cfg.token))
        .header("Notion-Version", "2022-06-28")
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .json(body)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<Value>()
        .await
        .map_err(|e| e.to_string())
}

async fn patch_json(cfg: &NotionConfig, url: &str, body: &Value) -> Result<Value, String> {
    reqwest::Client::new()
        .patch(url)
        .header("Authorization", format!("Bearer {}", cfg.token))
        .header("Notion-Version", "2022-06-28")
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .json(body)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<Value>()
        .await
        .map_err(|e| e.to_string())
}

async fn delete_req(cfg: &NotionConfig, url: &str) -> Result<Value, String> {
    reqwest::Client::new()
        .delete(url)
        .header("Authorization", format!("Bearer {}", cfg.token))
        .header("Notion-Version", "2022-06-28")
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<Value>()
        .await
        .map_err(|e| e.to_string())
}

// ─── Helpers de formatage ─────────────────────────────────────────────────────

/// Extrait le texte plat depuis un tableau rich_text Notion.
fn extract_plain_text(rich_text: &Value) -> String {
    rich_text
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter_map(|rt| rt["plain_text"].as_str())
                .collect::<Vec<_>>()
                .join("")
        })
        .unwrap_or_default()
}

/// Extrait le titre d'une page Notion depuis ses properties.
fn extract_page_title(page: &Value) -> String {
    // Cherche la propriété "title" ou "Name" dans les properties
    if let Some(props) = page["properties"].as_object() {
        for (_key, val) in props {
            if val["type"].as_str() == Some("title") {
                return extract_plain_text(&val["title"]);
            }
        }
    }
    // Fallback : titre dans l'objet directement (pour les pages simples)
    page["title"]
        .as_array()
        .map(|arr| extract_plain_text(&Value::Array(arr.clone())))
        .unwrap_or_default()
}

/// Formate un objet utilisateur Notion en ligne lisible.
fn format_user(user: &Value) -> String {
    let id    = user["id"].as_str().unwrap_or("—");
    let utype = user["type"].as_str().unwrap_or("—");
    let name  = user["name"].as_str().unwrap_or("—");
    match utype {
        "person" => {
            let email = user["person"]["email"].as_str().unwrap_or("—");
            format!("[{id}] {name} <{email}>")
        }
        "bot" => format!("[{id}] {name} (bot)"),
        _ => format!("[{id}] {name} ({utype})"),
    }
}

// ─── Tool definitions ────────────────────────────────────────────────────────

pub fn tools() -> Vec<Value> {
    vec![
        // ── Search ──────────────────────────────────────────────────────────
        json!({
            "name": "notion_search",
            "description": "NOTION 🔍 — Recherche dans toutes les pages et bases de données Notion accessibles par l'intégration. Retourne une liste compacte (id, titre, type, url). Utiliser notion_get_page ou notion_get_database pour le détail.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query":           { "type": "string", "description": "Texte à rechercher" },
                    "filter_type":     { "type": "string", "enum": ["page", "database"], "description": "Filtrer par type : 'page' ou 'database' (optionnel)" },
                    "sort_direction":  { "type": "string", "enum": ["ascending", "descending"], "description": "Direction du tri (optionnel, défaut : descending)" },
                    "start_cursor":    { "type": "string", "description": "Curseur de pagination (optionnel)" },
                    "page_size":       { "type": "integer", "description": "Nombre de résultats (1-100, défaut : 20)" }
                }
            }
        }),
        // ── Pages ───────────────────────────────────────────────────────────
        json!({
            "name": "notion_get_page",
            "description": "NOTION 📄 — Récupère les métadonnées d'une page Notion (titre, propriétés, parent, url, dates). Pour lire le contenu textuel, utiliser notion_get_block_children avec le même page_id.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "page_id": { "type": "string", "description": "ID de la page Notion" }
                },
                "required": ["page_id"]
            }
        }),
        json!({
            "name": "notion_create_page",
            "description": "NOTION ➕ — Crée une nouvelle page Notion dans une page parente ou une base de données. Pour une base de données, passer parent_database_id. Pour une page, passer parent_page_id. Le contenu de la page peut être ajouté via content_blocks (tableau de blocs Notion).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "title":               { "type": "string", "description": "Titre de la page" },
                    "parent_page_id":      { "type": "string", "description": "ID de la page parente (exclusif avec parent_database_id)" },
                    "parent_database_id":  { "type": "string", "description": "ID de la base de données parente (exclusif avec parent_page_id)" },
                    "properties":          { "type": "object", "description": "Propriétés supplémentaires pour les pages dans une base de données (JSON Notion)" },
                    "content_blocks":      { "type": "array", "description": "Blocs de contenu Notion (JSON, optionnel)" }
                },
                "required": ["title"]
            }
        }),
        json!({
            "name": "notion_update_page",
            "description": "NOTION ✏️ — Met à jour les propriétés d'une page Notion (titre, autres propriétés) ou archive/restaure la page. Ne modifie pas le contenu des blocs — utiliser notion_append_block_children pour ajouter du contenu.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "page_id":    { "type": "string", "description": "ID de la page à modifier" },
                    "properties": { "type": "object", "description": "Propriétés à mettre à jour (JSON Notion, optionnel)" },
                    "archived":   { "type": "boolean", "description": "true pour archiver, false pour restaurer (optionnel)" }
                },
                "required": ["page_id"]
            }
        }),
        json!({
            "name": "notion_move_page",
            "description": "NOTION 🔀 — Déplace une page Notion vers un nouveau parent (page ou base de données). Utilise PATCH /pages/{page_id} avec un nouveau champ parent.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "page_id":               { "type": "string", "description": "ID de la page à déplacer" },
                    "new_parent_page_id":     { "type": "string", "description": "ID de la nouvelle page parente (exclusif avec new_parent_database_id)" },
                    "new_parent_database_id": { "type": "string", "description": "ID de la nouvelle base de données parente (exclusif avec new_parent_page_id)" }
                },
                "required": ["page_id"]
            }
        }),
        // ── Page properties ─────────────────────────────────────────────────
        json!({
            "name": "notion_get_page_property",
            "description": "NOTION 🏷️ — Récupère la valeur d'une propriété spécifique d'une page Notion (ex: relation, rollup, formula). Utile pour les propriétés paginées ou volumineuses.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "page_id":     { "type": "string", "description": "ID de la page" },
                    "property_id": { "type": "string", "description": "ID de la propriété (visible dans notion_get_page sous properties)" }
                },
                "required": ["page_id", "property_id"]
            }
        }),
        // ── Databases ───────────────────────────────────────────────────────
        json!({
            "name": "notion_get_database",
            "description": "NOTION 🗄️ — Récupère le schéma d'une base de données Notion (titre, propriétés disponibles et leur type). Utiliser notion_query_database pour lister les entrées.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "database_id": { "type": "string", "description": "ID de la base de données Notion" }
                },
                "required": ["database_id"]
            }
        }),
        json!({
            "name": "notion_query_database",
            "description": "NOTION 📋 — Interroge une base de données Notion avec des filtres et un tri. Retourne les pages correspondantes avec leurs propriétés. Utiliser notion_get_page pour le détail d'une entrée.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "database_id":  { "type": "string", "description": "ID de la base de données" },
                    "filter":       { "type": "object", "description": "Filtre Notion (JSON, optionnel)" },
                    "sorts":        { "type": "array",  "description": "Tableau de critères de tri (JSON Notion, optionnel)" },
                    "start_cursor": { "type": "string", "description": "Curseur de pagination (optionnel)" },
                    "page_size":    { "type": "integer", "description": "Nombre de résultats (1-100, défaut : 20)" }
                },
                "required": ["database_id"]
            }
        }),
        // ── Blocks ──────────────────────────────────────────────────────────
        json!({
            "name": "notion_get_block",
            "description": "NOTION 🧱 — Récupère un bloc Notion par son ID (type, contenu, enfants, dates). Pour lister les blocs d'une page, utiliser notion_get_block_children avec le page_id.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "block_id": { "type": "string", "description": "ID du bloc Notion" }
                },
                "required": ["block_id"]
            }
        }),
        json!({
            "name": "notion_get_block_children",
            "description": "NOTION 🧱 — Liste les blocs enfants d'une page ou d'un bloc Notion (contenu textuel, titres, listes, etc.). Utiliser le page_id pour lire le contenu d'une page entière.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "block_id":     { "type": "string", "description": "ID du bloc ou de la page parente" },
                    "start_cursor": { "type": "string", "description": "Curseur de pagination (optionnel)" },
                    "page_size":    { "type": "integer", "description": "Nombre de blocs (1-100, défaut : 50)" }
                },
                "required": ["block_id"]
            }
        }),
        json!({
            "name": "notion_append_block_children",
            "description": "NOTION ➕ — Ajoute des blocs de contenu à la fin d'une page ou d'un bloc Notion. Les blocs sont définis en JSON selon le format Notion (paragraph, heading_1, bulleted_list_item, etc.).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "block_id": { "type": "string", "description": "ID du bloc ou de la page cible" },
                    "children": { "type": "array",  "description": "Tableau de blocs Notion à ajouter (JSON)" }
                },
                "required": ["block_id", "children"]
            }
        }),
        json!({
            "name": "notion_update_block",
            "description": "NOTION ✏️ — Met à jour le contenu d'un bloc Notion existant (texte, type, etc.). Le corps doit être un objet JSON conforme à l'API Notion pour le type de bloc concerné.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "block_id":      { "type": "string", "description": "ID du bloc à modifier" },
                    "block_content": { "type": "object", "description": "Contenu du bloc au format Notion (JSON)" }
                },
                "required": ["block_id", "block_content"]
            }
        }),
        json!({
            "name": "notion_delete_block",
            "description": "NOTION 🗑️ — Supprime (archive) définitivement un bloc Notion. Cette action est irréversible via l'API.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "block_id": { "type": "string", "description": "ID du bloc à supprimer" }
                },
                "required": ["block_id"]
            }
        }),
        // ── Users ────────────────────────────────────────────────────────────
        json!({
            "name": "notion_get_user",
            "description": "NOTION 👤 — Récupère le profil d'un utilisateur Notion (nom, email, type). Utiliser notion_list_users pour obtenir les user_id.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "user_id": { "type": "string", "description": "ID de l'utilisateur Notion" }
                },
                "required": ["user_id"]
            }
        }),
        json!({
            "name": "notion_list_users",
            "description": "NOTION 👥 — Liste tous les utilisateurs de l'espace de travail Notion accessibles par l'intégration. Retourne id, nom et email.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "start_cursor": { "type": "string",  "description": "Curseur de pagination (optionnel)" },
                    "page_size":    { "type": "integer", "description": "Nombre d'utilisateurs (1-100, défaut : 50)" }
                }
            }
        }),
        json!({
            "name": "notion_get_self",
            "description": "NOTION 🤖 — Retourne le profil du bot associé à l'intégration Notion (nom, id, espace de travail). Utile pour vérifier la configuration.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }),
        // ── Data Sources ─────────────────────────────────────────────────────
        json!({
            "name": "notion_list_data_source_templates",
            "description": "NOTION 📦 — Liste les templates de data sources disponibles dans Notion (Google Drive, GitHub, etc.).",
            "inputSchema": { "type": "object", "properties": {} }
        }),
        json!({
            "name": "notion_create_data_source",
            "description": "NOTION ➕ — Crée une nouvelle data source Notion (connexion à une source externe). Fournir le type et la config selon le template.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "type":   { "type": "string", "description": "Type de data source (ex: 'github', 'google_drive')" },
                    "config": { "type": "object", "description": "Configuration spécifique au type de data source" }
                },
                "required": ["type"]
            }
        }),
        json!({
            "name": "notion_get_data_source",
            "description": "NOTION 🔍 — Récupère les détails d'une data source Notion par son ID.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "data_source_id": { "type": "string", "description": "ID de la data source" }
                },
                "required": ["data_source_id"]
            }
        }),
        json!({
            "name": "notion_update_data_source",
            "description": "NOTION ✏️ — Met à jour la configuration d'une data source Notion existante.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "data_source_id": { "type": "string", "description": "ID de la data source" },
                    "config":         { "type": "object", "description": "Nouvelle configuration" }
                },
                "required": ["data_source_id"]
            }
        }),
        // ── Comments ────────────────────────────────────────────────────────
        json!({
            "name": "notion_create_comment",
            "description": "NOTION 💬 — Ajoute un commentaire sur une page Notion (via page_id) ou dans une discussion existante (via discussion_id). Le texte est passé en plain text.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "page_id":       { "type": "string", "description": "ID de la page cible (exclusif avec discussion_id)" },
                    "discussion_id": { "type": "string", "description": "ID de la discussion existante (exclusif avec page_id)" },
                    "content":       { "type": "string", "description": "Texte du commentaire" }
                },
                "required": ["content"]
            }
        }),
        json!({
            "name": "notion_get_comments",
            "description": "NOTION 💬 — Récupère les commentaires d'une page ou d'un bloc Notion. Retourne la liste des commentaires avec auteur, texte et date.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "block_id":     { "type": "string",  "description": "ID de la page ou du bloc" },
                    "start_cursor": { "type": "string",  "description": "Curseur de pagination (optionnel)" },
                    "page_size":    { "type": "integer", "description": "Nombre de commentaires (1-100, défaut : 50)" }
                },
                "required": ["block_id"]
            }
        }),
    ]
}

// ─── Handler ─────────────────────────────────────────────────────────────────

pub async fn handle(name: &str, args: &Value) -> Result<String, String> {
    let cfg = NotionConfig::load()
        .ok_or_else(|| "Notion non configuré — créer ~/.osmozzz/notion.toml avec token = \"secret_xxx\"".to_string())?;

    match name {
        // ── Search ──────────────────────────────────────────────────────────
        "notion_search" => {
            let mut body = json!({});

            if let Some(q) = args["query"].as_str() {
                body["query"] = json!(q);
            }
            if let Some(ft) = args["filter_type"].as_str() {
                body["filter"] = json!({ "value": ft, "property": "object" });
            }
            let direction = args["sort_direction"].as_str().unwrap_or("descending");
            body["sort"] = json!({ "direction": direction, "timestamp": "last_edited_time" });

            let page_size = args["page_size"].as_u64().unwrap_or(20).min(100);
            body["page_size"] = json!(page_size);

            if let Some(cursor) = args["start_cursor"].as_str() {
                body["start_cursor"] = json!(cursor);
            }

            let url  = cfg.api("search");
            let resp = post_json(&cfg, &url, &body).await?;

            let results = resp["results"].as_array().cloned().unwrap_or_default();
            if results.is_empty() {
                return Ok("Aucun résultat Notion.".to_string());
            }

            let has_more    = resp["has_more"].as_bool().unwrap_or(false);
            let next_cursor = resp["next_cursor"].as_str().unwrap_or("—");

            let mut out = format!("{} résultat(s) :\n", results.len());
            for r in &results {
                let id    = r["id"].as_str().unwrap_or("—");
                let rtype = r["object"].as_str().unwrap_or("—");
                let url_r = r["url"].as_str().unwrap_or("—");
                let title = extract_page_title(r);
                let title_display = if title.is_empty() { "(sans titre)".to_string() } else { title };
                out.push_str(&format!("• [{id}] ({rtype}) {title_display}\n  {url_r}\n"));
            }
            if has_more {
                out.push_str(&format!("\n(Suite disponible — start_cursor: {next_cursor})"));
            }
            Ok(out.trim_end().to_string())
        }

        // ── Pages ───────────────────────────────────────────────────────────
        "notion_get_page" => {
            let page_id = args["page_id"].as_str().ok_or("Paramètre 'page_id' requis")?;
            let url     = cfg.api(&format!("pages/{page_id}"));
            let resp    = get(&cfg, &url).await?;

            let id          = resp["id"].as_str().unwrap_or("—");
            let page_url    = resp["url"].as_str().unwrap_or("—");
            let created_at  = resp["created_time"].as_str().unwrap_or("—");
            let edited_at   = resp["last_edited_time"].as_str().unwrap_or("—");
            let archived    = resp["archived"].as_bool().unwrap_or(false);
            let title       = extract_page_title(&resp);
            let title_display = if title.is_empty() { "(sans titre)".to_string() } else { title };

            let parent_type = resp["parent"]["type"].as_str().unwrap_or("—");
            let parent_id   = match parent_type {
                "page_id"      => resp["parent"]["page_id"].as_str().unwrap_or("—"),
                "database_id"  => resp["parent"]["database_id"].as_str().unwrap_or("—"),
                "workspace"    => "workspace",
                _              => "—",
            };

            Ok(format!(
                "Page Notion\nID         : {id}\nTitre      : {title_display}\nParent     : {parent_type} / {parent_id}\nArchivé    : {archived}\nURL        : {page_url}\nCréée le   : {created_at}\nModifiée   : {edited_at}"
            ))
        }

        "notion_create_page" => {
            let title = args["title"].as_str().ok_or("Paramètre 'title' requis")?;

            // Construire le parent
            let parent = if let Some(pid) = args["parent_page_id"].as_str() {
                json!({ "type": "page_id", "page_id": pid })
            } else if let Some(did) = args["parent_database_id"].as_str() {
                json!({ "type": "database_id", "database_id": did })
            } else {
                return Err("Paramètre 'parent_page_id' ou 'parent_database_id' requis".to_string());
            };

            // Construire les properties (toujours inclure le titre)
            let title_block = json!([{ "type": "text", "text": { "content": title } }]);
            let mut properties = if let Some(p) = args["properties"].as_object() {
                Value::Object(p.clone())
            } else {
                json!({})
            };
            properties["title"] = json!(title_block);

            let mut body = json!({
                "parent":     parent,
                "properties": properties
            });

            // Ajouter le contenu si fourni
            if let Some(blocks) = args["content_blocks"].as_array() {
                body["children"] = Value::Array(blocks.clone());
            }

            let url  = cfg.api("pages");
            let resp = post_json(&cfg, &url, &body).await?;

            let id       = resp["id"].as_str().unwrap_or("—");
            let page_url = resp["url"].as_str().unwrap_or("—");
            Ok(format!(
                "Page créée.\nID  : {id}\nURL : {page_url}"
            ))
        }

        "notion_update_page" => {
            let page_id = args["page_id"].as_str().ok_or("Paramètre 'page_id' requis")?;

            let mut body = json!({});
            if let Some(props) = args["properties"].as_object() {
                body["properties"] = Value::Object(props.clone());
            }
            if let Some(archived) = args["archived"].as_bool() {
                body["archived"] = json!(archived);
            }

            if body.as_object().map(|o| o.is_empty()).unwrap_or(true) {
                return Err("Au moins un paramètre parmi 'properties' ou 'archived' est requis".to_string());
            }

            let url  = cfg.api(&format!("pages/{page_id}"));
            let resp = patch_json(&cfg, &url, &body).await?;

            let id       = resp["id"].as_str().unwrap_or(page_id);
            let edited   = resp["last_edited_time"].as_str().unwrap_or("—");
            let archived = resp["archived"].as_bool().unwrap_or(false);
            Ok(format!("Page {id} mise à jour.\nArchivée    : {archived}\nModifiée le : {edited}"))
        }

        "notion_move_page" => {
            let page_id = args["page_id"].as_str().ok_or("Paramètre 'page_id' requis")?;

            let new_parent = if let Some(pid) = args["new_parent_page_id"].as_str() {
                json!({ "type": "page_id", "page_id": pid })
            } else if let Some(did) = args["new_parent_database_id"].as_str() {
                json!({ "type": "database_id", "database_id": did })
            } else {
                return Err("Paramètre 'new_parent_page_id' ou 'new_parent_database_id' requis".to_string());
            };

            let body     = json!({ "parent": new_parent });
            let url      = cfg.api(&format!("pages/{page_id}"));
            let resp     = patch_json(&cfg, &url, &body).await?;

            let id       = resp["id"].as_str().unwrap_or(page_id);
            let edited   = resp["last_edited_time"].as_str().unwrap_or("—");
            Ok(format!("Page {id} déplacée.\nModifiée le : {edited}"))
        }

        // ── Page properties ─────────────────────────────────────────────────
        "notion_get_page_property" => {
            let page_id     = args["page_id"].as_str().ok_or("Paramètre 'page_id' requis")?;
            let property_id = args["property_id"].as_str().ok_or("Paramètre 'property_id' requis")?;
            let url         = cfg.api(&format!("pages/{page_id}/properties/{property_id}"));
            let resp        = get(&cfg, &url).await?;

            Ok(serde_json::to_string_pretty(&resp).unwrap_or_else(|_| format!("{resp}")))
        }

        // ── Databases ───────────────────────────────────────────────────────
        "notion_get_database" => {
            let database_id = args["database_id"].as_str().ok_or("Paramètre 'database_id' requis")?;
            let url         = cfg.api(&format!("databases/{database_id}"));
            let resp        = get(&cfg, &url).await?;

            let id         = resp["id"].as_str().unwrap_or("—");
            let db_url     = resp["url"].as_str().unwrap_or("—");
            let created_at = resp["created_time"].as_str().unwrap_or("—");
            let edited_at  = resp["last_edited_time"].as_str().unwrap_or("—");
            let title      = extract_plain_text(&resp["title"]);
            let title_display = if title.is_empty() { "(sans titre)".to_string() } else { title };

            let mut props_out = String::new();
            if let Some(props) = resp["properties"].as_object() {
                for (pname, pval) in props {
                    let ptype = pval["type"].as_str().unwrap_or("—");
                    props_out.push_str(&format!("  • {pname} ({ptype})\n"));
                }
            }

            Ok(format!(
                "Base de données Notion\nID         : {id}\nTitre      : {title_display}\nURL        : {db_url}\nCréée le   : {created_at}\nModifiée   : {edited_at}\nPropriétés :\n{props}",
                props = if props_out.is_empty() { "  (aucune)\n".to_string() } else { props_out }
            ))
        }

        "notion_query_database" => {
            let database_id = args["database_id"].as_str().ok_or("Paramètre 'database_id' requis")?;
            let page_size   = args["page_size"].as_u64().unwrap_or(20).min(100);

            let mut body = json!({ "page_size": page_size });
            if let Some(f) = args["filter"].as_object() {
                body["filter"] = Value::Object(f.clone());
            }
            if let Some(s) = args["sorts"].as_array() {
                body["sorts"] = Value::Array(s.clone());
            }
            if let Some(cursor) = args["start_cursor"].as_str() {
                body["start_cursor"] = json!(cursor);
            }

            let url  = cfg.api(&format!("databases/{database_id}/query"));
            let resp = post_json(&cfg, &url, &body).await?;

            let results = resp["results"].as_array().cloned().unwrap_or_default();
            if results.is_empty() {
                return Ok(format!("Aucun résultat dans la base de données {database_id}."));
            }

            let has_more    = resp["has_more"].as_bool().unwrap_or(false);
            let next_cursor = resp["next_cursor"].as_str().unwrap_or("—");

            let mut out = format!("{} entrée(s) :\n", results.len());
            for r in &results {
                let id    = r["id"].as_str().unwrap_or("—");
                let title = extract_page_title(r);
                let title_display = if title.is_empty() { "(sans titre)".to_string() } else { title };
                let edited = r["last_edited_time"].as_str().unwrap_or("—");
                out.push_str(&format!("• [{id}] {title_display} (modifié: {edited})\n"));
            }
            if has_more {
                out.push_str(&format!("\n(Suite disponible — start_cursor: {next_cursor})"));
            }
            Ok(out.trim_end().to_string())
        }

        // ── Blocks ──────────────────────────────────────────────────────────
        "notion_get_block" => {
            let block_id = args["block_id"].as_str().ok_or("Paramètre 'block_id' requis")?;
            let url      = cfg.api(&format!("blocks/{block_id}"));
            let resp     = get(&cfg, &url).await?;

            let id         = resp["id"].as_str().unwrap_or("—");
            let btype      = resp["type"].as_str().unwrap_or("—");
            let created_at = resp["created_time"].as_str().unwrap_or("—");
            let edited_at  = resp["last_edited_time"].as_str().unwrap_or("—");
            let has_children = resp["has_children"].as_bool().unwrap_or(false);
            let archived   = resp["archived"].as_bool().unwrap_or(false);

            // Extraire le texte du bloc selon son type
            let text = if let Some(block_data) = resp[btype].as_object() {
                if let Some(rt) = block_data.get("rich_text") {
                    extract_plain_text(rt)
                } else {
                    String::new()
                }
            } else {
                String::new()
            };

            let mut out = format!(
                "Bloc Notion\nID          : {id}\nType        : {btype}\nCréé le     : {created_at}\nModifié le  : {edited_at}\nA des enfants: {has_children}\nArchivé     : {archived}"
            );
            if !text.is_empty() {
                out.push_str(&format!("\nContenu     : {text}"));
            }
            Ok(out)
        }

        "notion_get_block_children" => {
            let block_id  = args["block_id"].as_str().ok_or("Paramètre 'block_id' requis")?;
            let page_size = args["page_size"].as_u64().unwrap_or(50).min(100);

            let mut url = format!("{}&page_size={page_size}", cfg.api(&format!("blocks/{block_id}/children?")));
            if let Some(cursor) = args["start_cursor"].as_str() {
                url.push_str(&format!("&start_cursor={cursor}"));
            }

            let resp  = get(&cfg, &url).await?;
            let blocs = resp["results"].as_array().cloned().unwrap_or_default();

            if blocs.is_empty() {
                return Ok(format!("Aucun bloc enfant pour {block_id}."));
            }

            let has_more    = resp["has_more"].as_bool().unwrap_or(false);
            let next_cursor = resp["next_cursor"].as_str().unwrap_or("—");

            let mut out = format!("{} bloc(s) :\n", blocs.len());
            for b in &blocs {
                let id    = b["id"].as_str().unwrap_or("—");
                let btype = b["type"].as_str().unwrap_or("—");
                let text  = if let Some(block_data) = b[btype].as_object() {
                    block_data.get("rich_text")
                        .map(|rt| extract_plain_text(rt))
                        .unwrap_or_default()
                } else {
                    String::new()
                };
                let preview = if text.len() > 100 { &text[..100] } else { text.as_str() };
                out.push_str(&format!("• [{id}] ({btype}) {preview}\n"));
            }
            if has_more {
                out.push_str(&format!("\n(Suite disponible — start_cursor: {next_cursor})"));
            }
            Ok(out.trim_end().to_string())
        }

        "notion_append_block_children" => {
            let block_id = args["block_id"].as_str().ok_or("Paramètre 'block_id' requis")?;
            let children = args["children"].as_array()
                .ok_or("Paramètre 'children' requis (tableau de blocs Notion)")?;

            let body = json!({ "children": children });
            let url  = cfg.api(&format!("blocks/{block_id}/children"));
            let resp = patch_json(&cfg, &url, &body).await?;

            let added = resp["results"].as_array().map(|a| a.len()).unwrap_or(0);
            Ok(format!("{added} bloc(s) ajouté(s) à {block_id}."))
        }

        "notion_update_block" => {
            let block_id      = args["block_id"].as_str().ok_or("Paramètre 'block_id' requis")?;
            let block_content = args["block_content"].as_object()
                .ok_or("Paramètre 'block_content' requis (objet JSON Notion)")?;

            let url  = cfg.api(&format!("blocks/{block_id}"));
            let resp = patch_json(&cfg, &url, &Value::Object(block_content.clone())).await?;

            let id      = resp["id"].as_str().unwrap_or(block_id);
            let edited  = resp["last_edited_time"].as_str().unwrap_or("—");
            Ok(format!("Bloc {id} mis à jour. Modifié le : {edited}"))
        }

        "notion_delete_block" => {
            let block_id = args["block_id"].as_str().ok_or("Paramètre 'block_id' requis")?;
            let url      = cfg.api(&format!("blocks/{block_id}"));
            let resp     = delete_req(&cfg, &url).await?;

            let id       = resp["id"].as_str().unwrap_or(block_id);
            let archived = resp["archived"].as_bool().unwrap_or(true);
            Ok(format!("Bloc {id} supprimé (archivé: {archived})."))
        }

        // ── Users ────────────────────────────────────────────────────────────
        "notion_get_user" => {
            let user_id = args["user_id"].as_str().ok_or("Paramètre 'user_id' requis")?;
            let url     = cfg.api(&format!("users/{user_id}"));
            let resp    = get(&cfg, &url).await?;

            Ok(format!("Utilisateur Notion\n{}", format_user(&resp)))
        }

        "notion_list_users" => {
            let page_size = args["page_size"].as_u64().unwrap_or(50).min(100);
            let mut url   = format!("{}?page_size={page_size}", cfg.api("users"));
            if let Some(cursor) = args["start_cursor"].as_str() {
                url.push_str(&format!("&start_cursor={cursor}"));
            }

            let resp  = get(&cfg, &url).await?;
            let users = resp["results"].as_array().cloned().unwrap_or_default();

            if users.is_empty() {
                return Ok("Aucun utilisateur trouvé.".to_string());
            }

            let has_more    = resp["has_more"].as_bool().unwrap_or(false);
            let next_cursor = resp["next_cursor"].as_str().unwrap_or("—");

            let mut out = format!("{} utilisateur(s) :\n", users.len());
            for u in &users {
                out.push_str(&format!("• {}\n", format_user(u)));
            }
            if has_more {
                out.push_str(&format!("\n(Suite disponible — start_cursor: {next_cursor})"));
            }
            Ok(out.trim_end().to_string())
        }

        "notion_get_self" => {
            let url  = cfg.api("users/me");
            let resp = get(&cfg, &url).await?;

            let id   = resp["id"].as_str().unwrap_or("—");
            let name = resp["name"].as_str().unwrap_or("—");
            let bot_owner_type = resp["bot"]["owner"]["type"].as_str().unwrap_or("—");
            let workspace_name = resp["bot"]["workspace_name"].as_str().unwrap_or("—");

            Ok(format!(
                "Bot Notion\nID              : {id}\nNom             : {name}\nPropriétaire    : {bot_owner_type}\nEspace de travail: {workspace_name}"
            ))
        }

        // ── Comments ────────────────────────────────────────────────────────
        "notion_create_comment" => {
            let content = args["content"].as_str().ok_or("Paramètre 'content' requis")?;

            let rich_text = json!([{ "type": "text", "text": { "content": content } }]);

            let mut body = json!({ "rich_text": rich_text });

            if let Some(pid) = args["page_id"].as_str() {
                body["parent"] = json!({ "type": "page_id", "page_id": pid });
            } else if let Some(did) = args["discussion_id"].as_str() {
                body["discussion_id"] = json!(did);
            } else {
                return Err("Paramètre 'page_id' ou 'discussion_id' requis".to_string());
            }

            let url  = cfg.api("comments");
            let resp = post_json(&cfg, &url, &body).await?;

            let id         = resp["id"].as_str().unwrap_or("—");
            let created_at = resp["created_time"].as_str().unwrap_or("—");
            Ok(format!("Commentaire créé.\nID        : {id}\nCréé le   : {created_at}"))
        }

        "notion_get_comments" => {
            let block_id  = args["block_id"].as_str().ok_or("Paramètre 'block_id' requis")?;
            let page_size = args["page_size"].as_u64().unwrap_or(50).min(100);

            let mut url = format!(
                "{}?block_id={block_id}&page_size={page_size}",
                cfg.api("comments")
            );
            if let Some(cursor) = args["start_cursor"].as_str() {
                url.push_str(&format!("&start_cursor={cursor}"));
            }

            let resp     = get(&cfg, &url).await?;
            let comments = resp["results"].as_array().cloned().unwrap_or_default();

            if comments.is_empty() {
                return Ok(format!("Aucun commentaire pour {block_id}."));
            }

            let has_more    = resp["has_more"].as_bool().unwrap_or(false);
            let next_cursor = resp["next_cursor"].as_str().unwrap_or("—");

            let mut out = format!("{} commentaire(s) :\n", comments.len());
            for c in &comments {
                let id         = c["id"].as_str().unwrap_or("—");
                let created_at = c["created_time"].as_str().unwrap_or("—");
                let author     = c["created_by"]["id"].as_str().unwrap_or("—");
                let text       = extract_plain_text(&c["rich_text"]);
                out.push_str(&format!("• [{id}] ({created_at}) par {author}: {text}\n"));
            }
            if has_more {
                out.push_str(&format!("\n(Suite disponible — start_cursor: {next_cursor})"));
            }
            Ok(out.trim_end().to_string())
        }

        // ── Data Sources ─────────────────────────────────────────────────────
        "notion_list_data_source_templates" => {
            let url  = cfg.api("data-source-templates");
            let resp = get(&cfg, &url).await?;
            Ok(serde_json::to_string_pretty(&resp).unwrap_or_else(|_| resp.to_string()))
        }

        "notion_create_data_source" => {
            let mut body = json!({});
            if let Some(t) = args["type"].as_str()    { body["type"]    = json!(t); }
            if let Some(c) = args.get("config")       { body["config"]  = c.clone(); }
            let url  = cfg.api("data-sources");
            let resp = post_json(&cfg, &url, &body).await?;
            let id   = resp["id"].as_str().unwrap_or("—");
            let kind = resp["type"].as_str().unwrap_or("—");
            Ok(format!("Data source créée.\nID   : {id}\nType : {kind}"))
        }

        "notion_get_data_source" => {
            let ds_id = args["data_source_id"].as_str().ok_or("Paramètre 'data_source_id' requis")?;
            let url   = cfg.api(&format!("data-sources/{ds_id}"));
            let resp  = get(&cfg, &url).await?;
            Ok(serde_json::to_string_pretty(&resp).unwrap_or_else(|_| resp.to_string()))
        }

        "notion_update_data_source" => {
            let ds_id = args["data_source_id"].as_str().ok_or("Paramètre 'data_source_id' requis")?;
            let mut body = json!({});
            if let Some(c) = args.get("config") { body["config"] = c.clone(); }
            let url  = cfg.api(&format!("data-sources/{ds_id}"));
            let resp = patch_json(&cfg, &url, &body).await?;
            let id   = resp["id"].as_str().unwrap_or(ds_id);
            Ok(format!("Data source {id} mise à jour."))
        }

        _ => Err(format!("Tool Notion inconnu : {name}")),
    }
}
