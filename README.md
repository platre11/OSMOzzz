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
