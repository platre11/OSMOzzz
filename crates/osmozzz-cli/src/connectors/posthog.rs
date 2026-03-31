/// Connecteur PostHog — Analytics & Feature Flags REST API.
/// Toutes les fonctions retournent Result<String, String> sans jamais appeler send().
/// La couche sécurité (sanitize_proxy_response + audit_log) est appliquée par le dispatcher (mcp.rs).
use serde_json::{json, Value};

// ─── Config ──────────────────────────────────────────────────────────────────

#[derive(serde::Deserialize)]
struct PosthogConfig {
    api_key:    String,
    project_id: String,
    #[serde(default = "default_host")]
    host:       String,
}

fn default_host() -> String {
    "https://us.posthog.com".to_string()
}

impl PosthogConfig {
    fn load() -> Option<Self> {
        let path = dirs_next::home_dir()?.join(".osmozzz/posthog.toml");
        toml::from_str(&std::fs::read_to_string(path).ok()?).ok()
    }

    /// Base URL for project-scoped endpoints.
    fn project_api(&self, path: &str) -> String {
        format!(
            "{}/api/projects/{}{}",
            self.host.trim_end_matches('/'),
            self.project_id,
            path
        )
    }

    /// Base URL for global endpoints (not project-scoped).
    fn global_api(&self, path: &str) -> String {
        format!("{}{}", self.host.trim_end_matches('/'), path)
    }
}

// ─── HTTP helpers ─────────────────────────────────────────────────────────────

async fn get(cfg: &PosthogConfig, url: &str) -> Result<Value, String> {
    reqwest::Client::new()
        .get(url)
        .header("Authorization", format!("Bearer {}", cfg.api_key))
        .header("Accept", "application/json")
        .send().await.map_err(|e| e.to_string())?
        .json::<Value>().await.map_err(|e| e.to_string())
}

async fn post_json(cfg: &PosthogConfig, url: &str, body: &Value) -> Result<Value, String> {
    reqwest::Client::new()
        .post(url)
        .header("Authorization", format!("Bearer {}", cfg.api_key))
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .json(body)
        .send().await.map_err(|e| e.to_string())?
        .json::<Value>().await.map_err(|e| e.to_string())
}

async fn patch_json(cfg: &PosthogConfig, url: &str, body: &Value) -> Result<Value, String> {
    reqwest::Client::new()
        .patch(url)
        .header("Authorization", format!("Bearer {}", cfg.api_key))
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .json(body)
        .send().await.map_err(|e| e.to_string())?
        .json::<Value>().await.map_err(|e| e.to_string())
}

async fn delete_req(cfg: &PosthogConfig, url: &str) -> Result<(), String> {
    reqwest::Client::new()
        .delete(url)
        .header("Authorization", format!("Bearer {}", cfg.api_key))
        .send().await.map_err(|e| e.to_string())?;
    Ok(())
}

// ─── Tools definitions ────────────────────────────────────────────────────────

pub fn tools() -> Vec<Value> {
    vec![
        // ── Events ──
        json!({
            "name": "posthog_capture_event",
            "description": "Captures a custom event in PostHog for a given distinct_id. Use for tracking user actions or custom analytics events.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "event":       { "type": "string", "description": "Event name (e.g. 'user_signed_up')" },
                    "distinct_id": { "type": "string", "description": "Unique identifier for the user or entity" },
                    "properties":  { "type": "object", "description": "Additional event properties as key-value pairs (optional)" }
                },
                "required": ["event", "distinct_id"]
            }
        }),
        json!({
            "name": "posthog_query_events",
            "description": "Queries recent PostHog events, optionally filtered by event name. Returns up to limit events.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "event_name": { "type": "string", "description": "Filter by specific event name (optional)" },
                    "limit":      { "type": "integer", "description": "Max number of events to return (default 20, max 100)" }
                },
                "required": []
            }
        }),
        json!({
            "name": "posthog_get_event_definitions",
            "description": "Lists all PostHog event definitions (the event types that have been captured). Returns up to 50 by default.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "limit": { "type": "integer", "description": "Max number of event definitions to return (default 50)" }
                },
                "required": []
            }
        }),
        // ── Persons ──
        json!({
            "name": "posthog_list_persons",
            "description": "Lists PostHog persons (identified users) with their distinct IDs and properties.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "limit": { "type": "integer", "description": "Max number of persons to return (default 20)" }
                },
                "required": []
            }
        }),
        json!({
            "name": "posthog_get_person",
            "description": "Gets full details of a PostHog person by their internal ID, including properties and distinct IDs.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": { "type": "string", "description": "PostHog internal person ID" }
                },
                "required": ["id"]
            }
        }),
        json!({
            "name": "posthog_search_persons",
            "description": "Searches PostHog persons by name, email, or distinct ID. Returns up to 20 matches.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Search query (name, email, or distinct ID)" }
                },
                "required": ["query"]
            }
        }),
        json!({
            "name": "posthog_delete_person",
            "description": "Permanently deletes a PostHog person by their internal ID.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": { "type": "string", "description": "PostHog internal person ID to delete" }
                },
                "required": ["id"]
            }
        }),
        // ── Feature Flags ──
        json!({
            "name": "posthog_list_feature_flags",
            "description": "Lists all PostHog feature flags with their key, name, active status, and rollout percentage.",
            "inputSchema": {
                "type": "object",
                "properties": {},
                "required": []
            }
        }),
        json!({
            "name": "posthog_get_feature_flag",
            "description": "Gets full details of a PostHog feature flag by its numeric ID.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": { "type": "string", "description": "Feature flag numeric ID" }
                },
                "required": ["id"]
            }
        }),
        json!({
            "name": "posthog_create_feature_flag",
            "description": "Creates a new PostHog feature flag with optional rollout percentage (0-100).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "key":                { "type": "string",  "description": "Unique flag key (e.g. 'new-dashboard')" },
                    "name":               { "type": "string",  "description": "Human-readable flag name" },
                    "rollout_percentage": { "type": "integer", "description": "Rollout percentage 0-100 (default 100)" }
                },
                "required": ["key", "name"]
            }
        }),
        json!({
            "name": "posthog_update_feature_flag",
            "description": "Updates a PostHog feature flag's active state and/or rollout percentage.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id":                 { "type": "string",  "description": "Feature flag numeric ID" },
                    "active":             { "type": "boolean", "description": "Whether the flag is active (optional)" },
                    "rollout_percentage": { "type": "integer", "description": "New rollout percentage 0-100 (optional)" }
                },
                "required": ["id"]
            }
        }),
        json!({
            "name": "posthog_toggle_feature_flag",
            "description": "Toggles a PostHog feature flag on or off by ID.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id":     { "type": "string",  "description": "Feature flag numeric ID" },
                    "active": { "type": "boolean", "description": "True to enable the flag, false to disable it" }
                },
                "required": ["id", "active"]
            }
        }),
        // ── Insights ──
        json!({
            "name": "posthog_list_insights",
            "description": "Lists PostHog saved insights (charts/queries) with their name and type.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "limit": { "type": "integer", "description": "Max number of insights to return (default 20)" }
                },
                "required": []
            }
        }),
        json!({
            "name": "posthog_get_insight",
            "description": "Gets full details of a PostHog insight by its numeric ID, including filters and last refresh.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": { "type": "string", "description": "Insight numeric ID" }
                },
                "required": ["id"]
            }
        }),
        json!({
            "name": "posthog_create_trend_insight",
            "description": "Creates a new TRENDS insight in PostHog tracking one or more events over time.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name":   { "type": "string", "description": "Insight name" },
                    "events": {
                        "type":  "array",
                        "items": { "type": "string" },
                        "description": "List of event names to track (e.g. [\"user_signed_up\", \"page_viewed\"])"
                    }
                },
                "required": ["name", "events"]
            }
        }),
        // ── Other ──
        json!({
            "name": "posthog_list_cohorts",
            "description": "Lists all PostHog cohorts (saved user segments) with name, count, and last calculation date.",
            "inputSchema": {
                "type": "object",
                "properties": {},
                "required": []
            }
        }),
        json!({
            "name": "posthog_list_dashboards",
            "description": "Lists all PostHog dashboards with their name, description, and creation date.",
            "inputSchema": {
                "type": "object",
                "properties": {},
                "required": []
            }
        }),
        json!({
            "name": "posthog_list_projects",
            "description": "Lists all PostHog projects accessible with the configured API key.",
            "inputSchema": {
                "type": "object",
                "properties": {},
                "required": []
            }
        }),
    ]
}

// ─── Handler ──────────────────────────────────────────────────────────────────

pub async fn handle(name: &str, args: &Value) -> Result<String, String> {
    let cfg = PosthogConfig::load().ok_or("posthog.toml not configured")?;

    match name {
        // ── Events ───────────────────────────────────────────────────────────
        "posthog_capture_event" => {
            let event       = args["event"].as_str().ok_or("Missing event")?;
            let distinct_id = args["distinct_id"].as_str().ok_or("Missing distinct_id")?;
            let props       = args.get("properties").cloned().unwrap_or(json!({}));
            let url  = cfg.global_api("/capture/");
            let body = json!({
                "api_key":     cfg.api_key,
                "event":       event,
                "distinct_id": distinct_id,
                "properties":  props
            });
            let resp = post_json(&cfg, &url, &body).await?;
            // PostHog capture returns {"status": 1} on success
            let status = resp["status"].as_i64().unwrap_or(0);
            if status == 1 {
                Ok(format!("Event \"{event}\" captured successfully for distinct_id \"{distinct_id}\"."))
            } else {
                Ok(format!("Event \"{event}\" sent. Response: {}", serde_json::to_string(&resp).unwrap_or_default()))
            }
        }

        "posthog_query_events" => {
            let limit      = args["limit"].as_i64().unwrap_or(20).min(100);
            let event_name = args["event_name"].as_str();
            let url = if let Some(ev) = event_name {
                cfg.project_api(&format!("/events?event={}&limit={}", urlenc(ev), limit))
            } else {
                cfg.project_api(&format!("/events?limit={}", limit))
            };
            let resp    = get(&cfg, &url).await?;
            let results = resp["results"].as_array()
                .ok_or("Unexpected response format")?;
            if results.is_empty() {
                return Ok("No events found.".to_string());
            }
            let filter_desc = event_name.map(|e| format!(" (filter: {e})")).unwrap_or_default();
            let mut out = format!("PostHog Events{filter_desc} — {} returned:\n\n", results.len());
            for ev in results {
                let eid    = ev["id"].as_str().or_else(|| ev["uuid"].as_str()).unwrap_or("?");
                let ename  = ev["event"].as_str().unwrap_or("?");
                let did    = ev["distinct_id"].as_str().unwrap_or("?");
                let ts     = ev["timestamp"].as_str().unwrap_or("?");
                out.push_str(&format!("• [{eid}] {ename} | user: {did} | {ts}\n"));
            }
            Ok(out)
        }

        "posthog_get_event_definitions" => {
            let limit = args["limit"].as_i64().unwrap_or(50);
            let url   = cfg.project_api(&format!("/event_definitions?limit={}", limit));
            let resp  = get(&cfg, &url).await?;
            let results = resp["results"].as_array()
                .ok_or("Unexpected response format")?;
            if results.is_empty() {
                return Ok("No event definitions found.".to_string());
            }
            let mut out = format!("PostHog Event Definitions ({} found):\n\n", results.len());
            for ed in results {
                let ename    = ed["name"].as_str().unwrap_or("?");
                let count    = ed["query_usage_30_day"].as_i64().unwrap_or(0);
                let last_see = ed["last_seen_at"].as_str().unwrap_or("?");
                out.push_str(&format!("• {ename} | uses (30d): {count} | last seen: {last_see}\n"));
            }
            Ok(out)
        }

        // ── Persons ──────────────────────────────────────────────────────────
        "posthog_list_persons" => {
            let limit = args["limit"].as_i64().unwrap_or(20);
            let url   = cfg.project_api(&format!("/persons?limit={}", limit));
            let resp  = get(&cfg, &url).await?;
            let results = resp["results"].as_array()
                .ok_or("Unexpected response format")?;
            if results.is_empty() {
                return Ok("No persons found.".to_string());
            }
            let mut out = format!("PostHog Persons ({} found):\n\n", results.len());
            for p in results {
                let id    = p["id"].as_str().or_else(|| p["uuid"].as_str()).unwrap_or("?");
                let name  = p["name"].as_str().unwrap_or("(unnamed)");
                let props = &p["properties"];
                let email = props["email"].as_str().unwrap_or("");
                let dids  = p["distinct_ids"].as_array()
                    .map(|a| a.iter().filter_map(|v| v.as_str()).take(2).collect::<Vec<_>>().join(", "))
                    .unwrap_or_default();
                out.push_str(&format!("• [{id}] {name}"));
                if !email.is_empty() { out.push_str(&format!(" — {email}")); }
                if !dids.is_empty()  { out.push_str(&format!(" | ids: {dids}")); }
                out.push('\n');
            }
            Ok(out)
        }

        "posthog_get_person" => {
            let id   = args["id"].as_str().ok_or("Missing id")?;
            let url  = cfg.project_api(&format!("/persons/{id}"));
            let resp = get(&cfg, &url).await?;
            let props = &resp["properties"];
            let name  = resp["name"].as_str().unwrap_or("(none)");
            let email = props["email"].as_str().unwrap_or("(none)");
            let dids  = resp["distinct_ids"].as_array()
                .map(|a| a.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join(", "))
                .unwrap_or_default();
            let created = resp["created_at"].as_str().unwrap_or("(none)");
            Ok(format!(
                "Person [{id}]\nName:        {name}\nEmail:       {email}\nDistinct IDs: {dids}\nCreated:     {created}"
            ))
        }

        "posthog_search_persons" => {
            let query = args["query"].as_str().ok_or("Missing query")?;
            let url   = cfg.project_api(&format!("/persons?search={}&limit=20", urlenc(query)));
            let resp  = get(&cfg, &url).await?;
            let results = resp["results"].as_array()
                .ok_or("Unexpected response format")?;
            if results.is_empty() {
                return Ok(format!("No persons found for query: {query}"));
            }
            let mut out = format!("Person search results for \"{query}\" ({} found):\n\n", results.len());
            for p in results {
                let id    = p["id"].as_str().or_else(|| p["uuid"].as_str()).unwrap_or("?");
                let name  = p["name"].as_str().unwrap_or("(unnamed)");
                let email = p["properties"]["email"].as_str().unwrap_or("");
                out.push_str(&format!("• [{id}] {name}"));
                if !email.is_empty() { out.push_str(&format!(" — {email}")); }
                out.push('\n');
            }
            Ok(out)
        }

        "posthog_delete_person" => {
            let id  = args["id"].as_str().ok_or("Missing id")?;
            let url = cfg.project_api(&format!("/persons/{id}"));
            delete_req(&cfg, &url).await?;
            Ok(format!("Person [{id}] deleted successfully."))
        }

        // ── Feature Flags ────────────────────────────────────────────────────
        "posthog_list_feature_flags" => {
            let url  = cfg.project_api("/feature_flags");
            let resp = get(&cfg, &url).await?;
            // PostHog returns {"results": [...]} or directly an array
            let results = if let Some(arr) = resp["results"].as_array() {
                arr.clone()
            } else if resp.is_array() {
                resp.as_array().cloned().unwrap_or_default()
            } else {
                return Err("Unexpected response format".to_string());
            };
            if results.is_empty() {
                return Ok("No feature flags found.".to_string());
            }
            let mut out = format!("PostHog Feature Flags ({} found):\n\n", results.len());
            for f in &results {
                let id      = f["id"].as_i64().map(|n| n.to_string()).unwrap_or_else(|| "?".to_string());
                let key     = f["key"].as_str().unwrap_or("?");
                let name    = f["name"].as_str().unwrap_or("(unnamed)");
                let active  = f["active"].as_bool().unwrap_or(false);
                let pct     = f["filters"]["groups"].as_array()
                    .and_then(|g| g.first())
                    .and_then(|g| g["rollout_percentage"].as_i64())
                    .map(|p| format!("{p}%"))
                    .unwrap_or_else(|| "?".to_string());
                let status  = if active { "ENABLED" } else { "DISABLED" };
                out.push_str(&format!("• [{id}] {key} ({name}) — {status} | rollout: {pct}\n"));
            }
            Ok(out)
        }

        "posthog_get_feature_flag" => {
            let id   = args["id"].as_str().ok_or("Missing id")?;
            let url  = cfg.project_api(&format!("/feature_flags/{id}"));
            let resp = get(&cfg, &url).await?;
            let key    = resp["key"].as_str().unwrap_or("(none)");
            let name   = resp["name"].as_str().unwrap_or("(none)");
            let active = resp["active"].as_bool().unwrap_or(false);
            let pct    = resp["filters"]["groups"].as_array()
                .and_then(|g| g.first())
                .and_then(|g| g["rollout_percentage"].as_i64())
                .map(|p| format!("{p}%"))
                .unwrap_or_else(|| "?".to_string());
            let created = resp["created_at"].as_str().unwrap_or("(none)");
            let status  = if active { "ENABLED" } else { "DISABLED" };
            Ok(format!(
                "Feature Flag [{id}]\nKey:      {key}\nName:     {name}\nStatus:   {status}\nRollout:  {pct}\nCreated:  {created}"
            ))
        }

        "posthog_create_feature_flag" => {
            let key  = args["key"].as_str().ok_or("Missing key")?;
            let name = args["name"].as_str().ok_or("Missing name")?;
            let pct  = args["rollout_percentage"].as_i64().unwrap_or(100);
            let url  = cfg.project_api("/feature_flags");
            let body = json!({
                "key":  key,
                "name": name,
                "filters": {
                    "groups": [{ "rollout_percentage": pct }]
                }
            });
            let resp   = post_json(&cfg, &url, &body).await?;
            let new_id = resp["id"].as_i64().map(|n| n.to_string()).unwrap_or_else(|| "?".to_string());
            Ok(format!("Feature flag \"{key}\" created successfully. ID: {new_id}"))
        }

        "posthog_update_feature_flag" => {
            let id  = args["id"].as_str().ok_or("Missing id")?;
            let url = cfg.project_api(&format!("/feature_flags/{id}"));
            let mut body = json!({});
            if let Some(active) = args["active"].as_bool() {
                body["active"] = json!(active);
            }
            if let Some(pct) = args["rollout_percentage"].as_i64() {
                body["filters"] = json!({
                    "groups": [{ "rollout_percentage": pct }]
                });
            }
            let resp       = patch_json(&cfg, &url, &body).await?;
            let updated_id = resp["id"].as_i64().map(|n| n.to_string()).unwrap_or_else(|| id.to_string());
            Ok(format!("Feature flag [{updated_id}] updated successfully."))
        }

        "posthog_toggle_feature_flag" => {
            let id     = args["id"].as_str().ok_or("Missing id")?;
            let active = args["active"].as_bool().ok_or("Missing active (boolean)")?;
            let url    = cfg.project_api(&format!("/feature_flags/{id}"));
            let body   = json!({ "active": active });
            let resp   = patch_json(&cfg, &url, &body).await?;
            let updated_id = resp["id"].as_i64().map(|n| n.to_string()).unwrap_or_else(|| id.to_string());
            let state  = if active { "enabled" } else { "disabled" };
            Ok(format!("Feature flag [{updated_id}] {state} successfully."))
        }

        // ── Insights ─────────────────────────────────────────────────────────
        "posthog_list_insights" => {
            let limit = args["limit"].as_i64().unwrap_or(20);
            let url   = cfg.project_api(&format!("/insights?limit={}", limit));
            let resp  = get(&cfg, &url).await?;
            let results = resp["results"].as_array()
                .ok_or("Unexpected response format")?;
            if results.is_empty() {
                return Ok("No insights found.".to_string());
            }
            let mut out = format!("PostHog Insights ({} found):\n\n", results.len());
            for ins in results {
                let id      = ins["id"].as_i64().map(|n| n.to_string()).unwrap_or_else(|| "?".to_string());
                let name    = ins["name"].as_str().unwrap_or("(unnamed)");
                let insight = ins["filters"]["insight"].as_str().unwrap_or("?");
                let last    = ins["last_refresh"].as_str().unwrap_or("(never)");
                out.push_str(&format!("• [{id}] {name} | type: {insight} | last refresh: {last}\n"));
            }
            Ok(out)
        }

        "posthog_get_insight" => {
            let id   = args["id"].as_str().ok_or("Missing id")?;
            let url  = cfg.project_api(&format!("/insights/{id}"));
            let resp = get(&cfg, &url).await?;
            let name    = resp["name"].as_str().unwrap_or("(none)");
            let insight = resp["filters"]["insight"].as_str().unwrap_or("(none)");
            let desc    = resp["description"].as_str().unwrap_or("(none)");
            let last    = resp["last_refresh"].as_str().unwrap_or("(never)");
            let created = resp["created_at"].as_str().unwrap_or("(none)");
            Ok(format!(
                "Insight [{id}]\nName:         {name}\nType:         {insight}\nDescription:  {desc}\nLast refresh: {last}\nCreated:      {created}"
            ))
        }

        "posthog_create_trend_insight" => {
            let name   = args["name"].as_str().ok_or("Missing name")?;
            let events = args["events"].as_array().ok_or("Missing events (array)")?;
            let event_filters: Vec<Value> = events.iter()
                .filter_map(|e| e.as_str())
                .map(|e| json!({ "id": e }))
                .collect();
            if event_filters.is_empty() {
                return Err("events array must contain at least one event name".to_string());
            }
            let url  = cfg.project_api("/insights");
            let body = json!({
                "name": name,
                "filters": {
                    "events":  event_filters,
                    "insight": "TRENDS"
                }
            });
            let resp   = post_json(&cfg, &url, &body).await?;
            let new_id = resp["id"].as_i64().map(|n| n.to_string()).unwrap_or_else(|| "?".to_string());
            Ok(format!("Trend insight \"{name}\" created successfully. ID: {new_id}"))
        }

        // ── Other ─────────────────────────────────────────────────────────────
        "posthog_list_cohorts" => {
            let url  = cfg.project_api("/cohorts");
            let resp = get(&cfg, &url).await?;
            let results = resp["results"].as_array()
                .ok_or("Unexpected response format")?;
            if results.is_empty() {
                return Ok("No cohorts found.".to_string());
            }
            let mut out = format!("PostHog Cohorts ({} found):\n\n", results.len());
            for c in results {
                let id    = c["id"].as_i64().map(|n| n.to_string()).unwrap_or_else(|| "?".to_string());
                let name  = c["name"].as_str().unwrap_or("(unnamed)");
                let count = c["count"].as_i64().unwrap_or(0);
                let last  = c["last_calculation"].as_str().unwrap_or("(never)");
                out.push_str(&format!("• [{id}] {name} | {count} persons | last calc: {last}\n"));
            }
            Ok(out)
        }

        "posthog_list_dashboards" => {
            let url  = cfg.project_api("/dashboards");
            let resp = get(&cfg, &url).await?;
            let results = resp["results"].as_array()
                .ok_or("Unexpected response format")?;
            if results.is_empty() {
                return Ok("No dashboards found.".to_string());
            }
            let mut out = format!("PostHog Dashboards ({} found):\n\n", results.len());
            for d in results {
                let id      = d["id"].as_i64().map(|n| n.to_string()).unwrap_or_else(|| "?".to_string());
                let name    = d["name"].as_str().unwrap_or("(unnamed)");
                let desc    = d["description"].as_str().unwrap_or("");
                let created = d["created_at"].as_str().unwrap_or("?");
                out.push_str(&format!("• [{id}] {name} | created: {created}"));
                if !desc.is_empty() { out.push_str(&format!(" | {desc}")); }
                out.push('\n');
            }
            Ok(out)
        }

        "posthog_list_projects" => {
            let url  = cfg.global_api("/api/projects/");
            let resp = get(&cfg, &url).await?;
            // PostHog returns {"results": [...]} or just an array
            let results = if let Some(arr) = resp["results"].as_array() {
                arr.clone()
            } else if resp.is_array() {
                resp.as_array().cloned().unwrap_or_default()
            } else {
                return Err("Unexpected response format".to_string());
            };
            if results.is_empty() {
                return Ok("No projects found.".to_string());
            }
            let mut out = format!("PostHog Projects ({} found):\n\n", results.len());
            for p in &results {
                let id      = p["id"].as_i64().map(|n| n.to_string()).unwrap_or_else(|| "?".to_string());
                let name    = p["name"].as_str().unwrap_or("(unnamed)");
                let created = p["created_at"].as_str().unwrap_or("?");
                out.push_str(&format!("• [{id}] {name} | created: {created}\n"));
            }
            Ok(out)
        }

        _ => Err(format!("Unknown PostHog tool: {name}")),
    }
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

/// Minimal percent-encoding for URL query parameters.
fn urlenc(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9'
            | b'-' | b'_' | b'.' | b'~' => out.push(b as char),
            other => out.push_str(&format!("%{:02X}", other)),
        }
    }
    out
}
