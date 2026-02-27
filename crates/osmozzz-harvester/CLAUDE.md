# Harvester Agent

## Rôle
Implémenter les sources de données. Chaque harvester lit une source locale
(SQLite, fichiers texte, API locale) et produit des `Document` pour LanceDB.

## Sources existantes
| Harvester | Fichier | Source |
|---|---|---|
| ChromeHarvester | `src/chrome.rs` | `~/Library/Application Support/Google/Chrome/.../History` |
| FileHarvester | `src/files.rs` | `~/Desktop`, `~/Documents`, `~/Downloads` |
| GmailHarvester | `src/gmail.rs` | IMAP Gmail (async-imap) |

## Sources à implémenter (Niveau 1)
| Source | Fichier cible | Accès |
|---|---|---|
| iMessage | `src/imessage.rs` | `~/Library/Messages/chat.db` (SQLite) |
| Safari | `src/safari.rs` | `~/Library/Safari/History.db` (SQLite) |
| Apple Notes | `src/notes.rs` | SQLite dans Group Containers |
| Apple Calendar | `src/calendar.rs` | SQLite local |
| Terminal | `src/terminal.rs` | `~/.zsh_history` (texte brut) |

## Trait à implémenter (osmozzz-core)
```rust
pub trait Harvester {
    async fn harvest(&self) -> Result<Vec<Document>>;
}
```

## Structure Document (osmozzz-core)
```rust
Document {
    id: Uuid,
    source: SourceType,   // ← ajouter le nouveau SourceType dans osmozzz-core/src/types.rs
    url: String,          // identifiant unique ex: "imessage://chat/123/msg/456"
    title: Option<String>,
    content: String,      // texte indexé
    checksum: String,     // sha256 du contenu
    harvested_at: DateTime<Utc>,
    source_ts: Option<DateTime<Utc>>,  // date originale du message/fichier
    chunk_index: Option<u32>,
    chunk_total: Option<u32>,
}
```

## Modèle à suivre : ChromeHarvester
`src/chrome.rs` — lit un SQLite, mappe les lignes en Document, gère le checksum.
C'est le modèle le plus simple à reproduire pour iMessage et Safari.

## ⛔ Périmètre strict
**Interdiction de modifier un fichier hors de ce périmètre sans accord explicite du Chef de Projet.**
Fichiers autorisés :
- `crates/osmozzz-harvester/src/*.rs`
- `crates/osmozzz-harvester/Cargo.toml`
- `crates/osmozzz-core/src/types.rs` (ajout SourceType uniquement)

Tout autre fichier → demander au Chef de Projet d'abord.

## Règles
1. **Source unique par fichier** — 1 harvester = 1 fichier .rs
2. **Toujours vérifier `vault.exists(&checksum)`** avant d'indexer (évite les doublons)
3. **Gérer les erreurs silencieusement** — si la DB n'existe pas, retourner Ok(vec![])
4. **Ajouter le SourceType** dans `osmozzz-core/src/types.rs` pour chaque nouvelle source
5. **Exporter** depuis `src/lib.rs`

## Après implémentation d'un harvester
→ Prévenir le MCP Tools Agent pour exposer le tool dans Claude
→ Prévenir le Daemon Agent pour ajouter la surveillance si pertinent
