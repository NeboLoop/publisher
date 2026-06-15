# connector.json Format

A connector publishes an **MCP connection** to the marketplace. Its manifest is a standard MCP config block — the same `mcpServers` object Claude Desktop and VS Code use — so installing it wires that server into the user's setup. Install codes are `CONN-XXXX-XXXX`.

## Required shape

The file must contain a non-empty `mcpServers` (or `servers`) object. Each server is exactly one of:

- **stdio** — has a `command` (and usually `args`, `env`)
- **remote** — has a `url`

```json
{
  "mcpServers": {
    "fs":   { "command": "npx", "args": ["-y", "@modelcontextprotocol/server-filesystem", "/tmp"] },
    "acme": { "url": "https://mcp.acme.com/sse" }
  }
}
```

A server with neither `command` nor `url` is rejected, as is an empty `mcpServers`.

## Optional metadata

Top-level keys alongside `mcpServers` are read by the publisher tooling for the listing and ignored by the MCP-config validator:

| Key | Purpose |
|-----|---------|
| `name` | Connector name (also seeds the slug) |
| `title` | Clean Title Case display name (else derived from `name`) |
| `description` | Short card description (capped at 500 chars) |
| `category` | Marketplace shelf (a consumer category, e.g. "Build & connect") |
| `version` | Semver, default `1.0.0` |

```json
{
  "name": "filesystem",
  "title": "Local Filesystem",
  "description": "Browse and edit local files from chat.",
  "category": "Build & connect",
  "version": "1.0.0",
  "mcpServers": { "fs": { "command": "npx", "args": ["-y", "@modelcontextprotocol/server-filesystem", "/tmp"] } }
}
```

## Publishing

- **CLI:** `neboai publish ./my-connector` (auto-detected from `connector.json`). Add a `LISTING.md` for the long marketplace description.
- **MCP:** `connector(action: create, name: "...", manifestContent: "{...mcpServers...}")`, then `connector(action: submit, id, version)`. The full `mcpServers` JSON goes in `manifestContent`.

## Gotchas

- The manifest **must** validate as an `mcpServers` block even when carrying metadata — keep the block at the top level.
- `manifestContent` is the JSON itself, not a path.
- Remote servers should expose a stable `url`; stdio servers must name a `command` available on the installer's machine.
