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
| `upload_asset` | a NEW file (URL or local file, e.g. from another AI service) into the asset manager – never overwrites (409) |
| `validate` | the creator's checks (`@scutoons/content` validate.ts), one language or all + cross-language |
| `save_draft` | a new draft version: `ops` (set / insert / remove at a path) or the whole `content`, on a `base` version |
| `list_versions` / `restore_version` | history; restoring saves the old content as a new draft |
| `submit_for_review` | hands drafts to the toon's reviewers (for accounts that need a review before publishing) |
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

## The Ask service (the creator's "Ask" panel)

`src/ask.ts` + `src/ask-main.ts`: the agent behind the creator's Ask panel – one HTTP endpoint `POST /api/ask`
(Server-Sent Events). It checks the creator's session at the API (`/me/`, role creator), runs Claude (Sonnet 5.5) with
the tools above in-process, saves the chat in the API (`/api/agent-chats`) after every round and logs every AI call
(`/api/agent-usage`, else its own file). Locally: `npm run dev:ask` (port 9002; key `ANTHROPIC_API_KEY` in the
repo-root `.env`).

### Production: bunny.net Magic Containers

The image is built by GitHub (`.github/workflows/ask-image.yml`) on every push to `main` that changes the service:
`ghcr.io/mandymozart/scutoons-ask:latest` (and `:<commit>`). Build it by hand from the repository root:
`docker build -f tools/mcp/Dockerfile -t scutoons-ask .`

Once:

1. **Database** (phpMyAdmin): `server/migrations/mysql/2026-10-04-agent-chats.mysql.sql` (chat history) and
   `2026-10-04-agent-usage.mysql.sql` (usage log).
2. **Service key**: a long random value (`openssl rand -hex 32`) – GitHub `production` secret `AGENT_SERVICE_KEY` (goes
   into the API's `config.php` with the next API deploy) and the same value as `SCUTOONS_SERVICE_KEY` on bunny.
3. **bunny → Magic Containers → new app**: image `ghcr.io/mandymozart/scutoons-ask:latest` (GitHub registry; the package
   is private by default – give bunny a GitHub token with `read:packages`, or make the package public), port **8080**,
   health check `GET /api/ask/health`, a region in the EU, the smallest size. Environment:

   | name | value |
   |---|---|
   | `ANTHROPIC_API_KEY` | the Claude API key (platform.claude.com) – secret |
   | `SCUTOONS_API` | `https://api.scutoons.com/api` |
   | `ASK_ORIGINS` | `https://creator.scutoons.com` |
   | `SCUTOONS_SERVICE_KEY` | the service key from 2. – secret |

4. **Address**: `ask.scutoons.com` → the container (a CNAME to the address bunny shows). It must be a scutoons.com name:
   the creator's sign-in cookie is sent only there (and only under `/api`, which is where the endpoint lives).
5. **Creator build**: GitHub variable `VITE_ASK_URL` = `https://ask.scutoons.com/api/ask`, then deploy the creator.
   Without it the panel says "not available".

Updates: push to `main` → new image → bunny pulls it (automatic update on, or "redeploy" in the dashboard).

## Example

"Set up the chapter 2 stroll" (what `scripts/dev/stroll-chapter2.mjs` does): the agent reads `scutoons://rules/vn-events`
and `/flow`, finds the stroll groups with `list_chapters` (catpope, chapter 2: scene-14 … scene-18, scene-20), reads each
scene in de and en, adds the `stroll-end` section (label, setVariable add, if, addChoice with `jumpTo.target.anyOf`,
showChoices) with `save_draft` ops per language, and checks the result with `validate`.
