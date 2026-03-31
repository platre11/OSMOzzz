/// Connecteur Resend — REST API officielle.
/// Toutes les fonctions retournent Result<String, String> sans jamais appeler send().
/// La couche sécurité (sanitize_proxy_response + audit_log) est appliquée par le dispatcher (mcp.rs).
use serde_json::{json, Value};

// ─── Config ──────────────────────────────────────────────────────────────────

#[derive(serde::Deserialize)]
struct ResendConfig {
    api_key: String,
}

impl ResendConfig {
    fn load() -> Option<Self> {
        let path = dirs_next::home_dir()?.join(".osmozzz/resend.toml");
        toml::from_str(&std::fs::read_to_string(path).ok()?).ok()
    }

    fn api(&self, path: &str) -> String {
        format!("https://api.resend.com/{}", path.trim_start_matches('/'))
    }
}

// ─── HTTP helpers ─────────────────────────────────────────────────────────────

async fn get(cfg: &ResendConfig, url: &str) -> Result<Value, String> {
    reqwest::Client::new()
        .get(url)
        .header("Authorization", format!("Bearer {}", cfg.api_key))
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<Value>()
        .await
        .map_err(|e| e.to_string())
}

async fn post_json(cfg: &ResendConfig, url: &str, body: &Value) -> Result<Value, String> {
    reqwest::Client::new()
        .post(url)
        .header("Authorization", format!("Bearer {}", cfg.api_key))
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

async fn delete_req(cfg: &ResendConfig, url: &str) -> Result<Value, String> {
    reqwest::Client::new()
        .delete(url)
        .header("Authorization", format!("Bearer {}", cfg.api_key))
        .header("Accept", "application/json")
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<Value>()
        .await
        .map_err(|e| e.to_string())
}

// ─── Tool definitions ────────────────────────────────────────────────────────

pub fn tools() -> Vec<Value> {
    vec![
        // ── Emails ──────────────────────────────────────────────────────────
        json!({
            "name": "resend_send_email",
            "description": "RESEND 📧 — Envoie un email transactionnel via Resend. Supporte le HTML et/ou le texte brut. Retourne l'id de l'email envoyé.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "from":    { "type": "string", "description": "Adresse expéditeur (ex: hello@example.com ou 'Name <hello@example.com>')" },
                    "to":      { "type": "string", "description": "Adresse destinataire (ex: user@example.com)" },
                    "subject": { "type": "string", "description": "Sujet de l'email" },
                    "html":    { "type": "string", "description": "Corps HTML de l'email (optionnel)" },
                    "text":    { "type": "string", "description": "Corps texte brut de l'email (optionnel)" }
                },
                "required": ["from", "to", "subject"]
            }
        }),
        json!({
            "name": "resend_get_email",
            "description": "RESEND 📧 — Récupère le détail d'un email envoyé via Resend : statut de livraison, destinataire, sujet, date. Utiliser resend_send_email pour obtenir l'email_id.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "email_id": { "type": "string", "description": "ID de l'email (ex: 49a3999c-0ce1-4ea6-ab68-e08835cf401e)" }
                },
                "required": ["email_id"]
            }
        }),
        json!({
            "name": "resend_cancel_email",
            "description": "RESEND 📧 — Annule un email schedulé qui n'a pas encore été envoyé. Retourne le statut de l'annulation.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "email_id": { "type": "string", "description": "ID de l'email à annuler" }
                },
                "required": ["email_id"]
            }
        }),
        // ── Domains ─────────────────────────────────────────────────────────
        json!({
            "name": "resend_list_domains",
            "description": "RESEND 📧 — Liste tous les domaines configurés dans le compte Resend avec leur statut de vérification.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }),
        json!({
            "name": "resend_get_domain",
            "description": "RESEND 📧 — Récupère les détails complets d'un domaine Resend : statut, enregistrements DNS à configurer, région. Utiliser resend_list_domains pour obtenir le domain_id.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "domain_id": { "type": "string", "description": "ID du domaine" }
                },
                "required": ["domain_id"]
            }
        }),
        json!({
            "name": "resend_create_domain",
            "description": "RESEND 📧 — Ajoute un nouveau domaine d'envoi dans Resend. Retourne les enregistrements DNS à configurer pour la vérification.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name":   { "type": "string", "description": "Nom de domaine (ex: example.com)" },
                    "region": { "type": "string", "description": "Région d'envoi : us-east-1, eu-west-1, sa-east-1 (défaut: us-east-1)", "default": "us-east-1" }
                },
                "required": ["name"]
            }
        }),
        json!({
            "name": "resend_verify_domain",
            "description": "RESEND 📧 — Déclenche la vérification DNS d'un domaine Resend. À utiliser après avoir configuré les enregistrements DNS.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "domain_id": { "type": "string", "description": "ID du domaine à vérifier" }
                },
                "required": ["domain_id"]
            }
        }),
        json!({
            "name": "resend_delete_domain",
            "description": "RESEND 📧 — Supprime définitivement un domaine du compte Resend. Action irréversible.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "domain_id": { "type": "string", "description": "ID du domaine à supprimer" }
                },
                "required": ["domain_id"]
            }
        }),
        // ── API Keys ─────────────────────────────────────────────────────────
        json!({
            "name": "resend_list_api_keys",
            "description": "RESEND 📧 — Liste toutes les clés API du compte Resend avec leur nom, permission et date de création.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }),
        json!({
            "name": "resend_create_api_key",
            "description": "RESEND 📧 — Crée une nouvelle clé API Resend. Attention : la clé secrète n'est visible qu'à la création. Retourne la clé générée.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name":       { "type": "string", "description": "Nom de la clé API" },
                    "permission": { "type": "string", "description": "Permission : full_access ou sending_access (défaut: full_access)", "enum": ["full_access", "sending_access"] }
                },
                "required": ["name"]
            }
        }),
        json!({
            "name": "resend_delete_api_key",
            "description": "RESEND 📧 — Révoque et supprime définitivement une clé API Resend. Action irréversible.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "api_key_id": { "type": "string", "description": "ID de la clé API à supprimer" }
                },
                "required": ["api_key_id"]
            }
        }),
        // ── Audiences ────────────────────────────────────────────────────────
        json!({
            "name": "resend_list_audiences",
            "description": "RESEND 📧 — Liste toutes les audiences (listes de contacts) du compte Resend avec leur nom et nombre de contacts.",
            "inputSchema": {
                "type": "object",
                "properties": {}
            }
        }),
        json!({
            "name": "resend_create_audience",
            "description": "RESEND 📧 — Crée une nouvelle audience (liste de contacts) dans Resend. Retourne l'id de l'audience créée.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "name": { "type": "string", "description": "Nom de l'audience" }
                },
                "required": ["name"]
            }
        }),
        json!({
            "name": "resend_delete_audience",
            "description": "RESEND 📧 — Supprime définitivement une audience Resend et tous ses contacts. Action irréversible.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "audience_id": { "type": "string", "description": "ID de l'audience à supprimer" }
                },
                "required": ["audience_id"]
            }
        }),
    ]
}

// ─── Handler ─────────────────────────────────────────────────────────────────

pub async fn handle(name: &str, args: &Value) -> Result<String, String> {
    let cfg = ResendConfig::load()
        .ok_or_else(|| "Resend non configuré — créer ~/.osmozzz/resend.toml avec api_key".to_string())?;

    match name {
        // ── Emails ──────────────────────────────────────────────────────────
        "resend_send_email" => {
            let from    = args["from"].as_str().ok_or("Paramètre 'from' requis")?;
            let to      = args["to"].as_str().ok_or("Paramètre 'to' requis")?;
            let subject = args["subject"].as_str().ok_or("Paramètre 'subject' requis")?;

            let mut body = json!({
                "from":    from,
                "to":      [to],
                "subject": subject,
            });

            if let Some(html) = args["html"].as_str() {
                body["html"] = json!(html);
            }
            if let Some(text) = args["text"].as_str() {
                body["text"] = json!(text);
            }

            let url = cfg.api("/emails");
            let resp = post_json(&cfg, &url, &body).await?;

            let id = resp["id"].as_str().unwrap_or("—");
            Ok(format!(
                "Email envoyé.\nID : {id}\nDe : {from}\nÀ  : {to}\nSujet : {subject}"
            ))
        }

        "resend_get_email" => {
            let email_id = args["email_id"].as_str().ok_or("Paramètre 'email_id' requis")?;
            let url = cfg.api(&format!("/emails/{email_id}"));
            let resp = get(&cfg, &url).await?;

            let from       = resp["from"].as_str().unwrap_or("—");
            let to         = resp["to"].as_array()
                .map(|a| a.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join(", "))
                .unwrap_or_else(|| "—".to_string());
            let subject    = resp["subject"].as_str().unwrap_or("—");
            let status     = resp["last_event"].as_str().unwrap_or("—");
            let created_at = resp["created_at"].as_str().unwrap_or("—");

            Ok(format!(
                "Email {email_id}\nDe      : {from}\nÀ       : {to}\nSujet   : {subject}\nStatut  : {status}\nCréé le : {created_at}"
            ))
        }

        "resend_cancel_email" => {
            let email_id = args["email_id"].as_str().ok_or("Paramètre 'email_id' requis")?;
            let url  = cfg.api(&format!("/emails/{email_id}/cancel"));
            let resp = post_json(&cfg, &url, &json!({})).await?;

            let id     = resp["id"].as_str().unwrap_or(email_id);
            let status = resp["object"].as_str().unwrap_or("cancelled");
            Ok(format!("Email {id} annulé. Statut : {status}"))
        }

        // ── Domains ─────────────────────────────────────────────────────────
        "resend_list_domains" => {
            let url  = cfg.api("/domains");
            let resp = get(&cfg, &url).await?;

            let domains = resp["data"].as_array()
                .or_else(|| resp.as_array())
                .cloned()
                .unwrap_or_default();

            if domains.is_empty() {
                return Ok("Aucun domaine configuré.".to_string());
            }

            let mut out = format!("{} domaine(s) :\n", domains.len());
            for d in &domains {
                let id     = d["id"].as_str().unwrap_or("—");
                let name   = d["name"].as_str().unwrap_or("—");
                let status = d["status"].as_str().unwrap_or("—");
                let region = d["region"].as_str().unwrap_or("—");
                out.push_str(&format!("• [{id}] {name} — statut: {status} — région: {region}\n"));
            }
            Ok(out.trim_end().to_string())
        }

        "resend_get_domain" => {
            let domain_id = args["domain_id"].as_str().ok_or("Paramètre 'domain_id' requis")?;
            let url  = cfg.api(&format!("/domains/{domain_id}"));
            let resp = get(&cfg, &url).await?;

            let name   = resp["name"].as_str().unwrap_or("—");
            let status = resp["status"].as_str().unwrap_or("—");
            let region = resp["region"].as_str().unwrap_or("—");
            let created_at = resp["created_at"].as_str().unwrap_or("—");

            let mut out = format!(
                "Domaine {domain_id}\nNom    : {name}\nStatut : {status}\nRégion : {region}\nCréé   : {created_at}\n"
            );

            if let Some(records) = resp["records"].as_array() {
                if !records.is_empty() {
                    out.push_str("\nEnregistrements DNS :\n");
                    for r in records {
                        let rtype  = r["type"].as_str().unwrap_or("—");
                        let rname  = r["name"].as_str().unwrap_or("—");
                        let value  = r["value"].as_str().unwrap_or("—");
                        let rstatus = r["status"].as_str().unwrap_or("—");
                        out.push_str(&format!("  [{rstatus}] {rtype} {rname} → {value}\n"));
                    }
                }
            }

            Ok(out.trim_end().to_string())
        }

        "resend_create_domain" => {
            let name   = args["name"].as_str().ok_or("Paramètre 'name' requis")?;
            let region = args["region"].as_str().unwrap_or("us-east-1");

            let body = json!({ "name": name, "region": region });
            let url  = cfg.api("/domains");
            let resp = post_json(&cfg, &url, &body).await?;

            let id     = resp["id"].as_str().unwrap_or("—");
            let status = resp["status"].as_str().unwrap_or("—");

            let mut out = format!(
                "Domaine créé.\nID     : {id}\nNom    : {name}\nRégion : {region}\nStatut : {status}\n"
            );

            if let Some(records) = resp["records"].as_array() {
                if !records.is_empty() {
                    out.push_str("\nEnregistrements DNS à configurer :\n");
                    for r in records {
                        let rtype  = r["type"].as_str().unwrap_or("—");
                        let rname  = r["name"].as_str().unwrap_or("—");
                        let value  = r["value"].as_str().unwrap_or("—");
                        out.push_str(&format!("  {rtype} {rname} → {value}\n"));
                    }
                }
            }

            Ok(out.trim_end().to_string())
        }

        "resend_verify_domain" => {
            let domain_id = args["domain_id"].as_str().ok_or("Paramètre 'domain_id' requis")?;
            let url  = cfg.api(&format!("/domains/{domain_id}/verify"));
            let resp = post_json(&cfg, &url, &json!({})).await?;

            let id     = resp["id"].as_str().unwrap_or(domain_id);
            let name   = resp["name"].as_str().unwrap_or("—");
            let status = resp["status"].as_str().unwrap_or("—");
            Ok(format!("Vérification déclenchée.\nID     : {id}\nNom    : {name}\nStatut : {status}"))
        }

        "resend_delete_domain" => {
            let domain_id = args["domain_id"].as_str().ok_or("Paramètre 'domain_id' requis")?;
            let url  = cfg.api(&format!("/domains/{domain_id}"));
            let resp = delete_req(&cfg, &url).await?;

            let id      = resp["id"].as_str().unwrap_or(domain_id);
            let deleted = resp["object"].as_str().unwrap_or("deleted");
            Ok(format!("Domaine {id} supprimé ({deleted})."))
        }

        // ── API Keys ─────────────────────────────────────────────────────────
        "resend_list_api_keys" => {
            let url  = cfg.api("/api-keys");
            let resp = get(&cfg, &url).await?;

            let keys = resp["data"].as_array()
                .or_else(|| resp.as_array())
                .cloned()
                .unwrap_or_default();

            if keys.is_empty() {
                return Ok("Aucune clé API.".to_string());
            }

            let mut out = format!("{} clé(s) API :\n", keys.len());
            for k in &keys {
                let id         = k["id"].as_str().unwrap_or("—");
                let name       = k["name"].as_str().unwrap_or("—");
                let permission = k["permission"].as_str().unwrap_or("—");
                let created_at = k["created_at"].as_str().unwrap_or("—");
                out.push_str(&format!("• [{id}] {name} — {permission} — créée: {created_at}\n"));
            }
            Ok(out.trim_end().to_string())
        }

        "resend_create_api_key" => {
            let name       = args["name"].as_str().ok_or("Paramètre 'name' requis")?;
            let permission = args["permission"].as_str().unwrap_or("full_access");

            let body = json!({ "name": name, "permission": permission });
            let url  = cfg.api("/api-keys");
            let resp = post_json(&cfg, &url, &body).await?;

            let id    = resp["id"].as_str().unwrap_or("—");
            let token = resp["token"].as_str().unwrap_or("—");
            Ok(format!(
                "Clé API créée.\nID         : {id}\nNom        : {name}\nPermission : {permission}\nToken      : {token}\n\n⚠️  Sauvegardez ce token maintenant, il ne sera plus affiché."
            ))
        }

        "resend_delete_api_key" => {
            let api_key_id = args["api_key_id"].as_str().ok_or("Paramètre 'api_key_id' requis")?;
            let url  = cfg.api(&format!("/api-keys/{api_key_id}"));
            // DELETE /api-keys/{id} returns 200 with empty body on success
            let _resp = delete_req(&cfg, &url).await;
            Ok(format!("Clé API {api_key_id} révoquée et supprimée."))
        }

        // ── Audiences ────────────────────────────────────────────────────────
        "resend_list_audiences" => {
            let url  = cfg.api("/audiences");
            let resp = get(&cfg, &url).await?;

            let audiences = resp["data"].as_array()
                .or_else(|| resp.as_array())
                .cloned()
                .unwrap_or_default();

            if audiences.is_empty() {
                return Ok("Aucune audience.".to_string());
            }

            let mut out = format!("{} audience(s) :\n", audiences.len());
            for a in &audiences {
                let id         = a["id"].as_str().unwrap_or("—");
                let name       = a["name"].as_str().unwrap_or("—");
                let created_at = a["created_at"].as_str().unwrap_or("—");
                out.push_str(&format!("• [{id}] {name} — créée: {created_at}\n"));
            }
            Ok(out.trim_end().to_string())
        }

        "resend_create_audience" => {
            let name = args["name"].as_str().ok_or("Paramètre 'name' requis")?;
            let body = json!({ "name": name });
            let url  = cfg.api("/audiences");
            let resp = post_json(&cfg, &url, &body).await?;

            let id = resp["id"].as_str().unwrap_or("—");
            Ok(format!("Audience créée.\nID  : {id}\nNom : {name}"))
        }

        "resend_delete_audience" => {
            let audience_id = args["audience_id"].as_str().ok_or("Paramètre 'audience_id' requis")?;
            let url  = cfg.api(&format!("/audiences/{audience_id}"));
            let resp = delete_req(&cfg, &url).await?;

            let id      = resp["id"].as_str().unwrap_or(audience_id);
            let deleted = resp["object"].as_str().unwrap_or("deleted");
            Ok(format!("Audience {id} supprimée ({deleted})."))
        }

        _ => Err(format!("Tool Resend inconnu : {name}")),
    }
}
