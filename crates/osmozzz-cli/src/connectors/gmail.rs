/// Connecteur Gmail natif — IMAP direct (imap.gmail.com:993) + SMTP (smtp.gmail.com).
///
/// Même logique que les handlers gmail_* dans mcp.rs, exposée ici pour le chemin P2P.
/// Config : ~/.osmozzz/gmail.toml  →  username + app_password
use serde_json::{json, Value};

// ─── Config ───────────────────────────────────────────────────────────────────

struct GmailCreds { username: String, password: String }

fn load_creds() -> Result<GmailCreds, String> {
    let path = dirs_next::home_dir()
        .ok_or("Impossible de trouver le dossier home")?
        .join(".osmozzz/gmail.toml");
    let content = std::fs::read_to_string(&path)
        .map_err(|_| "gmail.toml introuvable — configure Gmail dans le dashboard OSMOzzz".to_string())?;
    let t: toml::Value = content.parse()
        .map_err(|e: toml::de::Error| format!("Erreur parsing gmail.toml : {e}"))?;
    let username = t.get("username").and_then(|v| v.as_str())
        .ok_or("Champ 'username' manquant dans gmail.toml")?.to_string();
    let password = t.get("app_password").and_then(|v| v.as_str())
        .ok_or("Champ 'app_password' manquant dans gmail.toml")?.to_string();
    Ok(GmailCreds { username, password })
}

// ─── IMAP ─────────────────────────────────────────────────────────────────────

fn imap_connect(creds: &GmailCreds) -> Result<imap::Session<native_tls::TlsStream<std::net::TcpStream>>, String> {
    let tls = native_tls::TlsConnector::new()
        .map_err(|e| format!("TLS init échoué : {e}"))?;
    let client = imap::connect(("imap.gmail.com", 993), "imap.gmail.com", &tls)
        .map_err(|e| format!("Connexion IMAP échouée : {e}"))?;
    let session = client.login(&creds.username, &creds.password)
        .map_err(|(e, _)| format!("Authentification IMAP échouée : {e}"))?;
    Ok(session)
}

fn fmt_envelope(uid: u32, msg: &imap::types::Fetch) -> String {
    let env = match msg.envelope() { Some(e) => e, None => return format!("UID:{uid}\n") };
    let subject = env.subject.as_deref()
        .and_then(|b| std::str::from_utf8(b).ok())
        .unwrap_or("(sans objet)");
    let from = env.from.as_deref()
        .and_then(|addrs| addrs.first())
        .map(|a| {
            let name = a.name.as_deref().and_then(|b| std::str::from_utf8(b).ok()).unwrap_or("");
            let mbox = a.mailbox.as_deref().and_then(|b| std::str::from_utf8(b).ok()).unwrap_or("");
            let host = a.host.as_deref().and_then(|b| std::str::from_utf8(b).ok()).unwrap_or("");
            if name.is_empty() { format!("{mbox}@{host}") } else { format!("{name} <{mbox}@{host}>") }
        })
        .unwrap_or_else(|| "(inconnu)".to_string());
    let date = env.date.as_deref()
        .and_then(|b| std::str::from_utf8(b).ok())
        .unwrap_or("?");
    format!("UID:{uid}  De:{from}\nObjet:{subject}\nDate:{date}\n")
}

fn imap_search(keyword: &str, limit: usize) -> Result<String, String> {
    let creds = load_creds()?;
    let mut session = imap_connect(&creds)?;
    session.select("INBOX").map_err(|e| format!("Erreur SELECT INBOX : {e}"))?;
    let query = format!("OR SUBJECT \"{}\" BODY \"{}\"", keyword, keyword);
    let uids = session.uid_search(&query).map_err(|e| format!("Erreur SEARCH : {e}"))?;
    if uids.is_empty() {
        let _ = session.logout();
        return Ok(format!("Aucun email trouvé contenant \"{}\".", keyword));
    }
    let mut uids_vec: Vec<u32> = uids.into_iter().collect();
    uids_vec.sort_unstable_by(|a, b| b.cmp(a));
    uids_vec.truncate(limit);
    let uid_set = uids_vec.iter().map(|u| u.to_string()).collect::<Vec<_>>().join(",");
    let messages = session.uid_fetch(&uid_set, "ENVELOPE")
        .map_err(|e| format!("Erreur FETCH : {e}"))?;
    let mut out = format!("📬 {} email(s) trouvé(s) pour \"{}\" :\n\n", messages.len(), keyword);
    for msg in messages.iter() {
        out.push_str(&fmt_envelope(msg.uid.unwrap_or(0), msg));
        out.push('\n');
    }
    out.push_str("─────\nUtilise gmail_read(uid) pour lire le contenu complet.");
    let _ = session.logout();
    Ok(out)
}

fn imap_recent(limit: usize) -> Result<String, String> {
    let creds = load_creds()?;
    let mut session = imap_connect(&creds)?;
    session.select("INBOX").map_err(|e| format!("Erreur SELECT INBOX : {e}"))?;
    let uids = session.uid_search("ALL").map_err(|e| format!("Erreur SEARCH : {e}"))?;
    if uids.is_empty() {
        let _ = session.logout();
        return Ok("Boîte de réception vide.".to_string());
    }
    let mut uids_vec: Vec<u32> = uids.into_iter().collect();
    uids_vec.sort_unstable_by(|a, b| b.cmp(a));
    uids_vec.truncate(limit);
    let uid_set = uids_vec.iter().map(|u| u.to_string()).collect::<Vec<_>>().join(",");
    let messages = session.uid_fetch(&uid_set, "ENVELOPE")
        .map_err(|e| format!("Erreur FETCH : {e}"))?;
    let mut out = format!("📬 {} dernier(s) email(s) :\n\n", messages.len());
    let mut msgs: Vec<_> = messages.iter().collect();
    msgs.sort_by(|a, b| b.uid.cmp(&a.uid));
    for msg in msgs {
        out.push_str(&fmt_envelope(msg.uid.unwrap_or(0), msg));
        out.push('\n');
    }
    out.push_str("─────\nUtilise gmail_read(uid) pour lire le contenu complet.");
    let _ = session.logout();
    Ok(out)
}

fn extract_plain_text(raw: &str) -> String {
    let lower = raw.to_lowercase();
    if let Some(pos) = lower.find("content-type: text/plain") {
        if let Some(body_start) = raw[pos..].find("\r\n\r\n").or_else(|| raw[pos..].find("\n\n")) {
            let body = &raw[pos + body_start..];
            let end = body.find("\n--").unwrap_or(body.len().min(5000));
            return body[..end].trim().to_string();
        }
    }
    raw.chars().take(3000).collect()
}

fn imap_read(uid: &str) -> Result<String, String> {
    let creds = load_creds()?;
    let mut session = imap_connect(&creds)?;
    session.select("INBOX").map_err(|e| format!("Erreur SELECT INBOX : {e}"))?;
    // BODY.PEEK[TEXT] = texte brut uniquement, sans pièces jointes, sans marquer comme lu
    let messages = session.uid_fetch(uid, "ENVELOPE BODY.PEEK[TEXT]")
        .map_err(|e| format!("Erreur FETCH : {e}"))?;
    let msg = messages.iter().next().ok_or("Email introuvable.")?;
    let mut out = String::new();
    out.push_str(&fmt_envelope(msg.uid.unwrap_or(0), msg));
    out.push_str("─────────────────────────────────────\n");
    if let Some(body) = msg.body() {
        let body_str = std::str::from_utf8(body).unwrap_or("(corps non lisible)");
        let text = extract_plain_text(body_str);
        // Limite à 30KB — évite les timeouts P2P sur les gros emails avec pièces jointes
        const MAX_BYTES: usize = 30_000;
        if text.len() > MAX_BYTES {
            let truncated: String = text.chars().take(MAX_BYTES / 4).collect();
            out.push_str(&truncated);
            out.push_str("\n\n[... contenu tronqué à 30KB — email trop volumineux ...]");
        } else {
            out.push_str(&text);
        }
    }
    let _ = session.logout();
    Ok(out)
}

fn imap_by_sender(sender: &str, limit: usize) -> Result<String, String> {
    let creds = load_creds()?;
    let mut session = imap_connect(&creds)?;
    session.select("INBOX").map_err(|e| format!("Erreur SELECT INBOX : {e}"))?;
    let query = format!("FROM \"{}\"", sender);
    let uids = session.uid_search(&query).map_err(|e| format!("Erreur SEARCH : {e}"))?;
    if uids.is_empty() {
        let _ = session.logout();
        return Ok(format!("Aucun email trouvé de \"{}\".", sender));
    }
    let mut uids_vec: Vec<u32> = uids.into_iter().collect();
    uids_vec.sort_unstable_by(|a, b| b.cmp(a));
    uids_vec.truncate(limit);
    let uid_set = uids_vec.iter().map(|u| u.to_string()).collect::<Vec<_>>().join(",");
    let messages = session.uid_fetch(&uid_set, "ENVELOPE")
        .map_err(|e| format!("Erreur FETCH : {e}"))?;
    let mut out = format!("📬 {} email(s) de \"{}\" :\n\n", messages.len(), sender);
    for msg in messages.iter() {
        out.push_str(&fmt_envelope(msg.uid.unwrap_or(0), msg));
        out.push('\n');
    }
    out.push_str("─────\nUtilise gmail_read(uid) pour lire le contenu complet.");
    let _ = session.logout();
    Ok(out)
}

fn imap_stats() -> Result<String, String> {
    let creds = load_creds()?;
    let mut session = imap_connect(&creds)?;
    let mailbox = session.select("INBOX").map_err(|e| format!("Erreur SELECT INBOX : {e}"))?;
    let total = mailbox.exists;
    let unseen_ids = session.search("UNSEEN").map_err(|e| format!("Erreur SEARCH UNSEEN : {e}"))?;
    let unseen = unseen_ids.len();
    let _ = session.logout();
    Ok(format!("📊 Gmail — Boîte de réception\nTotal : {} emails\nNon lus : {}\nCompte : {}", total, unseen, creds.username))
}

fn imap_fetch_headers(uid: &str) -> Result<(String, String, String), String> {
    let creds = load_creds()?;
    let mut session = imap_connect(&creds)?;
    session.select("INBOX").map_err(|e| format!("Erreur SELECT INBOX : {e}"))?;
    let messages = session.uid_fetch(uid, "ENVELOPE")
        .map_err(|e| format!("Erreur FETCH : {e}"))?;
    let msg = messages.iter().next().ok_or("Email introuvable.")?;
    let env = msg.envelope().ok_or("Envelope manquante.")?;
    let from = env.from.as_deref()
        .and_then(|addrs| addrs.first())
        .map(|a| {
            let mbox = a.mailbox.as_deref().and_then(|b| std::str::from_utf8(b).ok()).unwrap_or("");
            let host = a.host.as_deref().and_then(|b| std::str::from_utf8(b).ok()).unwrap_or("");
            format!("{mbox}@{host}")
        })
        .unwrap_or_default();
    let subject = env.subject.as_deref()
        .and_then(|b| std::str::from_utf8(b).ok())
        .unwrap_or("(sans objet)").to_string();
    let message_id = env.message_id.as_deref()
        .and_then(|b| std::str::from_utf8(b).ok())
        .unwrap_or("").to_string();
    let _ = session.logout();
    Ok((from, subject, message_id))
}

async fn smtp_send(to: &str, subject: &str, body: &str) -> Result<(), String> {
    use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor, transport::smtp::authentication::Credentials};
    let creds = load_creds()?;
    let email = Message::builder()
        .from(creds.username.parse().map_err(|e: lettre::address::AddressError| e.to_string())?)
        .to(to.parse().map_err(|e: lettre::address::AddressError| e.to_string())?)
        .subject(subject)
        .body(body.to_string())
        .map_err(|e| e.to_string())?;
    let smtp_creds = Credentials::new(creds.username.clone(), creds.password.clone());
    let mailer = AsyncSmtpTransport::<Tokio1Executor>::relay("smtp.gmail.com")
        .map_err(|e| e.to_string())?
        .credentials(smtp_creds)
        .build();
    mailer.send(email).await.map_err(|e| e.to_string())?;
    Ok(())
}

async fn smtp_reply(to: &str, subject: &str, body: &str, in_reply_to: &str) -> Result<(), String> {
    use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor, transport::smtp::authentication::Credentials};
    let creds = load_creds()?;
    let mut builder = Message::builder()
        .from(creds.username.parse().map_err(|e: lettre::address::AddressError| e.to_string())?)
        .to(to.parse().map_err(|e: lettre::address::AddressError| e.to_string())?)
        .subject(subject);
    if !in_reply_to.is_empty() {
        builder = builder.in_reply_to(in_reply_to.to_string());
    }
    let email = builder.body(body.to_string()).map_err(|e| e.to_string())?;
    let smtp_creds = Credentials::new(creds.username.clone(), creds.password.clone());
    let mailer = AsyncSmtpTransport::<Tokio1Executor>::relay("smtp.gmail.com")
        .map_err(|e| e.to_string())?
        .credentials(smtp_creds)
        .build();
    mailer.send(email).await.map_err(|e| e.to_string())?;
    Ok(())
}

// ─── Interface publique ───────────────────────────────────────────────────────

pub fn tools() -> Vec<Value> {
    vec![
        json!({
            "name": "gmail_search",
            "description": "GMAIL — recherche en temps réel via IMAP par mot-clé dans objet et corps. Retourne liste compacte (objet + expéditeur + UID). Enchaîner avec gmail_read(uid) pour le contenu complet.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "keyword": { "type": "string", "description": "Mot-clé à chercher dans objet et corps" },
                    "limit":   { "type": "number", "description": "Nombre max de résultats (défaut: 20, max: 50)" }
                },
                "required": ["keyword"]
            }
        }),
        json!({
            "name": "gmail_recent",
            "description": "GMAIL — N emails les plus récents de la boîte de réception. QUAND L'UTILISER : 'mes derniers emails', 'qu'est-ce que j'ai reçu'. Retourne liste compacte. Enchaîner avec gmail_read(uid) pour le contenu.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "limit": { "type": "number", "description": "Nombre d'emails à retourner (défaut: 10, max: 50)" }
                }
            }
        }),
        json!({
            "name": "gmail_read",
            "description": "GMAIL — lit le contenu complet d'un email par son UID. QUAND L'UTILISER : après gmail_search, gmail_recent ou gmail_by_sender pour lire le contenu intégral.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "uid": { "type": "string", "description": "UID de l'email obtenu depuis gmail_search ou gmail_recent" }
                },
                "required": ["uid"]
            }
        }),
        json!({
            "name": "gmail_by_sender",
            "description": "GMAIL — emails d'un expéditeur spécifique. Cherche par adresse email ou nom.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "sender": { "type": "string", "description": "Adresse email ou nom de l'expéditeur" },
                    "limit":  { "type": "number", "description": "Nombre max de résultats (défaut: 20)" }
                },
                "required": ["sender"]
            }
        }),
        json!({
            "name": "gmail_send",
            "description": "GMAIL — envoie un email via SMTP Gmail.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "to":      { "type": "string", "description": "Adresse email du destinataire" },
                    "subject": { "type": "string", "description": "Objet de l'email" },
                    "body":    { "type": "string", "description": "Corps de l'email (texte brut)" }
                },
                "required": ["to", "subject", "body"]
            }
        }),
        json!({
            "name": "gmail_reply",
            "description": "GMAIL — répond à un email existant. Utilise l'UID obtenu depuis gmail_search ou gmail_recent.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "uid":  { "type": "string", "description": "UID de l'email auquel répondre" },
                    "body": { "type": "string", "description": "Corps de la réponse" }
                },
                "required": ["uid", "body"]
            }
        }),
        json!({
            "name": "gmail_stats",
            "description": "GMAIL — statistiques de la boîte de réception : total emails, non lus, compte configuré.",
            "inputSchema": { "type": "object", "properties": {} }
        }),
    ]
}

pub async fn handle(name: &str, args: &Value) -> Result<String, String> {
    match name {
        "gmail_search" => {
            let keyword = args["keyword"].as_str().ok_or("Missing param: keyword")?.to_string();
            let limit = args["limit"].as_u64().unwrap_or(20) as usize;
            let limit = limit.clamp(1, 50);
            tokio::task::spawn_blocking(move || imap_search(&keyword, limit))
                .await
                .map_err(|e| e.to_string())?
        }
        "gmail_recent" => {
            let limit = args["limit"].as_u64().unwrap_or(10) as usize;
            let limit = limit.clamp(1, 50);
            tokio::task::spawn_blocking(move || imap_recent(limit))
                .await
                .map_err(|e| e.to_string())?
        }
        "gmail_read" => {
            let uid = args["uid"].as_str().ok_or("Missing param: uid")?.to_string();
            tokio::task::spawn_blocking(move || imap_read(&uid))
                .await
                .map_err(|e| e.to_string())?
        }
        "gmail_by_sender" => {
            let sender = args["sender"].as_str().ok_or("Missing param: sender")?.to_string();
            let limit = args["limit"].as_u64().unwrap_or(20) as usize;
            let limit = limit.clamp(1, 50);
            tokio::task::spawn_blocking(move || imap_by_sender(&sender, limit))
                .await
                .map_err(|e| e.to_string())?
        }
        "gmail_send" => {
            let to      = args["to"].as_str().ok_or("Missing param: to")?.to_string();
            let subject = args["subject"].as_str().ok_or("Missing param: subject")?.to_string();
            let body    = args["body"].as_str().ok_or("Missing param: body")?.to_string();
            smtp_send(&to, &subject, &body).await
                .map(|_| format!("✅ Email envoyé à {}", to))
        }
        "gmail_reply" => {
            let uid  = args["uid"].as_str().ok_or("Missing param: uid")?.to_string();
            let body = args["body"].as_str().ok_or("Missing param: body")?.to_string();
            let uid_clone = uid.clone();
            let (from, subject, message_id) = tokio::task::spawn_blocking(move || imap_fetch_headers(&uid_clone))
                .await
                .map_err(|e| e.to_string())??;
            let reply_subject = if subject.starts_with("Re:") { subject } else { format!("Re: {}", subject) };
            smtp_reply(&from, &reply_subject, &body, &message_id).await
                .map(|_| format!("✅ Réponse envoyée à {}", from))
        }
        "gmail_stats" => {
            tokio::task::spawn_blocking(imap_stats)
                .await
                .map_err(|e| e.to_string())?
        }
        _ => Err(format!("Tool Gmail inconnu : {}", name)),
    }
}
