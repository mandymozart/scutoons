# NEUTRAL – one language-neutral document per chapter, scene, season (owner 2026-10-06)

Owner: "I would love to do the entire rewrite with the real language-neutral chapter entry. It is early in the project,
so better to make these decisions now." Audit of every model (2026-10-06): the per-locale documents `chapter`, `scene`,
`season` (and `toon-settings`) hold structure that must be the same in every language; the creator copies it to every
locale (`structural`, `setShared` with overrides, `unifyGeometry`/`unifyAudio` on load) and `validateLocales` checks only
part of it (not `renderer`, `label`, slice `items`/`events`/`video`/`audio`, VN events, `isPreview`, season `download`…).
Real drift already exists (catpope ch2 scenes' `interaction` en-only, season covers, credits).

## Decision: model "inline locale maps" (like `map`, `characters`, `roles`)

One document per entity, stored under the entity itself (no locale segment): `chapter:<toon>:<n>`,
`scene:<toon>:<chapter>:<name>`, `season:<toon>:<n>`. Structure exists once. Every **text** leaf becomes a locale map
`{ "de": …, "en": … }` (`LocaleText`, `Record<locale, string>`; dialog text `Record<locale, string | string[]>`).
Fields that are legitimately per language but not text (`release`, season `sequence[].status`) are locale maps too.

Why not "neutral structure + per-locale text overlay": everything aligns by index today (slices have no id, VN events
and choices have none, `Dialog.id` optional); an overlay needs stable ids everywhere and two documents per edit (two
undo steps, two 409s, non-atomic saves). Inline maps need no ids, save atomically, one undo stack, one history.
Cost: two people editing different languages of the same chapter at the same time conflict (409, reload) – acceptable
for this team; noted.

### What is text / per language (everything else is neutral)

| Document | Locale maps |
|---|---|
| chapter | `title`, `description`, `release` |
| group (inline or scene) | – |
| slice dialog (text box) | `text` |
| VN `showMessage` | `params.text` |
| VN `addChoice` | `params.label` |
| season | `title`, `description`, `release`, `sequence[].status` |
| season cover | `lettering.<fmt>[].custom` is already a map |

Dropped from stored documents: `language` (the served locale), chapter `status` (deprecated mirror – computed from the
season when served), per-locale overrides of neutral fields (image, text-box style, `label`, `renderer` …): the source
locale wins at conversion, every dropped difference is listed in the conversion report.

### What the reader gets: unchanged

The library resolves a neutral document for the asked locale into today's per-locale shape (`Chapter`, `Group`,
`Season` as now, `language` = served locale, chapter `status` from the season). A text missing in the asked locale falls
back to the toon's source language (then any language); the answer's `fallback` is set when the asked locale has no
`title`. Older viewer builds keep working. `Library::FORMAT` bumped. Resolution exists twice – TS
(`@scutoons/content`, the creator's previews) and PHP (`Library`) – tested against the same golden fixtures.

### Migration (production has no shell)

- `neutralize*` (TS, `@scutoons/content`): per-locale documents → one neutral document + a report of dropped
  differences. Pure, tested.
- Creator page **Convert** (owner): per toon, a dry run (report per entity) → "Convert" saves and publishes the neutral
  entities with the owner's session. Idempotent; the per-locale keys stay untouched (rollback = delete the neutral
  entity). Local dev: a script runs the same function against the dev database.
- Library: a neutral entity wins; without one it serves the per-locale documents as today (legacy path), so the deploy
  works before the conversion. The legacy path and the per-locale kinds' write paths are removed once production is
  converted (follow-up, CUTOVER).
- Verification: for every chapter / season × locale, the library answer from the neutral entity equals the legacy
  answer (sorted JSON), apart from the reported differences.

### Creator

One `ContentDoc` per entity with the neutral locale `_`; text fields edit `text[<header text language>]` (the header's
language switch stays the only one). No `structural` / `setShared` / overrides / `unifyGeometry` / `unifyAudio` for these
documents. A "new language" creates nothing – texts are simply missing (warnings per locale, like today's "missing en
text"). Validation runs once on the document plus per-locale completeness; `accepted_warnings` in the neutral document
(per-locale warnings keep their locale in the key).

### Toon settings

`toon-settings:<toon>:<l>` (title, description, release, status) folds into the toon entry `toon:<toon>` as locale
maps the same way – after chapter/scene/season, same pattern. Then the only per-locale kind left is `script` (a
translation by design).

## Work order (branch `chapter-neutral`, never `main`)

1. ~~Foundation~~ done 2026-10-06 (see Phase 1 below): types, keys, schemas, resolve (TS + PHP), neutralize (TS),
   library read path + legacy fallback, server validation/schemas/AssetUsages/Reviews/ContentFiles, conversion script +
   equivalence check on the dev DB (sandbox / fixtures stay per locale until phase 2).
2. Creator: chapter editor (inline groups + scenes), season editor, cover editor, visibility, new chapter / season /
   language, Convert page, MCP tools.
3. Linked groups (`groups/chains`) merged in; its fields live in the neutral document.
4. Toon settings into the toon entry.
5. Creator messages refactor (owner 2026-10-06): `creator/messages/{de,en}.json` + hints/tooltips in the code still
   name JSON files that no longer exist since the content lives in the database – say what it is today (the document,
   published / draft), shorter and consistent wording, drop unused keys.
6. Full checks, browser tests creator + viewer.

## Phase 1 "Foundation" – done (2026-10-06, branch `chapter-neutral`)

- **Types / schemas** (`@scutoons/content` `neutral.ts`): `ChapterDoc`, `GroupDoc` (inline group and scene), `SliceDoc`,
  `DialogDoc` (`text: DialogText` = `Record<locale, string | string[]>`), `VNEventDoc` (showMessage `text`, addChoice
  `label` as `LocaleText`), `SeasonDoc` (`sequence[].status: LocaleStatus`). Schemas `chapter-doc`, `group-doc`,
  `season-doc`; the server's save guard and `content:check` use them for a key without locale
  (`ContentSchemas::NEUTRAL_BY_KIND`), the old schemas for per-locale keys. Served types unchanged. Also typed:
  showMessage `params.mood`, the legacy `Group.interaction`.
- **Keys**: chapter / scene / season accept the neutral key and the legacy per-locale key (PHP `EntityKey::parse`,
  `NEUTRAL_DOC_KINDS`, `isNeutral()` / `isNeutralDoc()`; TS `NEUTRAL_DOC_KINDS`, `isNeutralKey`, `isNeutralDocKey`).
  `entityKeyFor` is unchanged: every caller (creator doc / labels / nav, MCP) still reads and writes per-locale keys.
  A four-part `scene:<toon>:<n>:<x>` is a neutral scene named `x` (the legacy form has five parts).
- **Resolve** (TS `resolveNeutral` / `servedLocale` / `resolveText`, PHP `NeutralDoc`): the same rules, golden fixtures
  `shared/content/test/fixtures/neutral/*.json` read by `neutral.test.ts` and `NeutralDocTest`.
- **Neutralize** (TS `neutralizeChapter` / `neutralizeGroup` / `neutralizeSeason` / `neutralizeDocs`), report kinds:
  `differs` (a neutral field differs; the source wins), `structure` (an array's length or a VN event's type differs:
  that locale's texts below are not taken), `text-lost` (those texts), `text-missing` (blank / missing in a locale: it
  shows the source's), `source-missing`.
- **Library**: `Context::pick` prefers the neutral document, else the per-locale ones (unchanged); scenes likewise, so a
  neutral chapter with a per-locale scene and the reverse both work; toon bundle, season overview, summaries, chapter
  gates, `cast?chapter=` read neutral seasons / chapters. `FORMAT` 5. `Library::legacyOnly()` = the answers without
  neutral documents (another cache key).
- **Server odds and ends**: reviews need no change (a neutral key belongs to its toon; the mails list "chapter 2"),
  AssetUsages indexes a neutral key once, AssetLayout reads neutral seasons, interchange layout
  `<toon>/_/chapter<N>.json`, `<toon>/_/chapter<N>/<scene>.json`, `<toon>/_/season<N>.json`. EntityStore unchanged.
- **Conversion (local)**: `npm run dev:neutralize [-- --apply]`, check: `npm run dev:db -- console content:neutral-check`.

### Decisions (made overnight, owner to confirm)

1. **Blank = missing.** A text that is empty or whitespace (JS `trim()`; the server matches it) falls back like a missing
   one (source, then the toon's other languages, then any) – like the cover lettering's `custom` texts. So untranslated
   en text boxes stored as `""` now show the German text instead of an empty box (chapter 2, below).
2. **Served locale** of a neutral chapter / season = the asked locale when its `title` has a text there, else source →
   toon languages → the document's first locale (the legacy path 404s when the only document is in a locale the toon
   doesn't list; the neutral path serves it). A scene has a locale when any of its texts has it; a scene without any text
   serves every locale (no fallback flag).
3. **Chapter `status`** in the chapter answer = the season's status in the *served* locale (what the creator mirrored);
   left out when no season lists the chapter; never part of the answer's fallback flag.
4. **Season `sequence[].status`** missing in a locale → the source's (like a text); none at all → no status (= published
   for the gates, as before).
5. **`accepted_warnings`**: the union of every locale's list (keys are hashes of per-locale values; the creator phase
   re-evaluates them on the neutral document).
6. **Drafts**: the conversion stores a published neutral version from the published per-locale versions and, when a
   locale has a newer draft (or the entity was never published), a draft neutral version from the newest ones after it.
7. **Sandbox generator, server test fixtures, MCP tools stay per locale** for now (they exercise the legacy path; the
   creator still edits per-locale documents) – switched together with the creator (phase 2).
8. Numbers: a stored `1.0` comes back as `1` from the conversion (JSON.stringify) – the same value for every reader.

## Conversion 2026-10-06 (dev DB)

Dev DB `scutoons` (production dump reloaded by the coordinator on 2026-10-06, 192 entity versions). Per-locale documents
exist for catpope only (toptoy and yellowmouse have none): 6 chapters (0–5), 6 scenes of chapter 2 (scene-7 never
published), season 1, each de + en. `npm run dev:neutralize -- --apply` stored 17 new versions (ids 193–209: 13
published + 4 drafts for entities whose newest per-locale versions are drafts – chapter 2, scenes 14, 17, 18 – and
scene-7 as a draft only); a second run stores nothing. 524 reported differences (source de wins):

| Entity | Report |
|---|---|
| chapter 1 | **70 text boxes styled differently in en**: the en boxes carry `mask: true` + `maskShape` + `pad` (the deprecated cover for lettering baked into old art) and a tuned `size` 4.04–4.15 instead of 4; 11 boxes `variant` bubble (en) vs caption (de), 5 `frame`, 2 `uppercase`; de sets `align` / `valign` / `font` explicitly where en uses the defaults |
| chapter 2 (published + draft) | 9 en text boxes are `""` (group "scene-17-24": narration and captions untranslated, one de placeholder `dfsfsdf…`) → en shows the German text |
| scene-14 (published) | 2 en VN messages are `""` (the cucumber hint) → German; `events.0.params.wait: true` only in de (kept) |
| scenes 14–18 | `interaction: "visual-novel"` only in en → dropped (nothing reads it) |
| season 1 | deprecated `credits.music` / `credits.sound`: de "Nova Lux", en "Nova Ludwig" → "Nova Lux" (the viewer never rendered season credits) |

**For the owner (source-wins is implemented):** the chapter 1 en text-box styling. The art is shared (no per-locale
`image` anywhere) and text-free since 2026-10-03, so the en masks look obsolete – but the en `size` / `variant` tuning
changes how chapter 1 looks in English. And the 9 + 2 untranslated en texts in chapter 2 now show German. No per-locale
slice images, no per-locale covers, no structural mismatches (every array lined up by index).

**Equivalence check** (`content:neutral-check`: de / en / fr × toon bundle, season 1, chapters 0–5, + cast per chapter,
published and draft mode = 60 answers): 53 identical (served locale, fallback flag, missing references, 403/404
refusals, bodies); 7 differ, every difference explained by the report: published + draft chapter 1 en (the text-box
styling: 493 leaf differences in size / mask / maskShape / pad / align / valign / font / variant / frame / uppercase),
published + draft toon bundle en and season 1 en (season `credits`), draft chapter 2 en (3 × `interaction`, 9 blank
texts → German). Nothing to fix.

Pre-existing (not from the conversion): the published scene-14 versions (de #127, en #128 – and so the neutral #201)
don't fit the group schema (an `addChoice` jumpTo `value` is a number; an `end` event type); the newer drafts are fine.
`content:check` reports it.

### Open for the creator phase

- After a conversion the creator still edits the per-locale documents while the reader and the creator's draft preview
  show the neutral ones: per-locale edits are shadowed. Production must not be converted before phase 2 ships.
- Creator: one `ContentDoc` per neutral entity, the Convert page (dry run per toon = the `neutralizeDocs` report, then
  save + publish), new chapter / season / language, validation of neutral documents (`validateFile` /
  `validateLocales` work on per-locale documents today: resolve per locale + check, plus per-locale completeness).
- MCP tools (`tools/mcp/src/server.ts`) read / write per-locale keys (`entityKeyFor`): switch with the creator.
- Sandbox generator (`make-sandbox.ts`) and `server/tests/fixtures/content` → neutral documents.
- Once production is converted: remove the legacy per-locale path (library fallback, per-locale schemas of these kinds,
  the write path) – a CUTOVER follow-up.
