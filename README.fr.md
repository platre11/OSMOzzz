# OSMOzzz

🇬🇧 [Read in English](README.md)

**Votre hub de données privé pour Claude.** OSMOzzz connecte tous vos outils — emails, fichiers, bases de données, gestion de projet, messagerie — et les rend accessibles à Claude Code via MCP, sans jamais envoyer vos données brutes à l'extérieur de votre machine.

---

## Ce que ça fait

Claude Code peut chercher dans toutes vos sources de données, comprendre le contexte de vos vrais outils, et déclencher des actions — le tout via un serveur MCP local qui tourne sur votre Mac.

Vous restez maître de vos données. Tout est filtré, anonymisé et journalisé avant que Claude ne le voie.

---

## Sources de données (20 connecteurs)

### Toujours actives — aucune configuration requise
| Source | Ce qui est indexé |
|--------|------------------|
| **Fichiers** | Documents, Bureau, Téléchargements — `.md`, `.txt`, `.pdf`, `.csv` et plus |
| **Chrome** | Historique de navigation |
| **Safari** | Historique de navigation |
| **Gmail** | Emails (objet + corps) via IMAP |
| **iMessage** | SMS et iMessages |
| **Apple Notes** | Toutes vos notes |
| **Apple Calendar** | Événements et réunions |
| **Terminal** | Historique shell (`~/.zsh_history`) |
| **Contacts** | Carnet d'adresses macOS |
| **Arc** | Historique du navigateur Arc |

### Connecteurs cloud — à configurer une fois dans le dashboard
| Source | Ce qui est indexé |
|--------|------------------|
| **Notion** | Pages et bases de données |
| **GitHub** | Issues, PRs, code |
| **Linear** | Issues et projets |
| **Jira** | Tickets et epics |
| **Slack** | Messages des canaux |
| **Trello** | Cartes et tableaux |
| **Todoist** | Tâches et projets |
| **GitLab** | Issues et merge requests |
| **Airtable** | Enregistrements et bases |
| **Obsidian** | Notes du vault |

### Bases de données — requêtes en direct, sans indexation
| Source | Ce que vous pouvez faire |
|--------|------------------------|
| **Supabase** | Exécuter des requêtes SQL, inspecter le schéma, gérer les migrations, déployer des edge functions |

---

## Ce que Claude peut faire avec

### Recherche (25 outils)
Claude peut chercher dans n'importe quelle source — par sens ou par mot-clé. Posez des questions comme :
- *"Trouve les emails sur le budget Q1"*
- *"Quelles issues Linear concernent l'authentification ?"*
- *"Montre-moi les fichiers récents sur l'API"*
- *"Cherche dans mes messages Slack ce qui parle du déploiement"*

### Actions (16 types d'actions)
Claude propose des actions, vous les approuvez dans le dashboard avant qu'elles ne s'exécutent :

| Catégorie | Actions |
|-----------|---------|
| **Communication** | Envoyer un email · Envoyer un message Slack · Envoyer un iMessage |
| **Gestion de projet** | Créer une issue Linear · Créer une issue GitHub · Créer un ticket Jira · Créer une carte Trello · Créer une issue GitLab · Créer une tâche Todoist |
| **Contenu** | Créer une page Notion · Créer un événement calendrier · Créer un dossier |
| **Fichiers** | Renommer · Supprimer un fichier · Supprimer une note · Supprimer un événement |
| **Système** | Exécuter une commande shell |

---

## Confidentialité & sécurité

OSMOzzz dispose de quatre couches de contrôle sur ce que Claude voit :

| Fonctionnalité | Ce qu'elle fait |
|----------------|----------------|
| **Filtre de confidentialité** | Masque automatiquement les numéros de CB, IBAN, clés API, adresses email, numéros de téléphone |
| **Alias d'identité** | Remplace les vrais noms par des pseudonymes — Claude travaille avec l'alias, ne voit jamais l'identité réelle |
| **Liste noire** | Exclut des documents, expéditeurs, domaines ou chemins de fichiers spécifiques de tous les résultats |
| **Journal d'accès** | Chaque appel MCP est enregistré — outil, requête, nombre de résultats, bloqué ou non |

Toutes les données restent sur votre Mac. OSMOzzz fonctionne entièrement hors ligne (embeddings ONNX locaux, base de données LanceDB locale). Rien n'est envoyé à Anthropic ni à un tiers.

---

## Dashboard

OSMOzzz inclut un dashboard web accessible sur `http://localhost:7878` une fois le daemon lancé.

| Page | Rôle |
|------|------|
| **Statut** | Compteurs par source, utilisation disque et mémoire |
| **Recherche** | Recherche cross-sources avec filtres de dates |
| **Récents** | Derniers documents indexés par source |
| **Configuration** | Connecter et configurer tous les connecteurs cloud |
| **Actions** | Approuver ou rejeter les actions proposées par Claude en temps réel |
| **Réseau** | Mesh P2P — partager la recherche avec des collègues, avec des permissions granulaires par source |

---

## Réseau P2P (Enterprise)

OSMOzzz supporte un réseau pair-à-pair entre plusieurs machines. Les membres d'une équipe peuvent chercher dans les données des uns et des autres sans que ces données ne quittent jamais leur propre machine — seuls les résultats filtrés transitent.

- Chaque machine possède sa propre identité Ed25519
- Les permissions sont granulaires par source et par pair
- Le filtre de confidentialité s'applique toujours avant l'envoi des résultats
- Toutes les requêtes entrantes sont enregistrées dans le journal d'audit

---

## Installation

Voir [INSTALL.md](INSTALL.md) pour les instructions de mise en place.

Requiert macOS · Rust 1.75+ · Homebrew
