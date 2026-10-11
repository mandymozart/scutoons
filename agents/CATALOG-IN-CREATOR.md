# CATALOG-IN-CREATOR – should the catalog move from the admin into the creator? (research, 2026-10-12)

## Overnight draft 2026-10-12 – summary for the morning review

Branch `claude/quizzical-goldstine-44c6a4` (worktree `.claude/worktrees/quizzical-goldstine-44c6a4`, own DB
`scutoons_catalog`). **Not merged, not pushed.** 7 commits, each with the checks green (see "Checks").

### What works

1. **Crafting keeps physical pieces** (bea142f1) – `InventoryService::remove` never touches a code-bound row any more
   (the "possible bug" of ACCOUNT-RESTART.md §2 – fixed); `craft` skips `kind: physical` inputs and code-bound pieces
   of any item (they only have to be there), the output comes in addition; `use` / `take` count only consumable pieces
   (`InventoryService::consumable`). Test `InventoryServiceTest::testCraftingNeverConsumesAPhysicalPiece`.
2. **`@scutoons/catalog`** (shared/catalog, 778fef49) – the admin's forms (item, recipe, album, product, edition, code
   type, permission, toon, meta), fields, `kinds.ts`, `problems.ts`, `CatalogProblems` moved out of the admin; own de/en
   texts generated from the admin's `catalog.*` messages (same keys, `m['catalog.x']()` reads as before);
   `setupCatalog({ locale, locales, adminUrl })` once per app. The admin's editor scaffold, history and image import stay
   in the admin and import the forms from the package – the admin page works as before.
3. **Every catalog entry its own versioned document** (58be6b81) – `item:<id>`, `recipe:<id>`, `album:<id>`,
   `product:<id>`, `edition:<id>` in `entity_versions` (`EntityKey::CATALOG_KINDS` + `$id`; `Domain/Catalog/CatalogDocs`).
   Drafts / versions / restore / review through the normal `/api/entities` path; **publish writes the catalog table**
   (validated, `catalog_history` note `published v<n>`); the table stays what readers and the ~25 consumers read. An admin
   write to the table becomes the entry's published version (`CatalogAdmin::syncDoc`), so document and table never
   disagree. No unpublish (422 `not_supported`). `GET /api/catalog/docs/<kind>` lists the entries with their state.
   Migration **`2026-10-12-catalog-docs.mysql.sql`**: first published version per existing entry – idempotent (dry-run
   on `scutoons_catalog`: 42 rows, second run 0). No column changes.
4. **Physical items need a review – enforced on the server** (58be6b81, 91ecfc00) – a third review rule **`physical`**
   (per toon / `*`, admin → Users → review rules: "Physical items"). A physical change = the entry before OR after is an
   item with `kind: physical`, `edition_id` (= claimable from every unclaimed code of that edition), `product` (also a
   variant's), `max_world`; a recipe whose output is such an item; an album with `owners` access; every product and
   edition. Only an account with `physical` for every toon it touches publishes it; everyone else gets
   `403 review_required { physical: true }` → submits; those submits go to and are decided only by `physical` accounts
   (content reviewers neither see nor decide them); nobody with the rule → `409 no_reviewer { rule: physical }`.
   Non-physical catalog changes follow the existing rules (`needs_review` / `reviewer` of the entry's toon; a recipe:
   its output item's toon; a cross-toon entry `toon: null` = only the `*` rules). **The admin's old catalog page answers
   to the same rule** (PUT / DELETE / undo of items, recipes, albums, products, editions: `403 review_required`) – a
   change readers don't see (an accepted warning only) is not physical. Codes: nothing added, no delete anywhere.
5. **The catalog in the creator** (0e3d8937, 63e69341) – sidebar "Catalog" → `/catalog/<kind>` (tabs Items · Recipes ·
   Albums · Products · Editions): list (search, toon filter, badges New / Draft / Physical) + the shared form +
   "More fields (JSON)", in the normal MetaPage shell: Save (draft, note), History (the entry's versions), Publish (the ONE
   checklist – `publish-tree.ts`: catalog lines sorted edition → album → item → product → recipe; a 403
   `review_required` from the server puts the line into the submit instead), Problems (validateCatalog for the open
   entry), PhoneBar on phones. The reviewers' dashboard links a catalog submit to its entry. Admin: a note + link "Open
   in the creator" on those five tabs.
   **Verified in the browser** (own API :9100 on `scutoons_catalog`, own creator :10100, test user agent@scutoons.test):
   renamed the physical `dago_hoodie` → Save = draft v275, table unchanged → Publish → server refused → "Submit for
   review" → "Nobody publishes physical catalog changes of catpope yet" → granted `physical` (catpope) → Publish →
   "1 published", table updated, history `published v275`. Phone width: list above the form, PhoneBar.

### Checks (last run)
- PHPUnit 344 tests green (`SCUTOONS_DB_NAME=scutoons_catalog npm run dev:db test`), phpstan clean. New:
  `CatalogDocsTest` (5 tests), the crafting test; `EntityStoreTest` adapted (catalog documents are platform-wide keys).
- `npm run check`: every workspace green **except two older failures not touched here**: `tools/mcp` (`server.ts:477`
  uses `MapPlace.unlock`, removed on main 2026-10-11) and `check:tools-css` (its known list – phone-bar font sizes,
  map-builder, viewer). Viewer warnings are older too.

### Open (not built tonight)
- **Undo / redo** of the creator's catalog forms: the shared forms edit the document in place, so Ctrl+Z only covers the
  "More fields" JSON; dirty / autosave / save work. Fix: the forms call an `onchange` the creator routes through `doc.edit`.
- **"Looks fine"** (accepting a catalog warning) in the creator: the Problems panel shows the warnings, accepting them
  is still the admin's (`accepted_warnings` in the entry – works through the admin page).
- Image **bulk import**, **reorder** (position), featured, code types, permissions, toons, meta: admin only (unchanged).
- New entries published from the creator go to the end of the table's order.
- `/api/items/` (pickers, readers) is cached up to 5 min after a publish – as for admin edits today.
- **MCP**: `save_draft` / `list_entities` see the catalog keys through `/entities` (not tried); `read_catalog` is
  unchanged; `propose_changes` for the catalog runs through the admin path (now guarded by `physical`).
- No toon ↔ creator relation (Q2): every creator can DRAFT any entry; the review rules decide what is PUBLISHED.
- Collation: `entity_key` is case-insensitive, catalog ids are case-sensitive – two ids differing only in case would
  share a document; none exist (checked on the dump); the creator's id field accepts the kinds' own id rules.
- The tab row wraps to two lines on phones.
- **Merge with the map-documents draft** (branch `claude/zen-shaw-d9c2dd`, the session told me its names): both add
  `$id` as the LAST `EntityKey` constructor param and `<kind>:<id>` keys without toon; theirs `ID_KINDS = ['place',
  'route']` with a lower-case `ID_PATTERN`, mine `CATALOG_KINDS` with a mixed-case pattern (catalog ids have upper case)
  → at merge: one parse branch over both lists, each with its own pattern; TS: their `ID_KINDS` / `EntityRef.id`, mine
  `CATALOG_ENTITY_KINDS` / `EntityRef.id` – same field. Their generic `GET /entities?kinds=&content=`, `POST
  /entities/batch`, `POST /entities/publish` could replace my `GET /catalog/docs/<kind>` later. Their removal = a
  tombstone version `removed: true` (see decision D5).

### Production, if merged (migrations now run by themselves with the deploy)
- `2026-10-12-catalog-docs` runs on deploy: 42 published versions (27 items, 5 recipes, 1 album, 6 products, 3 editions).
- **Right after the deploy nobody holds `physical`** – so nobody, admins included, can publish a physical change
  (creator or admin page) until an admin adds the rule: admin → Users → <you> → review rule "Physical items", toon `*`
  (and per toon for others). Deliberately not granted by the migration (D2).

### Decisions waiting (owner)
- **D1 Four eyes for physical?** Today an account with `physical` publishes its OWN physical changes directly (like a
  reviewer without `needs_review`). Should a physical change always need a second person?
- **D2 Who gets `physical` first** – only you (`*`)? Should the migration grant it to every admin (then the admin page
  keeps working as today)?
- **D3 Cross-toon entries** (`toon: null`: camera wear, cucumber …) answer only to `*` rules – right? (Q7)
- **D4 More "physical" fields?** `starter` (every new inventory), `grantable: scan`, `effect` (substances), `max` – not
  gated tonight; only `max_world` is. (Q4)
- **D5 Removing an entry**: no unpublish tonight (the admin's delete with its `in_use` check stays). The map draft
  removes with a tombstone version (`removed: true`, the id stays taken) – the same for the catalog?
- **D6 The admin's five tabs** during the move: keep editing there (now guarded by `physical`), or make them read-only
  with the link, and when to remove them? (Featured / code types / permissions / toons / meta stay in the admin – "Shop
  & codes"? Q8)
- **D7 Products and editions in the creator** at all (commerce), or only items / recipes / albums there? Both are built
  and always count as physical.
- **D8 Recipes' toon**: derived from the output item tonight; a `toon` column instead? (Q5)
- **D9 Physical recipe inputs**: presence suffices – a recipe can be crafted again and again with the same physical
  piece (only the output's `max` limits it). Intended?

---

## Owner decisions (2026-10-12)

- **Not now on main** – played through overnight as a draft in its own worktree / session (branch, own DB), reviewed in
  the morning. A parallel draft moves the map's places into their own documents (agents/MAP-DOCUMENTS.md on its branch):
  "alle einen eigenen Doc, den man versionieren kann – dann ist das gleichmäßig refactored".
- **Physical items need a review by the same rules as everything else:** the admin's review rules (who must be
  reviewed, who reviews, per toon) decide who may publish what – "damit nicht aus Versehen auf der Plattform was
  rausgesetzt werden kann". Enforced on the server.
- **Codes cannot be deleted** – a separate process; nothing to change there.
- **Crafting with physical items:** "physische Items kombiniert verschwinden nicht" – the physical original stays in the
  user's inventory, the crafted result appears in addition (fix InventoryService::remove's fallback to code-bound pieces).

Owner 2026-10-12: "der Katalog ist aber im Admin. Das ist verwirrend. Eigentlich müsste der logischerweise komplett in
den Creator umgezogen werden. Das einzige Problem ist, wie wir die Sache handeln mit den physikalischen Gegenständen, die
ja quasi andere Berechtigung brauchen, weil nicht jeder Creator einfach ein Item anlegen kann – oder doch? Bitte kurz
recherchieren."

Status: the research below (2026-10-12); the overnight draft built on it – summary at the top. Background: agents/CATALOG.md (tables, admin UI), agents/ITEMS.md (item
definition, codes → items, limits, ownership), agents/ACCOUNT-RESTART.md §2 (item sources).

## 1. What the catalog is today

One admin page `/catalog/<kind>` (`admin/src/routes/catalog/[kind]/+page.svelte`, scaffold
`admin/src/lib/components/catalog/catalog-editor.svelte`, kinds `admin/src/lib/catalog/kinds.ts`, one `<kind>-form.svelte`
each). API `/api/admin/catalog/<kind>[/<id>]`, `/reorder`, `/history`, `/history/<n>/undo`
(`server/src/Http/Controllers/Admin/CatalogController.php` → `Domain/Catalog/CatalogAdmin.php`, validation
`CatalogValidator.php`, rows `CatalogStore::KINDS`) – role **admin** only. A write is **live at once** (no draft /
publish / review; readers get it via the public `GET /api/items/`, cached 5 min); safety net = `catalog_history` + undo.

| Kind (table) | Story or commerce? | Why |
|---|---|---|
| items (`catalog_items`, cols `toon`, `edition_id`) | **mixed** | story items (cucumber, record player, coupon …) AND physical twins (`kind: physical`, `edition_id`, `product`, variants' `product`) in one row shape |
| recipes (`catalog_recipes`, no toon column) | story | craft inputs → output; risk: a recipe consuming a coded piece (ACCOUNT-RESTART §2) |
| albums (`catalog_albums`, `toon`) | story-ish | tracks / downloads; `access.download/listen` = `public|members|owners` – "owners" ties it to physical ownership |
| products (`catalog_products`, `toon`, `item_id`, `edition_id`) | commerce | shop entries, price, sizes, stock |
| editions (`catalog_editions`, `toon`) | commerce | numbered physical runs; codes / batches / certificates hang on them |
| featured (`catalog_featured`) | commerce | landing-page list (edited on `/homepage`) |
| permissions + code-types (`catalog_permissions`, `catalog_code_types`) | **security** | what a code grants (claimable, signup, grants) – `Domain/Access/Permissions.php`, "server-only"; the form already asks before saving |
| toons (`toons`) | platform | created by admins only (CATALOG.md: "created by admins only"); creator fills content |
| meta (`catalog_meta`) | technical | versions, currency |

**How an item becomes physical** (ITEMS.md "Codes → items", `Domain/Codes/CodeService.php`):
- a code (`qr`, created by admins in Codes / Batches – `Domain/Admin/CodeAdmin.php`, `Batches.php`) has fixed
  `code_items`, **or** no rows but an `edition_id` → then **every catalog item with that `edition_id` is a claim
  option** (`itemsForEdition`). So setting `edition_id` on an item silently adds it to every unclaimed code of that edition.
- a claim adds the item with `source = qr` + `code` (+ `edition_number`); gifts move it (`gift` with code). Rule of thumb
  (ACCOUNT-RESTART §2): "physical = the row has a `code`" and `kind: physical` in the catalog.
- certificates (`Domain/Certificates/CertificateVerifier.php`) and ownership (`Domain/Ownership/*`) work on codes /
  editions, not on items – but they show the item an edition / code stands for.

Item fields with weight beyond the story: `kind`, `edition_id`, `product`, `variants[].product`, `starter` (every new
inventory gets one), `grantable` (`story|scan` – what readers may self-grant), `max` / `max_world` / `placements`
(limits, ITEMS.md "Collection limits"), `effect` (reader effects, incl. substance items like `psilocybin`, `lsd`),
`skills` (`scan_qr`, `scan_object`, `craft` …), `takeable` (refused for physical by the validator, line ~166).

## 2. Roles and permissions today

- `server/src/Domain/Roles/Roles.php`: `member`, `creator`, `admin`, `developer`. The creator app + `/api/entities/*`
  = `CONTENT_EDITORS = [creator, admin]` (owner 2026-10-01: admin counts "FOR NOW"). `/api/admin/*` = admin.
- **The creator role is global** – no per-toon membership exists. The only per-toon relation is review
  (`Domain/Content/Reviews.php`, table `content_review_rules`, set by an admin): `reviewer` / `needs_review` per toon
  (`*` = all). `needs_review` may save + submit, not publish – and only for toon-level entities.
- "Leads" in the code are Embassy leads (`Domain/Embassy/Embassy.php::setLead`), not toon leads – no toon owner role.
- MCP: catalog read tools (`read_catalog`, `catalog_history`); changes only as `propose_changes` → the admin applies
  (agents/MCP.md).
- The creator already **reads** the catalog (`creator/src/lib/editor/catalog.svelte.ts` → `/api/items/`) for the item
  picker, placed items, giveItem, related items; its item inspector says "the item's rule (catalog, admin → Catalog)".
  Owner 2026-10-06 (BACKLOG 10.38): "with script we can not create items" – an item exists only in the catalog.

**The editors are NOT shared.** All forms live in `admin/src/lib/components/catalog/` and use the admin's paraglide
messages (`m['catalog.*']`) and `$lib/catalog/kinds.ts`. Only the media parts are shared (`@scutoons/media`: MediaSlot,
asset picker, upload). A move = extract them into a shared package (e.g. `@scutoons/catalog`) or re-home them.

## 3. Options

### (a) Split by meaning: story catalog in the creator, commerce stays in the admin
- Creator gets **Items, Recipes, Albums** per toon (e.g. `/<toon>/items`, plus a cross-toon "SCUTOONS" bucket for
  `toon: null`); admin keeps **Products, Editions, Featured, Permissions / code types, Toons, Meta** (+ Codes, Batches).
- Server: new creator endpoints (or open `/api/admin/catalog/{items,recipes,albums}` to `requireContentEditor`) with a
  **field guard**: a creator may not set / change `kind: physical`, `edition_id`, `product`, `variants[].product`,
  `max_world`, and may not edit an item that already is physical. Recipes need a toon (today none) or stay global.
- UI: move item / recipe / album forms + editor scaffold into a shared package; the admin keeps a read-only (or full)
  view of items for commerce links.
- Risk: items are one row shape for both worlds – the split runs THROUGH the item, so the guard is needed anyway.
  Two places to look for "the catalog". Live-at-once writes by every creator (no review).

### (b) Everything in the creator, the physical parts behind a permission
- The whole catalog page moves to the creator; commerce / security tabs and fields only for a new capability (e.g.
  role check `admin`, or a new `catalog:commerce` – note: `catalog_permissions` are READER permissions granted by codes,
  not staff rights; staff rights are roles, so this would be a role or a Reviews-like rule, not a `catalog_permissions` row).
- Server: one controller for both; per kind + per field `require` (admin for products, editions, code types, featured,
  toons, meta; physical fields on items).
- Risk: code types / permissions are security (signup, grants) – living in the content app widens the attack
  surface to every creator session + MCP token; the admin's Codes / Batches / Claims still need the editions next to
  them, so the admin would link out to the creator for half its domain. Biggest refactor.

### (c) Items in the creator, "make physical" only by an admin
- Like (a), but sharper: creators create and edit **virtual** items (+ recipes, albums with `access` ≠ `owners`); the
  item form shows the commerce block (kind, edition, product, variants' product, `max_world`) read-only with "set in the
  admin". The admin gets an item action **"Make physical"** (link to edition / product / code type) – the only path
  from story item to claimable twin; physical items are read-only in the creator.
- Server: creator-scoped writes refuse those fields + any write to a `kind: physical` row (403 `needs_admin`); the admin
  endpoints stay as they are. Optional: creator writes go through the content review rule (`needs_review` for that
  toon → a catalog change waits for a reviewer) – today the catalog has no draft state, so that is new work.
- Risk: smallest; one item form with two permission levels.

## 4. "Kann nicht jeder Creator einfach ein Item anlegen?"

**Yes for story items, no for physical ones.** A virtual item does nothing until a story places / grants it, and
creators already place items and write scripts that grant them – letting them define the item next to the story is
consistent. What must stay with the admin is everything that touches money, codes and access:
- `edition_id` on an item = instantly claimable from every unclaimed code of that edition (silent, no code change);
- `kind: physical`, `product`, products / editions / featured (shop, certificates, edition numbers);
- code types / permissions (signup + reader access – security);
- creating toons (already admin by decision).

Guard-rails even for virtual items (creator writes are live at once, the creator role is global):
- `starter` (every new inventory gets it) and `grantable: ["scan"]` act across ALL toons – admin-only or reviewed;
- `max_world` / `max` / `placements` – a creator setting `max_world: 1` creates a "unique" item; owner call who may;
- `effect` (substance effects) + sensitive themes – minors read SCUTOONS (the `hide_sensitive` setting exists): review;
- a creator edits only items of toons they work on (needs a toon ↔ creator relation, which does not exist yet), never
  `toon: null` (cross-toon) items; renames / deletes of released ids are refused already (`in_use`, packs rule 2).

**Recommendation: (c)** – items, recipes and albums move into the creator (per toon, next to the story that uses them);
"make physical" and all commerce / security kinds stay in the admin; the server guards the fields, not the UI. Then
the admin's catalog page shrinks to "Shop & codes" (products, editions, featured, code types, permissions, toons) and
the naming confusion goes away. First step: extract item / recipe / album forms into a shared package
(`@scutoons/catalog`, like `@scutoons/media`).

## 5. Open owner questions

- Q1. (a) / (b) / (c)? (recommended: c)
- Q2. Which creators: every creator, or only per toon? Today the role is global – do we need a toon ↔ creator relation
  (also useful for reviews), or is `needs_review` per toon enough?
- Q3. Do catalog changes by creators go through review (draft → reviewer publishes) like content, or live at once
  with history / undo as today? (Review needs a draft state the catalog doesn't have.)
- Q4. Who may set `starter`, `grantable: scan`, `max_world`, `effect`? (proposal: admin, or reviewed)
- Q5. Recipes have no toon – global (admin) or per toon (migration: a `toon` column)?
- Q6. Albums with `access: owners` (tied to physical ownership) – creator or admin?
- Q7. Cross-toon items (`toon: null`, e.g. camera, cucumber) – who owns them?
- Q8. Name of what stays in the admin ("Shop & codes"?), and does `/homepage` featured join it?
