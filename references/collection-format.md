# collection.json Format

A collection is a **bundle of existing published artifacts** that installs as one unit — a starter pack. It references other artifacts by ID; installing the collection cascades to install each item. Install codes are `COLL-XXXX-XXXX`.

## Shape

```json
{
  "name": "Sales Stack",
  "title": "Sales Stack",
  "description": "Everything for outbound sales, installed in one paste.",
  "category": "Find customers",
  "version": "1.0.0",
  "items": [
    { "targetId": "0e1f…uuid", "targetType": "skill", "position": 0 },
    { "targetId": "7a2c…uuid", "targetType": "agent", "position": 1 }
  ]
}
```

| Key | Required | Purpose |
|-----|----------|---------|
| `name` | yes | Collection name (seeds the slug) |
| `title` | no | Clean display name (else derived from `name`) |
| `description` | no | Short card description (≤500 chars) |
| `category` | no | Marketplace shelf |
| `version` | no | Semver, default `1.0.0` |
| `items` | no | Array of members; can be added after create |

Each `items` entry needs:
- `targetId` — the **UUID** of an already-published artifact (not an install code)
- `targetType` — `skill`, `plugin`, `agent`, `app`, or `connector`
- `position` — optional ordering (the CLI assigns order by array index)

## Publishing

- **CLI:** `neboai publish ./my-collection` creates the collection (`POST /collections`), adds each item (`POST /collections/{id}/items`), sets the listing, and submits when public. Add a `LISTING.md` for the long description.
- **MCP:**
  ```
  collection(action: create, name: "Sales Stack", description: "...")
  collection(action: add-item, id: "<coll-id>", targetId: "<artifact-uuid>", targetType: "skill")
  collection(action: submit, id: "<coll-id>", version: "1.0.0")
  ```

## Visibility & install

Collections support the standard visibilities plus `invite_only` (requires an artifact invite code to install). Install onto a bot with `collection(action: install, id, botId)` (or the install code in chat); the platform resolves and installs every member.

## Gotchas

- Items must reference **published** artifacts by UUID — resolve names/codes to IDs first (`marketplace(action: search, ...)` or `<type>(action: get, ...)`).
- A collection has no payload of its own; it is purely the bundle definition.
- Removing an item from the collection does not uninstall it from anyone who already installed the bundle.
