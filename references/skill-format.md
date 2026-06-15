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
plugins:
  - name: gws
    version: ">=1.2.0"
    optional: false
requires:
  - name: gws-shared
    version: "*"
metadata:
  secrets:
    - key: SERVICE_API_KEY
      label: "Service API Key"
      hint: "https://service.example.com/keys"
      required: true
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
| `name` | string | Skill identifier (lowercase letters, digits, hyphens). Must not start or end with a hyphen. Must not contain consecutive hyphens (`--`). |
| `description` | string | What the skill does and when to trigger it. Used by the LLM for routing decisions. |

## Optional Fields (Nebo Extensions)

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `version` | string | `"1.0.0"` | Semantic version |
| `license` | string | | License identifier (e.g., `MIT`, `Apache-2.0`) |
| `author` | string | | Skill author name or handle |
| `tags` | string[] | `[]` | Categorization tags for discovery |
| `capabilities` | string[] | `[]` | Platform capabilities needed: `storage`, `network`, `vision`, `python`, `typescript`, `calendar`, `email`, `browser`, `notification`. Only `storage`/`network` map to sandbox config and `python`/`typescript` trigger sandboxed execution; the rest are declarative metadata. |
| `triggers` | string[] | `[]` | Phrases for programmatic activation (case-insensitive substring matching) |
| `compatibility` | string | | Free-text compatibility notes (max 500 chars) |
| `allowed-tools` | string[] | `[]` | Space-delimited tool patterns to pre-approve (alias: `allowed_tools`) |
| `platform` | string[] | `[]` (all) | OS filter: `macos`, `linux`, `windows` |
| `priority` | int | `0` | Higher = matched first when multiple skills match |
| `max_turns` | int | `0` | Max agent turns (0 = unlimited) |
| `metadata` | HashMap | `{}` | Arbitrary key-value metadata; includes `secrets` array for secret declarations |
| `requires` | object[] | `[]` | Skill-to-skill dependencies: `[{name, version}]`. Version defaults to `"*"` |
| `dependencies` | string[] | `[]` | Legacy bare-name skill dependencies (prefer `requires`) |
| `plugins` | object[] | `[]` | Plugin dependencies: `[{name, version, optional}]` |

## Plugin Dependencies

```yaml
plugins:
  - name: gws
    version: ">=1.2.0"
    optional: false
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `name` | string | (required) | Plugin slug |
| `version` | string | `"*"` | Semver range |
| `optional` | bool | `false` | If true, skill works without it |

## Secret Declarations

Declare secrets your skill needs in `metadata.secrets`. Users are prompted for values at install time:

```yaml
metadata:
  secrets:
    - key: SERVICE_API_KEY
      label: "Service API Key"
      hint: "https://service.example.com/keys"
      required: true
```

Access secrets at runtime via the `${secret.KEY}` template variable.

## Progressive Disclosure

1. **Metadata** (~100 words) — Always in context. Determines trigger.
2. **SKILL.md body** — Loaded when triggered (<500 lines ideal).
3. **Bundled resources** — Loaded on demand (scripts, references).

Keep SKILL.md under 500 lines. Factor detailed content into `references/` files.

## Template Variables

Available in SKILL.md body and scripts (expanded at runtime by Nebo):

| Variable | Example | Description |
|----------|---------|-------------|
| `${NEBO_SKILL_DIR}` | `<platform data dir>/nebo/skills/my-skill` | Skill code directory |
| `${NEBO_DATA_DIR}` | `<platform data dir>/appdata/skills/my-skill` | Persistent data directory (separate from code) |
| `${NEBO_USER_NAME}` | `Alex` | User's display name |
| `${NEBO_OS}` | `macos` | Operating system |
| `${NEBO_ARCH}` | `aarch64` | CPU architecture |
| `${plugin.SLUG_BIN}` | `/path/to/gws` | Resolved plugin binary path |
| `${secret.KEY}` | `sk-abc...` | Decrypted secret value |

**Data directory is separate from code directory.** `${NEBO_DATA_DIR}` survives upgrades and reinstalls. Store databases, caches, generated files there — never in `${NEBO_SKILL_DIR}`.

The platform data dir is OS-native: `~/Library/Application Support/Nebo` (macOS), `~/.local/share/nebo` (Linux), `%APPDATA%\Nebo` (Windows). Override the root with `NEBO_HOME`.

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
- Name must be lowercase with letters, digits, and hyphens only (no leading/trailing/consecutive hyphens)

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
