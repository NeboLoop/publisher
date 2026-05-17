# Building Apps

How to architect, design, and build Nebo apps — agents with dedicated UIs that open in their own window.

## When to Build an App

| User Need | Build | Why |
|-----------|-------|-----|
| Visual dashboard with live data | App | Chat can't render charts/tables well |
| CRUD interface (contacts, deals, projects) | App | Users need to browse, search, edit |
| Document viewer with annotations | App | Needs spatial layout |
| Multi-step wizard with forms | App | Too complex for chat flow |
| Real-time monitoring | App | Needs continuous visual updates |
| Simple Q&A or analysis | Skill/Agent | Chat output is sufficient |

**Rule of thumb:** If the user needs to *see and interact with* a persistent visual interface, build an app. If chat bubbles handle it, write a skill or agent.

## Architecture

```
┌─────────────────────────────────────────────┐
│                 User's Browser               │
│  ┌─────────────────┐  ┌──────────────────┐  │
│  │   UI (HTML/JS)  │  │   Chat Panel     │  │
│  │   @neboai/      │  │   (built-in)     │  │
│  │   app-sdk       │  │                  │  │
│  └────────┬────────┘  └────────┬─────────┘  │
│           │                     │            │
└───────────┼─────────────────────┼────────────┘
            │                     │
            ▼                     ▼
┌───────────────────────────────────────────────┐
│              Nebo Server                       │
│  ┌──────────┐  ┌──────────┐  ┌────────────┐  │
│  │ HTTP     │  │ Agent    │  │ Storage    │  │
│  │ Proxy    │  │ Runtime  │  │ (KV, DB)   │  │
│  └────┬─────┘  └──────────┘  └────────────┘  │
│       │                                       │
└───────┼───────────────────────────────────────┘
        │ gRPC over Unix socket
        ▼
┌────────────────┐
│   Sidecar      │
│   (your Rust   │
│    binary)     │
│                │
│   Data Store   │
└────────────────┘
```

### Three Layers

1. **Frontend** (`ui/`) — Static HTML/JS/CSS. Uses `@neboai/app-sdk` for all platform communication.
2. **Agent** (`AGENT.md`) — Persona powering the chat panel. Reads your skills to know how to use tools.
3. **Sidecar** (optional) — Native binary handling data, computation, external APIs. Communicates via gRPC.

### Decision: With or Without Sidecar

| Approach | When |
|----------|------|
| No sidecar | Simple UIs, all data via `nebo.storage` or `nebo.agents.invoke()` |
| With sidecar | Complex data models, external API integration, heavy computation, SQLite |

**Start without a sidecar.** Add one when `nebo.storage` and `nebo.agents.invoke()` aren't enough.

## Frontend Development

### SDK Setup

```typescript
import { nebo } from '@neboai/app-sdk';
```

The SDK provides:

| API | Use For |
|-----|---------|
| `nebo.identity.get()` | Agent ID, name, version |
| `nebo.storage.setItem/getItem` | Simple KV persistence |
| `nebo.chat.mount(el, opts)` | Embed chat panel |
| `nebo.chat.send(msg)` | Programmatically send to agent |
| `nebo.agents.invoke(prompt)` | One-shot agent call (returns text) |
| `nebo.agents.stream(prompt)` | Streaming agent call |
| `nebo.janus.complete(opts)` | Direct LLM call (no persona) |
| `nebo.fetch(path, opts)` | HTTP to sidecar (relative) or external (absolute) |
| `surfaces.on(event, handler)` | React to agent state changes |

### UI Principles

1. **Dark theme default.** Nebo's shell is dark. Match it.
2. **No heavy frameworks required.** Vanilla JS works great. React/Vue/Svelte if you want.
3. **Responsive to window resize.** Users drag the window — handle it.
4. **Loading states.** Show skeleton/spinner while sidecar responds.
5. **Error states.** Show clear messages when things fail. Don't blank screen.

### Chat Integration

The chat panel is your agent's voice. Integrate it naturally:

```typescript
// Mount chat in a sidebar
nebo.chat.mount(document.getElementById('chat'), {
  placeholder: 'Ask about your deals...',
  theme: 'dark',
  contextId: currentView.id,  // Scopes conversation to current context
});

// Trigger agent from UI actions
document.getElementById('analyze-btn').onclick = () => {
  nebo.chat.send(`Analyze deal ${currentDeal.id} and flag risks`);
};
```

**contextId** scopes conversations. Different contexts = different chat histories. Use this for:
- Per-document conversations
- Per-project analysis
- Per-view states

### State Management with Surfaces

Listen for agent-pushed state:

```typescript
import { surfaces } from '@neboai/app-sdk';

surfaces.connect();

// Full state replacement
surfaces.on('state_snapshot', (e) => {
  appState = e.snapshot;
  render();
});

// Incremental updates (RFC 6902 JSON Patch)
surfaces.on('state_delta', (e) => {
  // Auto-applied to surfaces.state
  render();
});

// Agent created a new UI surface
surfaces.on('surface_create', (e) => {
  console.log('New surface:', e.surfaceId, e.components);
});
```

**Pattern:** Agent does work → pushes state via snapshot/delta → frontend re-renders.

This means the agent can update your UI without the user asking. Useful for:
- Background processing completion
- Real-time data updates
- Multi-step workflow progress

### Using nebo.fetch for Sidecar Communication

```typescript
// All relative URLs go to your sidecar
const deals = await nebo.fetch('/deals').then(r => r.json());

// POST with body
const newDeal = await nebo.fetch('/deals', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({ name: 'Oak Street Property', amount: 450000 })
}).then(r => r.json());

// Absolute URLs go through Nebo's CORS-free proxy
const weather = await nebo.fetch('https://api.weather.gov/points/40,-74')
  .then(r => r.json());
```

## Sidecar Development

### Language Choice

**Always use Rust for sidecars.** Same reasons as plugins:
- Static binary, no runtime deps
- No AV false positives
- Agent cannot modify the binary at runtime
- Fast startup (critical — 10s timeout)
- Memory-safe with excellent concurrency

### When You Need One

- Structured data storage (SQLite, multiple tables, queries)
- External API integration with complex auth
- Heavy computation (PDF processing, data analysis)
- Custom business logic that shouldn't be in the LLM

### Implementation (Rust)

```rust
use tonic::{transport::Server, Request, Response, Status};
use tokio::net::UnixListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let sock = std::env::var("NEBO_APP_SOCK")?;
    let data_dir = std::env::var("NEBO_APP_DATA")?;

    let _ = std::fs::remove_file(&sock);

    let service = MyAppService::new(&data_dir).await?;
    let uds = UnixListener::bind(&sock)?;
    let stream = tokio_stream::wrappers::UnixListenerStream::new(uds);

    Server::builder()
        .add_service(UiServiceServer::new(service))
        .serve_with_incoming(stream)
        .await?;

    Ok(())
}
```

### Request Routing

```rust
async fn handle_http(&self, method: &str, path: &str, body: &[u8]) -> HttpResponse {
    let parts: Vec<&str> = path.trim_start_matches('/').split('/').collect();

    match (method, parts.as_slice()) {
        ("GET", ["deals"]) => self.list_deals().await,
        ("POST", ["deals"]) => self.create_deal(body).await,
        ("GET", ["deals", id]) => self.get_deal(id).await,
        ("PUT", ["deals", id]) => self.update_deal(id, body).await,
        ("DELETE", ["deals", id]) => self.delete_deal(id).await,
        ("GET", ["_tools"]) => self.get_tools(),
        _ => json_response(404, &json!({"error": "not found"})),
    }
}
```

### Tool Discovery

Implement `GET /_tools` so the agent can call your sidecar directly:

```rust
("GET", ["_tools"]) => {
    json_response(200, &json!([
        {
            "name": "list_deals",
            "description": "List all deals, optionally filtered by stage",
            "method": "GET",
            "path": "/deals",
            "input_schema": {
                "type": "object",
                "properties": {
                    "stage": { "type": "string", "enum": ["prospect", "analysis", "negotiation", "closed"] }
                }
            }
        },
        {
            "name": "create_deal",
            "description": "Create a new deal in the pipeline",
            "method": "POST",
            "path": "/deals",
            "input_schema": {
                "type": "object",
                "properties": {
                    "name": { "type": "string" },
                    "amount": { "type": "number" },
                    "stage": { "type": "string", "default": "prospect" }
                },
                "required": ["name", "amount"]
            }
        }
    ]))
}
```

### Data Persistence

Use `$NEBO_APP_DATA` for all persistent storage:

```rust
let db_path = format!("{}/deals.db", data_dir);
let conn = rusqlite::Connection::open(&db_path)?;

conn.execute_batch("
    CREATE TABLE IF NOT EXISTS deals (
        id TEXT PRIMARY KEY,
        name TEXT NOT NULL,
        amount REAL NOT NULL,
        stage TEXT DEFAULT 'prospect',
        created_at TEXT DEFAULT (datetime('now'))
    );
")?;
```

**Rules:**
- All data goes in `$NEBO_APP_DATA` — nowhere else
- Data survives restarts, updates, and Nebo upgrades
- Use SQLite for anything more complex than a single JSON file
- Handle concurrent access (SQLite WAL mode)

### Startup Requirements

1. Read `$NEBO_APP_SOCK` — bind your Unix socket here
2. Read `$NEBO_APP_DATA` — your writable data directory
3. Create socket within 10 seconds (startup timeout)
4. Respond to `HealthCheck` RPC immediately
5. Handle `SIGTERM` — flush data, close connections, exit cleanly

### Binary Hot-Reload

During development, Nebo watches your binary. When it changes:
1. SIGTERM → old process
2. Wait for exit
3. Start new process
4. Re-discover tools

**Dev workflow:** Rebuild your binary → Nebo auto-restarts it. No manual intervention.

Symlinks work: `bin/my-app → sidecar/target/release/my-app`

## Skills for Apps

Bundle skills that teach the agent how to use your sidecar tools:

```
skills/
  workspace-mgmt/
    SKILL.md     # When to list/create/update deals
  analysis/
    SKILL.md     # When to run financial analysis
```

**Critical:** Tools give the agent the *ability*. Skills give it *judgment*. Without skills, the agent has `create_deal` but doesn't know when to use it, what to ask the user first, or how to validate inputs.

## manifest.json

```json
{
  "id": "deal-tracker",
  "name": "@acme/agents/deal-tracker",
  "version": "1.0.0",
  "description": "Track real estate deals with AI-powered analysis.",
  "artifact_type": "app",
  "permissions": ["storage:readwrite", "subagent:invoke"],
  "window": {
    "title": "Deal Tracker",
    "width": 1024,
    "height": 768,
    "min_width": 480,
    "resizable": true
  }
}
```

**Permission principle:** Request minimum. Don't ask for `network:outbound` if you only use `nebo.storage`.

## Testing

- [ ] `ui/index.html` loads without errors in browser
- [ ] Chat panel mounts and agent responds
- [ ] `nebo.fetch('/...')` reaches sidecar and returns data
- [ ] Sidecar starts within 10 seconds
- [ ] Sidecar handles SIGTERM gracefully
- [ ] Data persists across sidecar restarts
- [ ] `GET /_tools` returns valid tool definitions
- [ ] Agent can call sidecar tools via LLM reasoning
- [ ] Window resizes without layout breaks
- [ ] Loading states shown during async operations
- [ ] Error states shown when sidecar is unreachable

## Anti-Patterns

| Anti-Pattern | Fix |
|-------------|-----|
| All logic in the agent (no sidecar) | Move data/computation to sidecar, let agent reason |
| All logic in the sidecar (no agent) | Use the agent for judgment, user interaction, summarization |
| Frontend calls external APIs directly | Use `nebo.fetch` with absolute URLs (CORS-free proxy) |
| No loading states | Users think it's broken during async ops |
| Ignoring `contextId` | All conversations mix together |
| Giant monolithic sidecar | Split into route modules |
| No tool discovery (`/_tools`) | Agent can't call your sidecar during reasoning |
| No bundled skills | Agent doesn't know *when* to use tools |
| Sidecar stores data outside `$NEBO_APP_DATA` | Data lost on reinstall |
| Binary takes >10s to start | Startup timeout → launch failure |
