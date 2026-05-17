# Common Mistakes

## Upload Mistakes

| Mistake | What Happens | Fix |
|---------|-------------|-----|
| Upload `manifest.json` as `config` | Overwrites agent.json in `type_config` | Only upload `agent.json` as `config` |
| Template vars in plugin.json | Users see `{{gcp_project}}` errors | Hardcode all values |
| Trailing comma in JSON | Upload rejected: "config file must be valid JSON" | Validate: `python3 -c "import json; json.load(open('file.json'))"` |
| HTTP/2 with large uploads | curl exit code 92, stream error | Always use `--http1.1` |
| Expired upload token | 401 Unauthorized | Tokens expire in 5 min. Get a fresh one. |
| Missing `platform` on agent upload | 400: "platform is required" | Use `linux-amd64` for agents |
| `universal` as platform | 400: "invalid platform" | Use a real platform like `linux-amd64` |
| Duplicate version+platform binary | 500: duplicate key constraint | Delete existing binary first |

## SKILL.md Mistakes

| Mistake | What Happens | Fix |
|---------|-------------|-----|
| Duplicate YAML fields | Parse error at install time, skill not loaded | Each YAML key must appear exactly once |
| `triggers:` outside frontmatter | Not parsed, skill never triggers | Put triggers inside `---` block |
| Description too vague | Skill never activates | Include what it does AND when to use it |
| SKILL.md over 500 lines | Excessive context usage | Factor into `references/` files |
| `name` doesn't match directory | Validation fails | Name must equal directory name |

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
| Model not `"nebo-1"` | Workflow uses wrong provider | All models must be `"nebo-1"` |
| Required inputs without defaults | Users can't zero-config install | Make inputs optional or provide defaults |
| `{{key}}` placeholder not matching input | Watch command has literal `{{key}}` text | Placeholder name must exactly match an input `key` |
| Activity IDs not unique | Parse error | Each activity needs a unique `id` within its binding |
| Budget math wrong | Agent won't load | Sum of activity `token_budget.max` <= `budget.total_per_run` |

## App Mistakes

| Mistake | What Happens | Fix |
|---------|-------------|-----|
| Missing `artifact_type: "app"` | Agent loads but no UI/window | Set `"artifact_type": "app"` in manifest.json |
| Missing `ui/index.html` | App page shows 404 | Create `ui/` with `index.html` entry point |
| Sidecar doesn't read `$NEBO_APP_SOCK` | Connection timeout | Read env var and bind socket there |
| Sidecar startup > 10s | Launch fails | Optimize startup or increase timeout |

## Validation Command

Always validate before publishing:

```bash
neboai validate ./my-artifact
```

This catches all of the above locally before hitting the API.
