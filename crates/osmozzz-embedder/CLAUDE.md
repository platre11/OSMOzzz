# Storage Agent

## Rôle
Gérer le stockage, la recherche et les embeddings.
LanceDB est la base vectorielle locale. ONNX génère les embeddings 384d.

## Fichiers
| Fichier | Rôle |
|---|---|
| `src/store.rs` | Toutes les opérations LanceDB (upsert, search, query) |
| `src/vault.rs` | Facade publique — combine embedder + store |
| `src/embedder.rs` | ONNX Runtime — génère les vecteurs 384d |

## Schéma LanceDB (TABLE: documents)
```
id           : Utf8      — UUID
source       : Utf8      — "email", "chrome", "file", "imessage", "notes"...
url          : Utf8      — identifiant unique du document
title        : Utf8?     — titre ou objet
content      : Utf8      — texte indexé (tronqué ou chunké)
checksum     : Utf8      — sha256 pour déduplication
harvested_at : Int64     — timestamp Unix (secondes)
source_ts    : Int64?    — timestamp original de la source
chunk_index  : Int32?    — index du chunk (si document découpé)
chunk_total  : Int32?    — nombre total de chunks
embedding    : [f32; 384] — vecteur ONNX
```

## Deux modes de recherche
### 1. Vectorielle (sémantique)
```rust
vault.search(query, limit)           // global toutes sources
vault.search_filtered(query, limit, Some("email"))  // filtré par source
```
Bon pour : concepts, sujets généraux
Mauvais pour : noms propres, marques (Revolut, Qonto...)

### 2. Keyword scan (`.contains()`)
```rust
vault.search_emails_by_keyword(keyword, limit)
store.recent_by_source(source, limit)
```
Bon pour : noms propres, recherche exacte, aucune limite de date

## Pattern pour une nouvelle source
Ajouter dans `store.rs` :
```rust
pub async fn search_SOURCE_by_keyword(&self, keyword: &str, limit: usize)
    -> Result<Vec<(Option<String>, String, String)>>
// Même pattern que search_emails_by_keyword
// filter: only_if("source = 'SOURCENAME'")
```

Exposer dans `vault.rs` :
```rust
pub async fn search_SOURCE_by_keyword(&self, keyword: &str, limit: usize)
    -> osmozzz_core::Result<...> {
    self.store.search_SOURCENAME_by_keyword(keyword, limit).await
}
```

## Règles
1. **Ne jamais modifier le schéma** sans vérifier la compatibilité LanceDB
2. **Toujours utiliser `is_char_boundary`** avant de slicer des strings UTF-8
3. **Compact après bulk insert** : `vault.compact().await`
4. **Score vectoriel** = `1.0 - distance/2.0` (cosine dist [0,2] → score [0,1])
5. **Limit 100_000** pour les scans complets en mémoire (tri Rust, pas SQL)

## ⛔ Périmètre strict
**Interdiction de modifier un fichier hors de ce périmètre sans accord explicite du Chef de Projet.**
Fichiers autorisés :
- `crates/osmozzz-embedder/src/store.rs`
- `crates/osmozzz-embedder/src/vault.rs`
- `crates/osmozzz-embedder/src/embedder.rs`
- `crates/osmozzz-embedder/Cargo.toml`

Tout autre fichier → demander au Chef de Projet d'abord.
