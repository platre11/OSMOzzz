# OSMOzzz

🇬🇧 [Read in English](README.md)

**OSMOzzz est votre tentacule privée qui se lie à Claude dans votre monde.**

Là où Claude se connecte à des outils externes en clair, OSMOzzz s'interpose — vous branchez vos outils à OSMOzzz, et Claude y accède à travers lui, avec un contrôle total sur ce qui remonte.

---

## Ce que ça change concrètement

**Claude + MCP direct**
> 🤖 Claude appelle le MCP Gmail → vos emails partent chez Anthropic en clair, sans filtre.
> ⚠️ Ça fonctionne — mais vos données brutes transitent vers les serveurs d'Anthropic sans aucun contrôle.

**Claude + OSMOzzz**
> 🤖 Claude sélectionne un tool OSMOzzz → OSMOzzz cherche ou exécute l'action → retourne le résultat à Claude avec les données sensibles brouillées.
> ✅ Vos données brutes n'ont jamais quitté votre machine.

---

## Sources indexées

| | |
|---|---|
| Fichiers | ✅ |
| Chrome | ✅ |
| Safari | ✅ |
| Gmail | ✅ |
| iMessage | ✅ |
| Apple Notes | ✅ |
| Apple Calendar | ✅ |
| Terminal | ✅ |
| Contacts | ✅ |
| Arc | ✅ |

## Connecteurs externes

| | |
|---|---|
| Notion | ✅ |
| GitHub | ✅ |
| Linear | ✅ |
| Jira | ✅ |
| Slack | ✅ |
| Trello | ✅ |
| Todoist | ✅ |
| GitLab | ✅ |
| Airtable | ✅ |
| Obsidian | ✅ |

---

## Confidentialité

| | |
|---|---|
| Filtre de confidentialité | Masque CB, IBAN, clés API, emails, téléphones |
| Alias d'identité | Claude voit un alias, jamais l'identité réelle |
| Liste noire | Exclut documents, expéditeurs ou domaines |
| Journal d'accès | Chaque appel MCP est enregistré |

---

## Installation

Voir [INSTALL.md](INSTALL.md) — Requiert macOS · Rust 1.75+ · Homebrew
