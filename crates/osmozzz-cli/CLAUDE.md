# MCP Tools Agent

## Rôle
Gérer l'interface entre Claude et OSMOzzz.
Chaque tool MCP = 1 description + 1 handler + 1 formatter.
Claude lit les descriptions pour décider quel tool utiliser.

## Fichier principal
`src/commands/mcp.rs`

## Architecture d'un tool MCP
```
tools_list()     → description JSON que Claude lit (CRITIQUE pour la sélection)
match tool_name  → handler Rust qui appelle le vault
format_*()       → formatte le résultat en texte lisible pour Claude
```

## Tools email actuels (3 tools — architecture simplifiée)
| Tool | Action | Vault |
|---|---|---|
| `search_emails(keyword)` | Scan .contains() sur tous les emails | `vault.search_emails_by_keyword()` |
| `get_emails_by_date(query?)` | Filtre par date ou récents | `vault.get_emails_by_date()` / `recent_emails_full()` |
| `read_email(id)` | Contenu complet d'un email | `vault.get_full_content_by_url()` |

## Tools fichiers actuels
| Tool | Action |
|---|---|
| `find_file(name)` | Scan filesystem par nom + contenu (depth 20) |
| `fetch_content(path)` | Lit un fichier (mode RAG ou linéaire) |
| `get_recent_files(hours)` | Fichiers récemment modifiés |
| `list_directory(path)` | Liste un dossier |

## Règles de description des tools (CRITIQUE)
1. **Courte et précise** — Claude lit ça pour choisir le bon tool
2. **Indiquer ce que le tool retourne** — "retourne une liste compacte + IDs"
3. **Indiquer comment enchaîner** — "utilise read_email(id) pour le contenu complet"
4. **Pas de chevauchement** entre tools — chaque tool a un rôle unique
5. **Exemple concret** dans la description si le paramètre n'est pas évident

## Pattern pour ajouter un tool (ex: iMessage)
```rust
// 1. Dans tools_list() :
{
    "name": "search_messages",
    "description": "Cherche dans les iMessages/SMS par mot-clé...",
    "inputSchema": { ... }
}

// 2. Dans le match :
"search_messages" => {
    let keyword = args["keyword"].as_str()...;
    match vault.search_imessages_by_keyword(&keyword, limit).await {
        Ok(results) => send(format_message_list(&results)),
        Err(e) => send(Response::err(...))
    }
}

// 3. Formatter dédié :
fn format_message_list(results: &[(...)]) -> String { ... }
```

## Philosophie tool design
- **1 tool = 1 action claire** (liste OU lit, jamais les deux)
- **Retour compact par défaut** → Claude appelle un 2e tool pour le détail
- **Fallback explicite** dans la description ("si 0 résultat, essaie un mot plus court")
- **Jamais de logique métier dans le handler** → déléguer au vault

## ⛔ Périmètre strict
**Interdiction de modifier un fichier hors de ce périmètre sans accord explicite du Chef de Projet.**
Fichiers autorisés :
- `crates/osmozzz-cli/src/commands/mcp.rs`
- `crates/osmozzz-cli/src/commands/*.rs` (si nouveau command)
- `crates/osmozzz-cli/Cargo.toml`

Tout autre fichier → demander au Chef de Projet d'abord.
