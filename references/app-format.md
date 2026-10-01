# App Format

Apps are agents with a dedicated frontend UI: a persona plus a page that opens in its own window. Most apps are **page-only**: AGENT.md, manifest.json, agent.json and a static `ui/` folder, published as one bundle. A native **sidecar** binary is optional, for apps that need a backend of their own (SQLite, heavy computation, complex external APIs).

## Directory Structure

```
my-app/
├── AGENT.md              # Required: persona; frontmatter says artifact_type: app
├── manifest.json         # Required: identity, "type": "app", permissions, window
├── agent.json            # Optional: workflows, skills, user inputs ({} is fine)
├── ui/                   # Required: the page, exactly as served
│   ├── index.html        #   Entry point
│   ├── style.css
│   ├── app.js
│   └── assets/           #   images, video, sound, 3D models, fonts
├── skills/               # Optional: skill docs for the agent
│   └── workspace-mgmt/
│       └── SKILL.md
├── sidecar/              # Optional: native backend (build it, or put per-platform builds in dist/app/<platform>/)
│   ├── Cargo.toml
│   ├── src/main.rs
│   └── target/release/
│       └── my-app-sidecar
└── data/                 # Auto-created at runtime
```

Only AGENT.md, agent.json, manifest.json, `ui/` and `skills/` are published for a page-only app. Source you build from (`src/`, `package.json`, `node_modules/`) stays on your machine; build into `ui/`.

### Skills inside an app or employee bundle

Each folder under `skills/<name>/` is checked by the same rules as a skill uploaded on its own, judged inside its folder:

- its `SKILL.md` is kept (the installed employee loads it from `skills/<name>/SKILL.md`);
- files under its `scripts/` and `bin/` may be any type;
- every other file must be an allowed skill file type (`md json yaml yml txt csv toml cfg html css js ts py sh ps1 rb sql`, images, fonts, `pdf`), and reserved names such as `AGENT.md`, `agent.json` or `manifest.json` inside the folder are dropped.

The 10 MB per file and 50 MB per bundle limits cover these files too.

## AGENT.md

```markdown
---
name: deal-tracker
description: "Track deals through your pipeline: stages, amounts, and a chat that knows the deal in front of you."
artifact_type: app
metadata:
  version: "1.0.0"
---
# Deal Tracker

You are a deal analyst embedded in a visual pipeline app.
```

- **`artifact_type: app`** in the frontmatter (top level or under `metadata:`) is what makes the marketplace create an app. Without it the item becomes a plain agent with no page.
- **Quote any value that contains `: `** (a colon followed by a space). `description: Track deals: fast` is invalid YAML; the marketplace then cannot read the frontmatter and silently treats the app as a plain agent. Quoting every description avoids it.

## manifest.json

```json
{
  "id": "deal-tracker",
  "name": "@acme/agents/deal-tracker",
  "version": "1.0.0",
  "description": "Track deals through your pipeline.",
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

## Required manifest.json Fields

| Field | Description |
|-------|-------------|
| `id` | Unique identifier. Must match directory name. |
| `name` | Qualified name (`@org/agents/name`). |
| `version` | Semantic version. |
| `type` | Must be `"app"`. The JSON key is `type`, never `artifact_type`. |

## Window

| Key | Default | Meaning |
|-----|---------|---------|
| `title` | the agent's name | Window title |
| `width`, `height` | 1024, 768 | Desktop window size |
| `resizable` | `true` | Whether the window can be resized |
| `fullscreen` | `false` | Take the whole screen (games, films). On the phone: no app bar, system bars hidden, screen kept awake, pull-to-refresh off. Pad the page with `env(safe-area-inset-*)` and `viewport-fit=cover`. |
| `orientation` | `"portrait"` | `"portrait"`, `"landscape"` or `"any"` on the phone |

There are no `min_width` / `min_height` fields.

## Permissions

Every permission is `prefix:scope`; a permission with an unknown prefix is refused. Ask for the least the app needs.

| Permission | What It Grants |
|------------|---------------|
| `storage:readwrite` | Scoped KV store (`nebo.storage`) |
| `subagent:<agent-id>` | Invoke another agent from the page (the app's own agent needs none) |
| `network:<host>` / `network:*` | HTTP requests through Nebo's proxy (`nebo.fetch` with an absolute URL) |
| `device:motion` | Gyroscope and accelerometer (`devicemotion`, `deviceorientation`). On iPhone the page also calls `DeviceMotionEvent.requestPermission()` from a tap. |
| `filesystem:read` | Read user files |
| `shell:execute` | Run shell commands |
| `memory:read` | Read agent memories |
| `oauth:google` | Google OAuth flow |

## The Page (`ui/`)

- Served at `/apps/<id>/ui/`; `ui/index.html` is the entry. A path with no matching file falls back to `index.html`, so client-side routing works.
- File types a published page may carry (case does not matter):
  - page and code: `html css js mjs ts jsx tsx json map txt md yaml yml toml csv`
  - images: `png jpg jpeg gif svg webp avif ico bmp`
  - fonts: `woff woff2 ttf otf`
  - sound: `mp3 wav ogg oga m4a aac flac opus`
  - video: `mp4 webm mov m4v`
  - other: `wasm`, 3D models `glb gltf`, `pdf`

  A file of any other type is skipped. So are dot files, anything inside a dot folder, and anything inside `node_modules/`. Build output folders such as `dist/` or `build/` are kept when they sit inside `ui/` (outside `ui/` they are dropped).
- **Limits: 10 MB per file, 50 MB for the whole bundle.** A page file over 10 MB is skipped and counted in the upload result's `filesSkipped`; a bundle whose files add up to more than 50 MB is refused with `bundle exceeds 50MB total file size`, and a `.zip` over 50 MB with `bundle too large (max 50MB)`. Check `uiFilesStored` and `filesSkipped` in the result after every upload.
- Every file is served with its real content type (`video/mp4`, `audio/mpeg`, `model/gltf-binary`, `font/woff2`, `application/wasm` and so on). Pages are served with `nosniff`, so a file with an unknown extension arrives as `application/octet-stream` and the browser will not use it as media or a model.
- Video, sound and every other non-HTML file answer a single `Range: bytes=` request with `206 Partial Content` and `Content-Range`, and a range that starts past the end with `416`. Responses advertise `Accept-Ranges: bytes`. This is what lets a `<video>` seek and scroll-scrub on iPhone. Multi-range requests get the whole file.
- On the phone, `<video muted playsinline>` plays inline, without a tap and without opening the system player.
- Caching follows one rule:
  - A file whose name carries a content hash is cached for a year (`immutable`). The name must look like `<name>-<hash>.<ext>` where the hash is 8 to 64 letters and digits of one case with a digit between two letters somewhere in it: `main-0a8ksftt.js`, `chunk-5JFTZ4CW.js`, `app-a1b2c3d4.css`. This is what `bun build` writes with `[name]-[hash].[ext]` and esbuild with `--entry-names=[name]-[hash]`. Hand-made names such as `hero-section2.png` or `shot-20260930.png` are deliberately not treated as hashed.
  - Every other file, `index.html` included, is `no-cache` with a strong `ETag`: each open asks, and an unchanged file answers `304` with no body. Because `index.html` is always checked, it always points at the newest hashed files. A range request whose `If-Range` names an older version gets the whole new file.
  - Rebuild with hashed names instead of renaming files by hand to get past a cache.
- An app whose manifest asks for `device:motion` gets a `Permissions-Policy` that allows `accelerometer` and `gyroscope` for its own page. Without it, motion sensors are blocked.

## Frontend SDK

Load the SDK Nebo serves, then read the one global it defines:

```html
<script src="/sdk/nebo.global.js"></script>
<script>
  const { nebo } = window.NeboAppSDK;   // there is no bare `nebo` global
</script>
```

```javascript
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

// HTTP to the sidecar (apps with a sidecar only)
const resp = await nebo.fetch('/projects');
```

The `@neboai/app-sdk` npm package has the same API for bundled builds (`import { nebo } from '@neboai/app-sdk'`). An unbundled page cannot import a bare package name; use the served global.

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
4. Look for sidecar binaries: one per platform in `dist/app/<platform>/` (e.g. `dist/app/darwin-arm64/my-app`), or the local build in `sidecar/target/release/` for this machine's platform
5. Create the app with AGENT.md as its manifest (or update it to the new version)
6. **Page-only app (no sidecar):** upload one bundle (a .zip of AGENT.md, agent.json, manifest.json, `ui/` and any `skills/`). The marketplace stores `ui/` as the app's page, keeps manifest.json as the app's manifest (window, permissions), and builds the installable package.
   **App with a sidecar:** upload each sidecar binary; the first upload also carries `agent.json` (config) and a tar.gz of `ui/` (the `ui` field)
7. Submit for review. A page-only app is reviewed at once; sidecar binaries are scanned in the background and the app is listed once the scan passes.

Via MCP (page-only app):
1. `agent(action: create, name, description, manifestContent: "<AGENT.md with artifact_type: app>", version)`
2. `agent(action: bundle-token, id)` and run the returned curl with a `.zip` holding `AGENT.md`, `agent.json`, `manifest.json`, `ui/` and any `skills/`
3. `agent(action: submit, id, version)`

The upload token from `bundle-token` and `binary-token` lasts 5 minutes. A `.zip` made with `zip -r app.zip my-app/` is fine: a folder every entry shares is stripped.

Via MCP (app with a sidecar): `agent(action: binary-token, id)` and the returned curl per platform (`file=@<sidecar>`, `platform=<platform>`, `ui=@ui.tar.gz` and `config=@agent.json` on the first), then `submit`.

### Updates

For a private app or one shared with a loop, every bundle upload rebuilds the installable package at once, so the next install gets the new files. A bot that already installed the app picks up the change when the app is installed again, or when you publish a higher version number: Nebo checks for newer versions every few hours and applies one with the owner's yes, or on its own when automatic updates are on for that app. So to ship a fix to bots that already have the app, raise `version` and upload. A **public**, unlisted or invite-only app keeps its approved package until a new version passes review; uploading a bundle to it never pushes an unreviewed version to anyone.

> The marketplace `.napp` for an app carries the agent payload (`manifest.json`, `agent.json`, `AGENT.md`, `signatures.json`), the page under `ui/`, and the sidecar under `bin/` when there is one.

## Building an App Inside Nebo

You do not need a local folder to make an app. Ask an employee in Nebo:

- **"Build me an app for my orders"** creates a new app employee with its own page.
- **"You are the app"** or **"build yourself a game"** turns the employee you are talking to into the app. It keeps its chat, memory and persona, and its page reloads while you talk to it. This works for an employee hired in conversation too.
- If it is not clear which you mean, the employee asks once.

Everyday tools (a tracker, a form, a small dashboard) are built by the built-in app skill. Rich pages and games that should look designed rather than generated (cinematic landing pages, scroll-driven films, generated art, games) are built with the **App Studio** skill from the marketplace, which adds a brief, design boards, generated images and video, a real build step and a quality check.

An app you made on your own bot can always reload itself, read its own console, take screenshots of itself and publish itself. **App Developer mode** (Bot settings, Developer) adds three things: other employees can work on any of your apps, the page shows a floating console, and app files are never cached. Apps installed from the marketplace never get developer tools, with the mode on or off.

### Publishing from Nebo

Say "publish yourself" in the app's chat, or tap **Publish**:

- in the app's chat header (web, desktop and phone);
- on the phone's app screen (in the top bar, or as a small pill beside Close for a full-screen app);
- on desktop, **Publish This App…** in the File menu on macOS while an app window has focus, or in the app window's **App** menu on Windows and Linux.

The employee drafts the listing from the app itself, takes 3 to 5 screenshots, and shows you the listing to shape by talking ("shorter description", "use the second screenshot first"). Nothing is sent until you answer **Submit for review** on the card in the chat (you can answer it by voice). The listing needs a name, a short description of 10 to 500 characters, a version like `1.2.0`, at least one screenshot (at most 10) and a category. The package carries AGENT.md, agent.json, manifest.json, the page and the employee's own skills; skills it only references from the marketplace and skills it learned on its own are not copied in. You need a publisher profile first (https://neboai.com/publisher/register). Only apps you made can be published this way; installed apps never show Publish.

## Category

Optional. The marketplace accepts only its own category names: `Run your business`, `Create content`, `Find customers`, `Manage money`, `Get organized`, `Communicate`, `Learn & grow`, `Research & decide`, `Handle documents`, `Build & connect` (list them with `marketplace(action: list_categories)`). Via MCP pass the name exactly (`category: "Get organized"`); any other value, such as `productivity`, is refused. The CLI reads a slug from manifest.json `category` (`business`, `content`, `customers`, `money`, `organized`, `communicate`, `learn`, `research`, `documents`, `build`) and files anything else under Build & connect.

## Key Rules

- `type` MUST be `"app"` in manifest.json
- AGENT.md frontmatter MUST say `artifact_type: app`, and any value containing `: ` MUST be quoted
- `ui/index.html` MUST exist
- Page files: at most 10 MB each and 50 MB in total, allowed types only
- The page reads the SDK from `window.NeboAppSDK` (load `/sdk/nebo.global.js`)
- Sidecar (when there is one) must read `$NEBO_APP_SOCK` and bind a Unix socket there
- The launched sidecar binary must be a regular file; symlinks are rejected at launch (a symlinked dev binary like `bin/my-app → target/release/my-app` is fine for hot-reload detection, but the file that actually runs must resolve to a regular executable)
- Sidecar startup timeout: 10 seconds default, max 120s (set via `startup_timeout`)
- Window config accepts `title`, `width`, `height`, `resizable`, `fullscreen`, `orientation` (defaults: 1024 x 768, resizable, not fullscreen, portrait). There are no `min_width`/`min_height` fields.
- An app that declares a sidecar needs at least one sidecar binary for its version; a page-only app needs none
