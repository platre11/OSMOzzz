/// Exécuteur d'actions approuvées.
///
/// Appelé après qu'un utilisateur a cliqué "Approuver" dans le dashboard.
/// Chaque tool `act_*` a son implémentation ici.
use osmozzz_core::action::ActionRequest;
use tracing::{info, warn};

use lettre::{
    AsyncSmtpTransport, AsyncTransport, Message as EmailMessage,
    Tokio1Executor,
    transport::smtp::authentication::Credentials,
    message::header::ContentType,
};

// ─── Point d'entrée ──────────────────────────────────────────────────────────

/// Exécute une action approuvée. Retourne "ok: ..." ou "err: ...".
pub async fn execute(action: &ActionRequest) -> String {
    info!("[Executor] Exécution de '{}' (id={})", action.tool, action.id);
    match action.tool.as_str() {
        "act_send_email" => match execute_send_email(action).await {
            Ok(msg) => { info!("[Executor] Succès: {msg}"); format!("ok: {msg}") }
            Err(e)  => { warn!("[Executor] Erreur: {e}"); format!("err: {e}") }
        },
        other => {
            let msg = format!("tool '{other}' non supporté par l'executor");
            warn!("[Executor] {msg}");
            format!("err: {msg}")
        }
    }
}

// ─── act_send_email ───────────────────────────────────────────────────────────

async fn execute_send_email(action: &ActionRequest) -> Result<String, String> {
    // Paramètres depuis Claude
    let to = action.params["to"].as_str()
        .ok_or("paramètre 'to' manquant")?;
    let subject = action.params["subject"].as_str().unwrap_or("(sans objet)");
    let body = action.params["body"].as_str().unwrap_or("");

    // Config Gmail depuis ~/.osmozzz/gmail.toml
    let (username, password) = load_gmail_config()
        .ok_or("Gmail non configuré — configurez-le dans le dashboard")?;

    // Construction du message
    let from_addr = username.parse::<lettre::Address>()
        .map_err(|e| format!("adresse from invalide: {e}"))?;
    let to_addr = to.parse::<lettre::Address>()
        .map_err(|e| format!("adresse to invalide: {e}"))?;

    let email = EmailMessage::builder()
        .from(lettre::message::Mailbox::new(None, from_addr))
        .to(lettre::message::Mailbox::new(None, to_addr))
        .subject(subject)
        .header(ContentType::TEXT_PLAIN)
        .body(body.to_string())
        .map_err(|e| format!("construction email: {e}"))?;

    // Connexion SMTP Gmail (port 465, TLS implicite)
    let creds = Credentials::new(username.clone(), password);
    let mailer = AsyncSmtpTransport::<Tokio1Executor>::relay("smtp.gmail.com")
        .map_err(|e| format!("relay SMTP: {e}"))?
        .credentials(creds)
        .build();

    mailer.send(email).await
        .map(|_| format!("email envoyé à {to}"))
        .map_err(|e| format!("envoi SMTP: {e}"))
}

// ─── Helpers ──────────────────────────────────────────────────────────────────

fn load_gmail_config() -> Option<(String, String)> {
    let path = dirs_next::home_dir()?.join(".osmozzz/gmail.toml");
    let content = std::fs::read_to_string(path).ok()?;

    let mut username = None;
    let mut password = None;

    for line in content.lines() {
        let line = line.trim();
        if let Some(rest) = line.strip_prefix("username") {
            if let Some(val) = rest.trim_start_matches(|c: char| c == ' ' || c == '=').strip_prefix('"') {
                username = val.strip_suffix('"').map(String::from);
            }
        } else if let Some(rest) = line.strip_prefix("password") {
            if let Some(val) = rest.trim_start_matches(|c: char| c == ' ' || c == '=').strip_prefix('"') {
                password = val.strip_suffix('"').map(String::from);
            }
        }
    }

    Some((username?, password?))
}
