# @scutoons/mcp – the SCUTOONS MCP server

An agent (Claude Code, Claude Desktop, any MCP client) with the rights of one signed-in creator: it reads and edits
content through the existing API (`/api/entities`, `/api/assets`) with that creator's session token. The API's role
checks are the only authority – the agent can do exactly what that creator can. CUTOVER.md 10.28.

stdio only (step 1). Node ≥ 22, run from this repository (the shared packages are TypeScript sources, nothing is built).

## Tools

| tool | what |
| --- | --- |
| `list_toons` | toons, names, languages |
| `list_chapters` | chapters with their groups: label, scene entity, slices, script points |
| `read_entity` | a document by entity key – draft or published, one or every language, with version ids; `path` for a part |
| `read_characters` | the toon's cast, the registry's looks / portrait moods, the mood vocabulary |
| `read_map_places` | world map places |
| `search_assets` | media in the asset manager (paths to put into content) |
| `validate` | the creator's checks (`@scutoons/content` validate.ts), one language or all + cross-language |
| `save_draft` | a new draft version: `ops` (set / insert / remove at a path) or the whole `content`, on a `base` version |
| `list_versions` / `restore_version` | history; restoring saves the old content as a new draft |
| `publish_entity` | only with `SCUTOONS_MCP_PUBLISH=1`, and every call needs `confirm: true` |

Resources `scutoons://rules/*` (workflow, entity keys, VN events, flow rules, text presets, slices) are generated from
the source files when the server starts, so they follow the code.

## Safety model

- **Drafts only.** Every save is a new version (append-only); nothing is overwritten or deleted. Readers keep seeing
  the published version. Any version comes back with `restore_version` (also in the creator's history).
- **Validated.** `save_draft` runs the same checks as the creator first and refuses errors (`allow_errors: true`
  overrides); warnings come back with the result.
- **Conflicts.** `base` = the version the edit started from; a newer save in between → 409, nothing is saved.
- **Publish is off** unless the server is started with `SCUTOONS_MCP_PUBLISH=1`, and even then one explicit
  `confirm: true` per call.
- The token is the creator's session: whatever the API lets that account do. Use your own creator account, never a
  shared one; stop the session by signing out (or let it expire).

## Local (dev stack)

```bash
npm run dev:test-user
```

Without `SCUTOONS_API` the server uses the local API (`shared/dev-ports.json`) and the test user's session from
`logs/dev/test-user.json`. Claude Code, from the repository root:

```bash
claude mcp add scutoons -- node tools/mcp/bin/scutoons-mcp.mjs
```

Check (local only, restores what it changes):

```bash
npm run check:mcp
```

## Production (personal token)

The token is your creator session. Sign in to the creator (creator.scutoons.com), open the browser's developer tools →
Application / Storage → Cookies → the creator site → copy the value of `scutoons_sid` (domain scutoons.com). It is your browser's
session: signing out there ends the agent's access too. Keep it out of the repository and out of chats.

Claude Code (user scope, so the token is not in the project's `.mcp.json`):

```bash
claude mcp add scutoons-prod --scope user -e SCUTOONS_API=https://api.scutoons.com/api -e SCUTOONS_TOKEN=<token> -- node C:/Development/scutoons/tools/mcp/bin/scutoons-mcp.mjs
```

Claude Desktop (`claude_desktop_config.json`, Settings → Developer → Edit config):

```json
{
	"mcpServers": {
		"scutoons": {
			"command": "node",
			"args": ["C:/Development/scutoons/tools/mcp/bin/scutoons-mcp.mjs"],
			"env": { "SCUTOONS_API": "https://api.scutoons.com/api", "SCUTOONS_TOKEN": "<token>" }
		}
	}
}
```

Add `"SCUTOONS_MCP_PUBLISH": "1"` only for a session that should be able to publish.

## Example

"Set up the chapter 2 stroll" (what `scripts/dev/stroll-chapter2.mjs` does): the agent reads `scutoons://rules/vn-events`
and `/flow`, finds the stroll groups with `list_chapters` (catpope, chapter 2: scene-14 … scene-18, scene-20), reads each
scene in de and en, adds the `stroll-end` section (label, setVariable add, if, addChoice with `jumpTo.target.anyOf`,
showChoices) with `save_draft` ops per language, and checks the result with `validate`.
