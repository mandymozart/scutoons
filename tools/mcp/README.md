# @scutoons/mcp – the SCUTOONS MCP server

The member's own AI (Claude Desktop, Claude Code) working in SCUTOONS with the member's rights (agents/MCP.md). It runs
on the member's computer (stdio), started by their AI app, and calls the existing API with the member's **personal
token** (`sct_…`, scutoons.com/account → AI connections). The API's role checks are the only authority; the server only
picks which tools to list from the member's roles. Nothing runs on a SCUTOONS server, and SCUTOONS pays nothing for AI
(owner 2026-10-09: the in-app Ask panel is gone).

Readers (no member role) can't connect. ChatGPT / Cursor: not offered for now.

## Services by role

| role | service | tools |
| --- | --- | --- |
| creator | content (read + write) | `src/server.ts`: `list_toons`, `list_chapters`, `read_entity`, `list_entities`, `search_content`, `read_characters`, `read_map_places`, `search_assets`, `list_versions`, `validate`, `upload_asset`, `save_draft`, `restore_version`, `submit_for_review` – plus `src/story.ts` |
| admin (not creator) | content (read only) | the reading tools above + `src/story.ts` |
| creator / admin | story | `src/story.ts`: `read_flow` (SCUTOONS / toon / chapter / group – the Flow panel's own functions, stuck points), `story_variables`, `dead_things`, `item_placements`, `asset_usages`, `missing_assets`, `list_reviews`, `compare_versions` |
| creator / admin | storyboard (#122) | `src/storyboard.ts`: `read_storyboard` (the chapter's `storyboard:<toon>:<n>` as an outline with each shot's JSON path – edits go through `save_draft`), `storyboard_validate` (validateChapter + the rhythm), `storyboard_prompt` (a shot's Luma prompt + the files to attach found on this machine), `list_generations`; creators also `mark_generation`, `approve_generation`, `record_generation` (a generation made on the member's machine – a ledger row, nothing uploaded). Paid jobs: #121 |
| creator | Figma (the member's own file) | `src/figma.ts`: `figma_status`, `figma_read`, `figma_view` (a node as a picture), `figma_place_draft` (storyboard draft frames) – through the SCUTOONS Figma bridge on the member's computer (`tools/figma-bridge`: relay + development plugin in Figma desktop; no Figma token, no call cap) |
| liaison / ambassador / admin | Embassy | `src/embassy.ts`: `today`, `my_portion`, `search_orgs`, `read_org`, `list_team`, `list_releases`, `log_activity`, `set_lead`, `create_org`, `update_org`, `add_person`, `update_person`, `delete_person`, `draft_mail`, `check_inbox` |
| admin | admin | `src/admin.ts`: `dashboard`, `count_codes`, `list_codes`, `read_code`, `list_events`, `list_users`, `read_user`, `reading_stats`, `list_claims`, `read_claim`, `email_log`, `read_catalog`, `catalog_history`, `read_calendar`, `read_settings`, `propose_changes` |
| developer | migrations | `src/dev.ts`: `migration_status`, `plan_migrations` (a dry run: row counts, nothing changes), `apply_migrations` (that plan, backups, cache emptied) – the deploy runs pending ones by itself |
| everyone | – | `whoami` (member, services, the app links) |

**Prompts** (`src/prompts.ts`; Claude Desktop "+" menu, Claude Code `/`): `creator`, `embassy` (how SCUTOONS writes to
its network), `embassy-my-week`, `embassy-first-contact`, `admin`. **Resources** `scutoons://rules/*` (workflow, entity
keys, VN events, flow rules, text presets, slices) come from the source files (`src/resources.ts`), so they follow the code.

## Draft → link → the member confirms

The AI never publishes, sends or applies. It saves a draft and hands back the link (`open` in the result):

| tool | saves | the member, in the browser |
| --- | --- | --- |
| `save_draft` / `restore_version` | a new draft version (append-only, restorable) | checks it in the creator, **Publish** |
| `draft_mail` | a mail draft on the organisation (`/api/embassy/orgs/{id}/drafts`, checked like a send) | `embassy…/orgs/<id>?draft=<id>`: edits, **Send** (its author only) |
| `propose_changes` | a proposal (`/api/admin/proposals`) | `admin…/proposals/<id>`: before → after, **Apply** or Dismiss |

The API enforces it too: a personal token is refused (403 `not_for_ai`, `server/src/Http/ApiTokenScope.php`) on publish,
replacing a file in place, review decisions, sending a mail, admin changes other than proposals, sign-in and managing tokens. Saves through a token
are signed "(AI: <connection name>)" in the history. `save_draft` validates first (the creator's checks; errors refuse
the save unless `allow_errors`), and a newer save since `base` answers 409.

## Members: install

On scutoons.com/account → **AI connections** → New connection: the token is shown once, with the steps.

- **Claude Desktop:** download `scutoons.com/mcp.mcpb`, double-click, paste the token.
- **Claude Code:** download `scutoons.com/mcp` (= `scutoons-mcp.mjs`, Node ≥ 22), then
  `claude mcp add scutoons --scope user -e SCUTOONS_API=https://api.scutoons.com/api -e SCUTOONS_TOKEN=sct_… -- node <folder>/scutoons-mcp.mjs`.
  Claude Code's Remote Control (driving the session from claude.ai or the phone) works: the server runs where the session runs.

A token works for six months; renewing it on the account page keeps the same token (nothing to set up again), and a
mail comes two weeks before the end if the AI still uses it. Revoking it there ends the access at once.

## Build (the member's copy)

```bash
npm run build:mcp
```

`scripts/build.mjs` bundles the server with rolldown into ONE file (the format rules read at build time and embedded; the
API defaults to production) and packs `scutoons.mcpb` (MCPB manifest 0.3, the token as a sensitive setting) into
`server/mcp/` (gitignored) with `scutoons-mcp.json` (version, sha256). The api deploy builds it, so every api deploy
serves the current version: `GET /api/mcp/download` (`?file=mcpb`), `GET /api/mcp/version`; the short links
scutoons.com/mcp(.mcpb) redirect there. On start a member's copy checks the version and says in whoami / the first tool
result when a newer one is out (agents/MCP.md §3a). Bump `version` here and `VERSION` in src/member.ts together.

## Local (dev stack)

```bash
npm run dev:test-user
```

Without `SCUTOONS_API` the server (run from the repository) uses the local API (`shared/dev-ports.json`) and the test
user's session from `logs/dev/test-user.json`. Claude Code, from the repository root:

```bash
claude mcp add scutoons -- node tools/mcp/bin/scutoons-mcp.mjs
```

Against production from the repository (developers): `-e SCUTOONS_API=https://api.scutoons.com/api -e SCUTOONS_TOKEN=sct_…`
(user scope, so the token is not in the project's `.mcp.json`).

Check (local only, restores what it changes):

```bash
npm run check:mcp
```

`npm run check -w tools/mcp` = tsc + the unit tests (`test/*.test.ts`, `npm run test -w tools/mcp`). `storyboard_prompt`
finds the files to attach in a checkout's `storyboards/` + `media/`, or in `SCUTOONS_STORYBOARDS` / `SCUTOONS_MEDIA`.

## Example

"Set up the chapter 2 stroll" (what `scripts/dev/stroll-chapter2.mjs` does): the AI reads `scutoons://rules/vn-events`
and `/flow`, finds the stroll groups with `list_chapters` (catpope, chapter 2: scene-14 … scene-18, scene-20), reads each
scene, adds the `stroll-end` section (label, setVariable add, if, addChoice with `jumpTo.target.anyOf`, showChoices) with
`save_draft` ops, checks the result with `validate` and `read_flow`, and gives the creator the links to check and publish.
