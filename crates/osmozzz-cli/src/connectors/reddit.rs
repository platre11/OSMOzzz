/// Connecteur Reddit — REST API OAuth2 (script app).
/// Auth : échange client_id + client_secret + username + password contre un Bearer token.
/// La couche sécurité (sanitize_proxy_response + audit_log) est appliquée par le dispatcher (mcp.rs).
use reqwest::Client;
use serde_json::{json, Value};

// ─── Config ──────────────────────────────────────────────────────────────────

#[derive(serde::Deserialize, Clone)]
struct RedditConfig {
    client_id:     String,
    client_secret: String,
    username:      String,
    password:      String,
}

impl RedditConfig {
    fn load() -> Option<Self> {
        let path = dirs_next::home_dir()?.join(".osmozzz/reddit.toml");
        toml::from_str(&std::fs::read_to_string(path).ok()?).ok()
    }
}

// ─── Auth ─────────────────────────────────────────────────────────────────────

async fn get_token(cfg: &RedditConfig) -> Result<String, String> {
    let resp = Client::new()
        .post("https://www.reddit.com/api/v1/access_token")
        .basic_auth(&cfg.client_id, Some(&cfg.client_secret))
        .header("User-Agent", "OSMOzzz/1.0")
        .form(&[
            ("grant_type", "password"),
            ("username",   cfg.username.as_str()),
            ("password",   cfg.password.as_str()),
        ])
        .send()
        .await
        .map_err(|e| format!("Reddit auth error: {e}"))?
        .json::<Value>()
        .await
        .map_err(|e| format!("Reddit auth parse error: {e}"))?;

    resp["access_token"]
        .as_str()
        .map(|s| s.to_string())
        .ok_or_else(|| {
            let err = resp["error"].as_str().unwrap_or("unknown");
            format!("Reddit auth failed: {err}. Vérifiez client_id, client_secret, username, password dans ~/.osmozzz/reddit.toml")
        })
}

// ─── HTTP helpers ─────────────────────────────────────────────────────────────

async fn get(token: &str, url: &str) -> Result<Value, String> {
    Client::new()
        .get(url)
        .header("Authorization", format!("Bearer {token}"))
        .header("User-Agent", "OSMOzzz/1.0")
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<Value>()
        .await
        .map_err(|e| e.to_string())
}

async fn post_form(token: &str, url: &str, params: Vec<(&str, String)>) -> Result<Value, String> {
    let pairs: Vec<(&str, &str)> = params.iter().map(|(k, v)| (*k, v.as_str())).collect();
    Client::new()
        .post(url)
        .header("Authorization", format!("Bearer {token}"))
        .header("User-Agent", "OSMOzzz/1.0")
        .form(&pairs)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .json::<Value>()
        .await
        .map_err(|e| e.to_string())
}

// ─── Formatters ───────────────────────────────────────────────────────────────

fn format_post(p: &Value) -> String {
    let data      = if p["kind"].as_str().is_some() { &p["data"] } else { p };
    let title     = data["title"].as_str().unwrap_or("(sans titre)");
    let subreddit = data["subreddit_name_prefixed"].as_str()
        .or_else(|| data["subreddit"].as_str())
        .unwrap_or("—");
    let score     = data["score"].as_i64().unwrap_or(0);
    let comments  = data["num_comments"].as_i64().unwrap_or(0);
    let id        = data["id"].as_str().unwrap_or("—");
    let url       = data["permalink"].as_str()
        .map(|p| format!("https://reddit.com{p}"))
        .unwrap_or_else(|| "—".to_string());
    format!("• [{id}] {title}\n  {subreddit} | ▲{score} | 💬{comments}\n  {url}")
}

fn format_comment(c: &Value) -> String {
    let data   = if c["kind"].as_str().is_some() { &c["data"] } else { c };
    let body   = data["body"].as_str().unwrap_or("—");
    let author = data["author"].as_str().unwrap_or("—");
    let score  = data["score"].as_i64().unwrap_or(0);
    let id     = data["id"].as_str().unwrap_or("—");
    let sub    = data["subreddit_name_prefixed"].as_str().unwrap_or("");
    let body_preview = if body.len() > 300 { format!("{}…", &body[..300]) } else { body.to_string() };
    format!("• [{id}] u/{author} ({sub}) ▲{score}\n  {body_preview}")
}

// ─── Tool definitions ────────────────────────────────────────────────────────

pub fn tools() -> Vec<Value> {
    vec![
        // ── Contenu personnel ────────────────────────────────────────────────
        json!({
            "name": "reddit_list_saved",
            "description": "REDDIT 🟠 — Liste les posts et commentaires sauvegardés par l'utilisateur connecté. Retourne titre, subreddit, score et lien. Utiliser reddit_get_post_with_comments pour lire le détail.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "limit": { "type": "integer", "default": 25, "minimum": 1, "maximum": 100, "description": "Nombre d'éléments à retourner (défaut: 25)" },
                    "type":  { "type": "string", "enum": ["links", "comments", "all"], "default": "all", "description": "Filtrer par type : links (posts), comments, all (défaut: all)" }
                }
            }
        }),
        json!({
            "name": "reddit_list_upvoted",
            "description": "REDDIT 🟠 — Liste les posts upvotés par l'utilisateur connecté. Retourne titre, subreddit, score et lien.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "limit": { "type": "integer", "default": 25, "minimum": 1, "maximum": 100, "description": "Nombre de posts à retourner (défaut: 25)" }
                }
            }
        }),
        json!({
            "name": "reddit_get_my_comments",
            "description": "REDDIT 🟠 — Liste les derniers commentaires postés par l'utilisateur connecté. Utile pour voir son historique de participation.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "limit": { "type": "integer", "default": 25, "minimum": 1, "maximum": 100, "description": "Nombre de commentaires à retourner (défaut: 25)" }
                }
            }
        }),
        // ── Recherche & exploration ──────────────────────────────────────────
        json!({
            "name": "reddit_search_posts",
            "description": "REDDIT 🟠 — Cherche des posts par mot-clé. Si subreddit est fourni, cherche dans ce subreddit uniquement (ex: 'rust', 'LocalLLaMA'). Sinon cherche sur tout Reddit. Retourne titre, subreddit, score, date et lien.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query":     { "type": "string", "description": "Mots-clés à chercher (ex: 'local AI memory MCP')" },
                    "subreddit": { "type": "string", "description": "Chercher dans ce subreddit uniquement (ex: 'LocalLLaMA') — optionnel" },
                    "sort":      { "type": "string", "enum": ["relevance", "hot", "top", "new"], "default": "relevance", "description": "Tri des résultats (défaut: relevance)" },
                    "limit":     { "type": "integer", "default": 15, "minimum": 1, "maximum": 50, "description": "Nombre de résultats (défaut: 15)" }
                },
                "required": ["query"]
            }
        }),
        json!({
            "name": "reddit_list_hot_posts",
            "description": "REDDIT 🟠 — Liste les posts populaires (hot) d'un subreddit en ce moment. Idéal pour monitorer une communauté.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "subreddit": { "type": "string", "description": "Nom du subreddit sans le r/ (ex: 'rust', 'LocalLLaMA', 'privacy')" },
                    "limit":     { "type": "integer", "default": 15, "minimum": 1, "maximum": 50, "description": "Nombre de posts (défaut: 15)" }
                },
                "required": ["subreddit"]
            }
        }),
        json!({
            "name": "reddit_get_post_with_comments",
            "description": "REDDIT 🟠 — Récupère un post Reddit ET ses commentaires principaux. Indispensable pour lire le contexte avant de répondre. Utiliser reddit_search_posts pour obtenir le post_id.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "post_id":       { "type": "string", "description": "ID du post Reddit (ex: '1abc2de' — sans le t3_ prefix)" },
                    "subreddit":     { "type": "string", "description": "Subreddit du post (ex: 'rust') — optionnel mais accélère la requête" },
                    "comment_limit": { "type": "integer", "default": 10, "minimum": 1, "maximum": 50, "description": "Nombre de commentaires à retourner (défaut: 10)" }
                },
                "required": ["post_id"]
            }
        }),
        json!({
            "name": "reddit_search_subreddits",
            "description": "REDDIT 🟠 — Cherche des subreddits par mot-clé. Utile pour trouver les bonnes communautés où s'engager.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": { "type": "string", "description": "Mots-clés pour chercher des subreddits (ex: 'artificial intelligence', 'privacy tools')" },
                    "limit": { "type": "integer", "default": 10, "minimum": 1, "maximum": 25, "description": "Nombre de subreddits (défaut: 10)" }
                },
                "required": ["query"]
            }
        }),
        json!({
            "name": "reddit_get_subreddit_info",
            "description": "REDDIT 🟠 — Récupère les infos d'un subreddit : description, règles, nombre d'abonnés et statut NSFW. À consulter avant de poster pour respecter les règles.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "subreddit": { "type": "string", "description": "Nom du subreddit sans le r/ (ex: 'rust')" }
                },
                "required": ["subreddit"]
            }
        }),
        // ── Actions ──────────────────────────────────────────────────────────
        json!({
            "name": "reddit_submit_comment",
            "description": "REDDIT 🟠 — Poste un commentaire sur un post Reddit. Toujours lire le post avec reddit_get_post_with_comments avant de commenter. Retourne l'ID du commentaire créé.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "post_id": { "type": "string", "description": "ID du post (ex: '1abc2de' — le t3_ est ajouté automatiquement)" },
                    "text":    { "type": "string", "description": "Texte du commentaire en Markdown Reddit" }
                },
                "required": ["post_id", "text"]
            }
        }),
        json!({
            "name": "reddit_reply_to_comment",
            "description": "REDDIT 🟠 — Répond à un commentaire existant. Utiliser reddit_get_post_with_comments pour obtenir le comment_id. Retourne l'ID du commentaire créé.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "comment_id": { "type": "string", "description": "ID du commentaire auquel répondre (ex: 'abc123' — le t1_ est ajouté automatiquement)" },
                    "text":       { "type": "string", "description": "Texte de la réponse en Markdown Reddit" }
                },
                "required": ["comment_id", "text"]
            }
        }),
        json!({
            "name": "reddit_submit_post",
            "description": "REDDIT 🟠 — Publie un nouveau post dans un subreddit. Toujours vérifier les règles avec reddit_get_subreddit_info avant. Retourne le lien du post créé.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "subreddit": { "type": "string", "description": "Subreddit cible sans le r/ (ex: 'rust')" },
                    "title":     { "type": "string", "description": "Titre du post" },
                    "text":      { "type": "string", "description": "Corps du post en Markdown (pour un post texte)" },
                    "url":       { "type": "string", "description": "URL pour un post de type lien (optionnel — si fourni, text est ignoré)" }
                },
                "required": ["subreddit", "title"]
            }
        }),
        json!({
            "name": "reddit_vote",
            "description": "REDDIT 🟠 — Upvote ou downvote un post ou commentaire Reddit.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "thing_id":  { "type": "string", "description": "ID complet avec prefix : t3_xxx pour un post, t1_xxx pour un commentaire" },
                    "direction": { "type": "integer", "enum": [1, 0, -1], "description": "1 = upvote, 0 = retirer le vote, -1 = downvote" }
                },
                "required": ["thing_id", "direction"]
            }
        }),
    ]
}

// ─── Handler ─────────────────────────────────────────────────────────────────

pub async fn handle(name: &str, args: &Value) -> Result<String, String> {
    let cfg = RedditConfig::load()
        .ok_or_else(|| "Reddit non configuré — créer ~/.osmozzz/reddit.toml avec client_id, client_secret, username, password".to_string())?;

    let token = get_token(&cfg).await?;

    match name {
        // ── Contenu personnel ────────────────────────────────────────────────
        "reddit_list_saved" => {
            let limit     = args["limit"].as_u64().unwrap_or(25);
            let type_flt  = args["type"].as_str().unwrap_or("all");
            let url = format!(
                "https://oauth.reddit.com/user/{}/saved?limit={limit}&type={type_flt}",
                cfg.username
            );
            let resp = get(&token, &url).await?;
            let items = resp["data"]["children"].as_array().cloned().unwrap_or_default();
            if items.is_empty() { return Ok("Aucun élément sauvegardé.".to_string()); }
            let mut out = format!("{} élément(s) sauvegardé(s) :\n\n", items.len());
            for item in &items {
                let kind = item["kind"].as_str().unwrap_or("t3");
                if kind == "t1" {
                    out.push_str(&format_comment(&item["data"]));
                } else {
                    out.push_str(&format_post(item));
                }
                out.push('\n');
            }
            Ok(out.trim_end().to_string())
        }

        "reddit_list_upvoted" => {
            let limit = args["limit"].as_u64().unwrap_or(25);
            let url = format!(
                "https://oauth.reddit.com/user/{}/upvoted?limit={limit}",
                cfg.username
            );
            let resp  = get(&token, &url).await?;
            let items = resp["data"]["children"].as_array().cloned().unwrap_or_default();
            if items.is_empty() { return Ok("Aucun post upvoté (l'API Reddit peut masquer cet historique selon les paramètres de confidentialité).".to_string()); }
            let mut out = format!("{} post(s) upvoté(s) :\n\n", items.len());
            for item in &items { out.push_str(&format_post(item)); out.push('\n'); }
            Ok(out.trim_end().to_string())
        }

        "reddit_get_my_comments" => {
            let limit = args["limit"].as_u64().unwrap_or(25);
            let url = format!(
                "https://oauth.reddit.com/user/{}/comments?limit={limit}",
                cfg.username
            );
            let resp  = get(&token, &url).await?;
            let items = resp["data"]["children"].as_array().cloned().unwrap_or_default();
            if items.is_empty() { return Ok("Aucun commentaire trouvé.".to_string()); }
            let mut out = format!("{} commentaire(s) :\n\n", items.len());
            for item in &items { out.push_str(&format_comment(&item["data"])); out.push('\n'); }
            Ok(out.trim_end().to_string())
        }

        // ── Recherche & exploration ──────────────────────────────────────────
        "reddit_search_posts" => {
            let query     = args["query"].as_str().ok_or("Paramètre 'query' requis")?;
            let sort      = args["sort"].as_str().unwrap_or("relevance");
            let limit     = args["limit"].as_u64().unwrap_or(15);
            let subreddit = args["subreddit"].as_str().unwrap_or("");

            let url = if subreddit.is_empty() {
                format!("https://oauth.reddit.com/search?q={query}&sort={sort}&limit={limit}&type=link")
            } else {
                format!("https://oauth.reddit.com/r/{subreddit}/search?q={query}&sort={sort}&limit={limit}&restrict_sr=1&type=link")
            };

            let resp  = get(&token, &url).await?;
            let items = resp["data"]["children"].as_array().cloned().unwrap_or_default();
            if items.is_empty() { return Ok(format!("Aucun post trouvé pour \"{query}\".")); }
            let mut out = format!("{} résultat(s) pour \"{}\" :\n\n", items.len(), query);
            for item in &items { out.push_str(&format_post(item)); out.push('\n'); }
            Ok(out.trim_end().to_string())
        }

        "reddit_list_hot_posts" => {
            let subreddit = args["subreddit"].as_str().ok_or("Paramètre 'subreddit' requis")?;
            let limit     = args["limit"].as_u64().unwrap_or(15);
            let url       = format!("https://oauth.reddit.com/r/{subreddit}/hot?limit={limit}");
            let resp      = get(&token, &url).await?;
            let items     = resp["data"]["children"].as_array().cloned().unwrap_or_default();
            if items.is_empty() { return Ok(format!("Aucun post dans r/{subreddit}.")); }
            let mut out = format!("Posts hot de r/{subreddit} :\n\n", );
            for item in &items { out.push_str(&format_post(item)); out.push('\n'); }
            Ok(out.trim_end().to_string())
        }

        "reddit_get_post_with_comments" => {
            let post_id       = args["post_id"].as_str().ok_or("Paramètre 'post_id' requis")?;
            let subreddit     = args["subreddit"].as_str().unwrap_or("all");
            let comment_limit = args["comment_limit"].as_u64().unwrap_or(10);
            let url = format!(
                "https://oauth.reddit.com/r/{subreddit}/comments/{post_id}?limit={comment_limit}&depth=2"
            );
            let resp = get(&token, &url).await?;

            let listing = resp.as_array().ok_or("Réponse Reddit inattendue")?;
            if listing.is_empty() { return Err("Post introuvable.".to_string()); }

            // Post principal
            let post_children = listing[0]["data"]["children"].as_array().cloned().unwrap_or_default();
            let post_data = post_children.first()
                .map(|p| &p["data"])
                .ok_or("Post non trouvé")?;

            let title    = post_data["title"].as_str().unwrap_or("—");
            let author   = post_data["author"].as_str().unwrap_or("—");
            let score    = post_data["score"].as_i64().unwrap_or(0);
            let sub      = post_data["subreddit_name_prefixed"].as_str().unwrap_or("—");
            let selftext = post_data["selftext"].as_str().unwrap_or("");
            let num_com  = post_data["num_comments"].as_i64().unwrap_or(0);
            let link     = post_data["permalink"].as_str()
                .map(|p| format!("https://reddit.com{p}"))
                .unwrap_or_else(|| "—".to_string());

            let body_preview = if selftext.len() > 800 {
                format!("{}…", &selftext[..800])
            } else {
                selftext.to_string()
            };

            let mut out = format!(
                "📄 {title}\n{sub} | u/{author} | ▲{score} | 💬{num_com}\n{link}\n"
            );
            if !body_preview.is_empty() {
                out.push_str(&format!("\n{body_preview}\n"));
            }

            // Commentaires
            if listing.len() > 1 {
                let comments = listing[1]["data"]["children"].as_array().cloned().unwrap_or_default();
                let real_comments: Vec<&Value> = comments.iter()
                    .filter(|c| c["kind"].as_str() != Some("more"))
                    .collect();

                if !real_comments.is_empty() {
                    out.push_str(&format!("\n─── {} commentaire(s) ───\n\n", real_comments.len()));
                    for c in real_comments {
                        out.push_str(&format_comment(&c["data"]));
                        out.push('\n');
                    }
                }
            }

            Ok(out.trim_end().to_string())
        }

        "reddit_search_subreddits" => {
            let query = args["query"].as_str().ok_or("Paramètre 'query' requis")?;
            let limit = args["limit"].as_u64().unwrap_or(10);
            let url   = format!("https://oauth.reddit.com/subreddits/search?q={query}&limit={limit}");
            let resp  = get(&token, &url).await?;
            let items = resp["data"]["children"].as_array().cloned().unwrap_or_default();
            if items.is_empty() { return Ok(format!("Aucun subreddit trouvé pour \"{query}\".")); }
            let mut out = format!("{} subreddit(s) trouvé(s) :\n\n", items.len());
            for item in &items {
                let d    = &item["data"];
                let name = d["display_name_prefixed"].as_str().unwrap_or("—");
                let subs = d["subscribers"].as_i64().unwrap_or(0);
                let desc = d["public_description"].as_str().unwrap_or("").trim();
                let desc_short = if desc.len() > 150 { format!("{}…", &desc[..150]) } else { desc.to_string() };
                out.push_str(&format!("• {name} — {subs} membres\n  {desc_short}\n"));
            }
            Ok(out.trim_end().to_string())
        }

        "reddit_get_subreddit_info" => {
            let subreddit = args["subreddit"].as_str().ok_or("Paramètre 'subreddit' requis")?;
            let url  = format!("https://oauth.reddit.com/r/{subreddit}/about");
            let resp = get(&token, &url).await?;
            let d    = &resp["data"];

            let name        = d["display_name_prefixed"].as_str().unwrap_or("—");
            let subs        = d["subscribers"].as_i64().unwrap_or(0);
            let active      = d["active_user_count"].as_i64().unwrap_or(0);
            let desc        = d["public_description"].as_str().unwrap_or("").trim();
            let nsfw        = d["over18"].as_bool().unwrap_or(false);
            let created     = d["created_utc"].as_f64().map(|t| t as i64).unwrap_or(0);

            // Règles
            let rules_url = format!("https://oauth.reddit.com/r/{subreddit}/rules");
            let rules_resp = get(&token, &rules_url).await.unwrap_or(json!({}));
            let rules = rules_resp["rules"].as_array().cloned().unwrap_or_default();

            let created_str = chrono::DateTime::from_timestamp(created, 0)
                .map(|dt| dt.format("%Y-%m-%d").to_string())
                .unwrap_or_else(|| "—".to_string());

            let mut out = format!(
                "{name}\nAbonnés : {subs} ({active} actifs) | Créé : {created_str} | NSFW : {nsfw}\n\n{desc}\n"
            );

            if !rules.is_empty() {
                out.push_str(&format!("\n─── {} règle(s) ───\n", rules.len()));
                for r in &rules {
                    let rname = r["short_name"].as_str().unwrap_or("—");
                    let rdesc = r["description"].as_str().unwrap_or("").trim();
                    let rdesc_short = if rdesc.len() > 200 { format!("{}…", &rdesc[..200]) } else { rdesc.to_string() };
                    out.push_str(&format!("• {rname}\n  {rdesc_short}\n"));
                }
            }

            Ok(out.trim_end().to_string())
        }

        // ── Actions ──────────────────────────────────────────────────────────
        "reddit_submit_comment" => {
            let post_id = args["post_id"].as_str().ok_or("Paramètre 'post_id' requis")?;
            let text    = args["text"].as_str().ok_or("Paramètre 'text' requis")?;
            let parent  = format!("t3_{post_id}");
            let resp = post_form(&token, "https://oauth.reddit.com/api/comment", vec![
                ("api_type", "json".to_string()),
                ("parent",   parent),
                ("text",     text.to_string()),
            ]).await?;
            let id   = resp["json"]["data"]["things"][0]["data"]["id"].as_str().unwrap_or("—");
            let link = resp["json"]["data"]["things"][0]["data"]["permalink"]
                .as_str()
                .map(|p| format!("https://reddit.com{p}"))
                .unwrap_or_else(|| "—".to_string());
            Ok(format!("Commentaire posté !\nID   : {id}\nLien : {link}"))
        }

        "reddit_reply_to_comment" => {
            let comment_id = args["comment_id"].as_str().ok_or("Paramètre 'comment_id' requis")?;
            let text       = args["text"].as_str().ok_or("Paramètre 'text' requis")?;
            let parent     = format!("t1_{comment_id}");
            let resp = post_form(&token, "https://oauth.reddit.com/api/comment", vec![
                ("api_type", "json".to_string()),
                ("parent",   parent),
                ("text",     text.to_string()),
            ]).await?;
            let id   = resp["json"]["data"]["things"][0]["data"]["id"].as_str().unwrap_or("—");
            let link = resp["json"]["data"]["things"][0]["data"]["permalink"]
                .as_str()
                .map(|p| format!("https://reddit.com{p}"))
                .unwrap_or_else(|| "—".to_string());
            Ok(format!("Réponse postée !\nID   : {id}\nLien : {link}"))
        }

        "reddit_submit_post" => {
            let subreddit = args["subreddit"].as_str().ok_or("Paramètre 'subreddit' requis")?;
            let title     = args["title"].as_str().ok_or("Paramètre 'title' requis")?;

            let mut params = vec![
                ("api_type", "json".to_string()),
                ("sr",       subreddit.to_string()),
                ("title",    title.to_string()),
            ];

            if let Some(url_val) = args["url"].as_str() {
                params.push(("kind", "link".to_string()));
                params.push(("url",  url_val.to_string()));
            } else {
                let text = args["text"].as_str().unwrap_or("");
                params.push(("kind", "self".to_string()));
                params.push(("text", text.to_string()));
            }

            let resp = post_form(&token, "https://oauth.reddit.com/api/submit", params).await?;
            let link = resp["json"]["data"]["url"].as_str()
                .unwrap_or("Post créé (lien non disponible)");
            let id   = resp["json"]["data"]["id"].as_str().unwrap_or("—");
            Ok(format!("Post publié dans r/{subreddit} !\nID   : {id}\nLien : {link}"))
        }

        "reddit_vote" => {
            let thing_id  = args["thing_id"].as_str().ok_or("Paramètre 'thing_id' requis")?;
            let direction = args["direction"].as_i64().ok_or("Paramètre 'direction' requis (1, 0 ou -1)")?;
            post_form(&token, "https://oauth.reddit.com/api/vote", vec![
                ("id",  thing_id.to_string()),
                ("dir", direction.to_string()),
            ]).await?;
            let action = match direction {
                1  => "upvoté",
                -1 => "downvoté",
                _  => "vote retiré de",
            };
            Ok(format!("Vote enregistré : {thing_id} {action}."))
        }

        _ => Err(format!("Tool Reddit inconnu : {name}")),
    }
}
