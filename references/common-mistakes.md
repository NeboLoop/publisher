# Common Mistakes

## Upload Mistakes

| Mistake | What Happens | Fix |
|---------|-------------|-----|
| Upload `manifest.json` as `config` | Overwrites agent.json in `type_config` | Only upload `agent.json` as `config` |
| Template vars in plugin.json | Users see `{{gcp_project}}` errors | Hardcode all values |
| Trailing comma in JSON | Upload rejected: "config file must be valid JSON" | Validate: `python3 -c "import json; json.load(open('file.json'))"` |
| HTTP/2 with large uploads | curl exit code 92, stream error | Always use `--http1.1` |
| Expired upload token | 401 Unauthorized | Tokens expire in 5 min. Get a fresh one. |
| Missing `config` on agent upload | 400: "config file is required for agents" | Attach `config=@agent.json` (the agent branch reads only `config` — no `platform`/`file`) |
| Sending `platform`/`file` on an agent upload | Ignored — agent uploads read only `config` | Send `config=@agent.json` alone |
| `universal` as platform (plugins/sidecars) | 400: "invalid platform" | Use a real platform like `linux-amd64` (or a `macos-*` alias, auto-normalized to `darwin-*`) |
| Duplicate version+platform binary | 500: duplicate key constraint | Delete existing binary first |

## SKILL.md Mistakes

| Mistake | What Happens | Fix |
|---------|-------------|-----|
| Duplicate YAML fields | Parse error at install time, skill not loaded | Each YAML key must appear exactly once |
| `triggers:` outside frontmatter | Not parsed, skill never triggers | Put triggers inside `---` block |
| Description too vague | Skill never activates | Include what it does AND when to use it |
| SKILL.md over 500 lines | Excessive context usage | Factor into `references/` files |
| `name` doesn't match directory | Validation warns (does not fail) | The loader keys on the YAML `name`; matching the directory is optional but tidy |

## Plugin Mistakes

| Mistake | What Happens | Fix |
|---------|-------------|-----|
| Missing `id` field | Deserialization fails, plugin unresolvable | Always include `"id": "<slug>"` |
| `version: "latest"` | Validation fails | Use valid semver: `"1.0.0"` |
| No platforms declared | Validation fails | At least one platform entry required |
| Binary name with path separators | Rejected | No `/` or `\` in `binaryName` |
| Empty `auth.commands.login` | Validation fails | Must be non-empty if `auth` present |

## Agent Mistakes

| Mistake | What Happens | Fix |
|---------|-------------|-----|
| Model not recognized | Selector can't resolve model, falls back unpredictably | Use fuzzy names: `sonnet`, `haiku`, `opus`. The selector resolves these to full model IDs. |
| Required inputs without defaults | Users can't zero-config install | Make inputs optional or provide defaults |
| `{{key}}` placeholder not matching input | Watch command has literal `{{key}}` text | Placeholder name must exactly match an input `key` |
| Activity IDs not unique | Parse error | Each activity needs a unique `id` within its binding |
| Budget math wrong | Agent won't load | Sum of activity `token_budget.max` <= `budget.total_per_run` |

## App Mistakes

| Mistake | What Happens | Fix |
|---------|-------------|-----|
| Missing `type: "app"` | Agent loads but no UI/window | Set `"type": "app"` in manifest.json |
| Missing `ui/index.html` | App page shows 404 | Create `ui/` with `index.html` entry point |
| Sidecar doesn't read `$NEBO_APP_SOCK` | Connection timeout | Read env var and bind socket there |
| Sidecar startup > 10s | Launch fails | Optimize startup or increase via `manifest.startup_timeout` (max 120s) |
| manifest.json uses `artifact_type` | Deserialization may fail | Use `type` (serde renames it) |
| Sidecar implemented as HTTP server | Connection refused — Nebo expects gRPC | Must be gRPC over Unix socket (`UIService.HandleRequest`) |
| Tool definitions in `GET /_tools` endpoint | Tools never discovered | Define tools in `agent.json` `tools` array |
| Plugin tool input passed as CLI args | Input lost or garbled | Input JSON arrives on stdin; CLI args come from the `command` field only |
| `on_error` fallback set to skip/abort without knowing default | Unexpected error handling | Default is `notify_owner` |

## Validation Command

Always validate before publishing:

```bash
neboai validate ./my-artifact
```

This catches all of the above locally before hitting the API.
