# App Format

Apps are agents with a dedicated frontend UI. They bundle a persona, an HTML frontend, and a native sidecar binary. The marketplace installs an app from its sidecar binaries, so a published app needs at least one.

## Directory Structure

```
my-app/
├── AGENT.md              # Required — persona and instructions
├── manifest.json         # Required — identity, permissions, window config
├── agent.json            # Optional — workflows, skills, user inputs
├── ui/                   # Required — static frontend files
│   ├── index.html        #   Entry point
│   ├── style.css
│   └── app.js
├── skills/               # Optional — skill docs for the agent
│   └── workspace-mgmt/
│       └── SKILL.md
├── sidecar/              # Native backend (build it, or put per-platform builds in dist/app/<platform>/)
│   ├── Cargo.toml
│   ├── src/main.rs
│   └── target/release/
│       └── my-app-sidecar
└── data/                 # Auto-created at runtime
```

## manifest.json

```json
{
  "id": "deal-tracker",
  "name": "@acme/agents/deal-tracker",
  "version": "1.0.0",
  "description": "Track real estate deals with AI-powered analysis.",
  "type": "app",
  "permissions": [
    "storage:readwrite",
    "subagent:invoke",
    "network:outbound"
  ],
  "window": {
    "title": "Deal Tracker",
    "width": 1024,
    "height": 768,
    "resizable": true
  }
}
```

## Required manifest.json Fields

| Field | Description |
|-------|-------------|
| `id` | Unique identifier. Must match directory name. |
| `name` | Qualified name (`@org/agents/name`). |
| `version` | Semantic version. |
| `type` | Must be `"app"`. |

## Permissions

| Permission | What It Grants |
|------------|---------------|
| `storage:readwrite` | Scoped KV store |
| `subagent:invoke` | Invoke other agents |
| `network:outbound` | HTTP requests through proxy |
| `filesystem:read` | Read user files |
| `shell:execute` | Run shell commands |
| `memory:read` | Read agent memories |
| `oauth:google` | Google OAuth flow |

## Frontend SDK

Install: `pnpm add @neboai/app-sdk`

```typescript
import { nebo } from '@neboai/app-sdk';

// Identity
const agent = await nebo.identity.get();

// Storage (KV)
await nebo.storage.setItem('key', 'value');
const val = await nebo.storage.getItem('key');

// Agent invocation
const { text } = await nebo.agents.invoke('Analyze deal #42');

// Streaming
for await (const chunk of nebo.agents.stream('Summarize')) {
  output.textContent += chunk.text;
}

// Direct LLM (no persona)
const summary = await nebo.janus.complete({
  messages: [{ role: 'user', content: 'Summarize: ...' }],
  temperature: 0.3
});

// Embedded chat
nebo.chat.mount(document.getElementById('chat'), {
  placeholder: 'Ask about your deals...',
  theme: 'auto',
  contextId: currentDoc.id
});

// HTTP to sidecar
const resp = await nebo.fetch('/projects');
```

## Sidecar (Optional)

The sidecar is a native binary serving gRPC over a Unix socket. Nebo proxies `/apps/{id}/api/*` to it.

### gRPC Contract

```protobuf
service UIService {
  rpc HealthCheck(HealthCheckRequest) returns (HealthCheckResponse);
  rpc Configure(SettingsMap) returns (Empty);
  rpc HandleRequest(HttpRequest) returns (HttpResponse);
}
```

### Environment Variables

| Variable | Description |
|----------|-------------|
| `NEBO_APP_ID` | App identifier |
| `NEBO_APP_SOCK` | Unix socket path |
| `NEBO_DATA_DIR` | Writable data directory |
| `NEBO_APP_DIR` | App root directory |
| `NEBO_APP_TOKEN` | Per-launch auth token for callbacks to Nebo |
| `NEBO_API_URL` | Callback URL to Nebo's HTTP API |
| `NEBO_APP_NAME` | App display name |
| `NEBO_APP_VERSION` | App version string |

The sidecar's environment is sanitized: only the `NEBO_APP_*` / `NEBO_API_URL` vars above plus the allowlisted system vars `PATH`, `HOME`, `TMPDIR`, `LANG`, `LC_ALL`, and `TZ` are passed through. Everything else (API keys, secrets) is stripped.

### Tool Definitions

Tools are defined in the `tools` array in `agent.json` (not discovered at runtime):

```json
{
  "tools": [
    {
      "name": "list_projects",
      "description": "List all projects",
      "method": "GET",
      "path": "/projects"
    }
  ]
}
```

## Publishing

```bash
neboai publish ./my-app
```

The CLI will:
1. Validate manifest.json (`type: "app"` present)
2. Validate AGENT.md and agent.json (if present)
3. Verify `ui/index.html` exists
4. Find the sidecar binaries: one per platform in `dist/app/<platform>/` (e.g. `dist/app/darwin-arm64/my-app`), or the local build in `sidecar/target/release/` for this machine's platform. At least one is required.
5. Create the app with AGENT.md as its manifest (or update it to the new version)
6. Upload each sidecar binary; the first upload also carries `agent.json` (config) and a tar.gz of `ui/` (the `ui` field)
7. Submit for review. The binaries are scanned in the background and the app is listed once the scan passes.

> The marketplace `.napp` for an app carries the agent payload (`manifest.json`, `agent.json`, `AGENT.md`, `signatures.json`), the sidecar under `bin/`, and the uploaded UI under `ui/`.

Via MCP: `agent(action: create, manifestContent: "<AGENT.md with artifact_type: app in its frontmatter>")`, then `agent(action: binary-token, id)` and the returned curl per platform (`file=@<sidecar>`, `platform=<platform>`, `ui=@ui.tar.gz`), then `agent(action: submit, id, version)`.

## Key Rules

- `type` MUST be `"app"` in manifest.json
- `ui/index.html` MUST exist
- Sidecar must read `$NEBO_APP_SOCK` and bind a Unix socket there
- The launched sidecar binary must be a regular file — symlinks are rejected at launch (a symlinked dev binary like `bin/my-app → target/release/my-app` is fine for hot-reload detection, but the file that actually runs must resolve to a regular executable)
- Sidecar startup timeout: 10 seconds default, max 120s (set via `startup_timeout`)
- Window config accepts `title`, `width`, `height`, `resizable` (defaults: width 1024, height 768, resizable true). There are no `min_width`/`min_height` fields.
- A published app needs at least one sidecar binary for its version; submit is refused without one
