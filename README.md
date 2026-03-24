# OSMOzzz

🌐 **[osm-ozzz.vercel.app](https://osm-ozzz.vercel.app/)**

**OSMOzzz is your private tentacle that binds Claude to your world.**

Where Claude connects to external tools in plain text, OSMOzzz steps in between — you connect your tools to OSMOzzz, and Claude accesses them through it, with full control over what comes back.

---

## What changes in practice

**Claude + direct MCP**
> 🤖 Claude calls the Gmail MCP → your emails are sent to Anthropic's servers unfiltered.
> ⚠️ It works — but your raw data transits to Anthropic's servers with no control.

**Claude + OSMOzzz**
> 🤖 Claude selects an OSMOzzz tool → OSMOzzz searches or executes the action → returns the result to Claude with sensitive data scrambled.
> ✅ Your raw data never left your machine.

---

## Indexed sources

| | |
|---|---|
| Files | ✅ |
| Chrome | ✅ |
| Safari | ✅ |
| Gmail | ✅ |
| iMessage | ✅ |
| Apple Notes | ✅ |
| Apple Calendar | ✅ |
| Terminal | ✅ |
| Contacts | ✅ |
| Arc | ✅ |

## External connectors

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

## Privacy

| | |
|---|---|
| Privacy filter | Masks credit cards, IBANs, API keys, emails, phone numbers |
| Identity aliases | Claude sees an alias, never the real identity |
| Blacklist | Exclude documents, senders or domains |
| Access log | Every MCP call is recorded |

---

## Installation

See [INSTALL.md](INSTALL.md) — Requires macOS · Rust 1.75+ · Homebrew
