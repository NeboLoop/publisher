# NeboAI Publisher

Build, validate and publish skills, employees, plugins, apps, connectors and collections to the [NeboAI marketplace](https://neboai.com/marketplace), from the AI tools you already use.

This repo has two parts:

1. **The publisher skill** (`SKILL.md`, `references/`, `scripts/`, `examples/`): an [Agent Skills](https://agentskills.io) skill that teaches an AI tool how to turn an idea into a marketplace item and publish it.
2. **The `neboai` CLI** (`cli/`): a small Rust binary that signs you in, validates an item's files and publishes them.

Learn more at [neboai.com/learn/neboai](https://neboai.com/learn/neboai).

## Install

### macOS and Linux: CLI and skill

```bash
curl -fsSL https://raw.githubusercontent.com/NeboLoop/publisher/main/install.sh | bash
```

Installs the `neboai` binary to `/usr/local/bin` (override with `INSTALL_DIR=...`) and the skill to `~/.claude/skills/neboai` (override with `SKILLS_DIR=...`), both from the latest release, checked against its `SHA256SUMS`.

### Windows: CLI and skill

```powershell
irm https://raw.githubusercontent.com/NeboLoop/publisher/main/install.ps1 | iex
```

Installs `neboai.exe` to `%LOCALAPPDATA%\Programs\neboai` (added to your PATH) and the skill to `%USERPROFILE%\.claude\skills\neboai`.

### Homebrew: CLI only

```bash
brew tap NeboLoop/tap
brew trust neboloop/tap   # Homebrew 7+ asks you to trust a third-party tap
brew install neboai
```

### npm or pnpm: CLI only

```bash
pnpm add -g @neboai/publisher
# or: npm install -g @neboai/publisher
```

The package fetches the matching `neboai` binary from this repo's release on install, or on first run if your package manager skips install scripts.

### The skill in any other AI tool

The skill uses the open Agent Skills format. Copy it into your tool's skills folder:

```bash
git clone https://github.com/NeboLoop/publisher.git
mkdir -p <your-tool-skills-folder>/neboai
cp -R publisher/SKILL.md publisher/references publisher/scripts publisher/examples <your-tool-skills-folder>/neboai/
```

Each release also has the skill as `neboai-skill.tar.gz`.

### Supported platforms

| Platform | Binary |
|----------|--------|
| macOS, Apple Silicon | `neboai-darwin-arm64` |
| macOS, Intel | `neboai-darwin-amd64` |
| Linux, ARM64 | `neboai-linux-arm64` |
| Linux, x86_64 | `neboai-linux-amd64` |
| Windows, x86_64 (also runs on Windows on Arm) | `neboai-windows-amd64.exe` |

## Sign in

```bash
neboai auth login
```

Opens your browser to sign in with your NeboAI account. Any account works: there is no fee, no developer-account step and no verification. Your publisher account (`@your-handle`) is set up for free the first time you sign in or publish. `neboai publish` also starts sign-in by itself if you haven't signed in yet.

```bash
neboai auth status    # signed in? which handle?
neboai auth logout
```

## In a chat app: the NeboAI MCP connector

In Claude Desktop, ChatGPT or any app that supports remote MCP, add the NeboAI connector (`https://neboai.com/mcp`) instead of installing anything. The skill works through its tools. See [Publishing via MCP](https://neboai.com/docs/mcp-publishing).

## Publish in three steps

1. Install the skill and CLI, and sign in (above).
2. Tell your AI tool what you want, for example: *"I have an idea for a skill that turns my meeting notes into a follow-up email. Build it and publish it to NeboAI."*
3. It builds the files, checks them with `neboai validate`, publishes with `neboai publish` and tells you the review outcome.

Or by hand:

```bash
neboai validate ./my-skill
neboai publish ./my-skill        # public: submitted for review, listed once approved
neboai list                      # everything you've published, with its status
neboai status <id>               # Draft / In review / Published
```

`neboai publish ./dir --visibility private` (or `loop`) saves an item without listing it, so it isn't reviewed. Publishing a directory again updates the item to the version in its files. To publish under a team's account you belong to, set `NEBOAI_ACCOUNT=<team-handle>`.

## Review

Anyone can publish, free. Every public version is reviewed:

- **Text-only items** (skills, employees, connectors, collections) get an automated content scan. A clean scan is approved at once and the item is listed immediately.
- **Items with binaries** (plugins and apps) are scanned in the background, and approved automatically when the scan is clean. Plugins and apps need at least one platform binary.
- Anything the scan flags waits for a person on the NeboAI team.

A listed item stays listed when you publish an update. More in the [publishing overview](https://neboai.com/help/publish-overview) and the [publisher skill guide](https://neboai.com/help/publisher-skill).

## What you can publish

| Type | Files | Guide |
|------|-------|-------|
| Skill | `SKILL.md` (+ `references/`, `scripts/`, `assets/`) | [building-skills](references/building-skills.md), [skill-format](references/skill-format.md) |
| Employee | `AGENT.md` + `agent.json` | [building-agents](references/building-agents.md), [agent-format](references/agent-format.md) |
| Plugin | `PLUGIN.md` + `plugin.json` + binaries in `dist/plugin/<platform>/` | [building-plugins](references/building-plugins.md), [plugin-format](references/plugin-format.md) |
| App | `AGENT.md` + `manifest.json` + `ui/` + sidecar binaries | [building-apps](references/building-apps.md), [app-format](references/app-format.md) |
| Connector | `connector.json` (an `mcpServers` block) | [connector-format](references/connector-format.md) |
| Collection | `collection.json` | [collection-format](references/collection-format.md) |

Working examples are in [`examples/`](examples/). Listing advice: [listing-quality](references/listing-quality.md).

## CLI reference

```
neboai auth login | status | logout
neboai validate <dir> [--type skill|plugin|agent|app|connector|collection]
neboai publish <dir> [--type ...] [--visibility public|private|loop]
neboai list
neboai status <id>
neboai binaries list <id>
neboai binaries delete <item-id> <binary-id>
```

Credentials are saved in your user config folder (`~/Library/Application Support/neboai/credentials.json` on macOS, `~/.config/neboai/credentials.json` on Linux, `%APPDATA%\neboai\credentials.json` on Windows). `NEBOAI_BASE_URL` points the CLI at another API (for local development).

## Build from source

```bash
cd cli
cargo build --release
./target/release/neboai --version
```

## Releasing

Bump `version` in `cli/Cargo.toml` and `package.json`, merge, then push a tag `vX.Y.Z`. The release workflow builds the five binaries, publishes them with `neboai-skill.tar.gz` and `SHA256SUMS` to the GitHub release, updates `Formula/neboai.rb` in [NeboLoop/homebrew-tap](https://github.com/NeboLoop/homebrew-tap) (secret `TAP_GITHUB_TOKEN`), and publishes `@neboai/publisher` to npm (secret `NPM_TOKEN`).

## License

[Apache-2.0](LICENSE)
