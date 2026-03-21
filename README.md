# OSMOzzz

🇫🇷 [Lire en français](README.fr.md)

**Your private data hub for Claude.** OSMOzzz connects all your tools — emails, files, databases, project management, messaging — and makes them available to Claude Code via MCP, without ever sending raw data outside your machine.

---

## What it does

Claude Code can search across all your data sources, understand context from your real tools, and take actions — all through a local MCP server running on your Mac.

You stay in control. Everything is filtered, anonymized, and logged before Claude sees it.

---

## Data sources (20 connectors)

### Always on — no configuration needed
| Source | What gets indexed |
|--------|------------------|
| **Files** | Documents, Desktop, Downloads — `.md`, `.txt`, `.pdf`, `.csv` and more |
| **Chrome** | Browsing history |
| **Safari** | Browsing history |
| **Gmail** | Emails (subject + body) via IMAP |
| **iMessage** | SMS and iMessages |
| **Apple Notes** | All your notes |
| **Apple Calendar** | Events and meetings |
| **Terminal** | Shell history (`~/.zsh_history`) |
| **Contacts** | macOS address book |
| **Arc** | Arc browser history |

### Cloud connectors — configure once in the dashboard
| Source | What gets indexed |
|--------|------------------|
| **Notion** | Pages and databases |
| **GitHub** | Issues, PRs, code |
| **Linear** | Issues and projects |
| **Jira** | Tickets and epics |
| **Slack** | Channel messages |
| **Trello** | Cards and boards |
| **Todoist** | Tasks and projects |
| **GitLab** | Issues and merge requests |
| **Airtable** | Records and bases |
| **Obsidian** | Vault notes |

### Databases — query live, no indexing needed
| Source | What you can do |
|--------|----------------|
| **Supabase** | Run SQL queries, inspect schema, manage migrations, deploy edge functions |

---

## What Claude can do with it

### Search (25 tools)
Claude can search across any source — semantically or by keyword. Ask things like:
- *"Find emails about the Q1 budget"*
- *"What Linear issues are related to auth?"*
- *"Show me recent files I modified about the API"*
- *"Search my Slack messages about the deployment"*

### Actions (16 action types)
Claude proposes actions, you approve them in the dashboard before anything happens:

| Category | Actions |
|----------|---------|
| **Communication** | Send email · Send Slack message · Send iMessage |
| **Project management** | Create Linear issue · Create GitHub issue · Create Jira ticket · Create Trello card · Create GitLab issue · Create Todoist task |
| **Content** | Create Notion page · Create calendar event · Create folder |
| **Files** | Rename file · Delete file · Delete note · Delete calendar event |
| **System** | Run shell command |

---

## Privacy & security

OSMOzzz has four built-in layers to control what Claude sees:

| Feature | What it does |
|---------|-------------|
| **Privacy filter** | Automatically masks credit cards, IBANs, API keys, emails, phone numbers |
| **Identity aliases** | Replace real names with aliases — Claude works with `Alias`, never sees the real identity |
| **Blacklist** | Exclude specific documents, senders, domains, or file paths from all results |
| **Access log** | Every MCP call is logged — tool name, query, result count, blocked or not |

All data stays on your Mac. OSMOzzz runs entirely offline (local ONNX embeddings, local LanceDB database). Nothing is sent to Anthropic or any third party.

---

## Dashboard

OSMOzzz includes a web dashboard accessible at `http://localhost:7878` once the daemon is running.

| Page | Purpose |
|------|---------|
| **Status** | Live counts per source, disk and memory usage |
| **Search** | Cross-source search with date filters |
| **Recent** | Latest indexed documents per source |
| **Configuration** | Connect and configure all cloud connectors |
| **Actions** | Approve or reject Claude's proposed actions in real time |
| **Network** | P2P mesh — share search with teammates, with granular permissions per source |

---

## P2P Network (Enterprise)

OSMOzzz supports a peer-to-peer mesh between multiple machines. Team members can search each other's data without the data ever leaving their own machine — only filtered results transit.

- Each machine has its own Ed25519 identity
- Permissions are granular per source per peer
- The privacy filter always applies before sending results to a peer
- All incoming queries are logged in the audit trail

---

## Installation

See [INSTALL.md](INSTALL.md) for setup instructions.

Requires macOS · Rust 1.75+ · Homebrew
