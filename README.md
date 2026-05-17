# NeboAI Publisher

Build, validate, and publish skills, plugins, agents, and apps to the [NeboLoop](https://neboloop.com) marketplace.

This repo contains two things:

1. **`neboai` CLI** — A Rust binary that handles authentication, validation, and publishing
2. **Publisher Skill** — An [Agent Skills](https://agentskills.io)-standard skill that teaches AI coding agents (Claude Code, Cursor, VS Code Copilot, etc.) how to build and publish NeboLoop artifacts

## Install

### Quick Install (recommended)

```bash
curl -fsSL https://raw.githubusercontent.com/NeboLoop/publisher/main/install.sh | bash
```

Downloads the pre-built binary for your platform and optionally installs the skill for Claude Code.

### npm

```bash
pnpm add -g @neboai/publisher
```

### Cargo (build from source)

```bash
cargo install --path cli
```

### Manual

Download the binary for your platform from [Releases](https://github.com/NeboLoop/publisher/releases), make it executable, and place it on your PATH.

| Platform | Binary |
|----------|--------|
| macOS (Apple Silicon) | `neboai-darwin-arm64` |
| macOS (Intel) | `neboai-darwin-amd64` |
| Linux (ARM64) | `neboai-linux-arm64` |
| Linux (x86_64) | `neboai-linux-amd64` |
| Windows (x86_64) | `neboai-windows-amd64.exe` |

---

## Install the Skill (for AI Coding Agents)

### Claude Code

```bash
mkdir -p ~/.claude/skills
ln -s "$(pwd)" ~/.claude/skills/neboai
```

### Other Agent Skills-compatible tools

Copy or symlink this directory into your agent's skills path. The skill follows the [Agent Skills standard](https://agentskills.io) and works with any compatible tool (Cursor, VS Code, Gemini CLI, OpenAI Codex, Roo Code, etc.).

---

## Usage

### Authenticate

```bash
neboai auth login     # Opens browser for OAuth
neboai auth status    # Check if authenticated
neboai auth logout    # Clear credentials
```

### Build + Publish (one command)

```bash
neboai publish ./my-artifact
```

Auto-detects the artifact type from directory contents, validates everything locally, uploads, and submits for review.

### Validate Only

```bash
neboai validate ./my-artifact
```

Checks structure, YAML/JSON validity, required fields, naming conventions, budget math, and common mistakes — all locally before touching the API.

### Manage Artifacts

```bash
neboai list                    # List your published artifacts
neboai status <id>             # Check submission/review status
neboai binaries list <id>      # List uploaded binaries for an artifact
neboai binaries delete <id>    # Delete a binary (fix duplicates)
```

---

## What Can You Publish?

| Type | What It Is | Key Files |
|------|-----------|-----------|
| **Skill** | Markdown instructions that teach an agent | `SKILL.md` |
| **Plugin** | Native binary providing tools, auth, events | `plugin.json` + `dist/` binaries |
| **Agent** | Autonomous workflows with a persona | `AGENT.md` + `agent.json` |
| **App** | Agent with a dedicated UI | `AGENT.md` + `manifest.json` + `ui/` |

### Type Detection

The CLI auto-detects what you're publishing:

| Present in Directory | Detected As |
|---------------------|-------------|
| `manifest.json` with `"artifact_type": "app"` | App |
| `plugin.json` | Plugin |
| `agent.json` + `AGENT.md` | Agent |
| `SKILL.md` (alone) | Skill |

Override with `--type`: `neboai publish ./dir --type agent`

---

## Building Artifacts

The included skill teaches AI agents how to build each artifact type from scratch. If you're using Claude Code (or any compatible tool), just ask:

- *"Create a new skill that teaches the agent to draft sales emails"*
- *"Build a plugin that connects to the Stripe API"*
- *"Scaffold an agent that monitors my inbox every 30 minutes"*
- *"Build me a deal tracker app with a pipeline UI"*

The agent will use the skill's references and examples to generate correct, publish-ready artifacts.

### Language Preference

For plugins and app sidecars (compiled binaries), **Rust is strongly preferred**:

- Single static binary — no runtime dependencies
- Does not trigger antivirus heuristics (unlike Go/Python)
- Cannot be modified by the agent at runtime (compiled, not interpreted)
- Cross-compilation is straightforward
- Memory-safe

---

## Project Structure

```
.
├── SKILL.md              # The Agent Skills-standard skill
├── README.md             # This file
├── install.sh            # curl | bash installer
├── package.json          # npm package wrapper
├── references/           # Deep-dive guides (loaded on demand by agents)
│   ├── building-skills.md
│   ├── building-plugins.md
│   ├── building-agents.md
│   ├── building-apps.md
│   ├── skill-format.md
│   ├── plugin-format.md
│   ├── agent-format.md
│   ├── app-format.md
│   └── common-mistakes.md
├── scripts/
│   ├── validate.sh       # Quick validation without the full CLI
│   └── postinstall.js    # npm postinstall binary downloader
├── examples/             # Working examples of each artifact type
│   ├── skill-example/
│   ├── plugin-example/
│   ├── agent-example/
│   └── app-example/
└── cli/                  # Rust CLI source code
    ├── Cargo.toml
    └── src/
        ├── main.rs
        ├── auth.rs
        ├── api.rs
        ├── detect.rs
        ├── validate.rs
        └── publish.rs
```

---

## Development

### Build the CLI locally

```bash
cd cli
cargo build --release
```

Binary will be at `cli/target/release/neboai`.

### Run tests

```bash
cd cli
cargo test
```

### Build for all platforms

```bash
cd cli
./build-all.sh
```

Outputs binaries to `dist/` for each platform.

---

## How It Works

1. **`neboai auth login`** — Opens your browser for OAuth PKCE authentication with NeboLoop. Tokens stored in `~/.config/neboai/credentials.json`.

2. **`neboai validate <dir>`** — Checks your artifact directory locally:
   - Structure matches the detected type
   - YAML frontmatter is valid (no duplicates, required fields)
   - JSON parses cleanly (no trailing commas, no template vars in plugin.json)
   - Names follow conventions (lowercase, hyphens, 1-64 chars)
   - Versions are valid semver
   - Budget math balances (activity budgets ≤ total_per_run)
   - Platform binaries exist (plugins)
   - `ui/index.html` exists (apps)

3. **`neboai publish <dir>`** — Validates, then:
   - Creates or updates the artifact on NeboLoop
   - Uploads manifest (SKILL.md / AGENT.md / PLUGIN.md)
   - Uploads config (agent.json / plugin.json) — NEVER manifest.json
   - Uploads binaries per-platform (plugins, app sidecars)
   - Submits for review

---

## License

Apache-2.0
