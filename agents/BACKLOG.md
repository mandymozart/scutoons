# BACKLOG — everything open after the beta (one list)

One list for the open work (owner 2026-10-06: "we need to merge cutover post items and minor bugs"): the former
CUTOVER.md Phase 10 ("after the beta", items keep their numbers 10.x so commits and docs that cite "CUTOVER 10.x" still
find them here) and the known bugs that were in MEMORY.md. CUTOVER.md stays the runbook of the cutover itself.

New entries: the next free number (10.39, …), owner + date, then **O –** for steps the owner runs on production. Tick
an item ([x]) when it is done and move it to "Done" at the bottom.

## Priorities (owner 2026-10-06 evening: "prioritise now and then set a plan to work through it alone through the night")

**Night run 2026-10-06 → 07 (agent alone, in this order).** Rules: commit / merge on `main` in small steps, **never
push** (a push deploys – owner: "just merge to main, but do not push to production"), nothing on production; the dev DB
is free to change (owner: "you can do anything to the dev database" – its content, the sandbox's and catpope's stories,
is the test case for migrations); `npm run check` + the step's browser checks before each commit; a step that needs an owner
decision stops there – the question goes into "Morning: for the owner" and the next step starts.

1. **10.38 Items are not variables** – step 2 (warn), step 3 (`giveItem`), step 5 (analysis + migration + drop the
   bridge's variable side effect); step 4 (`takeItem`) as a proposal only (owner decision).
2. **B.1–B.3** – re-check the three old bugs on today's code; fix what is still broken, close the rest.
3. **`tokens-switch-knob`** – the one unmerged commit (tokens check) after a check run.
4. **10.20** Asset picker: the selected tile's icon inverted, the size label (small, no layout change).
5. **10.13** Visibility rule: why the signed-in creator's viewer showed draft chapters 4–5 as "coming soon" – find, fix.
6. **10.11** Server errors in admin: the admin-only log endpoint + the "Server errors" page.
7. **10.22** Creator chapter tree: one "Add group" (the storage is the editor's business), scene keys never as names.
8. **10.18** Batches + code generation from the database instead of the static lists.
9. **10.19** One setup step for a fresh environment (migrate → seed → asset scan, each checked).

**Morning: for the owner** (production steps, decisions – the night run doesn't touch them):
- Production steps still open (which of these already ran?): 10.1c `check:fonts` on production, 10.3 steps 6–8,
  10.5 qr deploy with `PROXY_FULL_API=0`, 10.24 code-case migration, 10.32–10.35 migrations, 10.37 `content:check`.
- Decisions: 10.12 (link or drop credits / download), 10.14 (claim page – details), 10.21 (picker drafts), 10.25 (map
  places ↔ story), 10.26 (delete-a-chapter rules), 10.38 step 4 (`takeItem` rules).
- With the owner, not alone: 10.17 atomic deploys (only testable by deploying), 10.23 asset jobs through the API,
  10.2 / 10.4 asset layer, 10.7 / 10.9 cleanups, 10.8 the release.
- 10.1 ("fix the beta's issues on `platform`") – `platform` is abandoned, everything is on `main`: close it?

## Open

- [ ] 10.1 Fix the beta's issues on `platform` → `main` (deploys automatically).
- [ ] 10.1c Fonts (IBM Plex Mono, Bangers + OFL licences) to `scutoons/fonts/` on the CDN. Branch `10.1c-cdn-fonts`
      (2026-10-05): staged in `media/scutoons/fonts/` (18 files); the apps and qr's landing page load `@font-face` from
      `<CDN>/scutoons/fonts/` with their local copy as fallback (ASSETS.md "Fonts").
      Merged into platform 2026-10-05 (owner: "merge it now", after 10.3). Uploaded by hand to
      `scutoons/fonts/` and registered (admin Rescan: 18 new assets) 2026-10-05; cdn.scutoons.com answers 200 + ACAO \*.
      Tick after `check:fonts` on production shows 200s from the CDN.
- [ ] 10.2 Asset layer: merge `assets` (migration `2026-10-02-assets` in phpMyAdmin, `ASSET_STORAGE=bunny` + `BUNNY_*`
      into `production`/targets, `assets:scan`) – ASSETS.md.
- [ ] 10.3 cdn.scutoons.com → bunny _(code ready on branch `10.3-cdn-domain`, 2026-10-05; steps 1–5 done + verified 2026-10-05, GitHub `CDN_PUBLIC_URL` changed; open: 6 deploy, 7 optional, 8 verify, local `.env`)_:
      custom hostname on pull zone 6731366 + CNAME → `scutoons.b-cdn.net` + SSL; then every CDN URL in `deploy/targets.json`
      (api `CDN_URL` + `CDN_PUBLIC_URL`, qr `CDN_URL`, every app's `VITE_CDN_URL`) = `https://cdn.scutoons.com` + redeploy.
      The legacy system is gone, so nothing live uses cdn.scutoons.com: the apps stay on `scutoons.b-cdn.net` until the
      redeploy, which comes only after cdn.scutoons.com is verified on bunny → **no downtime for the apps**. Only old deep
      links to cdn.scutoons.com change source (up to the 2 h KAS TTL, plus a few minutes without a certificate until step 4).
  - **State before (read-only, 2026-10-05):** cdn.scutoons.com = KAS A record `85.13.151.120`, TTL 7200
    (ns5/ns6.kasserver.com), no AAAA; it 404s every new-layout path. `scutoons.b-cdn.net` on samples (trailer
    `catpope/s1/s1-trailer-loop.720p.mp4`, slice `catpope/s1/c2/s1c2-slice-14.webp`,
    `scutoons/models/mobilenet_v2_100_224/model.json`) with Origin `https://scutoons.com` + Range → 206, ACAO `*`,
    content-range sizes = the files. `assets:check-storage` (main checkout, read-only): zone 866 files / 787.3 MB, pull
    zone hostnames `scutoons.b-cdn.net` only; vs the dev copy `media/`: 686 same, 180 only on bunny + 39 different size
    (= production uploads since the init – bunny is ahead), 56 only local (dev-only: catpope/characters SVGs,
    merch/bandcamp, a few items/slices). **No `assets:sync` before the switch:** the switch does not change which bytes
    are served, and a local → bunny `--overwrite` would replace the 39 newer production files. The decisive check is
    production admin → Assets → Missing (nothing new). Old FTP CDN paths that still answer there but not on bunny
    (moved/deleted by the relayout, `logs/cutover/relayout-report.json`): 12 – `catpope/audio/CatPope-Awakening-Music.zip`
    (now `catpope/s1/…`), `catpope/images/s1s{1..6}-thumbnail.jpg`, `catpope/images/collectables/cucumber.png`,
    4 × `catpope/images/vectors/*.svg`; they 404 after the switch (none is a printed URL – PLAN.md RULE; optional: a
    bunny Edge Rule redirect for the album zip if that link was shared).
  - **Owner steps, in this order (the agent verifies each read-only):** 1. Read-only check SQL in phpMyAdmin (absolute media URLs stored in the DB):
    `sql
       SELECT 'entity_versions' t, SUM(json LIKE '%scutoons.b-cdn.net%') bcdn, SUM(json LIKE '%cdn.scutoons.com/%') cdn FROM entity_versions
       UNION ALL SELECT 'content_versions', SUM(json LIKE '%scutoons.b-cdn.net%'), SUM(json LIKE '%cdn.scutoons.com/%') FROM content_versions
       UNION ALL SELECT 'toons', SUM(data LIKE '%scutoons.b-cdn.net%'), SUM(data LIKE '%cdn.scutoons.com/%') FROM toons
       UNION ALL SELECT 'settings', SUM(value LIKE '%scutoons.b-cdn.net%'), SUM(value LIKE '%cdn.scutoons.com/%') FROM settings
       UNION ALL SELECT 'catalog_items', SUM(data LIKE '%scutoons.b-cdn.net%'), SUM(data LIKE '%cdn.scutoons.com/%') FROM catalog_items
       UNION ALL SELECT 'catalog_albums', SUM(data LIKE '%scutoons.b-cdn.net%'), SUM(data LIKE '%cdn.scutoons.com/%') FROM catalog_albums
       UNION ALL SELECT 'catalog_products', SUM(data LIKE '%scutoons.b-cdn.net%'), SUM(data LIKE '%cdn.scutoons.com/%') FROM catalog_products;
       `
    (the catalog lines need `2026-10-04-catalog`; drop them if it has not run.) `bcdn` > 0 → the migration
    `2026-10-05-cdn-domain` (step 7) rewrites them – both names serve the same files, so it is cleanup; nothing breaks
    before. `cdn` > 0 → absolute URLs that today load the OLD FTP CDN: the agent checks those paths on bunny first
    (`SELECT id, entity_key, status FROM entity_versions WHERE json LIKE '%cdn.scutoons.com/%'`). 2. bunny dashboard → Pull Zones → `scutoons` (6731366) → General → Hostnames → add `cdn.scutoons.com` (harmless
    before DNS). "Force SSL" not yet. 3. KAS → Tools → DNS-Einstellungen → `scutoons.com`: the row `cdn` type A `85.13.151.120` → edit to type `CNAME`,
    Prio 0, Data **`scutoons.b-cdn.net.`** (trailing dot). If KAS refuses to edit the subdomain's automatic record:
    delete the `cdn` A row and add a new entry with those values. **Leave the KAS subdomain cdn.scutoons.com and its
    folder `/cdn.scutoons.com/` as they are** (backup until 10.7). Agent: `nslookup -type=CNAME cdn.scutoons.com
ns5.kasserver.com` → `scutoons.b-cdn.net`. 4. bunny → Hostnames → `cdn.scutoons.com` → "Verify & Activate SSL" (free certificate; needs step 3 visible to bunny –
    retry after a few minutes if it fails). Then "Force SSL" on for that hostname. 5. Agent: `https://cdn.scutoons.com/…` on the three samples with Origin + Range → 206, ACAO `*`, same sizes,
    `server: BunnyCDN-…`; `assets:check-storage` lists both hostnames on the pull zone. 6. Merge + deploy (`targets.json` is in every target's `watch`, so all apps rebuild): `git checkout main` ·
    `git merge --ff-only platform` · `git push origin main` · `git checkout platform`. 7. phpMyAdmin: `server/migrations/mysql/2026-10-05-cdn-domain.mysql.sql` (idempotent; a no-op when step 1 found
    nothing), then admin → Assets → Rescan. 8. Agent: the live apps (scutoons.com reader, qr., creator., admin., shop.) load media from `cdn.scutoons.com`
    (network requests, own browser tab); `npm run deploy:status`. 9. Tidy (not read by the deploy): GitHub environment `production` `CDN_PUBLIC_URL` → `https://cdn.scutoons.com` or
    delete it (`targets.json` fixes the value); the local root `.env` `CDN_PUBLIC_URL=https://cdn.scutoons.com` (only
    the local `assets:check-storage` HEAD uses it).
  - **Rollback:** before step 6 – KAS: the `cdn` row back to type A `85.13.151.120` (the old FTP CDN answers again; the
    apps never left `scutoons.b-cdn.net`). After step 6 – revert the `targets.json` commit and redeploy: the apps go back
    to `scutoons.b-cdn.net`, which keeps working whatever the DNS says. The migration needs no rollback (both names serve
    the same zone; reverse = the same REPLACE the other way round).
  - **Done 2026-10-05:** step 1 – no absolute media URL in the DB (all 0; content_versions empty), so step 7 is a no-op.
    Steps 2–4 by the owner; agent check: ns5.kasserver.com + 8.8.8.8 answer CNAME `scutoons.b-cdn.net`; pull zone
    hostnames `scutoons.b-cdn.net, cdn.scutoons.com`; certificate CN cdn.scutoons.com (Let's Encrypt, to 2027-01-03),
    http → 301 https; the three samples through cdn.scutoons.com (bunny edge) → 206, ACAO `*`, sizes = the files.
  - Afterwards: **10.9 is unblocked** (nothing uses the old FTP CDN any more); the folder `/cdn.scutoons.com/` stays as a
    backup until 10.7 and is the later backup target (ASSETS.md "Backup", `FtpStorage`).
- [ ] 10.4 Folder restructure: `assets:layout` re-run + owner review, batch relink (to build), the model path code change,
      then prune (ASSETS.md "Migration plan").
- [ ] 10.5 _(test hosts + T1's origin removed 2026-10-05)_ Remove the test hosts `next.` / `qr-next.` (and T1's origin); later `PROXY_FULL_API=0` when qr's log shows
      no `/api/*` traffic; remove `generate/` + `GENERATE_PASSWORD` after admin parity.
      _`PROXY_FULL_API=0` (branch `10.5-qr-proxy-off`, 2026-10-05):_ `deploy/targets.json` qr "0" and the default in
      `qr/src/Config.php` ("1" = rollback, then deploy qr). Still proxied: `/api/validate/`, `/api/keys/keys.php`,
      `/api/certificates/*`, `generate/get-signature.php` + signature files; the rest of `/api/*` → 404 JSON. Evidence: the
      old viewer (built from this repo's old `main`, `viewer/build`) never had a service worker – no `service-worker.*`,
      `serviceWorker`, workbox or PWA plugin anywhere in git history (only `site.webmanifest`, no SW), the old qr repo neither;
      the new viewer registers none either, so nothing can keep serving the old app. Every live app bundle (viewer,
      creator, admin, shop, embassy at 82c325d; all chunks crawled) calls `https://api.scutoons.com/api`, none
      `qr.scutoons.com/api`; their prod fallbacks now point there too. Mails link only `?login=` / `/account`; the Ask
      service needs `SCUTOONS_API`. Neither the qr app nor the API logs proxied requests – traffic only in all-inkl's
      stats (owner check before the deploy). `generate/` + `GENERATE_PASSWORD` stay (after admin parity, 10.18).
- [ ] 10.7 After one release without problems: delete the old folders (keep backups); the old DB stays as an archive.
- [ ] 10.9 KAS FTP cleanup (after the switch + the CDN move – unblocked once 10.3 is live): delete the users `cdn-sync` (f017c2d3; its repo tooling is gone since 2026-10-04), github actions
      `f017b642` and `f017b651`; remove their old secrets from GitHub (old qr repo, repo-level FTP secrets of the old viewer
      workflow). The main user stays, used only by hand.
- [ ] 10.11 Server errors in admin (owner 2026-10-02, after reading `/web/api/logs/error.log` by FTP to find the dashboard
      500): an admin-only API endpoint that reads the newest lines of `LOG_PATH` (newest first, size-capped, no secrets –
      messages only), and an admin page "Server errors" showing them (filter by day / text). Same for the qr app's log.
- [ ] 10.12 Viewer: `/catpope/credits` and `/catpope/download` work but no page links to them (owner 2026-10-02) –
      **owner decision: link them (toon page / season slot / menu) or drop the routes.** Also: the QR camera never decoded on the owner's webcam (blurry
      image) – check focus / resolution constraints and a sharper decode path once the phone test (Phase 9) is in.
- [ ] 10.13 Visibility rule (owner 2026-10-02): `draft` is seen by creators only, `coming-soon` is the only non-published
      status the public sees. Production's public API follows it (season 1 lists chapters 0, 1, 2 = coming soon). Check
      the signed-in creator's viewer list: it showed chapters 4 and 5 as "coming soon" while the creator marks them draft.
      Later: drafts only for creators who are authors of that toon.
- [ ] 10.14 Admin claim page rework (owner 2026-10-02, during 6.8: "the ui in admin for claim needs rework") – details
      from the owner; first guess: the two side-by-side note fields + the decision buttons hidden in the bottom bar.
      Also: the viewer's claim form promises the current owner never reads the statement – true for owners, but an owner who
      is also an admin recipient gets it in `ownership_claim_admin`; reword if the owner wants.
- [ ] 10.16b Admin claim page: no raw data (owner 2026-10-02) – part of 10.14.
- [ ] 10.17 Deploys are not atomic: FTP replaces files one by one, so for a few seconds the API runs half old / half new
      code (2026-10-02 23:49: "No entry or class found for Transport" during a deploy). Upload to a fresh folder and switch
      (rename / symlink), or a maintenance flag during the upload.
- [ ] 10.18 Batches + code generation on real data (owner 2026-10-03): admin → Batches (and the generate side) still
      read the static lists (editions / items / batch manifests from files); populate them from the database (catalog,
      editions, codes) instead.
- [ ] 10.19 Setup / installer for a fresh environment (owner 2026-10-03): one step that runs the migrations, the content
      seed and the asset scan in order and checks each (the cutover did them by hand – 6.9 found the empty asset registry
      only through the creator's picker). Content keeps plain CDN paths; the registry is the index for the tools.
      _(locally largely `npm run dev:db` since 10.30: migrate + prod seed incl. the asset tables; no content seed any more)_
- [ ] 10.20 Asset picker: the selected tile turns black but its kind icon stays black (blank white box) – invert the icon
      too (owner screenshot 2026-10-03). The tile's size sums every file incl. the master (roar-little-kitty 491 MB) – show the
      default rendition's size or label the total.
- [ ] 10.21 Asset picker rework (owner 2026-10-03: "we do have to fix the ui of the picker and its relation to the page
      where we pick media – there is redundance and confusion"): drafts first (2–3 layouts, owner picks). Known: the choice box
      opens above the grid, the upload is a heading-looking fold-out, the page's own path fields repeat what the picker shows;
      includes 10.20.
- [ ] 10.22 Creator chapter tree: "Add group" + "Add separate scene" (cut off, owner 2026-10-03: "weird") – the second is a
      storage detail (a ref group = its own scene entity, from the old file layout). One "Add group"; the editor picks the
      storage. Scene keys (`chapter2/scene-14`) are internal IDs only, never shown as names (owner 2026-10-03: "legacy from
      files – in the db we don't care about ids that way"); the label is the name (done for the tree / crumbs: 3109fd8).
- [ ] 10.23 Asset jobs on production (owner 2026-10-03: "Optimize" stays queued): `assets:jobs` reads the queue from its
      own DB – locally the dev DB, and production's MariaDB is not reachable from outside. The runner on the owner's machine
      (ffmpeg) should take jobs through the API (admin token): list queued → download the source from bunny → transcode →
      upload → mark done. Job #1 (video-renditions, 2026-10-02 22:45) can be dropped.
- [ ] 10.24 Codes case-sensitive + strict session sql_mode (owner 2026-10-03, branch `db-case`): **O – run
      `server/migrations/mysql/2026-10-03-code-case.mysql.sql` in phpMyAdmin** (whole file at once; before or after the API
      deploy, both orders are safe; idempotent). Every code lookup is exact, also `/api/certificates/` + the verify page
      (no case-insensitive fallback; certificates printed before show UPPER CASE – typed that way they are unknown now, the
      QR / signature file still verify); new certificates print the exact code. The API sets a strict session `sql_mode`
      (`DB_SQL_MODE`, production's server mode is not strict). Optional check afterwards: the information_schema SQL in
      server/docs/DATABASE.md "Matching production" (all utf8mb4; the code columns utf8mb4_bin).
- [ ] 10.25 Map places ↔ story: where a place lives (owner 2026-10-03, "lets investigate … isn't group the better place?" –
      **decide first, after the open UI fixes**). Findings 2026-10-03:
  - **Unlock today:** on the place (world map `place.unlock` = toon, chapter, `slice` = the slice's image path, `at`).
    The viewer unlocks it when the reader _sees_ that slice (`world.svelte.ts` `sliceSeen` → `placesForSlice`); a place
    whose chapter has no slices yet unlocks from the chapter position `at` / a finished chapter (`unlocksFromProgress`).
    The position never unlocks a slice place (VN choices skip groups). The slice inspector's "Unlocks place" box is the same
    setting from the slice's side – its wording confuses ("the slice represents the place").
  - **Jump today:** VN `jumpTo` = chapter + optional group label (`/catpope/chapter/2?label=scene-15`), the next bar too –
    groups are the jump unit. The map card links only `place.chapter` (opens the chapter at its start, not at the place).
  - **Idea:** the place on the **group** ("this scene is: Cathedral"): unlocks when the reader enters the group, the card
    jumps to `chapter/N?label=<group>`, one setting shown from both sides (map builder: "Chapter 2 · scene-15"); chapter +
    position stays the fallback for chapters without groups (3–5).
  - **Doesn't line up yet:** chapter 1 is one group of 95 slices with two places (apartment slice 2, apartment-blocks
    slice 72); chapter 2 `scene-20` holds church-square (slice 20) + cathedral (21); `scene-17` has none. Group labels are
    not the screenplay scene numbers (places say scene 13–17, groups are scene-14 … scene-20).
  - **Blocker:** splitting groups breaks the reader's infinite scroll (owner 2026-10-03) – fix that first, or keep groups
    as they are and let a place point at a group + an optional slice (unlock precision) instead of splitting.
  - **Open decisions:** split vs. group + slice; rename groups to their places (labels are in jump links – only with OK).
- [ ] 10.26 Delete a chapter safely (owner 2026-10-03: "how could i delete a chapter? … super dangerous"). Today nothing
      can be deleted: the API has no delete for entities (versions are append-only), so no chapter is lost. Numbers are given
      automatically (next free, never reused) and are the chapter's identity (entity `chapter:<toon>:<n>`, URL, release order;
      shown big in the chapter's Meta since f345743). Proposed rules, enforced by the **server**, the creator only mirrors them:
      only a chapter that was **never published** in any language (status draft) and is the **last of its season**; a typed
      confirmation (the chapter number, in-page dialog); delete = removed from the season's sequence + the entity marked deleted,
      its versions kept (admin can restore; purging is a separate admin step). A published chapter is never deleted – at most
      hidden (draft / coming soon), so links and reading progress keep working.
- [ ] 10.29 **Catalog into the database** (owner 2026-10-04: "lets just do it properly … we have to do them all now"): items, recipes,
      albums, products, editions, featured, permissions / code types, toons (admin-created) → tables + history; slots / stats → code;
      basic admin UI with the asset picker and bulk imports. Plan and status: agents/CATALOG.md. Part 1 (tables, loaders,
      admin API) built 2026-10-04 on branch `catalog/db` _(production migration has run – owner check 2026-10-05: 11 tables, old settings folded)_: **O – run `server/migrations/mysql/2026-10-04-catalog.mysql.sql` in
      phpMyAdmin** (whole file at once; before or after the deploy; idempotent) – steps + check query in CATALOG.md "Status".
- [ ] 10.28 **SCUTOONS MCP server** – an agent with the rights of the signed-in creator (owner 2026-10-03: "very important
      … i want to also connect other services that help me create more sophisticated assets"; first step right after the
      release, branch `mcp`). Why MCP and not only an "Ask" panel: the API can do much more than the UI, any MCP client can use
      it (Claude Code / Desktop today, claude.ai connector + the creator's own panel later), and other MCP servers (asset
      generation, image / audio tools) combine with it in the same client – the agent makes an asset there and puts it into a
      slice here.
  - **Package** `tools/mcp` (`@scutoons/mcp`, TypeScript, official MCP TypeScript SDK; versions checked when building).
    Calls the existing PHP API with the creator's token (`Authorization: Bearer`, like the creator) → the API's role
    checks apply unchanged: the agent can do exactly what that creator can.
  - **Tools** (step 1): `list_toons`, `list_chapters` / groups / points (`scriptLabels`), `read_entity` (draft or published,
    one or all languages, with version ids), `save_draft` (content + base version → 409 on a newer edit; runs
    `@scutoons/content` `validateFile` first and returns the problems), `validate`, `list_versions` / `restore_version`,
    `search_assets` (+ upload later), characters / cast / moods, map places. **Publish** is a separate tool, off by default
    (env flag), one explicit confirmation per call. Saves only ever create new draft versions (append-only, restorable).
  - **Resources**: the format rules the agent reads first – VN events (label / divert / if / setVariable / choice actions /
    anyOf, `vn-script.ts`), text presets, slice sizes and media variants, entity keys, the one-click and flow rules.
  - **Transport**: stdio first (local: Claude Code / Desktop against local or production API, token from `.env` /
    `npm run dev:test-user` locally). Remote (Streamable HTTP + sign-in, claude.ai connector, the creator's "Ask" panel via
    the API's MCP connector) is step 2 – the shared hosting (FTP deploy) can't run a long-lived Node server: a small Node
    host or a PHP implementation, owner decision then.
  - **Checks**: a script like `check:stroll` that drives the tools against the local stack (read → patch → validate → save
    draft → restore), never against production.
  - **Example task**: "set up the chapter 2 stroll" (today `scripts/dev/stroll-chapter2.mjs`) as one request.
  - **Step 1 built (2026-10-03/04, branch `mcp`)**: `tools/mcp` (`@scutoons/mcp`, MCP TypeScript SDK v2
    `@modelcontextprotocol/server` 2.3 – the split packages that replace `@modelcontextprotocol/sdk`; zod 4). Runs from
    the repo with Node's type stripping (`bin/scutoons-mcp.mjs` → `src/main.ts`, nothing built). Config: `SCUTOONS_API`
    (default local), `SCUTOONS_TOKEN` (locally falls back to `logs/dev/test-user.json`, never for a remote API),
    `SCUTOONS_MCP_PUBLISH=1` (publish tool, off by default). Tools: `list_toons`, `list_chapters` (groups: label, scene
    entity, slices, `scriptLabels` points), `list_entities` (every stored document of a toon / the platform),
    `search_content` (full text in a toon's documents, the screenplay per line), `read_entity` (draft / published /
    version id, one or every locale, `path` / `lines` for a part), `read_characters` (cast season 1 like the creator,
    looks, moods), `read_map_places`, `search_assets`, `upload_asset` (a new file from a URL or local file → the asset
    manager, 409 when the path is taken), `validate` (validateFile with the chapter editor's context + validateLocales;
    validateCast for cast), `save_draft` (`ops` set / insert / remove on the `base` version, or whole `content`; errors
    refuse the save unless `allow_errors`; message gets "(agent)"; 409 explained), `list_versions`, `restore_version`,
    `submit_for_review`, `publish_entity` (only with the env flag, `confirm: true` per call). Resources
    `scutoons://rules/{workflow,entity-keys,vn-events,flow,text-presets,slices}` generated at start from the source.
    `npm run check:mcp` (`scripts/dev/mcp-check.mjs`, 31 checks, local only, restores what it changes). README: Claude
    Code / Desktop, local + production. Not validated yet: season status of a chapter and placed items against the
    catalog (the creator passes both).
  - **Step 2 decisions (owner 2026-10-04)**:
    - **Order:** the creator's "Ask" panel is the way for creators; the API token (step 1, stdio) stays for Claude Code /
      Desktop; no claude.ai connector for now (no OAuth needed – revisit only if creators want claude.ai).
    - **Host: bunny.net Magic Containers** (EU, the same bunny account as the CDN): the Ask service in a Docker container.
      Billed by usage (~$0.02 / core-hour + RAM, account minimum $1/month) – check the real idle cost in the 14-day trial.
      Rejected: Cloudflare Workers (US; free plan 10 ms CPU per request is tight for validating a big chapter; needs
      bundling), bunny Edge Scripting (Deno, a port), Koyeb / Render / Hetzner / Scaleway (a second provider).
      Also considered and set aside (owner 2026-10-04: "we stick to the node server on bunny"): the agent loop in the PHP
      API (tools in the browser, or a PHP port of the story rules). Separate idea, not decided: the API checking every
      save against the generated JSON Schemas (`shared/content/schema`).
    - **Auth: long-lived personal access tokens** for Claude Code / Desktop (owner's choice; OAuth was recommended as
      safer). To build: a creator page "Tokens" – create (shown once, only the hash stored), name, last used, revoke; never
      the browser session; the API accepts them as Bearer like a session. Expiry optional.
    - **"Ask" panel: the owner pays** (SCUTOONS, personal business); model Claude Sonnet 5.5. Every AI call is logged:
      creator, service, model, tokens, cost → an admin overview by creator and service per day / month. **No limits in
      SCUTOONS** (creators need to get the job done; the owner controls spending on the Anthropic account side).
    - **Asset upload: yes** (`upload_asset`; remote: from a URL only).
    - **Publish: keep the tool** (drafts get reviewed; the chat history shows what an agent did). Today off by default with
      `confirm: true` per call, and the API lets every creator publish. Review levels (who may publish without a review)
      are a separate task in the API, for agents and the UI alike.
    - **Chat history: yes** (owner 2026-10-04: "I need to see what I did before if something stops or breaks").
  - **Step 2 built (2026-10-04, branch `mcp`)**:
    - **Ask service** `tools/mcp/src/ask.ts` (`npm run dev:ask`, port 9002 in dev-ports; `npm run dev` starts it when the
      repo-root `.env` has `ANTHROPIC_API_KEY`): `POST /api/ask`, Server-Sent Events. The creator's session (Bearer or the
      `scutoons_sid` cookie) checked against `/me/` (role creator); the MCP tools in-process (InMemoryTransport) with that
      session; Claude Sonnet 5.5 (server-side fallback `default`, effort medium, cached system prompt with the format
      rules); context per message: page, the header's text language, the app language. Every API call → JSON line in
      `logs/ask/usage.jsonl` (creator, model, tokens, cost in USD). A loop guard of 60 rounds (not a spending limit).
    - **Chat history** in the API: table `agent_chats` (migration `2026-10-04-agent-chats`, **to run on production**),
      `/api/agent-chats` (list / get / put / delete, role creator, own chats only, 503 without the table). The service
      saves after every round and loads a chat to go on with it.
    - **Creator panel** (`creator/src/lib/components/ask/ask-panel.svelte`): round header button; docked right, the page
      makes room, drags wider; full screen on phones. Streams the answer, each tool call a step, saved drafts as links;
      stop, new chat, earlier chats (clock button), the open chat comes back after a reload. `VITE_ASK_URL` for
      production (empty = "not available").
    - **Deploy prepared (2026-10-04, branch `ask-deploy`)**: `tools/mcp/Dockerfile` (+ `Dockerfile.dockerignore`, build
      from the repo root; node:22-alpine, runs the TS sources, port 8080, health `/api/ask/health`, SIGTERM; built and run
      locally); `.github/workflows/ask-image.yml` pushes `ghcr.io/mandymozart/scutoons-ask:latest` on changes to main.
      Usage log in the API: table `agent_usage` (migration `2026-10-04-agent-usage`), `POST /api/agent-usage` (the
      creator's session + the shared `AGENT_SERVICE_KEY` / `SCUTOONS_SERVICE_KEY`; user from the session), admin
      `GET /api/admin/agent-usage?from&to&group=day|month` (per creator / service / model); the service falls back to its
      file. Deploy: `AGENT_SERVICE_KEY` into the API's config.php (targets.json), `VITE_ASK_URL` into the creator build.
      Verified locally: a question → 2 calls, $0.04, one row per call in the database, the overview sums them.
      **Owner steps** (tools/mcp/README.md "Production"): the two migrations in phpMyAdmin; the secret `AGENT_SERVICE_KEY`;
      bunny Magic Containers app (image from ghcr.io, port 8080, env `ANTHROPIC_API_KEY`, `SCUTOONS_API`, `ASK_ORIGINS`,
      `SCUTOONS_SERVICE_KEY`); DNS `ask.scutoons.com`; the variable `VITE_ASK_URL`.
    - **Live (2026-10-04):** the three migrations (reviews, agent-chats, agent-usage) ran on production; secret
      `AGENT_SERVICE_KEY`; bunny Magic Containers app `scutoons-ask` (id `ZBYjE7aB2VZvKZ6`, container `scutoons-ask`, region
      AT, CDN endpoint → `https://ask.scutoons.com`, env `ANTHROPIC_API_KEY` (its own production key), `SCUTOONS_API`,
      `ASK_ORIGINS`, `SCUTOONS_SERVICE_KEY`; registry: a GitHub token with `read:packages`); creator built with
      `VITE_ASK_URL=https://ask.scutoons.com/api/ask`. The answer streams through bunny (owner checked). **Auto-deploy:**
      `ask-image.yml` → bunny's action `BunnyWay/actions/container-update-image` with the environment variables
      `BUNNY_ASK_APP_ID` / `BUNNY_ASK_CONTAINER` and the secret `BUNNY_API_KEY` (verified: bunny switched to the pushed
      commit's image, rolling update, health 200 throughout).
    - **Open:** ~~an admin page for the usage overview~~ (Dashboard, 2026-10-05); the build's commit in `/api/ask/health` (to see
      which image runs); personal access tokens.
    - **Shared in three apps (2026-10-04, branch `ask-shared`; owner: "make Ask a shared integration in embassy,
      creator, admin")**: the panel is `@scutoons/ask` (shared/ask: `AskPanel` + `ask` / `askAbout` / `pagePanel`; the
      app passes its name, chats API, labels, context, links and an optional result snippet; `panelWidth` moved to
      `@scutoons/ui`). The request names its app; the service picks role, prompt and tools per app (`APPS` in ask.ts):
      creator → role creator, content tools (unchanged); embassy → liaison / ambassador / admin, `src/embassy.ts`
      (agents/EMBASSY.md "Ask + mail"); admin → role admin, the content tools that only read + `agent_usage`. Chats
      and usage rows carry `app` (migration `2026-10-04-ask-embassy-mail`); every message carries today's date.
      `ASK_ORIGINS` (bunny env) must list creator, embassy and admin (the code default does, the env wins); `VITE_ASK_URL` goes into the embassy and admin builds too.
    - **Admin agent (2026-10-05, branch `ask-admin`; owner: the admin's Ask could not answer "signed codes for Cat Pope –
      Awakening", "add an event", "replace item images" and guessed wrong):** step 1 – read tools over the admin API in
      `tools/mcp/src/admin.ts`: `dashboard`, `count_codes` (per edition; toon / edition / status / type), `list_codes`,
      `read_code` (+ timeline + ownership history), `list_events` (the activity log), `list_users` / `read_user`,
      `list_claims` / `read_claim`, `email_log`, `read_catalog` (any kind, `q`, `fields`), `catalog_history`,
      `read_calendar`, `asset_usages`, `read_settings`, `agent_usage`; the admin prompt (ask.ts `ADMIN_SYSTEM`) carries a
      map of the admin's pages, what the agent can't do, and the ambiguous words (event, edition, item image, season →
      the toon's editions). Step 2 – `propose_changes` (never writes): a **changes card** in the panel
      (`admin/src/lib/components/changes-card.svelte`, the mail-draft pattern; before → after, image thumbnails); Apply
      makes the changes in the admin app with the admin's session: catalog entries of items / recipes / albums / products /
      editions / toons (fields by dot path, one PUT per entry with note "Ask: …" → catalog history + undo; refused when
      the entry changed since the agent read it), calendar entries (add / update → ONE new draft of `events`; the Calendar
      page now offers Publish for a newer draft), an asset file replaced under its path (`PUT /assets/replace`; the new
      file from another asset path or picked in the card). Not offered: revoking / transferring codes, claims decisions,
      deletes, roles, asset cleanup, new catalog entries. **Dashboard → AI usage**: this month per app (tiles) and per
      person, cost per day in the dashboard's period (bars per app) – `admin.agentUsage` in `@scutoons/api`. No migration
      (uses `agent_usage` from 2026-10-04). Verified locally: the three questions, a catalog change applied through the
      card and undone in the History. Not verified live: an asset replace (the local asset registry was empty).
  - **Review levels (owner 2026-10-04; follows "Publish: keep the tool")** – who may publish without a review:
    - **Decided:** not a role – a relation per toon, set by an admin, like GitHub reviewers. `reviewer` (per toon or `*`)
      gets a mail on every submit and publishes or sends back; `needs_review` (per account per toon or `*`) = may save
      drafts and submit, not publish / unpublish. No rule = publishes directly (as before; no backfill). Reviewed levels:
      every toon-level entity (toon page + settings, season, chapter + scenes, cast, credits, **script** too); platform-wide
      ones (characters, moods, people, roles, events, map, asset-requests) publish directly for everyone. A submit is
      pinned to its version (the reviewer publishes what they read; newer drafts stay drafts); a newer submit of the same
      file annuls the open one. No reviewer assigned → `409 no_reviewer` "ask an admin to add a reviewer". Mails: review
      requested (reviewers), published / sent back (author) – one per person per action, listing the files.
    - **Built (API, branch `claude/vigilant-chandrasekhar-67b979`):** migration `2026-10-04-reviews` (**production: run it**;
      two new tables `content_review_rules`, `content_reviews`), `Domain\Content\Reviews`, `GET|POST /api/reviews`,
      `POST /api/reviews/publish|return`, `403 review_required` on `/api/entities/<key>/publish|unpublish`, admin
      `/api/admin/review-rules` + `/api/admin/users/{id}/review-rules/{toon}/{rule}`, mails `review_requested` /
      `review_published` / `review_returned` (de + en), setting `CREATOR_URL`. server/docs/API.md "Reviews", PHPUnit `ReviewsTest`.
    - **Built (UI, drafts approved by the owner 2026-10-04):** admin user page → section "Reviews" (the account's rules,
      add toon / All toons + Reviewer / Needs review, remove). Creator: for `needs_review` accounts every page's Publish
      button is "Submit for review" (the publish dialog + an optional note; `lib/editor/reviews.svelte.ts`), the history
      panel hides publish / unpublish for them and marks versions "in review"; the dashboard shows "To review" (one row per
      submit: author, time, languages, note, "newer drafts" hint, Send back / Publish with a note to the author) and "Your
      submissions" (in review / sent back with the reviewer's note) – both only when not empty. Mails name the toon
      ("Cat Pope", not the id) and link to the dashboard (owner: no further action needed than the list).
    - **Agents (2026-10-04, branch `ask-deploy`):** the MCP tool `submit_for_review` (`POST /api/reviews`, items pinned to a
      version + note); `publish_entity` answers `403 review_required` for `needs_review` accounts – the agent submits instead.
- [ ] 10.31 **Naked A-pose removed** (owner 2026-10-05: "Remove the naked A-pose variant. Keep the safe-for-work switch in
      the account.") _(branch `apose/no-naked`)_: the only standard A-pose look is `swim`; the API and the reader ignore a
      leftover `naked` look (or one still marked `sensitive: true`), the creator's validation warns about it; no cover rule
      any more (legs may be empty – the swim body has the pants painted in); `AVATAR_DEFAULT_UNLOCKS` = `{"catpope":["swim"]}`.
      "Hide sensitive content" stays in the account (it will blur / mute sensitive slices). **O – production steps, in order:**
  1. Deploy (API + viewer + creator).
  2. phpMyAdmin → d0487143 → SQL: run `server/migrations/mysql/2026-10-05-no-naked.mysql.sql` (idempotent: naked bodies →
     swim, naked unlock rows deleted – the character stays unlocked –, a stored settings override cleaned). If the
     installation's `config.php` / environment overrides `AVATAR_DEFAULT_UNLOCKS`, drop `"naked"` there too (harmless
     otherwise: the API ignores it).
  3. Delete the naked A-pose looks in the creator (registry: each character → A-pose → × on "naked"; the Problems panel
     lists them as "no longer used"), publish the registry, then delete the CDN files `characters/*/…_apose-naked.webp`.
- [ ] 10.32 **VN event `showOnomatopeia` → `showTextBox`** (owner 2026-10-06, branch `chapter-neutral`): the event
      reveals any text box; the misspelt old name is gone without an alias (types, validation, schemas, viewer, creator).
      The creator's picker is one select "choose or create new" (DESIGN.md "Onomatopoeia = a text box"). Dev DB done
      2026-10-06 (31 rows in entity_versions, all compact `"type":"showOnomatopeia"` in old catpope chapter 2 / scene-14
      versions; 0 left). **O – production steps, together (old code can't read the new type, new code not the old):**
  1. phpMyAdmin → d0487143 → SQL, check first: `SELECT COUNT(*) FROM entity_versions WHERE json LIKE '%"showOnomatopeia"%';`
  2. Deploy (API + viewer + creator), then right away run `server/migrations/mysql/2026-10-06-show-text-box.mysql.sql`
     (idempotent; entity_versions + content_versions); the SELECT of step 1 answers 0 afterwards.
- [ ] 10.33 **Expressions in texts + the clock syntax** (owner 2026-10-06, branch `chapter-neutral`, EXPRESSIONS.md step 2):
      texts use ink's inline syntax; the old `{name:clock}` is gone (no alias) – it is `{clock(name)}` now, validation
      calls the old form an error and the API refuses to save it (422). Dev DB (= the production dump) checked 2026-10-06:
      0 occurrences in entity_versions / content_versions; the migration is for production's newer test content.
      **O – production steps, together with the deploy of step 2:**
  1. phpMyAdmin → d0487143 → SQL, check first:
     `SELECT COUNT(*) FROM entity_versions WHERE json REGEXP '\\{[A-Za-z_][A-Za-z0-9_]*:clock\\}';` (and the same on
     `content_versions`).
  2. Deploy (API + viewer + creator), then run `server/migrations/mysql/2026-10-06-clock-syntax.mysql.sql` (idempotent,
     `REGEXP_REPLACE` → `{clock(name)}`; tested on MariaDB 10.11.14); the SELECTs of step 1 answer 0 afterwards.
- [ ] 10.34 **Events: repeat rules, ids, askText, the reader's name** (owner 2026-10-06, branch `chapter-neutral`,
      EXPRESSIONS.md step 3): a trigger's / placed item's boolean `once` is the repeat rule `repeat` now (no alias;
      every time = none – the new default; what `once` meant = `{"mode":"view"}`); events get hidden ids (the creator
      assigns them – stored ones get theirs with the next save, a bar says so); `users.display_name` for the reader's
      name. Dev DB done 2026-10-06 (81 versions changed: 166 `"once":true` triggers, 55 triggers and 4 tap-event items
      without once → view; 0 left; the result checked against a JSON-walking transform of all 457 versions).
      **O – production steps, together with the deploy (old code reads no `repeat`, new code no `once`):**
  1. phpMyAdmin → d0487143 → SQL, check first: `SELECT COUNT(*) FROM entity_versions WHERE json LIKE '%"once":%';` (and
     the two REGEXP SELECTs in the migration's header; the same on `content_versions`).
  2. Deploy (API + viewer + creator), then run `server/migrations/mysql/2026-10-06-repeat.mysql.sql` (idempotent, PCRE
     JSON walk – < 1 s on the dump) and `server/migrations/mysql/2026-10-06-display-name.mysql.sql` (`ADD COLUMN IF NOT
     EXISTS`); the SELECTs of step 1 answer 0 afterwards.
  3. Event ids: nothing to run – a chapter / scene opened and saved in the creator stores them; until then validation
     warns ("N events have no id yet") and the reader keeps such events' "once per page view" in memory.
- [ ] 10.35 **Jump targets: string values + jumps into another toon at a label** (owner 2026-10-06, branch
      `chapter-neutral`, DATA.md "Jumps into another toon at a label"): `JumpToTarget.toon` is a new optional field (no
      migration; old code ignores it – such a jump would land in the CURRENT toon's chapter of that number – so deploy
      viewer + creator + API together before saving such jumps). `content:check` found the old chapter 2 stroll's targets with a NUMBER value (`"value":2`
      – the type says string): 11 versions, all `scene:catpope:2:scene-14` (+ de / en), 3 targets each.
      **O – production steps:**
  1. phpMyAdmin → d0487143 → SQL, check first:
     `SELECT COUNT(*) FROM entity_versions WHERE json REGEXP '"type":"(chapter|season|toon|product|xr)","value":-?[0-9]';`
     (and the same on `content_versions`).
  2. Run `server/migrations/mysql/2026-10-06-target-value.mysql.sql` (idempotent, `"value":2` → `"value":"2"`, any
     time – old and new code read both); the SELECTs answer 0 afterwards.
  3. **Owner decision**: the published `scene:catpope:2:scene-14` (201, + de 127 / en 128) still has an event of the
     unknown type `end` (`{"type":"end","params":{"jumpTo":…scene-20}}` – the reader skips it, `content:check` reports
     it). Publishing the scene's newest version (no `end`) or deleting the event clears it; which one is yours.
- [ ] 10.36 **One save game across all toons** (owner 2026-10-06, branch `chapter-neutral`, DATA.md "The save game"):
      story variables, the history and every toon's checkpoints move from the per-toon game states (`progress` rows
      `toon_id = <toon>`) into the member's save game (`toon_id = 'scutoons'`, which holds the map). The new viewer reads
      and writes only `/api/progress/scutoons/` (PUT merges: map / checkpoints / history as sets, variables the newer
      write's); old per-toon calls keep working for older viewer builds (their rows are not read by the new viewer).
      Guests: the viewer takes the old `scutoons_game_<toon>` keys in once (same rule) and removes them.
      Variables have a domain (owner 2026-10-06, `@scutoons/content` variables.ts): a toon's old variables keep it –
      `x` of catpope's state becomes `catpope/x` (written `x` in catpope, `catpope.x` elsewhere); `global.x` is shared.
      **O – production steps (with the deploy of viewer + API):**
  1. phpMyAdmin → d0487143 → SQL, check first (the SELECT is in the migration's header): the members with per-toon
     states (`SELECT user_id, COUNT(*) … WHERE toon_id <> 'scutoons' GROUP BY user_id`) – each toon's variables go in
     as `<toon>/x`, so there are no clashes between toons.
  2. Run `server/migrations/mysql/2026-10-06-global-save-game.mysql.sql` (idempotent: a second run changes nothing; the
     per-toon rows stay as they were – rollback = the old viewer). Afterwards the header's last SELECT answers 0.
  3. Deploy viewer + API together right after (an old viewer keeps writing per-toon rows the new one doesn't read; a
     member who reads in between with an old build: their per-toon progress after the migration isn't merged – run the
     migration again after the deploy, it folds them in by the same rule).
      _(2026-10-06: the migration ran on production in its final form – the save games hold `catpope/…` keys (3 members
      with a catpope state: one empty, two with item-named story variables `cucumber`, `broken_record_player`,
      `repair_coupon` → `catpope/…`, correct: the items themselves are in the inventory). main pushed 908b97c1.)_
- [ ] 10.37 **No dead ends: the save guard** (owner 2026-10-06, EXPRESSIONS.md "Dead-end safety"): no data change, no
      migration. With the API deploy, `PUT /api/entities` refuses (422) chapters / scenes that can get a reader stuck – a
      question whose choices can all be hidden, a divert / jump into nothing (`FlowCheck`); `content:check` reports the
      same. **O:** after the deploy run `php bin/console content:check` on production: a document it lists can't be saved
      unchanged any more – fix it in the creator (the Problems panel names the place). Known from the dump: only the old
      `scene:catpope:2:scene-14` versions (its `scene-20` jump – the 10.35 decision).
- [ ] 10.8 Public release. Set one version everywhere (root + every app + api/qr → `1.0.0`; today root 0.0.0, frontends 0.1.0, api/qr 1.0.0) and tag the commit `v1.0.0`; optional tag `v1.0.0-beta.1` on the beta commit (owner 2026-10-03).
- [ ] 10.38 **Items are not variables** (owner 2026-10-06: "it is just a naming conflict"; "with script we can not
      create items"): an item exists only in the catalog (admin) and lives in the reader's inventory (global, per
      account); a story variable is the script's own number / text (`x` = this toon's, `global.x`, `<toon>.x` –
      variables.ts). Today they mix: `set` / `add` of a variable named like a catalog item also grants the item (the VN
      item bridge, `grantStoryItem` in viewer variable-actions.ts; a slice pickup grants and then `add`s the variable),
      so `catpope/cucumber` is a story count that drifts from the real inventory (eating, crafting). Steps:
  1. Read: `inventory.<item>` = how many the reader has (shorthand for `inventory.count("<item>")`; registry, the
     analyser, viewer expressions/reader.ts, the creator's sample readers). The expression namespaces and `global`
     can't be written as a variable's domain (`inventory.x = 1` is refused, not "toon inventory's x").
  2. Warn: a story variable named like a catalog item (validate.ts `checkVariableNames` with `ctx.items`) – "an item:
     Give item / inventory.<item>".
  3. Give: VN event `giveItem` { item (catalog picker), qty } – the grant rules stay (grantable story, max, once per
     place); types, validate, schema, flow / reach facts (+ FlowIndex.php), the runner, the creator form.
  4. Take: VN event `takeItem` { item, qty } – needs a server endpoint (`InventoryService::remove` is internal today:
     craft / use only) and rules (story only, once per place, too few pieces → ?). **Owner decision before building.**
  5. Data: analyse every `setVariable` / choice `set|add` on an item name in `entity_versions` / `content_versions`
     → a SQL migration to `giveItem` (the 2026-10-06-show-text-box pattern), then drop the bridge's side effect; a
     slice pickup only grants (no variable any more).
- [ ] B.1 Ch1 audio samples are at `slice.audio.samples` but code reads `slice.audio.audio.samples` → ch1 samples never
      fire _(from MEMORY.md "Known bugs spotted", 2026-09-30 – not re-checked since)_.
- [ ] B.2 Scene files use `{"type":"end"}` → "Unknown event type" _(MEMORY.md, 2026-09-30 – not re-checked)_.
- [ ] B.3 Ch3–5 `keyvisual` is a string, the type expects `{image, video?}` (ch2 fixed 2026-09-30) _(MEMORY.md – not
      re-checked)_.

## Done

- [x] 10.1b _(done 2026-10-02. Hint: setting secrets with `gh` gave 404 until gh was re-authorised – the fine-grained
      token did not cover the `scutoons` repo.)_ Right after the assets merge: the 7 bunny keys (`BUNNY_STORAGE_ZONE`, `BUNNY_STORAGE_ENDPOINT`,
      `BUNNY_STORAGE_PASSWORD`, `BUNNY_STORAGE_READONLY_PASSWORD`, `BUNNY_PULLZONE_ID`, `BUNNY_API_KEY`, `CDN_PUBLIC_URL`)
      as GitHub `production` secrets for the live asset manager (the merge adds them to `targets.json`); verify in admin →
      Assets → Storage (set / missing per key, no values).
- [x] 10.6 Embassy go-live (EMBASSY.md "To go live"): subdomain + SSL, roles, import (`order` done 2026-10-04). _(done, owner 2026-10-05)_
- [x] 10.10 Local MariaDB in Docker instead of SQLite for dev + tests (owner 2026-10-02, after the admin dashboard 500 on
      DB-T that SQLite never showed): same server version as all-inkl, the mysql migrations, CI runs the suite on it too.
      _(done 2026-10-03, branch `dev-mariadb`; server/docs/DATABASE.md "Local MariaDB")_ `docker/compose.yml` (MariaDB
      10.11, port 13306, volume, healthcheck; Adminer 13380 behind a profile); `npm run dev:db` = container + databases +
      migrate + the `*.mysql.sql` content seed + promo `devpromo` + switch + API restart + test user; `npm run dev:db sqlite`
      back (SQLite stays the default); `npm run dev:db test` / `phpunit -c phpunit.mysql.xml` = the unit suite on MariaDB;
      CI job `test-mariadb` in server.yml. Found on MariaDB: two signups at the same moment deadlocked in the basic dress
      (fixed, test); `qr.code` was case-insensitive in production (utf8mb4_general_ci) → 10.24; a flaky test fixture.
      Production's values (owner, phpMyAdmin 2026-10-03: 10.11.14, sql_mode NO_ENGINE_SUBSTITUTION, latin1 server
      defaults) are the container's defaults since branch `db-case`.
      _(10.30: SQLite, the switch, `dev:db sqlite`, phpunit.mysql.xml and the content seed are gone – MariaDB only, prod seed)_
- [x] 10.15 Content source of truth is the DB now (owner 2026-10-02): local dev from `export-content` of production
      instead of the stale `shared/content/seed`; the seed becomes a starter set for empty databases (or goes).
      _(covered by 10.30: the seed is gone, local dev loads production's export)_
- [x] 10.16 Mail Reply-To (owner 2026-10-02: a mistake, fixed before the beta): `MAIL_REPLY_TO` = hello@scutoons.com
      (Reply-To header; the texts say "reply to this email", mail comes from no-reply@).
- [x] 10.27 VN story logic – **built 2026-10-03 for the release** (owner: "we have to build it now. it is part of the release").
      Concept: jumpTo = navigation (groups are pages, by design); the story's own flow lives in the group's script. Done:
      one runner (`viewer/src/lib/visual-novel/runner.ts`, group script + slice / item triggers); `label` (named point) +
      `divert` (event, choice action, a slice event diverting into its group's script); `if` ("Only if") on any event;
      `setVariable` (set / add); choice actions as a list; `jumpTo.target.anyOf` (a random group, never the current one);
      `{var:clock}`; validation (points unique, diverts hit a point, VN flow per group); creator forms. Chapter 2 stroll:
      `scripts/dev/stroll-chapter2.mjs` (local), `npm run check:stroll` (headless play-through, 15 checks). Still open from the
      research: `once` choices + fallback, visit counts. Original notes:
- [ ] (history) 10.27 VN story logic, ink's ideas in our JSON events (owner 2026-10-03, **after 10.25**; research into inkle's ink).
      **Owner decision 2026-10-03: adopt some of the concepts below – which ones is still open (pick before starting).**
      Not ink itself: ink is a text script (our authors use the creator's forms), its localisation is one file per language
      or line-ID tooling (breaks our per-field de source / en override), and knots would be a second address system next to
      group labels. Today: variables are written (`set`/`add`, `{var}` in text, item grants) but nothing reads them – no
      conditions; a choice has one action; the only branch is leaving the group (a choice without jump continues with the same
      lines). Steps, all shared (structural) fields so translation and 10.25 stay untouched:
  1. One runner: `toon/interactions/visual-novel.svelte` and `visual-novel/event-processor.ts` both run events (the
     README calls the second deprecated, slice triggers + placed items still use it) – one shared step function first.
  2. `addChoice.params.if: { variable, op: '=='|'!='|'>='|'<', value }` – schema, `checkEvent` in validate.ts, an
     "Only if" fold under Advanced in `vn-events.svelte`, the runner skips the choice when false. Then on `showMessage`.
  3. `once` choices (+ a fallback choice) and `onSelect` as a list (set a flag _and_ jump).
  4. In-group labels + `jumpTo {type:'label'}` (branch for a few lines, rejoin – ink's gathers; guard against loops).
  5. Visit counts per group / label (`seen:scene-15`) next to the variables.
- [x] 10.30 Production export as the local seed (owner 2026-10-05: production is the source of truth for content and
      catalog; "i want no sqlite also anymore") _(branch `10.30-prod-seed`)_:
  - **Prod seed:** phpMyAdmin → d0487143 → Export → Custom → SQL, data only, tables entity_versions, toons, catalog_meta,
    catalog_items, catalog_recipes, catalog_albums, catalog_editions, catalog_products, catalog_featured,
    catalog_permissions, catalog_code_types, assets, asset_versions, asset_renditions, asset_usages → `server/db/prod-seed.sql`
    (gitignored – may hold creator ids; a full dump works too: `npm run dev:db -- seed --from=<dump.sql>` keeps those 15
    tables). `php bin/console db:prod-seed [--from=] [--force]` empties the 15 tables and loads the rows in one
    transaction; same file again (SHA-256 in setting `dev_prod_seed`) = no-op.
  - **MariaDB only:** SQLite removed everywhere (migrations/sqlite, `DB_DRIVER` / `DB_SQLITE_PATH`, `--db=` options,
    `npm run dev:db sqlite`, the db.json switch, phpunit.mysql.xml – phpunit.xml.dist targets MariaDB `TEST_DB_*`,
    default 127.0.0.1:13306 `scutoons_test`). `npm run dev:db` = container → databases → migrate → prod seed → sandbox
    toon (make-sandbox.ts from the local DB, written to logs/dev/sandbox) → devpromo → API → test user; subcommands `up`,
    `seed`, `sandbox`, `media [--apply]` (missing media from bunny.net, never deletes / overwrites), `status`, `stop`,
    `reset`, `test`, `console`, `sql`, `adminer`, `version`; `SCUTOONS_DB_NAME=scutoons_<x>` per worktree.
  - **No file sources:** catalog JSON → test fixture `server/tests/fixtures/catalog/` (CatalogSeed test-only;
    `catalog:seed`, `catalog:export-sql`, file fallback, featured default list, `/admin/catalog/respawn` removed);
    shared/content/seed + legacy + the one-off migrate-*.ts / slice-sizes.ts deleted (TS tests: fixture
    `server/tests/fixtures/content/`). Contract suites + `composer test:contract` removed (`composer test` = PHPUnit);
    CI = one MariaDB job (PHPUnit + PHPStan).
