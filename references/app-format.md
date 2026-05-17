# App Format

Apps are agents with a dedicated frontend UI. They bundle a persona, an HTML frontend, and an optional native sidecar binary.

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
├── sidecar/              # Optional — native backend binary
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
  "artifact_type": "app",
  "permissions": [
    "storage:readwrite",
    "subagent:invoke",
    "network:outbound"
  ],
  "window": {
    "title": "Deal Tracker",
    "width": 1024,
    "height": 768,
    "min_width": 480,
    "min_height": 400,
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
| `artifact_type` | Must be `"app"`. |

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

Install: `npm install @neboai/app-sdk`

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
| `NEBO_APP_ID` | Agent identifier |
| `NEBO_APP_SOCK` | Unix socket path |
| `NEBO_APP_DATA` | Writable data directory |
| `NEBO_APP_DIR` | App root directory |

### Tool Discovery

Implement `GET /_tools` to expose endpoints as LLM tools:

```json
[
  {
    "name": "list_projects",
    "description": "List all projects",
    "method": "GET",
    "path": "/projects"
  }
]
```

## Publishing

```bash
neboai publish ./my-app
```

The CLI will:
1. Validate manifest.json (`artifact_type: "app"` present)
2. Validate AGENT.md and agent.json (if present)
3. Verify `ui/index.html` exists
4. Upload AGENT.md as manifest
5. Upload agent.json as config (if present)
6. Upload sidecar binary per platform (if present)
7. Submit for review

## Key Rules

- `artifact_type` MUST be `"app"` in manifest.json
- `ui/index.html` MUST exist
- Sidecar must read `$NEBO_APP_SOCK` and bind a Unix socket there
- Sidecar startup timeout: 10 seconds (max 120)
- Apps without a sidecar (pure frontend) don't need binary uploads
- Max 5MB per file in `ui/` directory for marketplace distribution
