/// Connecteur HubSpot CRM — REST API v3.
/// Toutes les fonctions retournent Result<String, String> sans jamais appeler send().
/// La couche sécurité (sanitize_proxy_response + audit_log) est appliquée par le dispatcher (mcp.rs).
use serde_json::{json, Value};

// ─── Config ──────────────────────────────────────────────────────────────────

#[derive(serde::Deserialize)]
struct HubspotConfig {
    token: String,
}

impl HubspotConfig {
    fn load() -> Option<Self> {
        let path = dirs_next::home_dir()?.join(".osmozzz/hubspot.toml");
        toml::from_str(&std::fs::read_to_string(path).ok()?).ok()
    }

    fn api(&self, path: &str) -> String {
        format!("https://api.hubapi.com{}", path)
    }
}

// ─── HTTP helpers ─────────────────────────────────────────────────────────────

async fn get(cfg: &HubspotConfig, url: &str) -> Result<Value, String> {
    reqwest::Client::new()
        .get(url)
        .header("Authorization", format!("Bearer {}", cfg.token))
        .header("Accept", "application/json")
        .send().await.map_err(|e| e.to_string())?
        .json::<Value>().await.map_err(|e| e.to_string())
}

async fn post_json(cfg: &HubspotConfig, url: &str, body: &Value) -> Result<Value, String> {
    reqwest::Client::new()
        .post(url)
        .header("Authorization", format!("Bearer {}", cfg.token))
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .json(body)
        .send().await.map_err(|e| e.to_string())?
        .json::<Value>().await.map_err(|e| e.to_string())
}

async fn patch_json(cfg: &HubspotConfig, url: &str, body: &Value) -> Result<Value, String> {
    reqwest::Client::new()
        .patch(url)
        .header("Authorization", format!("Bearer {}", cfg.token))
        .header("Content-Type", "application/json")
        .header("Accept", "application/json")
        .json(body)
        .send().await.map_err(|e| e.to_string())?
        .json::<Value>().await.map_err(|e| e.to_string())
}

async fn delete_req(cfg: &HubspotConfig, url: &str) -> Result<(), String> {
    reqwest::Client::new()
        .delete(url)
        .header("Authorization", format!("Bearer {}", cfg.token))
        .send().await.map_err(|e| e.to_string())?;
    Ok(())
}

// ─── Tools definitions ────────────────────────────────────────────────────────

pub fn tools() -> Vec<Value> {
    vec![
        // ── Contacts ──
        json!({
            "name": "hubspot_list_contacts",
            "description": "Lists the first 50 HubSpot contacts with name, email, and phone.",
            "inputSchema": {
                "type": "object",
                "properties": {},
                "required": []
            }
        }),
        json!({
            "name": "hubspot_get_contact",
            "description": "Gets full details of a HubSpot contact by ID (name, email, phone, company).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": { "type": "string", "description": "HubSpot contact ID" }
                },
                "required": ["id"]
            }
        }),
        json!({
            "name": "hubspot_create_contact",
            "description": "Creates a new HubSpot contact. Returns the new contact ID.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "email":     { "type": "string", "description": "Contact email address" },
                    "firstname": { "type": "string", "description": "First name" },
                    "lastname":  { "type": "string", "description": "Last name" },
                    "phone":     { "type": "string", "description": "Phone number (optional)" }
                },
                "required": ["email", "firstname", "lastname"]
            }
        }),
        json!({
            "name": "hubspot_update_contact",
            "description": "Updates properties of an existing HubSpot contact by ID.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id":         { "type": "string", "description": "HubSpot contact ID" },
                    "properties": { "type": "object", "description": "Key-value pairs of properties to update (e.g. {\"phone\": \"+1555...\"})"}
                },
                "required": ["id", "properties"]
            }
        }),
        json!({
            "name": "hubspot_search_contacts",
            "description": "Searches HubSpot contacts by a text query (name, email, etc.). Returns up to 10 results.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Search query string" }
                },
                "required": ["query"]
            }
        }),
        json!({
            "name": "hubspot_delete_contact",
            "description": "Permanently deletes a HubSpot contact by ID.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": { "type": "string", "description": "HubSpot contact ID to delete" }
                },
                "required": ["id"]
            }
        }),
        // ── Companies ──
        json!({
            "name": "hubspot_list_companies",
            "description": "Lists the first 50 HubSpot companies with name, domain, and ID.",
            "inputSchema": {
                "type": "object",
                "properties": {},
                "required": []
            }
        }),
        json!({
            "name": "hubspot_get_company",
            "description": "Gets full details of a HubSpot company by ID.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": { "type": "string", "description": "HubSpot company ID" }
                },
                "required": ["id"]
            }
        }),
        json!({
            "name": "hubspot_create_company",
            "description": "Creates a new HubSpot company. Returns the new company ID.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name":   { "type": "string", "description": "Company name" },
                    "domain": { "type": "string", "description": "Company domain (optional, e.g. acme.com)" }
                },
                "required": ["name"]
            }
        }),
        json!({
            "name": "hubspot_update_company",
            "description": "Updates properties of an existing HubSpot company by ID.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id":         { "type": "string", "description": "HubSpot company ID" },
                    "properties": { "type": "object", "description": "Key-value pairs of properties to update" }
                },
                "required": ["id", "properties"]
            }
        }),
        json!({
            "name": "hubspot_search_companies",
            "description": "Searches HubSpot companies by name or domain query. Returns up to 10 results.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Search query string" }
                },
                "required": ["query"]
            }
        }),
        // ── Deals ──
        json!({
            "name": "hubspot_list_deals",
            "description": "Lists the first 50 HubSpot deals with name, amount, stage, and pipeline.",
            "inputSchema": {
                "type": "object",
                "properties": {},
                "required": []
            }
        }),
        json!({
            "name": "hubspot_get_deal",
            "description": "Gets full details of a HubSpot deal by ID.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": { "type": "string", "description": "HubSpot deal ID" }
                },
                "required": ["id"]
            }
        }),
        json!({
            "name": "hubspot_create_deal",
            "description": "Creates a new HubSpot deal. Returns the new deal ID.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "dealname":   { "type": "string", "description": "Deal name" },
                    "amount":     { "type": "string", "description": "Deal amount as string (optional)" },
                    "pipeline":   { "type": "string", "description": "Pipeline ID (optional)" },
                    "dealstage":  { "type": "string", "description": "Deal stage ID (optional)" }
                },
                "required": ["dealname"]
            }
        }),
        json!({
            "name": "hubspot_update_deal",
            "description": "Updates properties of an existing HubSpot deal by ID.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id":         { "type": "string", "description": "HubSpot deal ID" },
                    "properties": { "type": "object", "description": "Key-value pairs of properties to update" }
                },
                "required": ["id", "properties"]
            }
        }),
        json!({
            "name": "hubspot_move_deal_stage",
            "description": "Moves a HubSpot deal to a new pipeline stage.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "deal_id":  { "type": "string", "description": "HubSpot deal ID" },
                    "stage_id": { "type": "string", "description": "Target stage ID" }
                },
                "required": ["deal_id", "stage_id"]
            }
        }),
        json!({
            "name": "hubspot_search_deals",
            "description": "Searches HubSpot deals by name query. Returns up to 10 results.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Search query string" }
                },
                "required": ["query"]
            }
        }),
        // ── Tickets ──
        json!({
            "name": "hubspot_list_tickets",
            "description": "Lists the first 50 HubSpot support tickets with subject, status, and pipeline.",
            "inputSchema": {
                "type": "object",
                "properties": {},
                "required": []
            }
        }),
        json!({
            "name": "hubspot_get_ticket",
            "description": "Gets full details of a HubSpot ticket by ID.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id": { "type": "string", "description": "HubSpot ticket ID" }
                },
                "required": ["id"]
            }
        }),
        json!({
            "name": "hubspot_create_ticket",
            "description": "Creates a new HubSpot support ticket. Returns the new ticket ID.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "subject":  { "type": "string", "description": "Ticket subject" },
                    "content":  { "type": "string", "description": "Ticket body/description (optional)" },
                    "pipeline": { "type": "string", "description": "Pipeline ID (optional)" },
                    "status":   { "type": "string", "description": "Ticket status (optional, e.g. OPEN)" }
                },
                "required": ["subject"]
            }
        }),
        json!({
            "name": "hubspot_update_ticket",
            "description": "Updates properties of an existing HubSpot ticket by ID.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "id":         { "type": "string", "description": "HubSpot ticket ID" },
                    "properties": { "type": "object", "description": "Key-value pairs of properties to update" }
                },
                "required": ["id", "properties"]
            }
        }),
        // ── Activities ──
        json!({
            "name": "hubspot_create_note",
            "description": "Creates a note activity in HubSpot, optionally associated with a contact.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "body":       { "type": "string", "description": "Note body text" },
                    "contact_id": { "type": "string", "description": "HubSpot contact ID to associate (optional)" }
                },
                "required": ["body"]
            }
        }),
        json!({
            "name": "hubspot_create_task",
            "description": "Creates a task activity in HubSpot, optionally associated with a contact.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "subject":    { "type": "string", "description": "Task subject/title" },
                    "due_date":   { "type": "string", "description": "Due date as Unix timestamp in ms (optional)" },
                    "contact_id": { "type": "string", "description": "HubSpot contact ID to associate (optional)" }
                },
                "required": ["subject"]
            }
        }),
        json!({
            "name": "hubspot_log_call",
            "description": "Logs a call activity in HubSpot, optionally associated with a contact.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "subject":     { "type": "string", "description": "Call title/subject" },
                    "duration_ms": { "type": "integer", "description": "Call duration in milliseconds (optional)" },
                    "contact_id":  { "type": "string", "description": "HubSpot contact ID to associate (optional)" }
                },
                "required": ["subject"]
            }
        }),
        // ── Pipelines ──
        json!({
            "name": "hubspot_list_pipelines",
            "description": "Lists all HubSpot pipelines for a given object type (contacts, deals, or tickets).",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "object_type": {
                        "type": "string",
                        "description": "Object type: contacts, deals, or tickets",
                        "enum": ["contacts", "deals", "tickets"]
                    }
                },
                "required": ["object_type"]
            }
        }),
        json!({
            "name": "hubspot_list_pipeline_stages",
            "description": "Lists all stages in a specific HubSpot pipeline.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "object_type": {
                        "type": "string",
                        "description": "Object type: contacts, deals, or tickets"
                    },
                    "pipeline_id": {
                        "type": "string",
                        "description": "Pipeline ID"
                    }
                },
                "required": ["object_type", "pipeline_id"]
            }
        }),
    ]
}

// ─── Handler ──────────────────────────────────────────────────────────────────

pub async fn handle(name: &str, args: &Value) -> Result<String, String> {
    let cfg = HubspotConfig::load().ok_or("hubspot.toml not configured")?;

    match name {
        // ── Contacts ─────────────────────────────────────────────────────────
        "hubspot_list_contacts" => {
            let url = cfg.api("/crm/v3/objects/contacts?limit=50&properties=firstname,lastname,email,phone");
            let resp = get(&cfg, &url).await?;
            let results = resp["results"].as_array()
                .ok_or("Unexpected response format")?;
            if results.is_empty() {
                return Ok("No contacts found.".to_string());
            }
            let mut out = format!("HubSpot Contacts ({} found):\n\n", results.len());
            for c in results {
                let id = c["id"].as_str().unwrap_or("?");
                let p = &c["properties"];
                let first = p["firstname"].as_str().unwrap_or("");
                let last  = p["lastname"].as_str().unwrap_or("");
                let email = p["email"].as_str().unwrap_or("(no email)");
                let phone = p["phone"].as_str().unwrap_or("");
                out.push_str(&format!("• [{id}] {first} {last} — {email}"));
                if !phone.is_empty() { out.push_str(&format!(" | {phone}")); }
                out.push('\n');
            }
            Ok(out)
        }

        "hubspot_get_contact" => {
            let id = args["id"].as_str().ok_or("Missing id")?;
            let url = cfg.api(&format!(
                "/crm/v3/objects/contacts/{id}?properties=firstname,lastname,email,phone,company"
            ));
            let resp = get(&cfg, &url).await?;
            let p = &resp["properties"];
            let first   = p["firstname"].as_str().unwrap_or("");
            let last    = p["lastname"].as_str().unwrap_or("");
            let email   = p["email"].as_str().unwrap_or("(none)");
            let phone   = p["phone"].as_str().unwrap_or("(none)");
            let company = p["company"].as_str().unwrap_or("(none)");
            Ok(format!(
                "Contact [{id}]\nName:    {first} {last}\nEmail:   {email}\nPhone:   {phone}\nCompany: {company}"
            ))
        }

        "hubspot_create_contact" => {
            let email     = args["email"].as_str().ok_or("Missing email")?;
            let firstname = args["firstname"].as_str().ok_or("Missing firstname")?;
            let lastname  = args["lastname"].as_str().ok_or("Missing lastname")?;
            let mut props = json!({
                "email":     email,
                "firstname": firstname,
                "lastname":  lastname
            });
            if let Some(phone) = args["phone"].as_str() {
                props["phone"] = json!(phone);
            }
            let url  = cfg.api("/crm/v3/objects/contacts");
            let body = json!({ "properties": props });
            let resp = post_json(&cfg, &url, &body).await?;
            let new_id = resp["id"].as_str().unwrap_or("?");
            Ok(format!("Contact created successfully. ID: {new_id}"))
        }

        "hubspot_update_contact" => {
            let id         = args["id"].as_str().ok_or("Missing id")?;
            let properties = args.get("properties").ok_or("Missing properties")?;
            let url  = cfg.api(&format!("/crm/v3/objects/contacts/{id}"));
            let body = json!({ "properties": properties });
            let resp = patch_json(&cfg, &url, &body).await?;
            let updated_id = resp["id"].as_str().unwrap_or(id);
            Ok(format!("Contact [{updated_id}] updated successfully."))
        }

        "hubspot_search_contacts" => {
            let query = args["query"].as_str().ok_or("Missing query")?;
            let url   = cfg.api("/crm/v3/objects/contacts/search");
            let body  = json!({ "query": query, "limit": 10 });
            let resp  = post_json(&cfg, &url, &body).await?;
            let results = resp["results"].as_array()
                .ok_or("Unexpected response format")?;
            if results.is_empty() {
                return Ok(format!("No contacts found for query: {query}"));
            }
            let mut out = format!("Contact search results for \"{query}\" ({} found):\n\n", results.len());
            for c in results {
                let id    = c["id"].as_str().unwrap_or("?");
                let p     = &c["properties"];
                let first = p["firstname"].as_str().unwrap_or("");
                let last  = p["lastname"].as_str().unwrap_or("");
                let email = p["email"].as_str().unwrap_or("(no email)");
                out.push_str(&format!("• [{id}] {first} {last} — {email}\n"));
            }
            Ok(out)
        }

        "hubspot_delete_contact" => {
            let id  = args["id"].as_str().ok_or("Missing id")?;
            let url = cfg.api(&format!("/crm/v3/objects/contacts/{id}"));
            delete_req(&cfg, &url).await?;
            Ok(format!("Contact [{id}] deleted successfully."))
        }

        // ── Companies ────────────────────────────────────────────────────────
        "hubspot_list_companies" => {
            let url = cfg.api("/crm/v3/objects/companies?limit=50&properties=name,domain,phone");
            let resp = get(&cfg, &url).await?;
            let results = resp["results"].as_array()
                .ok_or("Unexpected response format")?;
            if results.is_empty() {
                return Ok("No companies found.".to_string());
            }
            let mut out = format!("HubSpot Companies ({} found):\n\n", results.len());
            for c in results {
                let id     = c["id"].as_str().unwrap_or("?");
                let p      = &c["properties"];
                let name   = p["name"].as_str().unwrap_or("(unnamed)");
                let domain = p["domain"].as_str().unwrap_or("");
                out.push_str(&format!("• [{id}] {name}"));
                if !domain.is_empty() { out.push_str(&format!(" ({domain})")); }
                out.push('\n');
            }
            Ok(out)
        }

        "hubspot_get_company" => {
            let id  = args["id"].as_str().ok_or("Missing id")?;
            let url = cfg.api(&format!(
                "/crm/v3/objects/companies/{id}?properties=name,domain,phone,city,country,industry"
            ));
            let resp    = get(&cfg, &url).await?;
            let p       = &resp["properties"];
            let name    = p["name"].as_str().unwrap_or("(none)");
            let domain  = p["domain"].as_str().unwrap_or("(none)");
            let phone   = p["phone"].as_str().unwrap_or("(none)");
            let city    = p["city"].as_str().unwrap_or("(none)");
            let country = p["country"].as_str().unwrap_or("(none)");
            let industry = p["industry"].as_str().unwrap_or("(none)");
            Ok(format!(
                "Company [{id}]\nName:     {name}\nDomain:   {domain}\nPhone:    {phone}\nCity:     {city}\nCountry:  {country}\nIndustry: {industry}"
            ))
        }

        "hubspot_create_company" => {
            let name = args["name"].as_str().ok_or("Missing name")?;
            let mut props = json!({ "name": name });
            if let Some(domain) = args["domain"].as_str() {
                props["domain"] = json!(domain);
            }
            let url  = cfg.api("/crm/v3/objects/companies");
            let body = json!({ "properties": props });
            let resp = post_json(&cfg, &url, &body).await?;
            let new_id = resp["id"].as_str().unwrap_or("?");
            Ok(format!("Company created successfully. ID: {new_id}"))
        }

        "hubspot_update_company" => {
            let id         = args["id"].as_str().ok_or("Missing id")?;
            let properties = args.get("properties").ok_or("Missing properties")?;
            let url  = cfg.api(&format!("/crm/v3/objects/companies/{id}"));
            let body = json!({ "properties": properties });
            let resp = patch_json(&cfg, &url, &body).await?;
            let updated_id = resp["id"].as_str().unwrap_or(id);
            Ok(format!("Company [{updated_id}] updated successfully."))
        }

        "hubspot_search_companies" => {
            let query = args["query"].as_str().ok_or("Missing query")?;
            let url   = cfg.api("/crm/v3/objects/companies/search");
            let body  = json!({ "query": query, "limit": 10 });
            let resp  = post_json(&cfg, &url, &body).await?;
            let results = resp["results"].as_array()
                .ok_or("Unexpected response format")?;
            if results.is_empty() {
                return Ok(format!("No companies found for query: {query}"));
            }
            let mut out = format!("Company search results for \"{query}\" ({} found):\n\n", results.len());
            for c in results {
                let id     = c["id"].as_str().unwrap_or("?");
                let p      = &c["properties"];
                let name   = p["name"].as_str().unwrap_or("(unnamed)");
                let domain = p["domain"].as_str().unwrap_or("");
                out.push_str(&format!("• [{id}] {name}"));
                if !domain.is_empty() { out.push_str(&format!(" ({domain})")); }
                out.push('\n');
            }
            Ok(out)
        }

        // ── Deals ─────────────────────────────────────────────────────────────
        "hubspot_list_deals" => {
            let url = cfg.api(
                "/crm/v3/objects/deals?limit=50&properties=dealname,amount,dealstage,pipeline,closedate"
            );
            let resp = get(&cfg, &url).await?;
            let results = resp["results"].as_array()
                .ok_or("Unexpected response format")?;
            if results.is_empty() {
                return Ok("No deals found.".to_string());
            }
            let mut out = format!("HubSpot Deals ({} found):\n\n", results.len());
            for d in results {
                let id     = d["id"].as_str().unwrap_or("?");
                let p      = &d["properties"];
                let name   = p["dealname"].as_str().unwrap_or("(unnamed)");
                let amount = p["amount"].as_str().unwrap_or("?");
                let stage  = p["dealstage"].as_str().unwrap_or("?");
                out.push_str(&format!("• [{id}] {name} — ${amount} | stage: {stage}\n"));
            }
            Ok(out)
        }

        "hubspot_get_deal" => {
            let id  = args["id"].as_str().ok_or("Missing id")?;
            let url = cfg.api(&format!(
                "/crm/v3/objects/deals/{id}?properties=dealname,amount,dealstage,pipeline,closedate,hs_deal_stage_probability"
            ));
            let resp      = get(&cfg, &url).await?;
            let p         = &resp["properties"];
            let name      = p["dealname"].as_str().unwrap_or("(none)");
            let amount    = p["amount"].as_str().unwrap_or("(none)");
            let stage     = p["dealstage"].as_str().unwrap_or("(none)");
            let pipeline  = p["pipeline"].as_str().unwrap_or("(none)");
            let closedate = p["closedate"].as_str().unwrap_or("(none)");
            Ok(format!(
                "Deal [{id}]\nName:      {name}\nAmount:    ${amount}\nStage:     {stage}\nPipeline:  {pipeline}\nCloseDate: {closedate}"
            ))
        }

        "hubspot_create_deal" => {
            let dealname = args["dealname"].as_str().ok_or("Missing dealname")?;
            let mut props = json!({ "dealname": dealname });
            if let Some(amount)    = args["amount"].as_str()    { props["amount"]    = json!(amount); }
            if let Some(pipeline)  = args["pipeline"].as_str()  { props["pipeline"]  = json!(pipeline); }
            if let Some(dealstage) = args["dealstage"].as_str() { props["dealstage"] = json!(dealstage); }
            let url  = cfg.api("/crm/v3/objects/deals");
            let body = json!({ "properties": props });
            let resp = post_json(&cfg, &url, &body).await?;
            let new_id = resp["id"].as_str().unwrap_or("?");
            Ok(format!("Deal created successfully. ID: {new_id}"))
        }

        "hubspot_update_deal" => {
            let id         = args["id"].as_str().ok_or("Missing id")?;
            let properties = args.get("properties").ok_or("Missing properties")?;
            let url  = cfg.api(&format!("/crm/v3/objects/deals/{id}"));
            let body = json!({ "properties": properties });
            let resp = patch_json(&cfg, &url, &body).await?;
            let updated_id = resp["id"].as_str().unwrap_or(id);
            Ok(format!("Deal [{updated_id}] updated successfully."))
        }

        "hubspot_move_deal_stage" => {
            let deal_id  = args["deal_id"].as_str().ok_or("Missing deal_id")?;
            let stage_id = args["stage_id"].as_str().ok_or("Missing stage_id")?;
            let url  = cfg.api(&format!("/crm/v3/objects/deals/{deal_id}"));
            let body = json!({ "properties": { "dealstage": stage_id } });
            let resp = patch_json(&cfg, &url, &body).await?;
            let updated_id = resp["id"].as_str().unwrap_or(deal_id);
            Ok(format!("Deal [{updated_id}] moved to stage [{stage_id}] successfully."))
        }

        "hubspot_search_deals" => {
            let query = args["query"].as_str().ok_or("Missing query")?;
            let url   = cfg.api("/crm/v3/objects/deals/search");
            let body  = json!({ "query": query, "limit": 10 });
            let resp  = post_json(&cfg, &url, &body).await?;
            let results = resp["results"].as_array()
                .ok_or("Unexpected response format")?;
            if results.is_empty() {
                return Ok(format!("No deals found for query: {query}"));
            }
            let mut out = format!("Deal search results for \"{query}\" ({} found):\n\n", results.len());
            for d in results {
                let id     = d["id"].as_str().unwrap_or("?");
                let p      = &d["properties"];
                let name   = p["dealname"].as_str().unwrap_or("(unnamed)");
                let amount = p["amount"].as_str().unwrap_or("?");
                let stage  = p["dealstage"].as_str().unwrap_or("?");
                out.push_str(&format!("• [{id}] {name} — ${amount} | stage: {stage}\n"));
            }
            Ok(out)
        }

        // ── Tickets ───────────────────────────────────────────────────────────
        "hubspot_list_tickets" => {
            let url = cfg.api(
                "/crm/v3/objects/tickets?limit=50&properties=subject,hs_pipeline_stage,hs_pipeline,content"
            );
            let resp = get(&cfg, &url).await?;
            let results = resp["results"].as_array()
                .ok_or("Unexpected response format")?;
            if results.is_empty() {
                return Ok("No tickets found.".to_string());
            }
            let mut out = format!("HubSpot Tickets ({} found):\n\n", results.len());
            for t in results {
                let id      = t["id"].as_str().unwrap_or("?");
                let p       = &t["properties"];
                let subject = p["subject"].as_str().unwrap_or("(no subject)");
                let status  = p["hs_pipeline_stage"].as_str().unwrap_or("?");
                out.push_str(&format!("• [{id}] {subject} | status: {status}\n"));
            }
            Ok(out)
        }

        "hubspot_get_ticket" => {
            let id  = args["id"].as_str().ok_or("Missing id")?;
            let url = cfg.api(&format!(
                "/crm/v3/objects/tickets/{id}?properties=subject,content,hs_pipeline_stage,hs_pipeline,createdate"
            ));
            let resp    = get(&cfg, &url).await?;
            let p       = &resp["properties"];
            let subject = p["subject"].as_str().unwrap_or("(none)");
            let content = p["content"].as_str().unwrap_or("(none)");
            let status  = p["hs_pipeline_stage"].as_str().unwrap_or("(none)");
            let pipeline = p["hs_pipeline"].as_str().unwrap_or("(none)");
            let created  = p["createdate"].as_str().unwrap_or("(none)");
            Ok(format!(
                "Ticket [{id}]\nSubject:  {subject}\nStatus:   {status}\nPipeline: {pipeline}\nCreated:  {created}\nContent:  {content}"
            ))
        }

        "hubspot_create_ticket" => {
            let subject = args["subject"].as_str().ok_or("Missing subject")?;
            let mut props = json!({ "subject": subject });
            if let Some(content)  = args["content"].as_str()  { props["content"]            = json!(content); }
            if let Some(pipeline) = args["pipeline"].as_str() { props["hs_pipeline"]        = json!(pipeline); }
            if let Some(status)   = args["status"].as_str()   { props["hs_pipeline_stage"]  = json!(status); }
            let url  = cfg.api("/crm/v3/objects/tickets");
            let body = json!({ "properties": props });
            let resp = post_json(&cfg, &url, &body).await?;
            let new_id = resp["id"].as_str().unwrap_or("?");
            Ok(format!("Ticket created successfully. ID: {new_id}"))
        }

        "hubspot_update_ticket" => {
            let id         = args["id"].as_str().ok_or("Missing id")?;
            let properties = args.get("properties").ok_or("Missing properties")?;
            let url  = cfg.api(&format!("/crm/v3/objects/tickets/{id}"));
            let body = json!({ "properties": properties });
            let resp = patch_json(&cfg, &url, &body).await?;
            let updated_id = resp["id"].as_str().unwrap_or(id);
            Ok(format!("Ticket [{updated_id}] updated successfully."))
        }

        // ── Activities ────────────────────────────────────────────────────────
        "hubspot_create_note" => {
            let body_text  = args["body"].as_str().ok_or("Missing body")?;
            let now_ms     = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as i64;
            let props = json!({
                "hs_note_body": body_text,
                "hs_timestamp": now_ms
            });
            let mut body = json!({ "properties": props });
            if let Some(contact_id) = args["contact_id"].as_str() {
                body["associations"] = json!([{
                    "to": { "id": contact_id },
                    "types": [{ "associationCategory": "HUBSPOT_DEFINED", "associationTypeId": 202 }]
                }]);
            }
            let url  = cfg.api("/crm/v3/objects/notes");
            let resp = post_json(&cfg, &url, &body).await?;
            let new_id = resp["id"].as_str().unwrap_or("?");
            Ok(format!("Note created successfully. ID: {new_id}"))
        }

        "hubspot_create_task" => {
            let subject = args["subject"].as_str().ok_or("Missing subject")?;
            let now_ms  = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as i64;
            let mut props = json!({
                "hs_task_subject": subject,
                "hs_timestamp":    now_ms,
                "hs_task_status":  "NOT_STARTED"
            });
            if let Some(due_date) = args["due_date"].as_str() {
                props["hs_task_due_date"] = json!(due_date);
            }
            let mut body = json!({ "properties": props });
            if let Some(contact_id) = args["contact_id"].as_str() {
                body["associations"] = json!([{
                    "to": { "id": contact_id },
                    "types": [{ "associationCategory": "HUBSPOT_DEFINED", "associationTypeId": 204 }]
                }]);
            }
            let url  = cfg.api("/crm/v3/objects/tasks");
            let resp = post_json(&cfg, &url, &body).await?;
            let new_id = resp["id"].as_str().unwrap_or("?");
            Ok(format!("Task created successfully. ID: {new_id}"))
        }

        "hubspot_log_call" => {
            let subject = args["subject"].as_str().ok_or("Missing subject")?;
            let now_ms  = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_millis() as i64;
            let duration_ms = args["duration_ms"].as_i64().unwrap_or(0);
            let mut props = json!({
                "hs_call_title":    subject,
                "hs_call_duration": duration_ms,
                "hs_call_status":   "COMPLETED",
                "hs_timestamp":     now_ms
            });
            // hs_timestamp is required; re-assert in case duration_ms overwrote
            props["hs_timestamp"] = json!(now_ms);
            let mut body = json!({ "properties": props });
            if let Some(contact_id) = args["contact_id"].as_str() {
                body["associations"] = json!([{
                    "to": { "id": contact_id },
                    "types": [{ "associationCategory": "HUBSPOT_DEFINED", "associationTypeId": 194 }]
                }]);
            }
            let url  = cfg.api("/crm/v3/objects/calls");
            let resp = post_json(&cfg, &url, &body).await?;
            let new_id = resp["id"].as_str().unwrap_or("?");
            Ok(format!("Call logged successfully. ID: {new_id}"))
        }

        // ── Pipelines ─────────────────────────────────────────────────────────
        "hubspot_list_pipelines" => {
            let object_type = args["object_type"].as_str().ok_or("Missing object_type")?;
            let url  = cfg.api(&format!("/crm/v3/pipelines/{object_type}"));
            let resp = get(&cfg, &url).await?;
            let results = resp["results"].as_array()
                .ok_or("Unexpected response format")?;
            if results.is_empty() {
                return Ok(format!("No pipelines found for object type: {object_type}"));
            }
            let mut out = format!("HubSpot Pipelines for {object_type} ({} found):\n\n", results.len());
            for pl in results {
                let id    = pl["id"].as_str().unwrap_or("?");
                let label = pl["label"].as_str().unwrap_or("(unnamed)");
                let order = pl["displayOrder"].as_i64().unwrap_or(0);
                out.push_str(&format!("• [{id}] {label} (order: {order})\n"));
            }
            Ok(out)
        }

        "hubspot_list_pipeline_stages" => {
            let object_type = args["object_type"].as_str().ok_or("Missing object_type")?;
            let pipeline_id = args["pipeline_id"].as_str().ok_or("Missing pipeline_id")?;
            let url  = cfg.api(&format!("/crm/v3/pipelines/{object_type}/{pipeline_id}/stages"));
            let resp = get(&cfg, &url).await?;
            let results = resp["results"].as_array()
                .ok_or("Unexpected response format")?;
            if results.is_empty() {
                return Ok(format!("No stages found for pipeline [{pipeline_id}]."));
            }
            let mut out = format!(
                "Stages for pipeline [{pipeline_id}] ({object_type}) — {} stages:\n\n",
                results.len()
            );
            for s in results {
                let id    = s["id"].as_str().unwrap_or("?");
                let label = s["label"].as_str().unwrap_or("(unnamed)");
                let order = s["displayOrder"].as_i64().unwrap_or(0);
                let prob  = s["metadata"]["probability"].as_str().unwrap_or("");
                out.push_str(&format!("• [{id}] {label} (order: {order})"));
                if !prob.is_empty() { out.push_str(&format!(" | probability: {prob}")); }
                out.push('\n');
            }
            Ok(out)
        }

        _ => Err(format!("Unknown HubSpot tool: {name}")),
    }
}
