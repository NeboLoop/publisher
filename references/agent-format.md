# Agent Format

Agents are autonomous workflow runners with a persona (AGENT.md) and operational wiring (agent.json).

## Directory Structure

```
my-agent/
  AGENT.md         # Persona, communication style, rules
  agent.json       # Inputs, workflows, activities, skills, pricing
  manifest.json    # NPM-style metadata (NOT uploaded as config)
  views.json       # Optional — deterministic workspace UI
  theme.css        # Optional — agent-specific styling
```

## AGENT.md

```yaml
---
name: my-agent
description: "What this agent does."
triggers:
  - keyword one
  - keyword two
metadata:
  version: 1.0.0
  category: "productivity"
  requires:
    plugins:
      - gws
---

# Agent Name

Persona description, communication style, judgment rules, what it does and doesn't do.
```

## agent.json

```json
{
  "workflows": {
    "morning-briefing": {
      "trigger": {
        "type": "schedule",
        "cron": "0 7 * * *"
      },
      "description": "Daily morning briefing",
      "activities": [
        {
          "id": "gather",
          "intent": "Gather today's priorities",
          "skills": ["@nebo/skills/briefing-writer"],
          "steps": ["Check calendar", "Scan inbox"],
          "token_budget": { "max": 4096 }
        }
      ],
      "budget": { "total_per_run": 6000 }
    }
  },
  "requires": {
    "plugins": ["PLUG-PJ3Z-ECFV"]
  },
  "skills": [
    "@nebo/skills/briefing-writer@^1.0.0"
  ],
  "inputs": [
    {
      "key": "timezone",
      "label": "Your Timezone",
      "type": "select",
      "required": true,
      "default": "US/Eastern",
      "options": [
        { "value": "US/Eastern", "label": "Eastern" },
        { "value": "US/Pacific", "label": "Pacific" }
      ]
    }
  ],
  "pricing": {
    "model": "monthly_fixed",
    "cost": 47.0
  },
  "defaults": {
    "timezone": "user_local",
    "configurable": ["workflows.morning-briefing.trigger.cron"]
  }
}
```

## Trigger Types

| Type | Fields | Description |
|------|--------|-------------|
| `schedule` | `cron` | 5-field cron expression |
| `heartbeat` | `interval`, `window` | Recurring interval with optional time window |
| `event` | `sources` | React to EventBus events |
| `watch` | `plugin`, `event`, `command` | Long-running NDJSON process |
| `manual` | — | Only via API or user request |

## Activity Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `id` | string | yes | Unique within binding |
| `intent` | string | yes | What the LLM should accomplish |
| `steps` | string[] | no | Step-by-step hints |
| `skills` | string[] | no | Skill qualified names |
| `model` | string | no | `"sonnet"`, `"haiku"`, `"opus"` |
| `token_budget` | object | no | `{ "max": 4096 }` |
| `on_error` | object | no | `{ "retry": 1, "fallback": "skip" }` |

## Input Fields

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `key` | string | yes | Used in `{{key}}` template substitution |
| `label` | string | yes | Display label |
| `type` | string | yes | `text`, `textarea`, `number`, `select`, `checkbox`, `radio` |
| `required` | boolean | no | Must be filled before activation |
| `default` | any | no | Pre-filled value |
| `options` | array | no | For select/radio: `[{ "value": "...", "label": "..." }]` |

## Key Rules

- All inputs should be optional for zero-config install
- Plugins in `requires.plugins` are auto-installed (use install codes like `PLUG-XXXX-XXXX`)
- `{{key}}` placeholders in trigger commands must exactly match an input `key`
- Template substitution works in ALL trigger configs (not just watch) — cron, intervals, etc.
- Activity IDs must be unique within each binding
- Budget math: sum of activity `token_budget.max` must not exceed `budget.total_per_run`
- Activities execute sequentially — each output becomes context for the next
- Empty activity output = branch termination (downstream activities skip)

## Memory Configuration

```json
{
  "memory": {
    "inherit_user": true,
    "context_isolated": true
  }
}
```

| Config | Reads | Writes |
|--------|-------|--------|
| Default (both false) | Layer 2 (agent) | Layer 2 |
| `inherit_user: true` | Layer 1 (user, read-only) + Layer 2 | Layer 2 |
| `context_isolated: true` | Layer 3 (per-context) | Layer 3 |
| Both true | All 3 layers | Layer 3 |

- `inherit_user: true` — agent reads user's Nebo preferences (timezone, language, style) — read-only
- `context_isolated: true` — memories isolated per SDK embed `contextId`

## Sidecar Tool Definitions

Declare HTTP endpoints as native LLM tools directly in agent.json:

```json
{
  "tools": [
    {
      "name": "list_items",
      "description": "List all items",
      "method": "GET",
      "path": "/items"
    },
    {
      "name": "get_item",
      "description": "Get item by ID",
      "method": "GET",
      "path": "/items/{id}",
      "input_schema": { "type": "object", "properties": { "id": { "type": "string" } } }
    }
  ]
}
```

- Path parameters resolved from input: `/items/{id}` + `{"id": "abc"}` → `/items/abc`
- GET → query params, POST/PUT/PATCH → JSON body
- Also discoverable via `GET /_tools` on the sidecar

## Tool Scoping

Restrict which tools/skills/plugins are available per embed context:

```json
{
  "scopes": {
    "read": { "tools": ["list_items", "get_item"], "skills": [], "plugins": [] },
    "write": { "tools": ["list_items", "get_item", "create_item"], "skills": [], "plugins": [] }
  }
}
```

SDK usage: `nebo.chat.mount(el, { scope: "read" });`

## Agent Soul (Optional)

Separate from AGENT.md. Where AGENT.md = job description, soul = personality/voice/values.

| | AGENT.md | soul |
|---|----------|------|
| Purpose | Job description | Personality |
| Contains | Capabilities, priorities, judgment | Voice, tone, quirks, values, boundaries |
| Analogy | What the agent *does* | Who the agent *is* |

Set via Settings UI, API, or `soul` field in agent.json. Injected as `agent_soul` context.

## Followup Suggestions

After each chat turn, the agent generates 2-3 contextual follow-up chips:
- 2-8 words each
- Not phrased as questions
- No "Tell me" / "Can you" patterns
- Generated asynchronously after main response

## Auto-Install Cascade

When a user installs an Agent:
1. Parse `requires.plugins` → install plugins
2. Parse workflow skill references → install skills
3. Parse top-level `skills` array → install additional skills
4. Skills cascade to their own plugin dependencies
5. Register trigger bindings
6. Load AGENT.md persona

## Publishing

```bash
neboai publish ./my-agent
```

The CLI will:
1. Validate AGENT.md frontmatter and agent.json structure
2. Upload AGENT.md as the manifest
3. Upload agent.json as the config (with `platform=linux-amd64`)
4. Submit for review

## CRITICAL

- The `config` upload field = `agent.json`. NEVER `manifest.json`.
- `manifest.json` is for marketplace identity only — it is NOT uploaded as config.
- If you upload manifest.json as config, it overwrites agent.json with just 3 keys.
- Agent with no workflows is valid — chat-only with persona + skills.
