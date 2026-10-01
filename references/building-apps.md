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

1. **Frontend** (`ui/`) — Static HTML/JS/CSS. Uses the app SDK (`window.NeboAppSDK`, loaded from `/sdk/nebo.global.js`) for all platform communication.
2. **Agent** (`AGENT.md`) — Persona powering the chat panel. Reads your skills to know how to use tools.
3. **Sidecar** (optional) — Native binary handling data, computation, external APIs. Communicates via gRPC.

### Decision: With or Without Sidecar

| Approach | When |
|----------|------|
| No sidecar | Simple UIs, all data via `nebo.storage` or `nebo.agents.invoke()` |
| With sidecar | Complex data models, external API integration, heavy computation, SQLite |

**Start without a sidecar.** Add one when `nebo.storage` and `nebo.agents.invoke()` aren't enough. A page-only app publishes as one bundle (AGENT.md, agent.json, manifest.json and `ui/`) and needs no binary at all; see [app-format.md](app-format.md#publishing).

## Frontend Development

### SDK Setup

Load the SDK Nebo serves (it always matches the running Nebo) and read the one global it defines. There is no bare `nebo` global:

```html
<script src="/sdk/nebo.global.js"></script>
<script>
  const { nebo } = window.NeboAppSDK;
</script>
```

For a bundled build, `pnpm add @neboai/app-sdk` gives the same API as an import (`import { nebo } from '@neboai/app-sdk'`). An unbundled page cannot import a bare package name.

### Building the page

`ui/` is served exactly as it is on disk, so any static build works. Keep the source (`src/`, `package.json`) beside `ui/` and build into it, for example with bun:

```bash
bun build ./src/main.js --outdir ./ui --entry-naming "[name]-[hash].[ext]" --minify
```

Content-hashed names (`main-0a8ksftt.js`) are cached for a year; every other file is revalidated on each open, so a rebuild with hashed names shows at once. Never rename files by hand to get past a cache.

A page may carry images, fonts, video (`mp4 webm mov`), sound (`mp3 wav ogg m4a`), `wasm` and 3D models (`glb gltf`); video and sound answer range requests, so seeking works. Limits: 10 MB per file, 50 MB in total.

The full list of types, the caching rule and how range requests are answered are in [app-format.md](app-format.md#the-page-ui).

### Games, films and the phone

- **Full screen.** `"window": { "fullscreen": true, "orientation": "landscape" }` opens the app over the whole screen: a full-screen window on desktop; on the phone no app bar, system bars hidden, the screen kept awake, no pull-to-refresh, the edge swipe back turned off, and a small Close pill in the top-left corner that fades after a few seconds. Keep your own controls clear of that corner and pad with `env(safe-area-inset-*)` and `viewport-fit=cover`. `orientation` is `portrait` (default), `landscape` or `any`; any other value is refused when the manifest is written.
- **Tilt.** Add `device:motion` to `permissions`, then read `devicemotion` / `deviceorientation`. On iPhone, call `DeviceMotionEvent.requestPermission()` from a tap first.
- **Video.** `<video muted playsinline>` plays in place on the phone (add `autoplay loop` for a loop). For a film that scrubs with the scroll, encode every frame as a keyframe (`ffmpeg ... -g 1 -keyint_min 1 -sc_threshold 0 -movflags +faststart -an`), call `load()` and prime it with one muted `play()` then `pause()` before the first seek, and start a new seek only after the previous `seeked` event, always toward the newest target.
- **Scrolling pages.** `html, body { touch-action: pan-y; overscroll-behavior-x: none; }` so nothing pans sideways; a game's play area uses `touch-action: none`.
- **Sound.** Browsers start audio only after a first tap. Ask for sound and tilt on the same first tap.
- **Voice.** Every app opened in the Nebo desktop app or on the phone shows a small voice control in its bottom-right corner, so the owner can talk to the app's employee while it works on the page. The control sits outside your page, so reloading the page never drops the call. On desktop it is a bar about 300 by 48 pixels, 16 pixels in from the window's bottom-right corner. On the phone it is a round 48-point button inside the safe area that the owner can drag to either side; in a full-screen app it folds back to that button a few seconds into a call. Keep important controls clear of the bottom-right corner. During a long task on a call, the employee says short updates such as "Now editing." and never reads out file names or commands.

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

#### `nebo.WebSocket()`

Auto-reconnecting WebSocket to the app's agent (`/ws/app/<id>`), with exponential backoff (1s → 30s max). It takes no arguments:

```typescript
const ws = new nebo.WebSocket();

ws.onopen = () => console.log('Connected');
ws.onmessage = (e) => console.log('Data:', e.data);
ws.onerror = (e) => console.error('Error:', e);
ws.onclose = (e) => console.log('Closed:', e.code);

ws.send(JSON.stringify({ subscribe: 'deals' }));
ws.close();
```

Reconnects automatically on disconnect — no manual retry logic needed.

To talk to a server of your own (a multiplayer game server, a live feed), open a plain `new WebSocket('wss://...')` straight from the page; app pages carry no content security policy that blocks it.

#### `nebo.storage`

Server-persisted async key-value store (like `localStorage` but async and persistent):

```typescript
await nebo.storage.setItem('preferences', { theme: 'dark', currency: 'USD' });
const prefs = await nebo.storage.getItem('preferences'); // the same object back
await nebo.storage.removeItem('preferences');
const allKeys = await nebo.storage.keys();
await nebo.storage.clear();

// Redraw when the data changes, whoever changed it
const stop = nebo.storage.onChange(({ keys, action, source }) => render());
```

`getItem` returns exactly what `setItem` stored, or `null`. See "Your App's Data" below for how the page and the app's employee share this store.

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
  max_tokens: 1024  // Optional
});

// Streaming
for await (const text of nebo.janus.stream({
  messages: [{ role: 'user', content: 'Explain DCF valuation' }]
})) {
  output.textContent += text;
}
```

#### `nebo.decide`

Typed decisions: named questions about some data, each answered with probabilities and a confidence in one fast call. Nothing is written as text, so use it for judgments (is this lead hot, which category, how urgent) and keep counting, dates and thresholds in your own code.

```typescript
const { answers } = await nebo.decide({
  state: { company: 'Example Co', status: 'asked for a quote today' },
  questions: {
    tier: {
      type: 'choice',
      instructions: 'How warm is this lead, judging by `status`?',
      criteria: { hot: 'ready to buy', warm: 'interested', cold: 'not now', other: "can't tell" }
    },
    fit: {
      type: 'score',
      instructions: 'How well does `company` fit our customers?',
      criteria: ['poor', 'fair', 'good', 'great']
    },
    reply: { type: 'noul', instructions: '`status` asks us for a reply.' }
  }
});

if (answers.tier.choice === 'hot' && answers.tier.confidence > 0.8) flagLead();
```

| Type | `criteria` | Answer fields |
|------|------------|---------------|
| `choice` | `{option: description}`, 2 to 255 options. Add an escape option (`other`) when the list is not complete | `choice`, `confidence` (0 to 1), `probabilities` per option |
| `score` | `[levels]`, 2 to 10, lowest first | `score` (fractional: 0 is the first level, 1.5 is between the second and third), `confidence`, `probabilities` |
| `noul` | none: the question is one statement | `noul`, the probability the statement holds (no separate `confidence`) |

- Resolves to `{ model, answers, usage }`; each answer comes back under its question's name.
- `state` is text or any JSON. Keep it to the fields the questions need and name them in backticks inside `instructions`. The whole question lives in `instructions`; the name only labels the answer. Very long state is shortened in the middle before it is sent.
- Throws with the reason when a question is malformed (a choice with one option, a `noul` with criteria, a missing `instructions`), when the bot is not signed in to NeboAI, or when the decision cannot be made.
- Billed to the bot owner's NeboAI account like any model call. See pricing at https://neboai.com/pricing.

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

> **What Nebo sends today.** App pages receive interactive cards from the app's employee (through `nebo.a2ui`, which needs `nebo.surfaces.connect()`) and storage changes (through `nebo.storage.onChange`). The typed events below are part of the SDK, but Nebo does not send them to app pages yet, and nothing answers `send()` or `requestState()`. Use `storage.onChange` for live data and `nebo.a2ui` for cards. A card reaches only the app it was made for, a click on it goes to that app's employee, and a page opened after the card was sent does not receive it. The SDK has no card renderer of its own: bundle `@a2ui/web_core` and pass its MessageProcessor to `nebo.a2ui.init()`.

Event system (reserved for future use):

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

### Your App's Data

The page's `nebo.storage` and the app's employee read and write **one store**: the same keys and the same values. A contact the owner adds by saying "add John Smith, 555 0100" to the app's employee is the contact the page shows, and a contact typed into the page is one the employee can find when someone asks "what's John's number?".

**What the employee can do.** The app's employee has a built-in tool for its own app's data. It can read a key, save any JSON value under a key, delete a key, list keys (optionally by prefix), and search. A search matches fields by text, any case (`name` contains "john smith"), reaches nested fields (`phone.mobile`), and looks inside a key that holds a list item by item. You do not declare or install anything for this; every app's employee has it.

**Who can reach it.** Only the app's own employee, and only for its own app. Another employee that needs the data asks the app's employee ("give me John Smith's number") and gets the answer back. Other apps never see your store.

**Keeping the page in step.** Every save or delete, by the employee or by any open window of the app, is reported to every open window through `nebo.storage.onChange`. Write the page so it redraws:

```typescript
async function load() {
  const contacts = (await nebo.storage.getItem('contacts')) ?? [];
  renderContacts(contacts);
}

load();
nebo.storage.onChange((change) => {
  // change = { appId, keys: ['contacts'], action: 'set' | 'delete', source: 'employee' | 'page' }
  if (change.keys.includes('contacts')) load();
});
```

Your page's own writes are reported too, so the handler above also covers a second window of the same app.

**Choosing keys.** Pick a shape both sides can find:

| Shape | Example | Good for |
|-------|---------|----------|
| One key holding a list | `contacts` = `[{ name, phone }, ...]` | Small collections the page draws all at once |
| One key per record, shared prefix | `contact:42` = `{ name, phone }` | Larger collections, records changed one at a time |
| One key per setting | `settings` = `{ theme, sound }` | Preferences and saves |

Tell the employee about your keys in its instructions (AGENT.md), for example: "Contacts are a list under the key `contacts`, each `{ name, phone: { mobile, work }, email }`."

**Judging the data.** The app's employee also has a `decide` tool that takes the same request as `nebo.decide`, so it can read records with its data tool and then ask typed questions about them ("which of these leads are hot?") without writing prose. The page can do the same with `nebo.decide`.

**Things to know.**
- A string that is itself valid JSON, such as `"42"` or `"true"`, comes back parsed (`42`, `true`). Store it inside an object (`{ "code": "42" }`) if the type matters.
- There is no per-key or per-app quota, but each write is one request: keep each value well under 2 MB. Large or relational data belongs in a sidecar with its own database.
- `setItem` and `removeItem` do not throw when a write is refused. Read back anything that must not be lost.

### UI Principles

1. **Dark theme default.** Nebo's shell is dark. Match it.
2. **No heavy frameworks required.** Vanilla JS works great. React/Vue/Svelte if you want.
3. **Responsive to window resize.** Users drag the window — handle it.
4. **Loading states.** Show skeleton/spinner while data loads.
5. **Error states.** Show clear messages when things fail. Don't blank screen.
6. **Phones.** A scrolling page pans vertically only (`html, body { touch-action: pan-y; overscroll-behavior-x: none; }`) and nothing is wider than the screen. Use `dvh` units, not `100vh`.

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
    let data_dir = std::env::var("NEBO_DATA_DIR")?;

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

The sidecar implements the `UIService` gRPC service. The `HandleRequest` RPC receives HTTP-shaped requests (method, path, headers, body) from the Nebo proxy:

```rust
use tonic::{Request, Response, Status};

#[tonic::async_trait]
impl UiService for MyAppService {
    async fn handle_request(
        &self,
        request: Request<HttpRequest>,
    ) -> Result<Response<HttpResponse>, Status> {
        let req = request.into_inner();
        let parts: Vec<&str> = req.path.trim_start_matches('/').split('/').collect();

        match (req.method.as_str(), parts.as_slice()) {
            ("GET", ["deals"]) => self.list_deals().await,
            ("POST", ["deals"]) => self.create_deal(&req.body).await,
            ("GET", ["deals", id]) => self.get_deal(id).await,
            ("PUT", ["deals", id]) => self.update_deal(id, &req.body).await,
            ("DELETE", ["deals", id]) => self.delete_deal(id).await,
            _ => Ok(Response::new(HttpResponse {
                status: 404,
                body: serde_json::to_vec(&json!({"error": "not found"}))?,
                ..Default::default()
            })),
        }
    }
}
```

### Tool Definitions

Tools are defined in the `tools` array in `agent.json`, not discovered at runtime. The agent reads tool definitions from the filesystem:

```json
// agent.json
{
  "tools": [
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
  ]
}
```

### Data Persistence

Use `$NEBO_DATA_DIR` for all persistent storage. It's the one data-dir variable
for every artifact type (plugins, apps, skills) → `<NEBO_HOME>/appdata/<type>/<slug>/`,
which Nebo **never** touches on update — you own your schema and migrate it across
versions. Your process also *runs* in this directory, so a relative path
(`./deals.db`) lands here too; never write into `$NEBO_APP_DIR` (your versioned
code, wiped on update).

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
- All data goes in `$NEBO_DATA_DIR` — nowhere else
- Data survives restarts, updates, and Nebo upgrades
- Use SQLite for anything more complex than a single JSON file
- Handle concurrent access (SQLite WAL mode)

### Environment Variables

| Variable | Description |
|----------|-------------|
| `NEBO_APP_SOCK` | Unix socket path — bind your gRPC server here |
| `NEBO_DATA_DIR` | Writable data directory for persistent storage |
| `NEBO_APP_TOKEN` | Per-launch auth token for callbacks to Nebo |
| `NEBO_API_URL` | Callback URL to Nebo's HTTP API |
| `NEBO_APP_ID` | App identifier |
| `NEBO_APP_NAME` | App display name |
| `NEBO_APP_VERSION` | App version string |
| `NEBO_APP_DIR` | App root directory on disk |

The environment is sanitized: only the vars above plus allowlisted system vars (`PATH`, `HOME`, `TMPDIR`, `LANG`, `LC_ALL`, `TZ`) reach the sidecar. Secrets in the host environment are stripped.

### Binary Discovery

The runtime searches for the sidecar binary in these locations (first match wins):

1. A file named `binary` at the app root
2. A file named `app` at the app root
3. First file in the `tmp/` directory
4. First file in the `bin/` directory
5. First extensionless executable in `sidecar/target/release/`

### Startup Requirements

1. Read `$NEBO_APP_SOCK` — bind your Unix socket here
2. Read `$NEBO_DATA_DIR` — your writable data directory
3. Create socket within 10 seconds (default startup timeout, configurable up to 120s via `manifest.startup_timeout`)
4. Respond to `HealthCheck` RPC immediately
5. Handle `SIGTERM` — flush data, close connections, exit cleanly

### Binary Hot-Reload

During development, Nebo watches your binary. When it changes:
1. SIGTERM → old process
2. Wait for exit
3. Start new process
4. Re-discover tools

**Dev workflow:** Rebuild your binary → Nebo auto-restarts it. No manual intervention.

The change watcher resolves through symlinks, so a dev layout like `bin/my-app → sidecar/target/release/my-app` is detected when the underlying target is rebuilt. Note that the binary that actually launches must resolve to a **regular file** — `validate_binary` rejects a symlinked binary at launch time.

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
  "type": "app",
  "permissions": ["storage:readwrite"],
  "window": {
    "title": "Deal Tracker",
    "width": 1024,
    "height": 768,
    "resizable": true
  }
}
```

**Permission principle:** Request minimum. Don't ask for `network:outbound` if you only use `nebo.storage`.

## Testing

- [ ] `ui/index.html` loads without errors in browser
- [ ] The page reads the SDK from `window.NeboAppSDK` (loads `/sdk/nebo.global.js`)
- [ ] Chat panel mounts and agent responds
- [ ] State survives closing and reopening the window (`nebo.storage`)
- [ ] AGENT.md frontmatter has `artifact_type: app`, and every value containing `: ` is quoted
- [ ] Every file in `ui/` is at most 10 MB and the whole `ui/` at most 50 MB
- [ ] Video plays and seeks on a phone (`muted playsinline`), and nothing pans sideways
- [ ] A full-screen app keeps its controls clear of the top-left Close pill
- [ ] `nebo.fetch('/...')` reaches sidecar and returns data (sidecar apps)
- [ ] Sidecar starts within 10 seconds
- [ ] Sidecar handles SIGTERM gracefully
- [ ] Data persists across sidecar restarts
- [ ] `agent.json` contains valid tool definitions in `tools` array
- [ ] Agent can call sidecar tools via LLM reasoning
- [ ] Window resizes without layout breaks
- [ ] Loading states shown during async operations
- [ ] Error states shown when sidecar is unreachable

## Anti-Patterns

| Anti-Pattern | Fix |
|-------------|-----|
| Heavy data model squeezed into `nebo.storage` | Move data/computation to a sidecar, let agent reason |
| All logic in the sidecar (no agent) | Use the agent for judgment, user interaction, summarization |
| Frontend calls external APIs directly | Use `nebo.fetch` with absolute URLs (CORS-free proxy) |
| No loading states | Users think it's broken during async ops |
| Ignoring `contextId` | All conversations mix together |
| Giant monolithic sidecar | Split into route modules |
| No tool definitions in `agent.json` | Agent can't call your sidecar during reasoning |
| No bundled skills | Agent doesn't know *when* to use tools |
| Sidecar stores data outside `$NEBO_DATA_DIR` | Data lost on reinstall |
| Binary takes >10s to start | Startup timeout → launch failure. Increase via `manifest.startup_timeout` (max 120s) |
| Page never redraws after the employee changes data | Listen with `nebo.storage.onChange` and reload what the page shows |
| Page built on `nebo.surfaces` events like `state_snapshot` | Nebo does not send them yet; use `nebo.storage.onChange` and `nebo.a2ui` |
| Missing `nebo.WebSocket` for real-time | Polling instead of streaming |
| Unquoted `: ` in AGENT.md frontmatter | The marketplace cannot read it and publishes a plain agent; quote the value |
| Page written against a bare `nebo` global | `ReferenceError`; read `window.NeboAppSDK.nebo` |
