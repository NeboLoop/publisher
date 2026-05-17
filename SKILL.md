---
name: neboai
description: Build, validate, and publish skills, plugins, agents, and apps to the NeboLoop marketplace. Use when the user wants to publish something to NeboLoop, create a new skill/plugin/agent/app, build something for Nebo, put their idea on the marketplace, monetize an automation, or share their creation. Also triggers on "publish to Nebo", "create a skill", "build a plugin", "make an agent", "I have an idea for...", "can I sell this on Nebo?".
compatibility: Works with NeboLoop MCP tools (Claude Desktop) or neboai CLI (Claude Code)
allowed-tools: Bash(neboai *) Bash(cargo *) Bash(rustc *) Read Write Edit Glob Grep
triggers:
  - publish to nebo
  - create a skill
  - build a plugin
  - make an agent
  - build an app
  - I have an idea
  - sell on nebo
  - neboloop
  - nebo marketplace
metadata:
  author: neboloop
  version: "0.2.0"
---
# NeboLoop — From Idea to Marketplace

You are the user's publishing partner. They have an idea — you turn it into a real, published product on the NeboLoop marketplace. They never need to understand file formats, YAML, JSON, or technical details. You handle everything.

## Your Role

1. **Listen** — Understand what the user wants to create
2. **Decide** — Pick the right artifact type (skill, plugin, agent, or app)
3. **Build** — Generate all required files with correct structure
4. **Validate** — Check everything before publishing
5. **Publish** — Submit to NeboLoop marketplace automatically

The user says things like:
- "I want to build something that sends me a morning briefing"
- "Can I make a tool that connects to Stripe?"
- "I have an idea for a deal tracker"
- "Publish this to Nebo"

You respond by asking clarifying questions (if needed), then you build it and publish it. No manual steps. No config files. No tokens. No terminal commands for them to run.

## How Publishing Works (Behind the Scenes)

**Claude Desktop users:** You use the NeboLoop MCP tools directly. The user is already authenticated through their MCP connection. They don't need to do anything.

**Claude Code users:** You use the `neboai` CLI. If they haven't authenticated yet, the CLI automatically opens their browser — they click one button, and it continues. Zero friction.

**The user never needs to know which path you're using.** Just build it and publish it.

## Conversational Flow (Non-Technical Users)

When a user describes an idea without technical specifics, follow this flow:

**1. Understand the idea (1-2 questions max)**
- "What should it do?" (if unclear)
- "Who is this for?" (if it helps scope)
- Don't ask about file formats, languages, or architecture — decide those yourself

**2. Tell them what you're building**
- "I'll create a [skill/agent/plugin/app] that does X. Let me build that for you."
- Keep it one sentence. No technical details unless they ask.

**3. Build it silently**
- Generate all files in memory or a temp directory
- Follow all format rules in this skill
- Validate everything yourself

**4. Publish it**
- Use MCP or CLI (whichever is available)
- Handle any errors yourself (retry, fix, re-publish)
- Tell the user: "Done! Your [thing] is now on the NeboLoop marketplace."

**Example conversation:**
```
User: "I want something that writes me a LinkedIn post every Monday based on my week"
You: "I'll build an agent that drafts a LinkedIn post every Monday morning based on what you worked on that week. Publishing now..."
[You generate AGENT.md + agent.json + manifest, validate, publish via MCP]
You: "Done! Your 'weekly-linkedin-writer' agent is submitted to the marketplace. It'll draft posts every Monday at 8am based on your activity. You'll be notified once it's approved."
```

**Never:**
- Ask which artifact type to use (you decide)
- Show them raw YAML or JSON (unless they ask)
- Ask them to run terminal commands
- Ask them to authenticate — it's automatic
- Explain the publishing process — just do it

---

## Artifact Hierarchy

```
APP  >  AGENT  >  WORKFLOW  >  SKILL
(UI)   (job)    (procedure)  (knowledge + actions)
```

Start with knowledge (Skill), chain into procedures (Workflow), compose into a job (Agent), add UI when chat isn't enough (App). Plugins are shared native binaries that skills depend on — they sit alongside the hierarchy.

## Language Preferences for Binaries

When generating code for plugins, sidecars, or any compiled binary:

| Priority | Language | Why |
|----------|----------|-----|
| 1 (preferred) | **Rust** | No runtime dependencies. Static binary. Does not trigger antivirus heuristics (unlike Go/Python). Cannot be modified by the agent at runtime (compiled, not interpreted). Smallest attack surface. |
| 2 (acceptable) | Go | Static binary, fast compilation. Some AV false positives. Agent cannot modify at runtime. |
| 3 (avoid) | Python/Node | Interpreted — agent CAN modify scripts at runtime. Requires runtime installed. Larger attack surface. AV signatures. |

**Rust is strongly preferred** because:
- Single static binary per platform (no glibc, no runtime)
- AV-friendly: compiled Rust binaries don't have the same heuristic signature as packed Go binaries or script interpreters
- Immutable: once compiled, the agent cannot alter the binary's behavior
- Cross-compilation is straightforward via `rustup target add`
- Small binary size with `--release` and `strip`

When scaffolding a new plugin or sidecar, always use Rust unless the user explicitly requests otherwise.

## What to Build — Decision Tree

| User Says | Build This |
|-----------|-----------|
| "teach the agent to..." / "when I say X, do Y" | **Skill** — markdown instructions |
| "connect to [service]" / "I need [API] access" | **Plugin** — native binary with tools + auth |
| "every morning..." / "monitor for..." / "when X happens, do Y" | **Agent** — workflows with triggers |
| "I need a dashboard" / "build me a [noun] tracker" | **App** — agent + frontend UI |
| "add a tool that..." / "give the agent the ability to..." | **Plugin** (if binary) or **Skill** (if instructions-only) |

---

## Building Skills

A skill is a folder with a `SKILL.md` that teaches the agent something. No manifest.json needed.

### Directory Structure

```
my-skill/
├── SKILL.md           # Required — YAML frontmatter + markdown body
├── scripts/           # Optional — executables the agent can run
├── references/        # Optional — detailed docs loaded on demand
├── assets/            # Optional — templates, images, fonts
└── examples/          # Optional — sample data
```

### SKILL.md Template

```yaml
---
name: my-skill-name
description: [What it does] + [When to use it — include trigger phrases users would say]
capabilities: []
triggers:
  - phrase one
  - phrase two
plugins:
  - name: plugin-slug
    version: ">=1.0.0"
    optional: false
---
# Skill Title

[Imperative instructions. Step-by-step. Specific. Under 500 lines.]
```

### Writing Effective Skills

1. **Description is the trigger.** It must say what AND when. Include the exact phrases users type.
2. **Be imperative.** "Check the inbox" not "You should check the inbox."
3. **Be specific.** "7 days" not "recent". "$50,000" not "a significant amount."
4. **Decision rules over judgment.** "If >20%, highlight" not "highlight important changes."
5. **Show output format.** The agent needs to know what the result looks like.
6. **Error cases explicitly.** What to do when data is missing or the API fails.
7. **Under 500 lines.** Factor details into `references/` files.
8. **One skill, one job.** Don't combine unrelated responsibilities.

### Frontmatter Fields

| Field | Required | Description |
|-------|----------|-------------|
| `name` | yes | Lowercase, hyphens, 1-64 chars. Must match directory name. |
| `description` | yes | What it does + when to use it. Max 1024 chars. |
| `capabilities` | no | Platform needs: `storage`, `network`, `vision`, `python`, `typescript`, `calendar`, `email`, `browser` |
| `triggers` | no | Case-insensitive substring phrases that activate the skill |
| `plugins` | no | Plugin dependencies: `[{name, version, optional}]` |
| `priority` | no | Higher = matched first (default 0) |
| `max_turns` | no | Max agent turns (0 = unlimited) |
| `platform` | no | OS filter: `macos`, `linux`, `windows` |

### Template Variables (Available in SKILL.md body)

| Variable | Description |
|----------|-------------|
| `${NEBO_SKILL_DIR}` | Directory containing the SKILL.md |
| `${NEBO_DATA_DIR}` | Persistent data dir (survives upgrades) |
| `${NEBO_USER_NAME}` | User's display name |
| `${NEBO_OS}` | `macos`, `linux`, `windows` |
| `${NEBO_ARCH}` | `aarch64`, `x86_64` |
| `${plugin.SLUG_BIN}` | Resolved path to a plugin binary |
| `${secret.KEY}` | Decrypted secret value |

---

## Building Plugins

A plugin is a native binary that provides tools, hooks, commands, routes, providers, and config to the platform.

### Directory Structure

```
my-plugin/
├── PLUGIN.md          # Marketplace description
├── plugin.json        # Config: platforms, capabilities, auth, permissions
├── skills/            # Bundled skills that teach agents to use the tools
│   └── my-tool/
│       └── SKILL.md
└── dist/              # Compiled binaries per platform
    ├── darwin-arm64/
    │   └── my-plugin
    ├── linux-amd64/
    │   └── my-plugin
    └── (other platforms)
```

### plugin.json Essential Fields

```json
{
  "id": "my-plugin",
  "slug": "my-plugin",
  "name": "My Plugin",
  "version": "1.0.0",
  "description": "What this plugin provides",
  "author": "Publisher Name",
  "platforms": {
    "darwin-arm64": { "binaryName": "my-plugin", "sha256": "", "size": 0 },
    "linux-amd64": { "binaryName": "my-plugin", "sha256": "", "size": 0 }
  },
  "capabilities": {
    "tools": [],
    "hooks": [],
    "commands": [],
    "routes": [],
    "providers": [],
    "configSchema": []
  },
  "auth": {
    "type": "oauth_cli",
    "label": "Service Name",
    "commands": { "login": "auth login", "status": "auth status", "logout": "auth logout" }
  },
  "permissions": {
    "envAllow": ["HOME", "PATH"],
    "network": true,
    "maxTimeoutSeconds": 300
  }
}
```

### Rust Plugin Binary Pattern

```rust
use clap::{Parser, Subcommand};
use serde_json::json;

#[derive(Parser)]
#[command(name = "my-plugin")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Auth { #[command(subcommand)] action: AuthAction },
    // One subcommand per tool domain
    Gmail { #[command(subcommand)] action: GmailAction },
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Commands::Auth { action } => handle_auth(action),
        Commands::Gmail { action } => handle_gmail(action),
    }
}
```

**Output contract:** JSON on stdout (success), JSON on stderr (error), exit code 0/1.

### Cross-Compilation (Rust)

```bash
rustup target add aarch64-apple-darwin x86_64-apple-darwin
rustup target add aarch64-unknown-linux-musl x86_64-unknown-linux-musl

cargo build --release --target aarch64-apple-darwin
cargo build --release --target x86_64-unknown-linux-musl
```

Use `musl` for Linux — avoids glibc version issues. Strip binaries:

```bash
strip target/aarch64-apple-darwin/release/my-plugin
```

### Tool Definition

```json
{
  "name": "service.action",
  "description": "What this does and WHEN to use it",
  "command": "service action",
  "inputSchema": {
    "type": "object",
    "properties": {
      "param": { "type": "string", "description": "What this is" }
    },
    "required": ["param"]
  },
  "approval": true,
  "timeoutSeconds": 30
}
```

- Every property needs a `description`
- Use `enum` for fixed value sets
- Set `"approval": true` for destructive/external actions (sends, deletes, purchases)
- Always bundle skills that teach the agent *when* to use each tool

### Valid Platforms

`darwin-arm64`, `darwin-amd64`, `linux-arm64`, `linux-amd64`, `windows-arm64`, `windows-amd64`

Minimum required: `darwin-arm64` + `linux-amd64`

---

## Building Agents

An agent is a job description with workflows. Three files: `AGENT.md` (persona), `agent.json` (wiring), `manifest.json` (identity).

### Directory Structure

```
my-agent/
├── AGENT.md           # Persona, communication style, rules
├── agent.json         # Workflows, triggers, inputs, pricing
├── manifest.json      # Marketplace identity
├── views.json         # Optional — deterministic workspace UI
└── theme.css          # Optional — styling
```

### AGENT.md Template

```markdown
---
name: agent-name
description: One-line job description for marketplace discovery.
triggers:
  - trigger phrase
metadata:
  version: "1.0.0"
---
# Agent Name

[Who this agent is — personality, voice, approach]

## Communication Style
[How it talks: bullet points vs prose, formal vs casual, verbose vs terse]

## Capabilities
[What it can do — bulleted]

## Rules
[Hard constraints — never/always statements]

## Judgment
[Decision criteria for ambiguous situations — what "important" means, how to prioritize]

## What You Don't Do
[Explicit boundaries]
```

### agent.json Template

```json
{
  "workflows": {
    "workflow-name": {
      "trigger": { "type": "schedule", "cron": "0 7 * * *" },
      "description": "What this workflow does",
      "activities": [
        {
          "id": "step-1",
          "intent": "What the LLM should accomplish",
          "skills": ["@org/skills/name"],
          "steps": ["Step hint 1", "Step hint 2"],
          "token_budget": { "max": 4096 },
          "on_error": { "retry": 1, "fallback": "skip" }
        }
      ],
      "budget": { "total_per_run": 6000 },
      "emit": "event.name"
    }
  },
  "requires": { "plugins": ["PLUG-XXXX-XXXX"] },
  "skills": ["@org/skills/name@^1.0.0"],
  "inputs": [
    {
      "key": "timezone",
      "label": "Your Timezone",
      "type": "select",
      "required": false,
      "default": "US/Eastern",
      "options": [{ "value": "US/Eastern", "label": "Eastern" }]
    }
  ],
  "pricing": { "model": "monthly_fixed", "cost": 47.0 },
  "defaults": {
    "timezone": "user_local",
    "configurable": ["workflows.workflow-name.trigger.cron"]
  },
  "memory": { "inherit_user": true, "context_isolated": false }
}
```

### Trigger Types

| Type | Use When | Fields |
|------|----------|--------|
| `schedule` | Fixed-time: "every weekday at 7am" | `cron` (5-field) |
| `heartbeat` | Recurring interval: "every 30 min" | `interval`, `window` (optional) |
| `event` | Reactive: "when X happens" | `sources` (pattern array) |
| `watch` | Real-time plugin stream | `plugin`, `event`, `command`, `restart_delay_secs` |
| `manual` | User-initiated only | — |

### Activity Design Rules

- Each activity does ONE thing
- `intent` is what the LLM should accomplish (imperative)
- `steps` are hints, not commands — the LLM has judgment
- Sum of `token_budget.max` values must ≤ `budget.total_per_run`
- IDs must be unique within each workflow binding
- Use `on_error.fallback: "skip"` for enrichment, `"abort"` for critical steps

### Input Fields

All inputs should have defaults (zero-config install). Template substitution: `{{key}}` in commands must exactly match an input `key`.

| Type | Use For |
|------|---------|
| `text` | Short free-form (name, URL) |
| `textarea` | Long text (custom instructions) |
| `number` | Numeric (threshold, count) |
| `select` | Pick one from known set |
| `checkbox` | Boolean toggle |
| `radio` | Pick one with all visible |

---

## Building Apps

An app is an agent with a dedicated frontend UI. Use when chat output isn't enough.

### Directory Structure

```
my-app/
├── AGENT.md              # Persona
├── manifest.json         # identity + artifact_type: "app" + permissions + window
├── agent.json            # Optional — workflows, skills, inputs, sidecar tools, scopes
├── ui/                   # Required — static frontend
│   ├── index.html
│   ├── style.css
│   └── app.js
├── skills/               # Optional — teach agent how to use tools
│   └── workspace-mgmt/
│       └── SKILL.md
└── sidecar/              # Optional — Rust native backend
    ├── Cargo.toml
    └── src/main.rs
```

### manifest.json (App)

```json
{
  "id": "my-app",
  "name": "@org/agents/my-app",
  "version": "1.0.0",
  "description": "What this app does",
  "artifact_type": "app",
  "permissions": ["storage:readwrite", "subagent:invoke", "network:outbound"],
  "window": { "title": "My App", "width": 1024, "height": 768, "resizable": true }
}
```

### Frontend SDK (`@neboai/app-sdk`)

```javascript
import { nebo } from '@neboai/app-sdk';

nebo.chat.mount(el, { placeholder: '...', theme: 'dark', contextId: id });
nebo.chat.send('prompt');
const { text } = await nebo.agents.invoke('prompt');
for await (const chunk of nebo.agents.stream('prompt')) { ... }
await nebo.storage.setItem('key', 'value');
const resp = await nebo.fetch('/sidecar-path');   // relative → sidecar
const ext = await nebo.fetch('https://...');      // absolute → CORS-free proxy
```

### Sidecar (Rust preferred)

- Reads `$NEBO_APP_SOCK` → binds Unix socket
- Reads `$NEBO_APP_DATA` → persistent storage directory
- Must start within 10 seconds
- Implement `GET /_tools` for LLM tool discovery
- Handle SIGTERM gracefully

### Sidecar Tool Definitions (in agent.json)

```json
{
  "tools": [
    { "name": "list_items", "description": "...", "method": "GET", "path": "/items" },
    { "name": "create_item", "description": "...", "method": "POST", "path": "/items",
      "input_schema": { "type": "object", "properties": { "name": { "type": "string" } } } }
  ]
}
```

### Tool Scopes

Restrict tools per embed context:

```json
{
  "scopes": {
    "read": { "tools": ["list_items", "get_item"], "skills": [], "plugins": [] },
    "write": { "tools": ["list_items", "create_item", "delete_item"], "skills": [], "plugins": [] }
  }
}
```

SDK: `nebo.chat.mount(el, { scope: "read" });`

---

## Publishing — Implementation

Detect your environment and use the appropriate path. **Never ask the user to choose** — just detect and use.

### Detecting Your Path

Check your available tools:
- If any of these exist in your tool list → **use MCP path:**
  - `mcp__claude_ai_NeboLoop__skill`, `mcp__claude_ai_NeboLoop__agent`, `mcp__claude_ai_NeboLoop__plugin`, `mcp__claude_ai_NeboLoop__developer`
  - `mcp__levee__skill`, `mcp__levee__agent`, `mcp__levee__plugin`, `mcp__levee__developer`
- Otherwise → **use CLI path** (`neboai publish <directory>`)

Use `ToolSearch` to discover NeboLoop tools if unsure: search for "neboloop" or "levee".

### MCP Path (Claude Desktop / any MCP-connected environment)

The user is already authenticated. No auth step needed.

**Step 1: Select developer account (required before any publish)**
```
developer(resource: account, action: select)
```
If no account exists, call `developer(resource: account, action: create)` first.

**Step 2: Create the artifact**

For skills:
```
skill(action: create, name: "my-skill-name", manifestContent: "<entire SKILL.md content>")
```

For agents:
```
agent(action: create, name: "my-agent-name", manifestContent: "<entire AGENT.md content>")
```

For plugins:
```
plugin(action: create, name: "my-plugin-name", category: "connectors")
```

**Step 3: Upload config (agents and plugins only)**
```
skill(action: binary-token, id: "<ID>")  OR  agent(action: binary-token, id: "<ID>")
```
Then use the returned curl command to upload agent.json/plugin.json + binaries.

**Step 4: Submit for review**
```
skill(action: submit, id: "<ID>", version: "1.0.0")
agent(action: submit, id: "<ID>", version: "1.0.0")
plugin(action: submit, id: "<ID>", version: "1.0.0")
```

**If the user has never connected NeboLoop:** Tell them: "To publish, you'll need to connect your NeboLoop account. In Claude Desktop, go to Settings → MCP Servers → Add the NeboLoop server. Once connected, just say 'publish' again and I'll handle the rest."

### CLI Path (Claude Code / Cursor / VS Code)

```bash
neboai publish <directory>
```

That's it. The CLI:
1. Detects artifact type from directory contents
2. Validates locally (structure, JSON, YAML, names, budgets)
3. Authenticates automatically (opens browser on first use — user clicks one button)
4. Uploads everything
5. Submits for review

**Override type:** `neboai publish ./dir --type agent`

**The user never runs auth commands.** Everything is automatic.

### What to Tell the User

After publishing succeeds, tell them:
- "Done! Your [skill/agent/plugin/app] has been submitted to the NeboLoop marketplace."
- "It'll be reviewed shortly. You can check its status anytime."
- Give them the artifact name and version

If publishing fails, diagnose and fix it yourself. Don't dump error messages on non-technical users.

---

## Validation

Always validate before publishing. Never skip this.

**If CLI is available:**
```bash
neboai validate <directory>
```

**If using MCP (no CLI):** Validate by checking these yourself:
- [ ] YAML frontmatter has `name` and `description`
- [ ] Name is lowercase + hyphens only, 1-64 chars
- [ ] JSON files parse cleanly (no trailing commas, no comments)
- [ ] No `{{template_vars}}` in plugin.json — all values must be literal
- [ ] Versions are valid semver (e.g., "1.0.0")
- [ ] Budget math: sum of activity `token_budget.max` ≤ workflow `budget.total_per_run`
- [ ] Required files exist (SKILL.md for skills, AGENT.md + agent.json for agents, etc.)

**If validation fails:** Fix it yourself. Don't ask the user to fix JSON or YAML — that's your job.

---

## Type Detection

| Present | Type |
|---------|------|
| `manifest.json` with `artifact_type: "app"` | app |
| `plugin.json` | plugin |
| `agent.json` + `AGENT.md` | agent |
| `SKILL.md` only | skill |

## What Gets Uploaded

| Type | Manifest | Config | Binary | Skills Tarball |
|------|----------|--------|--------|----------------|
| Skill | SKILL.md | — | — | — |
| Plugin | PLUGIN.md | plugin.json | Per-platform binary | skills/ tarball |
| Agent | AGENT.md | agent.json | — (platform=linux-amd64) | — |
| App | AGENT.md | agent.json | Sidecar per-platform | — |

## Managing Artifacts

When the user asks "what have I published?" or "check on my agent":

**MCP:**
```
skill(action: get, id: "<ID>")
plugin(action: get, id: "<ID>")
agent(action: get, id: "<ID>")
marketplace(action: search, query: "owner's artifacts")
```

**CLI:**
```bash
neboai list                    # List published artifacts
neboai status <id>             # Check review status
neboai binaries list <id>      # List uploaded binaries
neboai binaries delete <id>    # Delete a binary (fix duplicates)
```

**Updating an artifact:** Use `action: update` with the same ID and new `manifestContent`. Then re-submit with bumped version.

---

## Critical Rules

1. **Config = agent.json.** NEVER upload manifest.json as the config field.
2. **Agents use platform=linux-amd64** even though they're not platform-specific.
3. **Plugin.json must be hardcoded.** No `{{template_vars}}`. All values literal.
4. **JSON must be valid.** No trailing commas. Validate: `python3 -c "import json; json.load(open('file.json'))"`
5. **Upload tokens expire in 5 minutes.** The CLI handles this automatically.
6. **HTTP/1.1 for uploads.** HTTP/2 causes stream errors on large files. CLI handles this.
7. **Skills tarball + config only on first platform upload** (plugins). Subsequent platforms are binary-only.
8. **Plugin binaries: minimum darwin-arm64 + linux-amd64.**
9. **Duplicate version+platform = 500 error.** Delete existing binary first.
10. **Budget math must balance.** Sum of activity token_budget.max ≤ budget.total_per_run.

---

## Error Recovery

Handle these yourself — never dump errors on the user.

| Error | Fix |
|-------|-----|
| Duplicate version+platform (500) | Delete existing binary, re-upload |
| Upload token expired | Get a fresh token and retry |
| Validation failed | Fix the issue in your generated files and retry |
| Auth failed (CLI) | The browser flow will retry automatically |
| Name already taken | Suggest a variant or ask the user for a new name |

**CLI tools:**
```bash
neboai binaries list <id>      # See what was uploaded
neboai binaries delete <id>    # Clean up partial uploads
neboai publish <dir> --resume  # Re-attempt from last successful step
```

**MCP:** Use `skill/agent/plugin(action: get, id: "<ID>")` to check state, then delete/recreate as needed.

---

## Building Reference Guides

For deep dives on each artifact type:

- [references/building-skills.md](references/building-skills.md) — Writing skills that trigger reliably and produce consistent results
- [references/building-plugins.md](references/building-plugins.md) — Binary architecture, tool design, events, auth, cross-platform builds
- [references/building-agents.md](references/building-agents.md) — Persona craft, workflow design, triggers, budgets, testing
- [references/building-apps.md](references/building-apps.md) — Frontend SDK, sidecar architecture, state management, tool discovery

## Format Quick Reference

- [references/skill-format.md](references/skill-format.md) — Complete SKILL.md field reference
- [references/plugin-format.md](references/plugin-format.md) — plugin.json schema and capabilities
- [references/agent-format.md](references/agent-format.md) — agent.json full field reference
- [references/app-format.md](references/app-format.md) — manifest.json, SDK, sidecar contract
- [references/common-mistakes.md](references/common-mistakes.md) — Every known gotcha with fixes
