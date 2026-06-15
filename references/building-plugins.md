# Building Plugins

How to architect, implement, and polish a Nebo plugin that's reliable, fast, and easy to configure.

## Language Choice

**Rust is the preferred language for all plugin binaries.** Reasons:

1. **No AV false positives.** Go binaries (especially UPX-packed) trigger antivirus heuristics. Python scripts trigger "suspicious scripting" alerts. Rust binaries compile to clean native code that AV engines treat as standard applications.
2. **Immutable by design.** Once compiled, the binary cannot be modified by the agent or by other software on the system. Interpreted languages (Python, Node) can have their scripts modified at runtime — a security risk.
3. **No runtime dependencies.** Static linking with musl produces a single file with zero external dependencies. No Python interpreter, no Node runtime, no glibc version matching.
4. **Small binaries.** A typical plugin compiles to 5-15MB stripped. Go produces 15-30MB. Python bundles (PyInstaller) are 50-100MB+.
5. **Memory safety.** No buffer overflows, no use-after-free, no data races.
6. **Cross-compilation.** `cargo build --target aarch64-unknown-linux-musl` just works.

If Rust is not an option, use Go as a fallback. Never use interpreted languages for plugin binaries.

## Architecture Decisions

### When to Build a Plugin vs. a Skill

| Need | Build | Why |
|------|-------|-----|
| Teach the agent a process | Skill | Markdown instructions are enough |
| Wrap an external API | Plugin | Needs auth, network, structured responses |
| Provide OS-level tools | Plugin | Needs binary execution |
| Add a new model provider | Plugin | Needs the providers capability |
| Event-driven automation | Plugin | Needs watch/events capability |
| Custom auth flow | Plugin | Needs routes + auth capability |

### Plugin Design Principles

1. **Single responsibility.** One plugin = one service or domain. Don't build "everything-plugin".
2. **Tools over raw API.** Expose semantic actions (`gmail.triage`), not raw HTTP (`gmail.get /v1/messages`).
3. **Config over code.** Use `configSchema` for user settings. Don't hardcode anything user-specific.
4. **Skills teach, tools do.** Bundle skills that explain *when* and *why* to use each tool.
5. **Fail fast, fail loud.** Return structured errors. Never hang silently.

## Binary Design

### Command Pattern

Every tool invocation calls your binary with CLI subcommand args from the `command` field, then pipes the tool's input data as JSON on **stdin**:

```json
{
  "name": "gmail.triage",
  "command": "gmail triage",
  "inputSchema": { ... }
}
```

Nebo runs: `echo '{"..."}' | $PLUGIN_BIN gmail triage`

The `command` field provides the CLI subcommand args. The `inputSchema` data is delivered as a JSON object on **stdin**, NOT as CLI arguments. Design your binary to read stdin for tool input:

```
echo '{}' | my-plugin gmail triage
echo '{"to":"..."}' | my-plugin gmail send
my-plugin auth login          # auth commands may not need stdin input
my-plugin auth status
```

### Input/Output Contract

**Input:** JSON on stdin matching your `inputSchema`. The tool's input data always arrives on stdin, not as CLI flags.

**Output:** JSON on stdout. Structure:

```json
{
  "result": { ... },
  "metadata": {
    "elapsed_ms": 234,
    "source": "gmail-api"
  }
}
```

**Errors:** JSON on stderr with exit code > 0:

```json
{
  "error": "authentication_required",
  "message": "OAuth token expired. Run 'auth login' to re-authenticate.",
  "retryable": false
}
```

### Startup Performance

Your binary is invoked per tool call. Cold starts matter.

**Do:**
- Compile statically (no dynamic linking)
- Cache auth tokens to disk
- Use connection pooling for HTTP clients (if running as a daemon)
- Keep the binary under 20MB

**Don't:**
- Start a runtime (JVM, Node) per invocation
- Load large models or datasets on every call
- Make network requests during initialization

### Cross-Platform Builds (Rust)

Target at minimum:
- `darwin-arm64` (macOS Apple Silicon — most developers)
- `linux-amd64` (servers, CI, WSL)

Recommended full set:
- `darwin-arm64`, `darwin-amd64`
- `linux-arm64`, `linux-amd64`
- `windows-amd64`

#### Cargo.toml Setup

```toml
[package]
name = "my-plugin"
version = "1.0.0"
edition = "2024"

[dependencies]
clap = { version = "4", features = ["derive"] }
tokio = { version = "1", features = ["full"] }
reqwest = { version = "0.12", features = ["json"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
anyhow = "1"
keyring = "3"
dirs = "6"
open = "5"

[profile.release]
opt-level = "z"      # Optimize for size
lto = true           # Link-time optimization
codegen-units = 1    # Single codegen unit for better optimization
strip = true         # Strip debug symbols
panic = "abort"      # Smaller binary (no unwinding)
```

#### Build Script

```bash
#!/bin/bash
set -euo pipefail

TARGETS=(
  "aarch64-apple-darwin"
  "x86_64-apple-darwin"
  "aarch64-unknown-linux-musl"
  "x86_64-unknown-linux-musl"
)

PLATFORMS=(
  "darwin-arm64"
  "darwin-amd64"
  "linux-arm64"
  "linux-amd64"
)

BINARY_NAME="my-plugin"

for i in "${!TARGETS[@]}"; do
  target="${TARGETS[$i]}"
  platform="${PLATFORMS[$i]}"

  echo "Building for $platform ($target)..."
  cargo build --release --target "$target"

  mkdir -p "dist/$platform"
  cp "target/$target/release/$BINARY_NAME" "dist/$platform/$BINARY_NAME"
done

echo "Done. Binaries in dist/"
```

#### Key Build Rules

- **Always use musl for Linux:** `x86_64-unknown-linux-musl` not `x86_64-unknown-linux-gnu`. This produces a fully static binary with zero runtime deps.
- **Strip all binaries:** Set `strip = true` in `[profile.release]` or run `strip` post-build.
- **Use `opt-level = "z"`:** Optimizes for binary size. A typical plugin goes from 15MB to 5-8MB.
- **LTO + single codegen unit:** Slower build, smaller + faster binary.
- **Install cross-compilation toolchains:** `rustup target add aarch64-apple-darwin x86_64-unknown-linux-musl` etc.

#### Complete Plugin Binary Skeleton (Rust)

```rust
use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::io::{self, IsTerminal, Read};

#[derive(Parser)]
#[command(name = "my-plugin", version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Authentication
    Auth {
        #[command(subcommand)]
        action: AuthAction,
    },
    /// Service operations
    Service {
        #[command(subcommand)]
        action: ServiceAction,
    },
}

#[derive(Subcommand)]
enum AuthAction {
    Login,
    Status,
    Logout,
}

#[derive(Subcommand)]
enum ServiceAction {
    /// List items
    List,
    /// Create an item
    Create,
}

fn main() {
    let cli = Cli::parse();
    let result = match cli.command {
        Commands::Auth { action } => handle_auth(action),
        Commands::Service { action } => handle_service(action),
    };

    match result {
        Ok(output) => {
            println!("{}", serde_json::to_string(&output).unwrap());
            std::process::exit(0);
        }
        Err(e) => {
            eprintln!("{}", json!({"error": e.to_string(), "retryable": false}));
            std::process::exit(1);
        }
    }
}

fn read_input() -> serde_json::Value {
    if !io::stdin().is_terminal() {
        let mut buf = String::new();
        io::stdin().read_to_string(&mut buf).ok();
        serde_json::from_str(&buf).unwrap_or_default()
    } else {
        serde_json::Value::Object(Default::default())
    }
}

fn handle_auth(action: AuthAction) -> anyhow::Result<serde_json::Value> {
    match action {
        AuthAction::Login => {
            // OAuth flow
            Ok(json!({"status": "authenticated"}))
        }
        AuthAction::Status => {
            Ok(json!({"authenticated": true, "user": "user@example.com"}))
        }
        AuthAction::Logout => {
            Ok(json!({"status": "logged_out"}))
        }
    }
}

fn handle_service(action: ServiceAction) -> anyhow::Result<serde_json::Value> {
    match action {
        ServiceAction::List => {
            let _params = read_input();
            Ok(json!({"items": [], "total": 0}))
        }
        ServiceAction::Create => {
            let params = read_input();
            Ok(json!({"created": params}))
        }
    }
}
```

This skeleton gives you:
- clap for ergonomic CLI parsing
- JSON in/out contract (input on stdin, output on stdout)
- Structured error handling
- Auth subcommands
- `std::io::IsTerminal` to detect piped stdin (no external crates needed)

## Tool Design

### Naming Convention

`{domain}.{action}` — lowercase, dot-separated:

```
gmail.triage
gmail.send
gmail.search
calendar.events
calendar.create
```

### Input Schemas

Be precise. Don't use `"type": "string"` for everything:

```json
{
  "type": "object",
  "properties": {
    "to": {
      "type": "array",
      "items": { "type": "string", "format": "email" },
      "description": "Recipient email addresses"
    },
    "subject": {
      "type": "string",
      "maxLength": 200,
      "description": "Email subject line"
    },
    "body": {
      "type": "string",
      "description": "Email body (plain text or markdown)"
    },
    "priority": {
      "type": "string",
      "enum": ["low", "normal", "high"],
      "default": "normal"
    }
  },
  "required": ["to", "subject", "body"]
}
```

**Rules:**
- Every property needs a `description`
- Use `enum` when there's a fixed set of values
- Use `default` so the LLM doesn't have to guess
- Mark `required` fields — don't make the LLM infer what's optional
- Keep schemas flat. Avoid nested objects unless semantically necessary.

### Tool Descriptions

The description is what the LLM reads to decide whether to use this tool:

**Bad:**
```json
"description": "Sends an email"
```

**Good:**
```json
"description": "Send an email via Gmail. Use when the user explicitly asks to send, reply, or forward an email. Requires recipient, subject, and body."
```

Include: what it does, when to use it, what it requires.

### Approval Gates

Tool `approval` **defaults to `true`** — all tools require user confirmation unless you explicitly set `"approval": false`. Set `false` only for read-only, non-destructive tools:

```json
{
  "name": "gmail.search",
  "approval": false,
  "description": "Search Gmail messages. Read-only, no confirmation needed."
}
```

Keep the default `true` for:
- Sending messages (email, Slack, SMS)
- Deleting data
- Making purchases
- Modifying calendar events
- Any action that can't be undone

### Timeouts

Tool `timeoutSeconds` **defaults to `120`**. Override with a shorter value for fast operations, or request a longer one when needed:

```json
{
  "name": "gmail.search",
  "timeoutSeconds": 30
}
```

| Operation | Suggested Timeout |
|-----------|---------|
| Simple lookup | 10s |
| API call with auth | 30s |
| File processing | 60s |
| Long-running batch | 120s (the default) |

Never exceed `permissions.maxTimeoutSeconds`.

## Events & Watch Commands

### Event Design

Events notify agents that something happened:

```json
{
  "name": "gmail.new_message",
  "description": "Fires when a new email arrives in the inbox",
  "command": "gmail watch --inbox",
  "multiplexed": false
}
```

**NDJSON output format** (one JSON object per line):

```json
{"event": "gmail.new_message", "data": {"from": "alice@example.com", "subject": "Q4 Report", "snippet": "..."}}
{"event": "gmail.new_message", "data": {"from": "bob@example.com", "subject": "Meeting tomorrow", "snippet": "..."}}
```

### Watch Command Rules

1. **Long-running process.** The command runs continuously and outputs events as NDJSON.
2. **Clean shutdown.** Handle SIGTERM — flush buffers, close connections, exit 0.
3. **Reconnection.** If the upstream connection drops, reconnect with backoff. Don't crash.
4. **Deduplication.** Include a unique event ID so Nebo can deduplicate on restart.
5. **Heartbeat.** Output a heartbeat event every 60s so Nebo knows the process is alive:

```json
{"event": "_heartbeat", "data": {"ts": 1700000000}}
```

### Multiplexed vs. Per-User

```json
"multiplexed": false
```

- `false` — One watch process per user. Each gets their own auth context.
- `true` — One shared process. Events include user identifiers.

Most plugins should use `multiplexed: false` unless you're building a shared infrastructure service.

## Authentication

### OAuth CLI Pattern

```json
"auth": {
  "type": "oauth_cli",
  "label": "Google Workspace",
  "description": "Connect your Google account to access Gmail and Calendar.",
  "commands": {
    "login": "auth login",
    "status": "auth status",
    "logout": "auth logout"
  },
  "env": {
    "CLIENT_ID": "...",
    "CLIENT_SECRET": "..."
  }
}
```

**Implementation rules:**

1. `auth login` — Opens browser, runs OAuth flow, stores token locally
2. `auth status` — Prints JSON: `{"authenticated": true, "email": "user@example.com", "expires_at": "..."}`
3. `auth logout` — Clears stored tokens
4. Token storage: `~/.config/{plugin-slug}/credentials.json` (or OS keychain)
5. Auto-refresh: Every tool call should check token expiry and refresh silently
6. Clear error on expired/revoked: Tell the user to run login again

### API Key Pattern

For services with simple API key auth, use `configSchema`:

```json
"capabilities": {
  "configSchema": [
    {
      "key": "API_KEY",
      "label": "API Key",
      "fieldType": "string",
      "required": true,
      "secret": true,
      "description": "Your API key from service.com/settings"
    }
  ]
}
```

The key is passed to your binary as an environment variable.

## Config Schema

Design config for zero-friction setup:

```json
"configSchema": [
  {
    "key": "DEFAULT_CALENDAR",
    "label": "Default Calendar",
    "fieldType": "select",
    "required": false,
    "default": "primary",
    "options": ["primary", "work", "personal"],
    "description": "Which calendar to use when none is specified"
  }
]
```

**Rules:**
- Every field has a `default` → zero-config works out of the box
- `secret: true` for anything sensitive (keys, tokens, passwords)
- Use `select` with `options` when there's a known set of choices
- Keep config minimal — 3-5 fields max. More than that and you need a setup wizard.

## Bundled Skills

Every plugin should bundle at least one skill that teaches the agent:

1. **When** to use each tool (triggers, scenarios)
2. **How** to combine tools (workflows, sequences)
3. **What** the responses mean (interpretation guidance)

Without bundled skills, the agent has tools but no judgment about when to use them.

```
skills/
  gmail-management/
    SKILL.md      # When to triage, when to send, response interpretation
  calendar-ops/
    SKILL.md      # When to create events, conflict detection, timezone handling
```

## Channel Plugins (Bridges)

Channel plugins bridge Nebo to external messaging platforms (Slack, Discord, etc.). They run as long-lived sidecar processes using NDJSON over stdin/stdout.

By default Nebo spawns one bridge per `(agent_id, plugin_slug)` — each agent gets its own process and credentials. Set `channel.shared: true` in `plugin.json` to run a single bridge shared across all agents; inbound platform events carry the target agent's name and Nebo routes each message to the matching agent. Use the per-agent default unless your platform genuinely has one connection for all agents.

### NDJSON Protocol

The bridge communicates with Nebo via newline-delimited JSON on stdin (commands from Nebo) and stdout (events to Nebo):

```json
{"type": "message", "channel": "C12345", "text": "Hello from Slack", "user": "U99999"}
{"type": "reaction", "channel": "C12345", "emoji": "thumbsup", "user": "U99999"}
```

All messaging operations (post, reply, upload, DM) flow through this stdin/stdout protocol — NOT through CLI subcommands.

### Mandatory Stdin EOF Exit Contract

**Every long-running sidecar must self-exit when stdin reaches EOF.** This is critical for orphan prevention — if the host process is killed (including via SIGKILL, which cannot be caught), the pipe closes and stdin returns EOF. Your bridge must detect this and exit cleanly:

```rust
use std::io::{self, BufRead};

// In your main read loop:
let stdin = io::stdin().lock();
for line in stdin.lines() {
    match line {
        Ok(line) => handle_command(&line),
        Err(_) => break, // stdin closed — host is gone, exit
    }
}
// Clean up and exit
std::process::exit(0);
```

Without this contract, bridge processes become orphans that survive host restarts and accumulate indefinitely.

## Testing Checklist

Before publishing:

- [ ] Binary runs on all target platforms (test in VM/Docker)
- [ ] `auth login` → `auth status` → tool call → `auth logout` cycle works
- [ ] Every tool returns valid JSON on success
- [ ] Every tool returns structured error JSON on failure
- [ ] Timeouts actually timeout (don't hang forever)
- [ ] Watch commands handle SIGTERM gracefully
- [ ] Watch commands reconnect on network drop
- [ ] Config with all defaults works (zero-config)
- [ ] `plugin.json` has no template variables
- [ ] Binary is under 50MB per platform
- [ ] Skills bundled and tested with the plugin's tools

## Anti-Patterns

| Anti-Pattern | Fix |
|-------------|-----|
| Huge binary (>100MB) | Strip debug symbols, use static linking, compress |
| Dynamic dependencies (shared libs) | Compile statically (musl on Linux) |
| Unstructured text output | Always output JSON |
| Silent failures (exit 0, empty output) | Return error JSON on stderr, exit > 0 |
| Hardcoded user config | Use configSchema |
| Tools with no approval for destructive actions | Add `"approval": true` |
| Overly broad tool descriptions | Be specific about when to use |
| No bundled skills | Agent won't know when to call your tools |
