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

```bash
pnpm add @neboai/app-sdk
```

Or use the global build (no bundler needed):

```html
<script src="https://unpkg.com/@neboai/app-sdk/dist/nebo.global.js"></script>
```

```typescript
import { nebo } from '@neboai/app-sdk';
```

### Complete SDK API

#### `nebo.configure(options)`

Override auto-detected app ID and base URL:

```typescript
nebo.configure({ appId: 'my-app', baseUrl: 'http://localhost:27895' });
```

Auto-detection works in most cases — only use this for custom setups.

#### `nebo.fetch(input, init?)`

Drop-in replacement for `window.fetch` with auto-routing:

```typescript
// Relative URLs → your sidecar API
const deals = await nebo.fetch('/deals').then(r => r.json());

// POST with body
const newDeal = await nebo.fetch('/deals', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({ name: 'Oak Street Property', amount: 450000 })
}).then(r => r.json());

// Absolute URLs → Nebo's CORS-free proxy
const weather = await nebo.fetch('https://api.weather.gov/points/40,-74')
  .then(r => r.json());
```

#### `nebo.WebSocket(path?)`

Auto-reconnecting WebSocket with exponential backoff (1s → 30s max):

```typescript
const ws = new nebo.WebSocket('/events');

ws.onopen = () => console.log('Connected');
ws.onmessage = (e) => console.log('Data:', e.data);
ws.onerror = (e) => console.error('Error:', e);
ws.onclose = (e) => console.log('Closed:', e.code);

ws.send(JSON.stringify({ subscribe: 'deals' }));
ws.close();
```

Reconnects automatically on disconnect — no manual retry logic needed.

#### `nebo.storage`

Server-persisted async key-value store (like `localStorage` but async and persistent):

```typescript
await nebo.storage.setItem('preferences', { theme: 'dark', currency: 'USD' });
const prefs = await nebo.storage.getItem('preferences');
await nebo.storage.removeItem('preferences');
const allKeys = await nebo.storage.keys();
await nebo.storage.clear();
```

#### `nebo.agents`

Invoke the app's agent programmatically:

```typescript
// One-shot call — returns full response
const { text, tools } = await nebo.agents.invoke('Summarize my open deals');

// With options
const response = await nebo.agents.invoke('Analyze this quarter', {
  agent: 'analyst',                    // Specific agent (optional)
  data: { quarter: 'Q2', year: 2026 } // Context data (optional)
});

// Streaming — yields chunks as they arrive
for await (const chunk of nebo.agents.stream('Write a detailed report')) {
  document.getElementById('output').textContent += chunk.text;
  if (chunk.done) console.log('Complete');
}
```

#### `nebo.janus`

Direct LLM completion (no agent persona, no skills — raw model access):

```typescript
// One-shot
const answer = await nebo.janus.complete({
  messages: [
    { role: 'system', content: 'You are a financial analyst.' },
    { role: 'user', content: 'Summarize this data...' }
  ],
  model: 'claude-sonnet-4-6',  // Optional
  max_tokens: 1024              // Optional
});

// Streaming
for await (const text of nebo.janus.stream({
  messages: [{ role: 'user', content: 'Explain DCF valuation' }]
})) {
  output.textContent += text;
}
```

#### `nebo.chat`

Embed a full chat UI panel via iframe:

```typescript
// Mount chat in a container
nebo.chat.mount(document.getElementById('chat-panel'), {
  placeholder: 'Ask about your deals...',
  theme: 'dark',          // 'auto' | 'light' | 'dark'
  height: '100%',         // CSS height
  borderless: true,       // No border on iframe
  contextId: currentView, // Scope conversation to context
  scope: 'read',          // Tool scope from agent.json
});

// Programmatically send a message
nebo.chat.send('Analyze deal #42 and flag risks');

// Listen for chat events
const unsub = nebo.chat.onMessage((msg) => {
  console.log('Chat event:', msg.type, msg.text);
});

// Update app context (agent sees this in its next turn)
nebo.chat.setContext({
  route: '/deals/42',
  displayedDoc: { filename: 'terms.pdf', documentId: 'doc-123' },
  attachedDocuments: [{ filename: 'comps.xlsx', documentId: 'doc-456' }],
});

// Start fresh conversation
nebo.chat.newThread();

// Remove chat panel
nebo.chat.unmount();
```

**contextId** scopes conversations. Different contexts = different chat histories. Use for:
- Per-document conversations
- Per-project analysis
- Per-view states

#### `nebo.surfaces`

Real-time agent-to-app event system. The agent pushes state changes — your UI reacts:

```typescript
nebo.surfaces.connect();

// Full state replacement
nebo.surfaces.on('state_snapshot', (e) => {
  appState = e.snapshot;
  render();
});

// Incremental updates (RFC 6902 JSON Patch)
nebo.surfaces.on('state_delta', (e) => {
  // e.delta = [{ op: 'add', path: '/deals/3', value: {...} }]
  // Auto-applied to nebo.surfaces.state
  render();
});

// Agent run lifecycle
nebo.surfaces.on('run_started', (e) => showSpinner(e.runId));
nebo.surfaces.on('run_finished', (e) => hideSpinner(e.runId));
nebo.surfaces.on('run_error', (e) => showError(e.message));

// Streaming text from agent
nebo.surfaces.on('text_start', (e) => startMessage(e.messageId));
nebo.surfaces.on('text_content', (e) => appendText(e.messageId, e.delta));
nebo.surfaces.on('text_end', (e) => finalizeMessage(e.messageId));

// Tool execution
nebo.surfaces.on('tool_call_start', (e) => showToolRunning(e.toolName));
nebo.surfaces.on('tool_call_end', (e) => showToolResult(e.toolCallId, e.result));

// A2UI component surfaces
nebo.surfaces.on('surface_create', (e) => renderSurface(e.surfaceId, e.components));
nebo.surfaces.on('surface_update', (e) => updateSurface(e.surfaceId, e.components));
nebo.surfaces.on('surface_delete', (e) => removeSurface(e.surfaceId));

// Data model updates
nebo.surfaces.on('data_update', (e) => updateData(e.path, e.value));

// Custom app-specific events
nebo.surfaces.on('custom', (e) => handleCustom(e.name, e.value));

// Wildcard — listen to everything
nebo.surfaces.on('*', (e) => console.log('Event:', e.type, e));

// Send action back to agent
nebo.surfaces.send('approve_deal', { dealId: '42' });

// Request full state
nebo.surfaces.requestState();

// Disconnect when done
nebo.surfaces.disconnect();

// Unsubscribe from specific event
const unsub = nebo.surfaces.on('state_snapshot', handler);
unsub(); // Stop listening
```

**All surface event types:**

| Event | Data | When |
|-------|------|------|
| `run_started` | `runId, threadId?` | Agent begins processing |
| `run_finished` | `runId` | Agent completes |
| `run_error` | `runId, message, code?` | Agent errors |
| `text_start` | `messageId` | Agent begins streaming text |
| `text_content` | `messageId, delta` | Text chunk arrives |
| `text_end` | `messageId` | Text stream complete |
| `tool_call_start` | `toolCallId, toolName` | Agent calls a tool |
| `tool_call_end` | `toolCallId, result?` | Tool returns |
| `state_snapshot` | `snapshot` | Full state replacement |
| `state_delta` | `delta` (JSON Patch ops) | Incremental state update |
| `surface_create` | `surfaceId, components, data?` | New UI surface |
| `surface_update` | `surfaceId, components?, data?` | Surface changed |
| `surface_delete` | `surfaceId` | Surface removed |
| `data_update` | `surfaceId?, path?, value` | Data model changed |
| `custom` | `name, value` | App-specific event |

#### `nebo.identity`

Agent metadata (cached after first call):

```typescript
const me = await nebo.identity.get();
// { id, name, displayName, description, persona, model, skills, inputValues }

nebo.identity.invalidate(); // Clear cache, re-fetch on next get()
```

#### `nebo.a2ui`

A2UI v0.9 message bridge for agent-driven UI components. Use with `@a2ui/web_core`:

```typescript
import { createMessageProcessor } from '@a2ui/web_core/v0_9';

const processor = createMessageProcessor(container);
nebo.a2ui.init(processor);
nebo.surfaces.connect(); // A2UI messages flow through surfaces

// Send UI action to agent
nebo.a2ui.sendAction('surface-1', { type: 'click', target: 'approve-btn' });

// Report error
nebo.a2ui.sendError('surface-1', 'VALIDATION', 'Amount must be positive');
```

### UI Principles

1. **Dark theme default.** Nebo's shell is dark. Match it.
2. **No heavy frameworks required.** Vanilla JS works great. React/Vue/Svelte if you want.
3. **Responsive to window resize.** Users drag the window — handle it.
4. **Loading states.** Show skeleton/spinner while sidecar responds.
5. **Error states.** Show clear messages when things fail. Don't blank screen.

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
| Not using `nebo.surfaces` for state | UI and agent get out of sync |
| Missing `nebo.WebSocket` for real-time | Polling instead of streaming |
