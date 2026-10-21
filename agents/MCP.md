# MCP – replacing Ask with "bring your own AI" (analysis 2026-10-09)

Owner 2026-10-09: the Ask service goes because it costs too much for what it does. People use their own AI plan
(Claude; ChatGPT skipped for now, owner 2026-10-09) and connect it to SCUTOONS through the MCP server. Their plan pays for the model, and
SCUTOONS pays for no tokens. This file covers the analysis, the target, auth and the removal. Status: **built 2026-10-09** (phases 1–5, §7); production steps, the drop of the Ask tables and the hosting clean-up are done (owner 2026-10-12). "What exists today" (§1) and "Removing Ask" (§4) describe the state BEFORE.

## 1. What exists today

| piece | where | what it does |
| --- | --- | --- |
| MCP server (stdio) | `tools/mcp/src/main.ts`, `server.ts` (698 lines) | **creator tools only**: 17 tools (list / read / search / validate / save_draft / versions / upload_asset / submit / publish behind a flag), resources `scutoons://rules/*` |
| Embassy tools | `tools/mcp/src/embassy.ts` (464) | 15 tools: orgs, people, leads, log, `my_portion`, `draft_mail`, `check_inbox`. **Only reachable through Ask** |
| Admin tools | `tools/mcp/src/admin.ts` (489) | 16 read tools + `propose_changes`. **Only reachable through Ask** |
| Ask service | `tools/mcp/src/ask.ts` (656), `ask-main.ts` | Anthropic API agent loop, the system prompts per app (writing rules for Embassy!), web search, chat saving, usage and cost log. Runs on bunny Magic Containers |
| Auth | `server/src/Http/Middleware/Session.php`, `Domain/Auth/Sessions.php` | `Authorization: Bearer <session>` or the SSO cookie `scutoons_sid`, which is the same opaque token (SHA-256 in `sessions`). **No personal access tokens and no OAuth.** The MCP README tells people to copy their `scutoons_sid` cookie out of the dev tools |

The tools and the API's role checks are already the right core. The parts that cost money (the agent loop, the model
and the chat storage) are Ask-only, and Ask is the only part that goes. **Two things live only in Ask and have to move:**

1. **The embassy and admin tools.** The stdio server registers only `createScutoonsServer`.
2. **The system prompts** (`CREATOR_SYSTEM`, `EMBASSY_SYSTEM`, which holds the owner's communication rules, and
   `ADMIN_SYSTEM`). Without them an outside AI doesn't know how SCUTOONS wants mails written.

There are also two **panel-only results**: `draft_mail` and `propose_changes` return a "card" that only the Ask panel
can show. An outside client shows it as JSON, and nobody can press Send or Apply.

## 2. Target

```
the member's own computer                                   SCUTOONS
┌──────────────────────────────────────────────┐
│ Claude Code / Claude Desktop                  │
│   └─ starts scutoons-mcp (stdio, Node) ───────┼─ HTTPS, Bearer sct_… ──► api.scutoons.com (PHP: role checks, tokens)
└──────────────────────────────────────────────┘
```

**No SCUTOONS server on the internet for AI** (owner 2026-10-09, D3): the bunny container goes, no OAuth, no remote
MCP. The MCP server runs on the member's computer, started by their AI client, with a personal token. SCUTOONS pays
nothing for AI: no Anthropic API key (platform.claude.com) any more – the member's own Claude plan (or other AI plan)
runs the model.

What reaches it (owner 2026-10-09):
- **Claude Code, Claude Desktop:** yes – they start local MCP servers. (Cursor could too; not offered for now, owner 2026-10-09.)
- **Claude Code Remote Control** (the session runs on the owner's computer, driven from claude.ai / the phone): yes –
  the MCP server runs where the session runs, so no OAuth is needed. A Claude Code *cloud* session (claude.ai/code
  without a linked computer) would need the server started in that cloud checkout with the token as a secret – not
  planned.
- **ChatGPT / OpenAI: skipped for now** (owner 2026-10-09). Nothing is built or tested for it; the setup help names
  Claude only. (Codex CLI could start the same local server with the same token – revisit when wanted.)
- **claude.ai custom connectors (and ChatGPT's app):** no – they only connect to MCP servers on the
  internet (remote HTTPS + OAuth). Accepted; revisit only if needed (it would bring back a small host + OAuth).

The Ask service's auth (the SSO cookie to `ask.scutoons.com`, `AGENT_SERVICE_KEY` / `X-Service-Key`) goes with Ask;
the only AI auth left is the member's personal token, the same for every client.

- **The member's own permissions, per service** (owner 2026-10-09). The AI is the member, never more. The **API stays
  the only authority**: every tool call carries the member's token, so the role checks that hold in the apps apply
  unchanged (creator → content, toon reviews / reviewer rules; liaison / ambassador → Embassy, an ambassador's wider view;
  admin → admin). The MCP server **only picks which tools to list** from `/me/` roles at connect, so the AI doesn't see
  tools it would get 403s from. That's the `APPS` table from ask.ts, minus the agent loop; a role taken away applies
  on the next call (the API) and at the next connect (the list).
- **Prompts move into the server:** MCP `instructions` (short: what the server does and where to start) plus MCP
  **prompts** the user can pick (Claude shows them in the "+" menu, Claude Code as `/` commands):
  `embassy-first-contact`, `embassy-my-week`, `creator-edit-scene` … carrying the texts that are in ask.ts today. Format
  rules stay resources.
- **Web search, chat history, the model:** the client's job. SCUTOONS drops `agent_chats` and `agent_usage`.
- **What stays in SCUTOONS for audit:** every save names the token ("(agent: <token name>)", today a plain "(agent)"), and the token page shows "last used".

### Draft → link → the member confirms in the browser (owner 2026-10-09, D1 decided)

The rule for every outward or wide-reaching step: **the AI saves a draft and returns a link; the member opens it,
checks it and confirms in the app.** The AI never sends, applies or publishes.

| step | the tool saves | the link opens | the member confirms |
| --- | --- | --- | --- |
| content edit | a draft version (as today) | the creator at that document (Reader / History show it) | **Publish** in the creator |
| Embassy mail (`draft_mail`) | a mail draft on the organisation (new `embassy_mail_drafts` row: org, person, subject, body, reply_to, author, client) | `embassy…/orgs/<id>?draft=<id>`: the existing `mail-draft.svelte`, editable | **Send** (the existing EmbassyMail path; the draft is deleted after) |
| admin changes (`propose_changes`) | a stored proposal (new `admin_proposals` row: the changes as today's card JSON, base versions, author, client, status open / applied / dismissed) | `admin…/proposals/<id>`: today's `changes-card.svelte` (before → after) as a page | **Apply** (the admin app makes the changes as today; catalog history + undo) or Dismiss. Catalog: only `toons` – items / recipes / albums / products / editions are documents (D6), changed with `save_draft` (`item:<id>` …); a proposal for them failed on Apply (409 edited_in_creator), so the tool refuses them (2026-10-16) |

- A draft belongs to its author; the link works only signed in as a member who may do the step (the API checks again
  on Send / Apply / Publish – a role lost in between → 403).
- Conflicts: Apply checks the base versions (calendar, catalog) like today's card – changed meanwhile → "out of date,
  ask the AI again".
- Open drafts are visible where they act: "Drafts (n)" on the org page and the Embassy start page, "Proposals (n)" in the
  admin. Old ones: dismissed by hand; no auto-expiry until it's needed.
- `publish_entity` leaves the MCP server (also stdio): publishing is the creator's button (D4 decided by the same rule).
  Asset replace-in-place: a proposal like the others (the page shows how many uses change).

## 3. Auth: personal access tokens (decided 2026-10-04, built 2026-10-09)

The API takes them like a session (`Authorization: Bearer`). Claude Code and Claude Desktop pass them as an env
variable to the local server (`SCUTOONS_TOKEN`), as today with the copied cookie.

- Table `api_tokens`: id, user_id, name ("Claude on my laptop"), `token_hash` (SHA-256, like sessions), prefix `sct_`
  (never confused with a session), `created_at`, `last_used_at`, `expires_at` (optional), `revoked_at`.
- **Six months** (owner 2026-10-09, built): a token expires 182 days after it was made / renewed; "Renew" on the
  account page (a real sign-in) moves the same token on; used in its last 14 days → one reminder mail (no cron).
- `Session.php`: a `Bearer sct_…` → that user with their current roles (a role change applies at once); `last_used_at`
  updated at most once a minute.
- **Limits:** a token can't become a cookie (`POST /api/auth/session/` refuses it) and can't create tokens, change the
  password or email, or delete the account – those routes are marked "session only". The draft links (§2) are opened
  in the browser with the member's normal sign-in, never with the token. Every confirm step of §2 is refused for a token
  too (`ApiTokenScope`, 403 not_for_ai): publish (one document or the batch), replace a file in place, an Embassy mail,
  review decisions, admin changes but proposals – the list: server/docs/API.md "AI connections". Invites and gifts are
  the member's rights through the AI too: the admin's campaigns and the limits decide (owner 2026-10-21).
- UI: the account page (scutoons.com/account), section **"AI connections"**: create (name → the token shown once with a
  copy button and the setup steps for Claude Desktop / Claude Code), list (name, created, last used),
  revoke. One page for every role; only members with a role that has tools (creator, liaison, ambassador, admin) see it
  (D2).
- **Readers can't connect** (owner 2026-10-09): only members with a role that has tools (creator, liaison, ambassador,
  admin). The API refuses to create a token for anyone else (403), and a token stops working once its account has
  none of those roles left (401 "no member role" – checked on every request, like the role checks themselves). The MCP
  server says so at start and exits. Readers don't see "AI connections" on the account page.
- PHPUnit: token auth = the user's rights; revoked / expired → 401; session-only routes → 403 with a token; no cookie
  from a token.
- The MCP README stops saying "copy the cookie".

## 3a. Getting the server onto a member's computer

Today `tools/mcp` runs from a repository checkout (TypeScript sources, workspace packages, and `resources.ts` reads
the format rules from creator / viewer source files at start). Fine for the owner, not for a liaison. So:

- **Build one file**: `npm run build:mcp` bundles the server (esbuild) into `scutoons-mcp.mjs` with the rule texts
  embedded at build time – needs only Node ≥ 22, no checkout, no npm install.
- **Claude Desktop extension** (`scutoons.mcpb`, the bundle + a manifest): double-click → Claude Desktop installs it and
  asks for the token (a `user_config` field, kept in the OS keychain), the **Workspace folder** (`directory` →
  `SCUTOONS_WORKSPACE`) and the **ComfyUI address** (→ `COMFY_URL`, default http://127.0.0.1:8188); both optional – an
  empty one (or its unreplaced `${user_config.…}`) counts as unset (config.ts dropUnsetSettings). No Node install needed
  (Claude Desktop brings its own). Best path for non-developers.
- **The local kit** (#127, research/STUDIO-IN-CREATOR.md §2.4 – the same for every member): the MCP **carries the Figma
  relay** (`src/figma-relay.ts` + the pure `figma-relay-core.ts`: a TS port of tools/figma-bridge/relay.mjs, the same
  protocol on 3056 – tested against relay.mjs with the same stand-in-plugin scenarios). A figma_* call that finds nothing
  on the port starts it in the MCP process (no `supervise.mjs`); a running relay (relay.mjs, another MCP) is used. The
  command modules (`tools/figma-bridge/commands/*.js`) are embedded at build time (`figma-commands.ts` replaced). It
  starts by itself **only in the one-file copy / extension** – from a checkout only with `SCUTOONS_FIGMA_RELAY=mcp`
  (`off` turns it off anywhere): the owner's production scripts keep relay.mjs under supervise.mjs on the same port, and
  an MCP relay grabbing it during a relay restart would lock the supervisor out (2026-10-21, ch3). **The Figma plugin**
  is a download built with the MCP: `server/mcp/scutoons-figma-plugin.zip` (`scutoons-figma-plugin/` manifest.json,
  code.js, ui.html as in the checkout – only the UI's hint names the MCP –, README.txt), Figma desktop → Plugins →
  Development → Import plugin from manifest. **ComfyUI** (optional): `generate_local` (§9). The check: whoami's `kit`.
- **Switching the owner's checkout later** (after the art freeze, #127's second part): set `SCUTOONS_FIGMA_RELAY=mcp` for
  the checkout's MCP and stop the supervisor (`node tools/figma-bridge/supervise.mjs --stop`) – the scripts'
  `figma()` (scripts/storyboard/lib.mjs) would still start supervise.mjs when no relay answers, so that switch comes
  with the figma-* scripts becoming MCP tools; until then nothing changes for the checkout.
- **Shipped with the API** (no third party, owner 2026-10-09 "alles bei uns"; with the API since 2026-10-20, issue 76:
  "den MCP vom Server aus mitgeben"): the api deploy runs `tools/mcp/scripts/build.mjs mcp` in its staging copy
  (deploy/targets.json; `tools/mcp/**`, the text presets, scripts/vectors-core.mjs and the shared packages it imports
  redeploy the api, not the viewer). `server/mcp/` = scutoons-mcp.mjs, scutoons.mcpb, scutoons-mcp.json
  { version, git_sha, sha256, sha256_mcpb, built_at } (locally `npm run build:mcp`, gitignored). McpController:
  `GET /api/mcp/download` (the .mjs; `?file=mcpb` the extension; `?file=figma-plugin` the plugin zip; attachment, ETag,
  5 minutes) and `GET /api/mcp/version` ({ version, git_sha, sha256, sha256_mcpb, sha256_figma_plugin,
  figma_plugin_version }); not built → 404 not_built.
- **Links:** members get the short links `scutoons.com/mcp`, `scutoons.com/mcp.mcpb` and `scutoons.com/figma-plugin`
  (viewer/static/.htaccess → 302 to the API) – the account page's "AI connections" uses them, with a folded "Set up your
  machine" (the three steps + "ask your Claude whoami"; viewer account/machine-setup.svelte). The old `scutoons.com/ai/…` URLs are gone
  (owner 2026-10-20).
- **Updates:** a member's copy (bundle / extension; a checkout never) asks `/api/mcp/version` once on start
  (tools/mcp/src/update.ts, 3 s timeout, silent on errors). Newer = a higher version (tools/mcp/package.json = `VERSION`
  in member.ts – bump both) or the same version with another sha256 (a rebuild). Then whoami gets an `update` field and
  the first other tool result one line: "Eine neue SCUTOONS-MCP-Version ist da: https://scutoons.com/mcp" (`/mcp.mcpb`
  for the extension). No auto-update: the member downloads, replaces the file, restarts Claude.
- Claude Code: `claude mcp add scutoons --scope user -e SCUTOONS_TOKEN=sct_… -- node <path>/scutoons-mcp.mjs`
  (the snippet on the token page fills in the token).
- `upload_asset` from a local file works (it's their computer) – as today.

## 4. Removing Ask (the inventory, 2026-10-09)

Order: **build the replacement first** (Embassy and admin tools reachable, drafts stored), then remove. Otherwise
liaisons lose their tool in between.

- **Packages:** `shared/ask/` (whole), `shared/api/src/agent-chats.ts` (+ exports in `index.ts:169-173`), the
  agent-usage and propose types in `shared/api/src/admin.ts:673-702+` (some types move with `propose_changes`),
  `shared/dev-ports.json` "ask".
- **AppShell** (`shared/ui/src/app-shell.svelte`): the `ask` snippet, `askLabel`, askWidth / askRailed, the grid
  column, `.sc-shell-ask`. DOCKING.md rules that point at "Ask's header buttons" get a new reference (the dock's header
  buttons). Keep `askDialog` (it's a confirm dialog, unrelated).
- **creator:** layout (askRegion, toggle), `components/ask/`, `lib/ask.svelte.ts`, `agent-saves.svelte.ts` (+
  `doc-session.svelte.ts:13,63`), `api.ts` / `config.ts` ASK_URL, `.env.example`, `test/chat-markdown.test.ts`, the
  `creator.ask_*` messages (keep the VN `askText` ones).
- **admin:** layout, `changes-card.svelte`, `ai-usage.svelte` (+ the dashboard), api / config, the `ask` and `ai_*`
  messages.
- **embassy:** layout, `+page.svelte` (askAbout, `portion.ask`), `orgs/[id]`, `ask-page.svelte.ts`, `due-card.svelte`
  props, api / config, messages. `mail-draft.svelte` **stays** (manual mail, and it becomes the stored-draft editor).
- **server:** routes `/api/agent-chats` (4), `POST /api/agent-usage`, `/admin/agent-usage`; `Domain/Agent/*`,
  `AgentChatsController`, `AgentUsageController`, `AGENT_SERVICE_KEY` (settings, config example), Cors DELETE, the two
  tests, the API.md / DATABASE.md / CONFIGURATION.md sections.
- **DB:** a migration `DROP TABLE agent_chats, agent_usage` (+ the web-search columns). Per the stored-format rule, look
  first: no export (owner 2026-10-09, D5: not needed) – drop the tables. `2026-10-04-ask-embassy-mail` is
  mixed (it holds the Embassy mail tables), so drop only the `.app` columns it added, if any.
- **infra:** `.github/workflows/ask-image.yml`, `tools/mcp/Dockerfile` + `Dockerfile.dockerignore`, `bin/scutoons-ask.mjs`,
  `src/ask.ts` / `ask-main.ts` (their system prompts move to MCP prompts first), the `@anthropic-ai/sdk` and
  `@modelcontextprotocol/client` dependencies;
  `deploy/targets.json` (AGENT_SERVICE_KEY, VITE_ASK_URL ×3), `deploy/README.md`, root `package.json` `dev:ask`,
  `.env.example`, `.claude/launch.json`, `scripts/dev/up.mjs` / `launch-json.mjs`; the 4 browser checks that click
  `ask-toggle` (creator-dock, creator-flow-dock, creator-reader, tools-dock); README.md.
- **Production, after the deploy:** delete the bunny Magic Containers app, remove the `ask.` DNS and the
  ghcr.io image `scutoons-ask`, revoke
  the Anthropic API key, remove the GitHub vars `VITE_ASK_URL` and `BUNNY_ASK_*` and the secret `AGENT_SERVICE_KEY`.
- **docs:** BACKLOG 10.28 (step 2 decisions are superseded), HANDOVER (the UI rules mention Ask), DOCKING (38 hits),
  EMBASSY, DESIGN, UI-CONCEPTS, STORYBOARD §7.

## 5. Phases

| # | what | gives |
| --- | --- | --- |
| 1 | MCP server by role (Embassy and admin tools in stdio too), prompts out of ask.ts, `draft_mail` + `propose_changes` → stored drafts with a confirm link (embassy draft view, admin `/proposals/<id>`), `publish_entity` removed | everything Ask could do, for the owner's Claude Code / Desktop |
| 2 | personal access tokens + "AI connections" on the account page | no more cookie copying; revocable |
| 3 | **remove Ask** (§4), the bunny app deleted | the cost is gone |
| 4 | one-file build + Claude Desktop extension, served from our host, version check (§3a) | members without a checkout can connect |
| 5 | a short "Connect your AI" help (on the token page: Claude Desktop = install the extension, paste the token; Claude Code = the snippet) | people can do it alone |

## 6. Owner decisions

- ~~D1~~ decided 2026-10-09: draft saved, link, the member confirms in the browser (§2). ~~D4~~ follows: no publish tool.
- ~~D2~~ decided 2026-10-09: every member with a role (creator, liaison, ambassador, admin) sets it up themselves –
  the Claude Desktop extension + a personal token from their account page; developers the one-line command (§3a).
- ~~D3~~ decided 2026-10-09: no server on the internet – the bunny container goes, no OAuth; local clients with a
  personal token only (§2, §3, §3a).
- ~~D5~~ decided 2026-10-09: no export – the chat and usage tables are dropped.
- ~~D6~~ decided 2026-10-09: every member gets an AI plan – no fallback needed beyond the normal apps.

## 7. Built (2026-10-09)

| commit | what |
| --- | --- |
| 2dac9209 | one server by role (`member.ts`, `services.ts`), prompts (`prompts.ts`), story tools (`story.ts`), no publish tool, the Ask service deleted; `@scutoons/content` toon-flow-input.ts |
| 560104a1 | Ask out of the apps and the API; Embassy mail drafts + admin proposals (API, MCP, admin Proposals page) – migration `2026-10-09-mcp-drafts` |
| dc7d1d36 | Embassy: drafts in the org page's dock (?draft=<id>), "Drafts" on the start page; DialogHost in the Embassy |
| e966f508 | AI connections: `api_tokens` (migration `2026-10-09-api-tokens`), `/api/me/tokens`, ApiTokenScope (403 not_for_ai), saves signed "(AI: <name>)", the account page section; `npm run build:mcp` → scutoons.com/ai/scutoons-mcp.mjs + scutoons.mcpb (the viewer's build) |

Deviations from the plan above: the token page is on the viewer's account page with an inline revoke confirm (the
viewer has no DialogHost); `delete_person` stays allowed for a token (internal data, on an explicit request – the prompt
says so); `asset_usages` moved to the story tools (every content editor); the drop of the Ask tables is parked in
`server/migrations/mysql/2026-10-09-drop-ask.mysql.sql` (run on production 2026-10-09).

## 8. Developer: migrations (BACKLOG 10.61, built 2026-10-11)

A role `developer` above admin (the owner's account; only a developer grants it, the last one stays). With it the
member's normal AI connection lists `migration_status`, `plan_migrations` (a dry run) and `apply_migrations` (that plan)
(`src/dev.ts`) – an exception to "Draft → link" by the owner's choice (2026-10-11: agents run production migrations
from the computer; no admin UI). Only the files that came with the deploy run, never SQL from the agent. The deploy
itself runs pending migrations after the API upload (`/api/deploy/migrate`, MIGRATE_KEY). Details:
server/docs/DATABASE.md "Migrations", docs/API.md "Developer API".

A developer who writes content also gets `import_vectors` (`src/vectors.ts`, owner 2026-10-18: "Add an MCP tool that
writes the shapes as my account"): the vector library of this checkout into the asset manager as the member – the same
core as the local `npm run vectors:import` (`scripts/vectors-core.mjs`); dry run by default, `limit` per call, never
replaces or deletes a file, stops on a path held by a non-shape (research/VECTORS.md §6 step 5).

Open:
- **Owner:** install the extension once in Claude Desktop (account page → AI connections) – not tried (no Claude Desktop
  in the agent's reach).
- The four browser checks that lost their Ask steps (creator-dock, creator-flow-dock, creator-reader, tools-dock) were
  not run after the change – ISSUES 10.87.

## 9. Storyboard (#122, 2026-10-21)

The chapter storyboard `storyboard:<toon>:<n>` and its generations (research/STUDIO-IN-CREATOR.md §2.3, server/docs/API.md
"Storyboard"), `src/storyboard.ts` (tools) + `storyboard-model.ts` (pure: outline, shot paths, record bodies – unit tests
in `test/`) + `storyboard-refs.ts` (the files on the member's machine). Content editors; an admin who isn't a creator reads.
- `read_storyboard` (an outline: scenes → groups → shots with their JSON `path` for save_draft ops; `scene` / `shot` /
  `full` add the JSON), `storyboard_validate` (content's validateChapter + summarize's rhythm; `ops` = check before
  saving), `storyboard_prompt` (content's lumaPrompt; the render / style / character files when this machine has a
  `storyboards/` folder – a checkout, or `SCUTOONS_STORYBOARDS`), `list_generations`.
- Writes (creators): `mark_generation`, `approve_generation` (the record only), `record_generation` (a generation made on
  the member's machine – a row in the ledger, nothing uploaded). Edits of the document: `save_draft` – the storyboard's
  checks run there too (errors refuse the save), and every result links the creator's `/<toon>/chapter/<n>/storyboard`.
- Paid provider jobs (#121, server/docs/API.md "Storyboard" → "Provider jobs"): `list_generators` (what can run, the
  prices, the budget in euro), `start_generation` (SPENDS MONEY – the API's budget gate decides; local `file`
  references are uploaded to the work storage first, the server hands the provider signed links; `estimate_only`),
  `check_generation` (polls once; a finished result is saved into `SCUTOONS_WORKSPACE/<toon>/<chapter>/<shot>--g<n>.<ext>`
  and the row gets file + sha256 + machine – `SCUTOONS_MACHINE` or the host name), `cancel_generation` (Runway only).
- The member's own GPU (#127): `generate_local` (`src/storyboard-local.ts` + the pure `comfy-local.ts`) – comfy.mjs's
  Qwen-Image-Edit 2509 graph (image 1 = the shot's render, found here or given as `still`; up to two guides, default the
  character images) posted to `COMFY_URL`; models / LoRA from `COMFY_UNET` / `COMFY_CLIP` / `COMFY_VAE` / `COMFY_LORA`
  (`none` = no Lightning) / `COMFY_STEPS` / `COMFY_CFG` / `COMFY_MP` / `COMFY_CHAR_LORA` / `COMFY_REF_METHOD`, comfy.mjs's
  defaults. Prompt: the shot's lumaPrompt unless given. Waits up to `wait` s (default 50), else `running` + a prompt_id
  to collect with a second call. The result → `SCUTOONS_WORKSPACE/<toon>/<chapter>/<shot>--g<n>.png` + a record
  (generator comfy-local, model qwen-image-edit, seed, params, cost 0). No money, no budget. ComfyUI down → what to run.
  Not ported: comfy.mjs's owner-specific character descriptions, `--fresh` server per image, the greyscale pass.
- whoami → `kit` (creators): the workspace folder, the Figma relay (which one) + plugin (file, code version), ComfyUI
  reachable – each with its next step (src/kit.ts; probes ≤ 1.5 s, it never starts the relay).

## 10. Story texts: scripts and the Kanon (#84, 2026-10-21)
- Resource `scutoons://rules/story` (tools/mcp/src/resources.ts, generated from kanon.ts / script.ts): the keys
  `season-script:` / `chapter-script:<toon>:<n>:<l>` and `kanon:<subject>:<l>`, each role's sections (de / en), what
  generators read (Look / Scenery), drafting = `save_draft` on a stored key with `{ text }`, never publish (publishing
  makes it Kanon – the member, in the creator), the creator links to give. research/KANON.md.
- `read_entity` / `validate` / `save_draft` take the kinds like any localized document; a character's or place's Kanon
  text (no toon) uses the universe's languages (every toon's, `de` first – `universeLanguages`). `validate` warns on an
  unknown heading, one twice or out of order. `check:mcp` covers the resource, a Kanon validate and a character's read.

## 11. Unused storage files (#152, 2026-10-21)
- `unused_files` (admin, read only – src/unused.ts): the admin's Assets → Unused list (`GET /admin/assets/unused?all=1`,
  the API decides who may) as groups (a map tile set as one, else by folder; files, bytes, what) + totals; `files: true`
  lists path / size, paged. Index = published + newest draft only, so check older versions before proposing a delete.
  No delete tool: the cleanup stays in the admin.
- `old_versions` (admin, read only, #154): the dry run of Data → Old versions (`GET /admin/content/old-versions`) –
  history versions before a day (default 30 days ago) per document, totals; no purge tool.
