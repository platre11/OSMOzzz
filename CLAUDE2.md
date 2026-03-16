# OSMOzzz — Orchestrateur d'Actions

## Vision

OSMOzzz passe de **"mémoire de Claude"** à **"mémoire + bras de Claude"**.
Claude peut exécuter des actions réelles (envoyer un email, créer une page Notion)
avec validation humaine obligatoire avant chaque exécution.
Tout reste local. Rien ne passe par un cloud externe.

**Périmètre de développement et de test : Gmail + Notion.**
**Périmètre cible long terme : toutes les sources cloud existantes.**

---

## Objectif 1 — Pipeline d'interception et d'approbation

Claude appelle une action → elle n'est **jamais exécutée immédiatement** →
elle est mise en file d'attente → le dashboard affiche une modale de validation →
l'utilisateur approuve ou refuse → l'action est exécutée ou abandonnée →
Claude reçoit le résultat final.

**Règles :**

- Tout appel d'action (`act_*`) est intercepté avant toute exécution
- L'utilisateur voit un aperçu humain lisible de ce que Claude veut faire (pas du JSON brut)
- Timeout de 5 minutes sans réponse → refus automatique
- Claude reçoit un accusé de réception immédiat puis le résultat final après décision
- La file d'attente est vidée au redémarrage du daemon (actions non résolues = expirées)

---

## Objectif 2 — Exécution réelle des actions approuvées

Les connecteurs d'exécution sont en **Rust pur, zéro dépendance externe**.
Pas de Python, pas de Node, pas de script externe.

**Gmail :**

- Envoyer un email via SMTP TLS (smtp.gmail.com) avec l'app password existant
- Répondre à un thread existant en récupérant les métadonnées depuis LanceDB

**Notion :**

- Créer une nouvelle page dans un parent donné
- Modifier le contenu d'une page existante

**Règles :**

- Les secrets viennent des fichiers `.toml` déjà existants — pas de nouveau système
- Chaque exécution est isolée : instancié → exécute → libère
- Un connecteur par source, un fichier de config par source, sans exception

---

## Objectif 3 — Audit et politiques de confiance

**Audit log :**

- Chaque action (approuvée ou refusée) est enregistrée localement
- L'enregistrement se fait AVANT l'exécution, même si l'exécution échoue
- L'utilisateur peut consulter l'historique complet depuis le dashboard

**Politiques de confiance :**

- 3 niveaux par action par connecteur : `ask` (défaut) · `auto` · `block`
- `ask` → modale obligatoire
- `auto` → exécution directe sans demande (à activer explicitement, jamais par défaut)
- `block` → toujours refusé, Claude n'est même pas notifié
- Les politiques sont configurables uniquement via le dashboard, jamais manuellement

---

## Plan de développement

| Étape | Objectif                                                                         |
| ----- | -------------------------------------------------------------------------------- |
| 1     | Types partagés : ActionRequest, ActionStatus, ActionResult                       |
| 2     | File d'attente in-memory + routes API + Server-Sent Events vers le dashboard     |
| 3     | Nouveaux tools MCP `act_send_email` et `act_create_notion_page` (sans exécution) |
| 4     | Dashboard — page "Actions" avec modale temps réel                                |
| 5     | GmailActor — exécution réelle via SMTP                                           |
| 6     | NotionActor — exécution réelle via API Notion                                    |
| 7     | Audit log SQLite + page Historique dans le dashboard                             |
| 8     | Politiques de confiance + page Politiques dans le dashboard                      |
| 9     | Tests end-to-end complets Gmail + Notion                                         |
| 10    | Extension progressive aux autres connecteurs (Jira, Slack, Linear…)              |

---

## Pratiques de développement

1. **Zéro exécution sans validation** — jamais d'action directe, toujours passer par la queue
2. **Audit avant exécution** — loguer l'intention avant de tenter l'action
3. **Pas de breaking change** sur les 24 tools MCP existants
4. **Un connecteur = un fichier de config existant** — réutiliser ce qui est là
5. **100% Rust** — aucun script externe, aucune dépendance runtime supplémentaire
6. **Dashboard comme seul point de contrôle** — politiques et historique uniquement via l'UI
7. **Légèreté** — le binaire final ne doit pas grossir enormement
8. **Aperçu humain lisible** — toute modale montre ce que l'action va faire en langage naturel
