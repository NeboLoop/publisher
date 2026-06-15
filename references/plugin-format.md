# Plugin Format

Plugins are native binaries distributed per-platform with bundled skills.

## Directory Structure

```
my-plugin/
  PLUGIN.md          # Marketplace manifest (markdown)
  plugin.json        # Config: platforms, events, auth, capabilities
  skills/            # Bundled SKILL.md files
    skill-a/
      SKILL.md
    skill-b/
      SKILL.md
  dist/              # Built binaries
    darwin-arm64/
      my-plugin
    darwin-amd64/
      my-plugin
    linux-arm64/
      my-plugin
    linux-amd64/
      my-plugin
    windows-amd64/
      my-plugin.exe
```

## plugin.json

```json
{
  "id": "my-plugin",
  "slug": "my-plugin",
  "name": "My Plugin",
  "version": "1.0.0",
  "description": "What this plugin does",
  "author": "NeboLoop",
  "platforms": {
    "darwin-arm64": {
      "binaryName": "my-plugin",
      "sha256": "<hash>",
      "signature": "<ed25519-signature>",
      "size": 12345678,
      "downloadUrl": "https://cdn.neboai.com/plugins/my-plugin/1.0.0/darwin-arm64/my-plugin"
    }
  },
  "envVar": "MY_PLUGIN_BIN",
  "events": [
    {
      "name": "something.new",
      "description": "Fires when something happens",
      "command": "watch --project my-project",
      "multiplexed": false
    }
  ],
  "auth": {
    "type": "oauth_cli",
    "label": "My Account",
    "description": "Authenticate to access the service.",
    "help": {
      "url": "https://example.com/docs/auth",
      "urlLabel": "Setup instructions",
      "text": "Authenticate to access the service."
    },
    "commands": {
      "login": "auth login",
      "status": "auth status",
      "logout": "auth logout"
    },
    "env": {
      "CLIENT_ID": "...",
      "CLIENT_SECRET": "..."
    }
  },
  "capabilities": {
    "tools": [],
    "hooks": [],
    "commands": [],
    "routes": [],
    "providers": [],
    "configSchema": []
  },
  "permissions": {
    "envAllow": ["HOME", "PATH"],
    "envDeny": ["AWS_SECRET_ACCESS_KEY"],
    "network": true,
    "maxTimeoutSeconds": 300
  }
}
```

## Required plugin.json Fields

| Field | Rule |
|-------|------|
| `id` | Required. Use the slug. Deserialization fails without it. |
| `slug` | Lowercase alphanumeric + hyphens. No leading/trailing hyphens. No consecutive hyphens. Max 64 chars. |
| `version` | Valid semver (e.g., `"1.2.3"`, not `"latest"`). |
| `platforms` | At least one platform entry. |
| `binaryName` | No path separators. No `..`. Cannot be empty. |

## Optional plugin.json Fields

| Field | Type | Description |
|-------|------|-------------|
| `channel` | PluginChannel | Channel bridge configuration (for messaging platform plugins). Set `channel.shared: true` to run one bridge process shared across all agents (incoming messages routed to agents by name); otherwise each agent gets its own bridge. |
| `events` | Vec\<PluginEventDef\> | Event definitions the plugin can emit. |
| `dependencies` | Vec\<PluginDependency\> | Other plugins this plugin depends on. |
| `triggers` | Vec\<String\> | Trigger phrases that activate the plugin. |
| `signingKeyId` | String | ID of the ED25519 signing key used to verify binaries. |
| `envVar` | String | Environment variable name for the binary path (e.g., `"GWS_BIN"`). |
| `setup` | ArtifactSetup | Post-install setup instructions or steps. |
| `category` | String | Marketplace category for discovery. |

## PlatformBinary Fields

Each entry under `platforms` has 5 fields:

| Field | Required | Description |
|-------|----------|-------------|
| `binaryName` | Yes | Filename of the executable. No path separators or `..`. |
| `sha256` | Yes | SHA-256 hash of the binary for verification. |
| `signature` | Yes | ED25519 signature for authenticity verification. |
| `size` | Yes | Binary file size in bytes. |
| `downloadUrl` | Yes | CDN URL to download the binary. |

## PLUGIN.md

```yaml
---
name: my-plugin
description: "What this plugin provides."
version: "1.0.0"
source: https://github.com/org/repo
license: Apache-2.0
---

# my-plugin

Description and table of bundled skills.
```

## Capabilities

Plugins can declare structured capabilities:

### Tools
Typed, schema-validated tools the agent can call:
```json
{
  "name": "gws.gmail.triage",
  "description": "Triage Gmail inbox",
  "command": "gmail +triage",
  "inputSchema": { "type": "object", "properties": {} },
  "approval": true,
  "timeoutSeconds": 120
}
```

- `approval` defaults to `true`. Set `false` explicitly for read-only tools.
- `timeoutSeconds` defaults to `120`. Override as needed.

### Hooks
Intercept lifecycle events:
```json
{
  "hook": "tool.pre_execute",
  "hookType": "filter",
  "priority": 50,
  "command": "hooks tool-pre-execute",
  "timeoutMs": 500
}
```

### Commands
Slash commands:
```json
{
  "name": "/gmail",
  "description": "Gmail operations",
  "command": "gmail",
  "slash": true
}
```

### Routes
HTTP endpoints (OAuth callbacks, webhooks):
```json
{
  "path": "/oauth/callback",
  "method": "GET",
  "command": "auth callback",
  "auth": "public"
}
```

### Providers
AI model backends:
```json
{
  "id": "openrouter",
  "displayName": "OpenRouter",
  "providerType": "model",
  "modelsCommand": "models list",
  "chatCommand": "chat stream"
}
```

### ConfigSchema
User-configurable settings (rendered as UI form):
```json
{
  "key": "API_KEY",
  "label": "API Key",
  "fieldType": "string",
  "required": true,
  "secret": true
}
```

## Valid Platforms

`darwin-arm64`, `darwin-amd64`, `linux-arm64`, `linux-amd64`, `windows-arm64`, `windows-amd64`

At minimum, target `darwin-arm64` and `linux-amd64`.

## Critical Rules

- **Hardcode all static config values in plugin.json.** No template variables like `{{gcp_project}}` in static fields. However, `events[].command` fields DO support `{{key}}` substitution from agent inputs at runtime.
- **Binary must be a single executable file** — no runtime dependencies.
- **`id` field is required** — without it, the plugin cannot be resolved.
- **`auth.commands.login` must be non-empty** if `auth` is present (unless `auth.type` is `"env"`, which uses env vars and needs no login command). `auth.commands.status` and `auth.commands.logout` are optional.
- **`auth.env`** passes environment variables to auth commands. `auth.description` is user-facing setup text. `auth.help` is a structured object (`url`, `urlLabel`, `text`) shown in configuration modals — not a plain string.

## Publishing

```bash
neboai publish ./my-plugin
```

The CLI will:
1. Validate plugin.json and PLUGIN.md
2. Create the skills tarball from `skills/`
3. Upload first platform binary with config + skills
4. Upload remaining platforms in parallel
5. Submit for review

## Environment Variable Naming

Plugin binaries are exposed as `{SLUG}_BIN` (slug uppercased, hyphens → underscores):
- `gws` → `GWS_BIN`
- `my-tool` → `MY_TOOL_BIN`
