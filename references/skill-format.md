# Skill Format

Skills are SKILL.md files — markdown instructions that teach AI how to use a tool or perform a task.

## Directory Structure

```
my-skill/
  SKILL.md
```

## SKILL.md Format

```yaml
---
name: my-skill
description: "One-line description of what this skill does."
metadata:
  version: 1.0.0
  openclaw:
    category: "productivity"
    requires:
      bins:
        - gws
      skills:
        - gws-shared
triggers:
  - keyword one
  - keyword two
---

# Skill Title

Instructions, examples, and code blocks here.
```

## Required Fields

| Field | Type | Description |
|-------|------|-------------|
| `name` | string | Skill identifier (lowercase, hyphens). Must match directory name. |
| `description` | string | What the skill does and when to trigger it. Primary trigger mechanism. |

## Optional Fields (Nebo Extensions)

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `version` | string | `1.0.0` | Semantic version |
| `capabilities` | string[] | `[]` | Platform capabilities needed: `storage`, `network`, `vision`, `python`, `typescript` |
| `triggers` | string[] | `[]` | Phrases that activate the skill (case-insensitive substring) |
| `platform` | string[] | `[]` (all) | OS filter: `macos`, `linux`, `windows` |
| `priority` | int | `0` | Higher = matched first when multiple skills match |
| `max_turns` | int | `0` | Max agent turns (0 = unlimited) |
| `dependencies` | string[] | `[]` | Other skill qualified names this depends on |
| `plugins` | object[] | `[]` | Plugin dependencies |

## Plugin Dependencies

```yaml
plugins:
  - name: gws
    version: ">=1.2.0"
    optional: false
```

## Progressive Disclosure

1. **Metadata** (~100 words) — Always in context. Determines trigger.
2. **SKILL.md body** — Loaded when triggered (<500 lines ideal).
3. **Bundled resources** — Loaded on demand (scripts, references).

Keep SKILL.md under 500 lines. Factor detailed content into `references/` files.

## Template Variables

Available in SKILL.md body and scripts (expanded at runtime by Nebo):

| Variable | Example | Description |
|----------|---------|-------------|
| `${NEBO_SKILL_DIR}` | `~/.nebo/nebo/skills/my-skill` | Skill code directory |
| `${NEBO_DATA_DIR}` | `~/.nebo/appdata/skills/my-skill` | Persistent data directory (separate from code) |
| `${NEBO_USER_NAME}` | `Alex` | User's display name |
| `${NEBO_OS}` | `macos` | Operating system |
| `${NEBO_ARCH}` | `aarch64` | CPU architecture |
| `${plugin.SLUG_BIN}` | `/path/to/gws` | Resolved plugin binary path |
| `${secret.KEY}` | `sk-abc...` | Decrypted secret value |

**Data directory is separate from code directory.** `${NEBO_DATA_DIR}` survives upgrades and reinstalls. Store databases, caches, generated files there — never in `${NEBO_SKILL_DIR}`.

## Loading Order

Skills load in priority order (later overrides earlier by name):
1. Embedded bundled skills (compiled into Nebo)
2. Installed skills (`.napp` archives in `nebo/skills/`)
3. User skills (loose files in `user/skills/`)
4. App skills (agent's `skills/` directory — highest priority)

## Hot-Reload

The skill loader watches for changes with 1-second debounce. No restart needed.

## Publishing (Markdown-Only)

```bash
neboai publish ./my-skill
```

The CLI will:
1. Validate the SKILL.md frontmatter and body
2. Create or update the skill on NeboLoop
3. Submit for review

## YAML Rules

- No duplicate fields — YAML keys must be unique within a block
- Triggers go inside the frontmatter `---` block, not after it
- Description should include both what the skill does AND when to use it
- Name in frontmatter MUST match the directory name

## Compatibility

Skills are portable across the Agent Skills ecosystem:

| Platform | Support |
|----------|---------|
| Nebo | Full — frontmatter + body + resources + Nebo extensions |
| Claude Code | Compatible — Nebo extensions ignored |
| OpenAI Codex | Compatible — Nebo extensions ignored |
| Cursor | Compatible — Nebo extensions ignored |
| VS Code Copilot | Compatible — Nebo extensions ignored |
| Gemini CLI | Compatible — Nebo extensions ignored |
| Junie | Compatible — Nebo extensions ignored |
| OpenHands | Compatible — Nebo extensions ignored |

The required fields (`name`, `description`) are universal. Nebo-specific fields are silently ignored by other platforms.
