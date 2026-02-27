# Harvester Agent — Implémentation

## Checklist pour un nouveau harvester

### Étape 1 — osmozzz-core/src/types.rs
Ajouter le nouveau SourceType :
```rust
pub enum SourceType {
    Chrome,
    File,
    Pdf,
    Email,
    IMessage,   // ← nouveau
    Safari,     // ← nouveau
    Notes,      // ← nouveau
    Calendar,   // ← nouveau
    Terminal,   // ← nouveau
}
impl SourceType {
    pub fn to_string(&self) -> &str {
        match self {
            SourceType::IMessage => "imessage",
            // ...
        }
    }
}
```

### Étape 2 — Créer src/NOM.rs
```rust
use osmozzz_core::{Document, Harvester, Result, SourceType};

pub struct NomHarvester { /* config */ }

impl NomHarvester {
    pub fn new() -> Self { ... }
}

impl Harvester for NomHarvester {
    async fn harvest(&self) -> Result<Vec<Document>> {
        // 1. Ouvrir la source (SQLite, fichier...)
        // 2. Lire les entrées
        // 3. Pour chaque entrée → créer un Document
        // 4. Retourner Ok(docs)
    }
}
```

### Étape 3 — src/lib.rs
```rust
pub mod nom;
pub use nom::NomHarvester;
```

### Étape 4 — Cargo.toml si nouvelle dépendance
Ajouter dans `crates/osmozzz-harvester/Cargo.toml`

### Étape 5 — Prévenir MCP Tools Agent
Le MCP Tools Agent expose le tool dans Claude.

## Paths macOS utiles
```
iMessage  : ~/Library/Messages/chat.db
Safari    : ~/Library/Safari/History.db
Notes     : ~/Library/Group Containers/group.com.apple.notes/NoteStore.sqlite
Calendar  : ~/Library/Calendars/
Terminal  : ~/.zsh_history
```

## Gestion des permissions macOS
Ces chemins nécessitent que l'app ait accès via Paramètres Système → Confidentialité.
- Messages : Full Disk Access requis
- Safari : Full Disk Access requis
- Notes : Full Disk Access requis

Gérer silencieusement : si accès refusé → `return Ok(vec![])` avec un log warning.
