/// Connecteur Figma — REST API v1 officielle.
/// Toutes les fonctions retournent Result<String, String> sans jamais appeler send().
/// La couche sécurité (sanitize_proxy_response + audit_log) est appliquée par le dispatcher (mcp.rs).
use serde_json::{json, Value};

// ─── Config ───────────────────────────────────────────────────────────────────

#[derive(serde::Deserialize)]
struct FigmaConfig {
    token: String,
    #[serde(default)]
    team_id: String,
}

impl FigmaConfig {
    fn load() -> Option<Self> {
        let path = dirs_next::home_dir()?.join(".osmozzz/figma.toml");
        toml::from_str(&std::fs::read_to_string(path).ok()?).ok()
    }

    fn api(&self, path: &str) -> String {
        format!("https://api.figma.com/v1/{}", path.trim_start_matches('/'))
    }

    /// Resolve team_id from args or config.
    fn resolve_team<'a>(&'a self, args: &'a Value) -> Result<&'a str, String> {
        if let Some(t) = args["team_id"].as_str() {
            return Ok(t);
        }
        if !self.team_id.is_empty() {
            return Ok(&self.team_id);
        }
        Err("Missing 'team_id' — pass it as argument or set team_id in ~/.osmozzz/figma.toml".to_string())
    }
}

// ─── HTTP helpers ─────────────────────────────────────────────────────────────

async fn get(cfg: &FigmaConfig, url: &str) -> Result<Value, String> {
    reqwest::Client::new()
        .get(url)
        .header("X-Figma-Token", &cfg.token)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<Value>()
        .await
        .map_err(|e| e.to_string())
}

async fn post_json(cfg: &FigmaConfig, url: &str, body: &Value) -> Result<Value, String> {
    reqwest::Client::new()
        .post(url)
        .header("X-Figma-Token", &cfg.token)
        .header("Content-Type", "application/json")
        .json(body)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<Value>()
        .await
        .map_err(|e| e.to_string())
}

async fn delete_req(cfg: &FigmaConfig, url: &str) -> Result<String, String> {
    let resp = reqwest::Client::new()
        .delete(url)
        .header("X-Figma-Token", &cfg.token)
        .send()
        .await
        .map_err(|e| e.to_string())?;
    if resp.status().is_success() {
        Ok("Deleted".to_string())
    } else {
        Err(format!("Error: {}", resp.status()))
    }
}

// ─── Tool definitions ─────────────────────────────────────────────────────────

pub fn tools() -> Vec<Value> {
    vec![
        // ── Files ────────────────────────────────────────────────────────────
        json!({
            "name": "figma_get_file",
            "description": "Get a Figma file by key. Returns a summary: file name, pages, and top-level component count. Use depth to control how many node levels are returned.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "file_key": { "type": "string", "description": "Figma file key (from the URL: figma.com/file/{FILE_KEY}/...)" },
                    "depth":    { "type": "integer", "description": "Number of levels deep to traverse the document tree (default: 2)" }
                },
                "required": ["file_key"]
            }
        }),
        json!({
            "name": "figma_get_file_nodes",
            "description": "Get specific nodes from a Figma file by their IDs. Returns name, type, and bounding box per node.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "file_key":  { "type": "string", "description": "Figma file key" },
                    "node_ids":  { "type": "array", "items": { "type": "string" }, "description": "List of node IDs to retrieve (e.g. ['1:2', '3:4'])" }
                },
                "required": ["file_key", "node_ids"]
            }
        }),
        json!({
            "name": "figma_list_file_versions",
            "description": "List all saved versions of a Figma file. Returns version IDs, labels, descriptions, and timestamps.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "file_key": { "type": "string", "description": "Figma file key" }
                },
                "required": ["file_key"]
            }
        }),
        // ── Comments ─────────────────────────────────────────────────────────
        json!({
            "name": "figma_get_comments",
            "description": "Get all comments on a Figma file. Returns comment IDs, authors, messages, and node references.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "file_key": { "type": "string", "description": "Figma file key" }
                },
                "required": ["file_key"]
            }
        }),
        json!({
            "name": "figma_post_comment",
            "description": "Post a new comment on a Figma file, optionally anchored to a specific node.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "file_key": { "type": "string", "description": "Figma file key" },
                    "message":  { "type": "string", "description": "Comment text" },
                    "node_id":  { "type": "string", "description": "Optional node ID to anchor the comment to a specific element" }
                },
                "required": ["file_key", "message"]
            }
        }),
        json!({
            "name": "figma_delete_comment",
            "description": "Delete a comment on a Figma file by its comment ID.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "file_key":   { "type": "string", "description": "Figma file key" },
                    "comment_id": { "type": "string", "description": "Comment ID to delete" }
                },
                "required": ["file_key", "comment_id"]
            }
        }),
        // ── Components ───────────────────────────────────────────────────────
        json!({
            "name": "figma_get_team_components",
            "description": "List all published components in a Figma team library. Returns component keys, names, and file references.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "team_id": { "type": "string", "description": "Team ID (overrides team_id in config)" }
                }
            }
        }),
        json!({
            "name": "figma_get_component",
            "description": "Get details of a specific Figma component by its key. Returns name, description, and containing file info.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "component_key": { "type": "string", "description": "Component key (from team components list or file)" }
                },
                "required": ["component_key"]
            }
        }),
        json!({
            "name": "figma_get_component_sets",
            "description": "List all component sets (variants) defined in a Figma file.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "file_key": { "type": "string", "description": "Figma file key" }
                },
                "required": ["file_key"]
            }
        }),
        // ── Projects ─────────────────────────────────────────────────────────
        json!({
            "name": "figma_get_team_projects",
            "description": "List all projects in a Figma team. Returns project IDs and names.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "team_id": { "type": "string", "description": "Team ID (overrides team_id in config)" }
                }
            }
        }),
        json!({
            "name": "figma_get_project_files",
            "description": "List all files in a Figma project. Returns file keys, names, and last-modified timestamps.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "project_id": { "type": "string", "description": "Figma project ID" }
                },
                "required": ["project_id"]
            }
        }),
        // ── Variables ────────────────────────────────────────────────────────
        json!({
            "name": "figma_get_local_variables",
            "description": "Get all local variables (design tokens) defined in a Figma file. Returns variable names, types, and values.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "file_key": { "type": "string", "description": "Figma file key" }
                },
                "required": ["file_key"]
            }
        }),
        json!({
            "name": "figma_get_published_variables",
            "description": "Get all variables published to the team library from a Figma file.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "file_key": { "type": "string", "description": "Figma file key" }
                },
                "required": ["file_key"]
            }
        }),
        // ── Export ───────────────────────────────────────────────────────────
        json!({
            "name": "figma_export_images",
            "description": "Export Figma nodes as images. Returns download URLs per node ID. Supported formats: png, jpg, svg, pdf.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "file_key":  { "type": "string", "description": "Figma file key" },
                    "node_ids":  { "type": "array", "items": { "type": "string" }, "description": "List of node IDs to export" },
                    "format":    { "type": "string", "description": "Export format: png (default), jpg, svg, or pdf" },
                    "scale":     { "type": "number", "description": "Export scale factor (e.g. 1.0, 2.0, 3.0 — default 1.0)" }
                },
                "required": ["file_key", "node_ids"]
            }
        }),
        // ── Webhooks ─────────────────────────────────────────────────────────
        json!({
            "name": "figma_list_webhooks",
            "description": "List all webhooks configured for a Figma team.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "team_id": { "type": "string", "description": "Team ID (overrides team_id in config)" }
                }
            }
        }),
    ]
}

// ─── Handler ──────────────────────────────────────────────────────────────────

pub async fn handle(name: &str, args: &Value) -> Result<String, String> {
    let cfg = FigmaConfig::load()
        .ok_or_else(|| "Figma not configured. Create ~/.osmozzz/figma.toml with token (and optionally team_id).".to_string())?;

    match name {
        "figma_get_file"               => get_file(&cfg, args).await,
        "figma_get_file_nodes"         => get_file_nodes(&cfg, args).await,
        "figma_list_file_versions"     => list_file_versions(&cfg, args).await,
        "figma_get_comments"           => get_comments(&cfg, args).await,
        "figma_post_comment"           => post_comment(&cfg, args).await,
        "figma_delete_comment"         => delete_comment(&cfg, args).await,
        "figma_get_team_components"    => get_team_components(&cfg, args).await,
        "figma_get_component"          => get_component(&cfg, args).await,
        "figma_get_component_sets"     => get_component_sets(&cfg, args).await,
        "figma_get_team_projects"      => get_team_projects(&cfg, args).await,
        "figma_get_project_files"      => get_project_files(&cfg, args).await,
        "figma_get_local_variables"    => get_local_variables(&cfg, args).await,
        "figma_get_published_variables" => get_published_variables(&cfg, args).await,
        "figma_export_images"          => export_images(&cfg, args).await,
        "figma_list_webhooks"          => list_webhooks(&cfg, args).await,
        _ => Err(format!("Unknown figma tool: {}", name)),
    }
}

// ─── Files ────────────────────────────────────────────────────────────────────

async fn get_file(cfg: &FigmaConfig, args: &Value) -> Result<String, String> {
    let file_key = args["file_key"].as_str().ok_or("Missing 'file_key'")?;
    let depth    = args["depth"].as_u64().unwrap_or(2);

    let url  = cfg.api(&format!("files/{}?depth={}", file_key, depth));
    let resp = get(cfg, &url).await?;

    if let Some(e) = resp["err"].as_str() {
        return Err(format!("Figma error: {}", e));
    }
    if let Some(status) = resp["status"].as_u64() {
        if status >= 400 {
            let msg = resp["err"].as_str().unwrap_or("unknown error");
            return Err(format!("Figma API error {}: {}", status, msg));
        }
    }

    let name          = resp["name"].as_str().unwrap_or("?");
    let last_modified = resp["lastModified"].as_str().unwrap_or("?");
    let version       = resp["version"].as_str().unwrap_or("?");
    let editor_type   = resp["editorType"].as_str().unwrap_or("?");

    let pages = resp["document"]["children"]
        .as_array()
        .cloned()
        .unwrap_or_default();

    let mut out = format!(
        "File: {}\n  Key:           {}\n  Last modified: {}\n  Version:       {}\n  Editor type:   {}\n  Pages ({}):",
        name, file_key, last_modified, version, editor_type, pages.len()
    );

    for page in &pages {
        let page_name = page["name"].as_str().unwrap_or("?");
        let children  = page["children"].as_array().map(|c| c.len()).unwrap_or(0);
        out.push_str(&format!("\n    - {} ({} top-level nodes)", page_name, children));
    }

    // Count components in the file metadata
    let component_count = resp["components"]
        .as_object()
        .map(|m| m.len())
        .unwrap_or(0);
    let style_count = resp["styles"]
        .as_object()
        .map(|m| m.len())
        .unwrap_or(0);
    out.push_str(&format!(
        "\n  Components: {}\n  Styles:     {}",
        component_count, style_count
    ));

    Ok(out)
}

async fn get_file_nodes(cfg: &FigmaConfig, args: &Value) -> Result<String, String> {
    let file_key = args["file_key"].as_str().ok_or("Missing 'file_key'")?;
    let node_ids = args["node_ids"]
        .as_array()
        .ok_or("Missing 'node_ids'")?
        .iter()
        .filter_map(|v| v.as_str())
        .collect::<Vec<_>>();

    if node_ids.is_empty() {
        return Err("'node_ids' array is empty".to_string());
    }

    let ids_param = node_ids.join(",");
    let url  = cfg.api(&format!("files/{}/nodes?ids={}", file_key, ids_param));
    let resp = get(cfg, &url).await?;

    if let Some(e) = resp["err"].as_str() {
        return Err(format!("Figma error: {}", e));
    }

    let nodes_map = match resp["nodes"].as_object() {
        Some(m) => m.clone(),
        None => return Ok("No nodes found.".to_string()),
    };

    let mut out = format!("Nodes in file {} ({}):\n", file_key, nodes_map.len());
    for (node_id, node_data) in &nodes_map {
        let doc  = &node_data["document"];
        let name = doc["name"].as_str().unwrap_or("?");
        let ntype = doc["type"].as_str().unwrap_or("?");
        let abs_bb = &doc["absoluteBoundingBox"];
        let x = abs_bb["x"].as_f64().unwrap_or(0.0);
        let y = abs_bb["y"].as_f64().unwrap_or(0.0);
        let w = abs_bb["width"].as_f64().unwrap_or(0.0);
        let h = abs_bb["height"].as_f64().unwrap_or(0.0);
        out.push_str(&format!(
            "\n  [{}] {} ({})\n    Bounds: x={:.0} y={:.0} w={:.0} h={:.0}",
            node_id, name, ntype, x, y, w, h
        ));
    }
    Ok(out)
}

async fn list_file_versions(cfg: &FigmaConfig, args: &Value) -> Result<String, String> {
    let file_key = args["file_key"].as_str().ok_or("Missing 'file_key'")?;
    let url  = cfg.api(&format!("files/{}/versions", file_key));
    let resp = get(cfg, &url).await?;

    if let Some(e) = resp["err"].as_str() {
        return Err(format!("Figma error: {}", e));
    }

    let versions = resp["versions"].as_array().cloned().unwrap_or_default();
    if versions.is_empty() {
        return Ok("No saved versions found for this file.".to_string());
    }

    let mut out = format!("Versions of {} ({}):\n", file_key, versions.len());
    for v in &versions {
        let id          = v["id"].as_str().unwrap_or("?");
        let label       = v["label"].as_str().unwrap_or("(auto-save)");
        let description = v["description"].as_str().unwrap_or("");
        let created_at  = v["created_at"].as_str().unwrap_or("?");
        let user        = v["user"]["handle"].as_str().unwrap_or("?");
        out.push_str(&format!(
            "\n  [{}] {} — {} by {}\n    {}",
            id, label, created_at, user, description
        ));
    }
    Ok(out)
}

// ─── Comments ─────────────────────────────────────────────────────────────────

async fn get_comments(cfg: &FigmaConfig, args: &Value) -> Result<String, String> {
    let file_key = args["file_key"].as_str().ok_or("Missing 'file_key'")?;
    let url  = cfg.api(&format!("files/{}/comments", file_key));
    let resp = get(cfg, &url).await?;

    if let Some(e) = resp["err"].as_str() {
        return Err(format!("Figma error: {}", e));
    }

    let comments = resp["comments"].as_array().cloned().unwrap_or_default();
    if comments.is_empty() {
        return Ok("No comments on this file.".to_string());
    }

    let mut out = format!("Comments on {} ({}):\n", file_key, comments.len());
    for c in &comments {
        let id      = c["id"].as_str().unwrap_or("?");
        let author  = c["user"]["handle"].as_str().unwrap_or("?");
        let created = c["created_at"].as_str().unwrap_or("?");
        let resolved = c["resolved_at"].as_str();
        let node_id = c["client_meta"]["node_id"].as_str().unwrap_or("");

        // message is an array of paragraphs
        let message = if let Some(paragraphs) = c["message"].as_array() {
            paragraphs
                .iter()
                .filter_map(|p| {
                    p["paragraphs"]
                        .as_array()
                        .map(|items| {
                            items
                                .iter()
                                .filter_map(|item| item["text"].as_str())
                                .collect::<String>()
                        })
                })
                .collect::<Vec<_>>()
                .join(" ")
        } else {
            c["message"].as_str().unwrap_or("").to_string()
        };

        let status = if resolved.is_some() { "resolved" } else { "open" };
        let node_info = if node_id.is_empty() {
            String::new()
        } else {
            format!(" [node:{}]", node_id)
        };

        out.push_str(&format!(
            "\n  [{}] {} by {} on {}{} ({})\n    {}",
            id, status, author, created, node_info, status, message
        ));
    }
    Ok(out)
}

async fn post_comment(cfg: &FigmaConfig, args: &Value) -> Result<String, String> {
    let file_key = args["file_key"].as_str().ok_or("Missing 'file_key'")?;
    let message  = args["message"].as_str().ok_or("Missing 'message'")?;

    let body = if let Some(node_id) = args["node_id"].as_str() {
        json!({
            "message": message,
            "client_meta": { "node_id": node_id }
        })
    } else {
        json!({ "message": message })
    };

    let url  = cfg.api(&format!("files/{}/comments", file_key));
    let resp = post_json(cfg, &url, &body).await?;

    if let Some(e) = resp["err"].as_str() {
        return Err(format!("Figma error: {}", e));
    }

    let id     = resp["id"].as_str().unwrap_or("?");
    let author = resp["user"]["handle"].as_str().unwrap_or("?");
    let date   = resp["created_at"].as_str().unwrap_or("?");

    Ok(format!(
        "Comment posted\n  ID:      {}\n  Author:  {}\n  Created: {}\n  Message: {}",
        id, author, date, message
    ))
}

async fn delete_comment(cfg: &FigmaConfig, args: &Value) -> Result<String, String> {
    let file_key   = args["file_key"].as_str().ok_or("Missing 'file_key'")?;
    let comment_id = args["comment_id"].as_str().ok_or("Missing 'comment_id'")?;
    let url = cfg.api(&format!("files/{}/comments/{}", file_key, comment_id));
    delete_req(cfg, &url).await
}

// ─── Components ───────────────────────────────────────────────────────────────

async fn get_team_components(cfg: &FigmaConfig, args: &Value) -> Result<String, String> {
    let team_id = cfg.resolve_team(args)?;
    let url     = cfg.api(&format!("teams/{}/components", team_id));
    let resp    = get(cfg, &url).await?;

    if let Some(e) = resp["err"].as_str() {
        return Err(format!("Figma error: {}", e));
    }

    let components = resp["meta"]["components"]
        .as_array()
        .cloned()
        .unwrap_or_default();

    if components.is_empty() {
        return Ok(format!("No published components found for team {}.", team_id));
    }

    let mut out = format!("Team components ({}):\n", components.len());
    for c in &components {
        let key         = c["key"].as_str().unwrap_or("?");
        let name        = c["name"].as_str().unwrap_or("?");
        let description = c["description"].as_str().unwrap_or("");
        let file_name   = c["containing_frame"]["containingStateGroup"]["name"]
            .as_str()
            .unwrap_or(c["file_key"].as_str().unwrap_or("?"));
        out.push_str(&format!(
            "\n  [{}] {} — {} | {}",
            key, name, file_name, description
        ));
    }
    Ok(out)
}

async fn get_component(cfg: &FigmaConfig, args: &Value) -> Result<String, String> {
    let component_key = args["component_key"].as_str().ok_or("Missing 'component_key'")?;
    let url  = cfg.api(&format!("components/{}", component_key));
    let resp = get(cfg, &url).await?;

    if let Some(e) = resp["err"].as_str() {
        return Err(format!("Figma error: {}", e));
    }

    let meta = &resp["meta"];
    let name        = meta["name"].as_str().unwrap_or("?");
    let description = meta["description"].as_str().unwrap_or("");
    let file_key    = meta["file_key"].as_str().unwrap_or("?");
    let node_id     = meta["node_id"].as_str().unwrap_or("?");
    let created_at  = meta["created_at"].as_str().unwrap_or("?");
    let updated_at  = meta["updated_at"].as_str().unwrap_or("?");

    Ok(format!(
        "Component: {}\n  Key:         {}\n  File:        {}\n  Node ID:     {}\n  Description: {}\n  Created:     {}\n  Updated:     {}",
        name, component_key, file_key, node_id, description, created_at, updated_at
    ))
}

async fn get_component_sets(cfg: &FigmaConfig, args: &Value) -> Result<String, String> {
    let file_key = args["file_key"].as_str().ok_or("Missing 'file_key'")?;
    let url  = cfg.api(&format!("files/{}/component_sets", file_key));
    let resp = get(cfg, &url).await?;

    if let Some(e) = resp["err"].as_str() {
        return Err(format!("Figma error: {}", e));
    }

    let sets = resp["meta"]["component_sets"]
        .as_array()
        .cloned()
        .unwrap_or_default();

    if sets.is_empty() {
        return Ok(format!("No component sets found in file {}.", file_key));
    }

    let mut out = format!("Component sets in {} ({}):\n", file_key, sets.len());
    for s in &sets {
        let key         = s["key"].as_str().unwrap_or("?");
        let name        = s["name"].as_str().unwrap_or("?");
        let description = s["description"].as_str().unwrap_or("");
        let node_id     = s["node_id"].as_str().unwrap_or("?");
        out.push_str(&format!(
            "\n  [{}] {} (node:{}) — {}",
            key, name, node_id, description
        ));
    }
    Ok(out)
}

// ─── Projects ─────────────────────────────────────────────────────────────────

async fn get_team_projects(cfg: &FigmaConfig, args: &Value) -> Result<String, String> {
    let team_id = cfg.resolve_team(args)?;
    let url     = cfg.api(&format!("teams/{}/projects", team_id));
    let resp    = get(cfg, &url).await?;

    if let Some(e) = resp["err"].as_str() {
        return Err(format!("Figma error: {}", e));
    }

    let projects = resp["projects"].as_array().cloned().unwrap_or_default();
    if projects.is_empty() {
        return Ok(format!("No projects found for team {}.", team_id));
    }

    let mut out = format!("Projects in team {} ({}):\n", team_id, projects.len());
    for p in &projects {
        let id   = p["id"].as_str().unwrap_or("?");
        let name = p["name"].as_str().unwrap_or("?");
        out.push_str(&format!("\n  [{}] {}", id, name));
    }
    Ok(out)
}

async fn get_project_files(cfg: &FigmaConfig, args: &Value) -> Result<String, String> {
    let project_id = args["project_id"].as_str().ok_or("Missing 'project_id'")?;
    let url  = cfg.api(&format!("projects/{}/files", project_id));
    let resp = get(cfg, &url).await?;

    if let Some(e) = resp["err"].as_str() {
        return Err(format!("Figma error: {}", e));
    }

    let files = resp["files"].as_array().cloned().unwrap_or_default();
    if files.is_empty() {
        return Ok(format!("No files found in project {}.", project_id));
    }

    let mut out = format!("Files in project {} ({}):\n", project_id, files.len());
    for f in &files {
        let key           = f["key"].as_str().unwrap_or("?");
        let name          = f["name"].as_str().unwrap_or("?");
        let last_modified = f["last_modified"].as_str().unwrap_or("?");
        let thumbnail     = f["thumbnail_url"].as_str().unwrap_or("");
        out.push_str(&format!(
            "\n  [{}] {} — last modified: {}\n    Thumbnail: {}",
            key, name, last_modified, thumbnail
        ));
    }
    Ok(out)
}

// ─── Variables ────────────────────────────────────────────────────────────────

async fn get_local_variables(cfg: &FigmaConfig, args: &Value) -> Result<String, String> {
    let file_key = args["file_key"].as_str().ok_or("Missing 'file_key'")?;
    let url  = cfg.api(&format!("files/{}/variables/local", file_key));
    let resp = get(cfg, &url).await?;

    if let Some(e) = resp["err"].as_str() {
        return Err(format!("Figma error: {}", e));
    }
    if let Some(status) = resp["status"].as_u64() {
        if status >= 400 {
            return Err(format!(
                "Figma API error {}: {}",
                status,
                resp["err"].as_str().unwrap_or("unknown")
            ));
        }
    }

    let meta      = &resp["meta"];
    let variables = meta["variables"].as_object();
    let collections = meta["variableCollections"].as_object();

    let var_count = variables.map(|m| m.len()).unwrap_or(0);
    let col_count = collections.map(|m| m.len()).unwrap_or(0);

    if var_count == 0 && col_count == 0 {
        return Ok(format!("No local variables found in file {}.", file_key));
    }

    let mut out = format!(
        "Local variables in {} — {} variable(s) in {} collection(s):\n",
        file_key, var_count, col_count
    );

    if let Some(cols) = collections {
        for (col_id, col) in cols.iter().take(20) {
            let col_name = col["name"].as_str().unwrap_or(col_id.as_str());
            let modes    = col["modes"]
                .as_array()
                .map(|m| {
                    m.iter()
                        .filter_map(|mode| mode["name"].as_str())
                        .collect::<Vec<_>>()
                        .join(", ")
                })
                .unwrap_or_default();
            out.push_str(&format!("\n  Collection: {} — modes: [{}]", col_name, modes));
        }
    }

    if let Some(vars) = variables {
        out.push_str("\n\n  Variables (first 30):");
        for (var_id, var) in vars.iter().take(30) {
            let name     = var["name"].as_str().unwrap_or(var_id.as_str());
            let resolved = var["resolvedType"].as_str().unwrap_or("?");
            out.push_str(&format!("\n    {} ({})", name, resolved));
        }
        if vars.len() > 30 {
            out.push_str(&format!("\n    … and {} more", vars.len() - 30));
        }
    }

    Ok(out)
}

async fn get_published_variables(cfg: &FigmaConfig, args: &Value) -> Result<String, String> {
    let file_key = args["file_key"].as_str().ok_or("Missing 'file_key'")?;
    let url  = cfg.api(&format!("files/{}/variables/published", file_key));
    let resp = get(cfg, &url).await?;

    if let Some(e) = resp["err"].as_str() {
        return Err(format!("Figma error: {}", e));
    }
    if let Some(status) = resp["status"].as_u64() {
        if status >= 400 {
            return Err(format!(
                "Figma API error {}: {}",
                status,
                resp["err"].as_str().unwrap_or("unknown")
            ));
        }
    }

    let meta      = &resp["meta"];
    let variables = meta["variables"].as_object();
    let collections = meta["variableCollections"].as_object();

    let var_count = variables.map(|m| m.len()).unwrap_or(0);
    let col_count = collections.map(|m| m.len()).unwrap_or(0);

    if var_count == 0 && col_count == 0 {
        return Ok(format!("No published variables found in file {}.", file_key));
    }

    let mut out = format!(
        "Published variables in {} — {} variable(s) in {} collection(s):\n",
        file_key, var_count, col_count
    );

    if let Some(cols) = collections {
        for (col_id, col) in cols.iter().take(20) {
            let col_name = col["name"].as_str().unwrap_or(col_id.as_str());
            let hidden   = col["hiddenFromPublishing"].as_bool().unwrap_or(false);
            out.push_str(&format!(
                "\n  Collection: {} {}",
                col_name,
                if hidden { "(hidden from publishing)" } else { "" }
            ));
        }
    }

    if let Some(vars) = variables {
        out.push_str("\n\n  Variables (first 30):");
        for (var_id, var) in vars.iter().take(30) {
            let name     = var["name"].as_str().unwrap_or(var_id.as_str());
            let resolved = var["resolvedType"].as_str().unwrap_or("?");
            let hidden   = var["hiddenFromPublishing"].as_bool().unwrap_or(false);
            out.push_str(&format!(
                "\n    {} ({}){}",
                name,
                resolved,
                if hidden { " [hidden]" } else { "" }
            ));
        }
        if vars.len() > 30 {
            out.push_str(&format!("\n    … and {} more", vars.len() - 30));
        }
    }

    Ok(out)
}

// ─── Export ───────────────────────────────────────────────────────────────────

async fn export_images(cfg: &FigmaConfig, args: &Value) -> Result<String, String> {
    let file_key = args["file_key"].as_str().ok_or("Missing 'file_key'")?;
    let node_ids = args["node_ids"]
        .as_array()
        .ok_or("Missing 'node_ids'")?
        .iter()
        .filter_map(|v| v.as_str())
        .collect::<Vec<_>>();

    if node_ids.is_empty() {
        return Err("'node_ids' array is empty".to_string());
    }

    let format = args["format"].as_str().unwrap_or("png");
    let scale  = args["scale"].as_f64().unwrap_or(1.0);
    let ids    = node_ids.join(",");

    let url = cfg.api(&format!(
        "images/{}?ids={}&format={}&scale={}",
        file_key, ids, format, scale
    ));
    let resp = get(cfg, &url).await?;

    if let Some(e) = resp["err"].as_str() {
        return Err(format!("Figma error: {}", e));
    }

    let images = match resp["images"].as_object() {
        Some(m) => m.clone(),
        None => return Err("No image URLs returned.".to_string()),
    };

    let mut out = format!(
        "Export images (format={}, scale={:.1}x) — {} node(s):\n",
        format,
        scale,
        images.len()
    );
    for (node_id, url_val) in &images {
        let url_str = url_val.as_str().unwrap_or("(null — node may be invisible)");
        out.push_str(&format!("\n  Node {}: {}", node_id, url_str));
    }
    Ok(out)
}

// ─── Webhooks ─────────────────────────────────────────────────────────────────

async fn list_webhooks(cfg: &FigmaConfig, args: &Value) -> Result<String, String> {
    let team_id = cfg.resolve_team(args)?;
    let url     = cfg.api(&format!("webhooks?team_id={}", team_id));
    let resp    = get(cfg, &url).await?;

    if let Some(e) = resp["err"].as_str() {
        return Err(format!("Figma error: {}", e));
    }

    let webhooks = resp["webhooks"].as_array().cloned().unwrap_or_default();
    if webhooks.is_empty() {
        return Ok(format!("No webhooks configured for team {}.", team_id));
    }

    let mut out = format!("Webhooks for team {} ({}):\n", team_id, webhooks.len());
    for w in &webhooks {
        let id         = w["id"].as_str().unwrap_or("?");
        let event_type = w["event_type"].as_str().unwrap_or("?");
        let endpoint   = w["endpoint"].as_str().unwrap_or("?");
        let status     = w["status"].as_str().unwrap_or("?");
        let description = w["description"].as_str().unwrap_or("");
        out.push_str(&format!(
            "\n  [{}] {} → {} ({})\n    {}",
            id, event_type, endpoint, status, description
        ));
    }
    Ok(out)
}
