# NEUTRAL – one language-neutral document per chapter, scene, season (owner 2026-10-06)

Owner: "I would love to do the entire rewrite with the real language-neutral chapter entry. It is early in the project,
so better to make these decisions now." Audit of every model (2026-10-06): the per-locale documents `chapter`, `scene`,
`season` (and `toon-settings`) hold structure that must be the same in every language; the creator copies it to every
locale (`structural`, `setShared` with overrides, `unifyGeometry`/`unifyAudio` on load) and `validateLocales` checks only
part of it (not `renderer`, `label`, slice `items`/`events`/`video`/`audio`, VN events, `isPreview`, season `download`…).
Real drift already exists (catpope ch2 scenes' `interaction` en-only, season covers, credits).

## For the owner (state 2026-10-06 morning) – everything on branch `chapter-neutral`, not merged, not pushed

Done: steps 1–5 below + linked groups (`Group.continues` / `Group.end`, merged from `groups/chains`) + the tokens check
fix. All checks green: `npm run check`, every `check:creator-*`, `check:mcp`, PHPUnit 269, phpstan, `content:neutral-check`
(only the 7 explained catpope differences). The dev DB `scutoons` is fully converted.

**Owner decisions (2026-10-06):**
1. Text-box styles are the same in every language – no per-language style overrides (source wins, as converted).
2. A missing translation falls back to the source language (as built).
3. `/library/toons?locale=` leaves out a toon whose page is closed in that language – better (as built).
4. Open: the toon entry stores the translated name twice (`i18n.<l>.name` for landing / menu / creator nav, `title.<l>`
   for the toon page / cover) – proposal: keep only `title` (still per language), drop `i18n`.

**Ship (when you like it):** merge `chapter-neutral` into `main`, push (deploys api, creator, viewer, admin). Readers see
no change – without neutral documents the library serves the per-language ones as before. Then in the creator, per
toon: the bar "still stored per language" → **Convert** (dry run first, then Convert). Afterwards (follow-up): remove the
legacy per-language path (library fallback, per-locale schemas, MCP legacy read, the Convert page).

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
2. Creator: ~~2A editing core~~ done 2026-10-06 (see Phase 2A below: document model, in-memory conversion, Convert
   page, chapter editor, validation, history). ~~2B~~ done 2026-10-06 (see Phase 2B below: season editor, cover editor,
   status per language, new chapter / season / language flows, MCP tools, make-sandbox, test fixtures, dev checks).
3. Linked groups (`groups/chains`) merged in; its fields live in the neutral document.
4. ~~Toon settings into the toon entry~~ done 2026-10-06 (see "Step 4 – toon settings" below).
5. ~~Creator messages refactor~~ done 2026-10-06 (see "Step 5 – creator wording" below; the glossary is in
   agents/DESIGN.md "Creator wording").
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

- ~~After a conversion the creator still edits the per-locale documents~~ – phase 2A: the creator edits only neutral
  documents.
- MCP tools (`tools/mcp/src/server.ts`) read / write per-locale keys (`entityKeyFor`): switch with the creator (2B).
  Until then an Ask-panel edit of a converted chapter writes a shadowed per-locale version.
- Sandbox generator (`make-sandbox.ts`) and `server/tests/fixtures/content` → neutral documents (2B).
- Once production is converted: remove the legacy per-locale path (library fallback, per-locale schemas of these kinds,
  the write path) – a CUTOVER follow-up.

## Phase 2A "Creator editing core" – done (2026-10-06, branch `chapter-neutral`)

- **Document model** (`creator/src/lib/editor/doc.svelte.ts`, `languages.ts`): `toonDoc` gives a chapter / scene /
  season ONE `ContentDoc` under the entity key (locale `_`) with `texts = { source, languages }` (the toon's). New:
  `doc.neutral`, `textLanguages` / `textSource`, `edit(change)` / `set(path, value)` (one undo step on the document),
  `text(path, locale)` / `setText(path, locale, value)` / `hasText` (a text leaf's locale map; a blank text leaves the
  language out – it falls back like a missing one), `resolvedGroup` / `resolvedSlice` (the reader's shape in a locale,
  `resolveGroup` / new `resolveSlice`, for the canvas and the strip preview). `LocaleText` (fields/locale-text.svelte)
  is the one text field for both models (header text language, source, or all; translate mode; `type="date"` for a
  `release` per language). Toon settings and the script stay per locale (`structural` / `setShared` / `setLocal` remain
  for them; `prop-field` lost its per-locale override mode – it only edits neutral documents now).
- **Removed**: `unifyGeometry` / `unifyAudio` (`@scutoons/content` geometry.ts + tests), the load notices, the
  per-locale override rows (slice image, thumbnail, prop fields), the chapter's status mirror on save (`syncChapter`
  now drops `published` / `status` / `language` – a neutral chapter stores none), "create <locale> from <source>" for
  neutral documents (a language is just missing texts).
- **Opening an entity without a neutral document**: `load()` gets the neutral key; on 404 it loads the newest
  per-locale documents, `neutralizeDocs` → the document (unsaved: `doc.converted` = report + locales, `unconverted` until
  stored). DocNotices shows ONE bar for the page's converted documents ("converted from the per-language documents –
  saving stores the language-neutral documents") with the differences readers will see folded (`visibleDifferences`:
  text-lost, text-missing, structure, `differs` except the invisible `interaction` / season `credits`). Saving writes
  the neutral keys (chapter + the scenes the editor loaded + the season shown in the meta panel); the per-locale keys
  are never written by the creator again.
- **Decision**: the season page and the toon page don't save converted documents as a side effect (`unconverted` docs
  are left out of their companions / page docs) – otherwise publishing a season title would publish every chapter's
  newest per-locale draft. The chapter editor does save + publish the season it shows (it is one of its documents,
  named in the publish confirmation).
- **Convert page** `/<toon>/convert` (`lib/editor/conversion.ts`, `components/toon/convert-page.svelte`): the dry run
  runs on open (content index → every chapter / scene / season stored per locale WITHOUT a neutral document, their
  published + newest versions → `planConversion` from `@scutoons/content`, shared with `npm run dev:neutralize`); per
  entity: the steps (published / draft) and the visible differences folded. "Convert" (confirmDialog) stores per entity
  the published version (saved, then published) and the draft after it. Idempotent: entities with a neutral document
  are never touched (the creator may have edited them) – a second run lists nothing. Reached from the toon page's bar
  "… still stored per language. Convert …", shown only while the season or a chapter is unconverted. Note: an entity
  whose neutral document is a draft only (saved in the editor, never published) is not listed – publish it in the
  editor.
- **Validation** (`validateNeutral` in validate.ts): the document resolved for the source language through
  `validateFile` once (problems without a locale; a chapter's `seasonStatus` = its season entry's status in the source),
  the locale maps' shape (a text per language, `release` a date per language, `sequence[].status` a status per
  language – errors with the locale), and per toon language the completeness: missing text (warning, with the start of
  the source text), the same as the source (info), > 130 % of the source for text boxes / VN messages / choice labels
  (warning), a source without text where another language has one (warning on the source). `forEachText` (neutralize.ts)
  walks every locale-map leaf by the conversion's rules. `validateLocales` stays for the per-locale documents.
- **Accepted warnings**: `acceptedWarningKey` takes an optional `scope`; the creator passes the warning's locale for a
  neutral document (`en:<place>#<hash>`), per-locale documents' keys are unchanged. The keys converted from per-locale
  documents (decision 5) don't match the neutral document's values – those warnings show again once (accept anew).
- **History / restore**: generic per key – checked on `chapter:catpope:2` / scenes (restore API → a new draft with the
  old content; the History panel lists the neutral key's versions, no `_` heading; the confirm names the languages).
- **Server**: nothing changed (the save guard, reviews and the library already handle neutral keys).
- **Checks**: `check:creator-chapter` (catpope ch1 – converted – one neutral key; publish confirm names de, en),
  `check:creator-cover` (the sandbox's neutral season, `validateNeutral`) adapted; `dev:test-user` retries the Cat Pope
  unlock with the looks the server knows (the library listed `naked`, the server only `swim` – a data mismatch).

### Dev DB state after phase 2A testing (2026-10-06)

- catpope chapter 2 + scenes 14 / 18: the browser test's edits were restored (new drafts #325–327 with the content of
  #196 / #202 / #208).
- **sandbox converted** (in the editor: chapter 1 + its scenes + season, published by hand; the rest with the Convert
  page). Since 2B make-sandbox writes neutral documents itself – no caveat any more (Phase 2B).
- `check:creator-chapter`'s VN part failed on today's content (catpope scene-14 lost its script) – fixed in 2B (the
  check looks for a scene with a script).

## Phase 2B "Creator flows, tools, sandbox, fixtures" – done (2026-10-06, branch `chapter-neutral`)

- **Season page** (`season-editor.svelte`): title / description / release via LocaleText (the header's text language),
  everything else once through `PropField` (now also `type="list"` and a `testid`): authors, supporters (name / url),
  trailer, related items, the cover link. No `SharedField` / per-language split on any neutral document.
- **Cover editor**: writes `cover.<format>` / `cover.lettering` once with `doc.set` (one undo step); a custom line's text
  is the locale map `custom` in the header's text language, as before.
- **Status per language – decision**: edited in ONE place, the chapter's meta panel (where the chapter is edited; the
  season page has no chapter list since the owner's 2026-10-03 "no second list"), and like every other per-language
  field it follows the header's text language: source (default) = one select, a language = that one, "all" = side by
  side. A language without its own status shows the source's (the reader's rule). Lists (toon page, season sidebar)
  show it read-only as badges ("Draft · en"). `visibility.ts` reads / writes only the neutral season's locale map
  (`sequenceIndex` lost its locale argument; an older plain-string status counts for every language).
- **New chapter / season / language** (toon page): "New chapter" and "Create season 1" store neutral documents
  (`newChapterDoc` / `newSeasonDoc`, `sequenceStatus` = draft in every language); a new language = the language in the
  toon entry, nothing else – texts missing (warnings, "missing" chips). Only the per-locale toon settings still get the
  "create <l> from <source>" bar (until they move into the toon entry). Browser-tested on toptoy (no season before):
  season + chapter 1 created, `es` added, the library answers de / en / ko (fallback) / fr (fallback); then removed again
  (the test versions deleted from the dev DB). "New toon" is not a creator flow (toons are created in the admin
  catalog); a toon without a season gets "Create season 1".
- **Dead code removed**: `ContentDoc.structural` (cast / credits / map / characters editors use `edit`), the toon page's
  scene loading + per-locale copies for content documents, `visibility.ts`'s per-locale season branch. Kept for the
  per-locale toon settings / script: `setShared` / `setLocal` / `overrides` / `createLocale`, `SharedField`
  (toon-editor), DocNotices' "create <locale>".
- **MCP tools** (`tools/mcp`): a chapter / scene / season entity = its one neutral document (`keysOf`); `read_entity`
  reads it (an entity not converted yet: its per-locale documents, marked `legacy`, read only); `save_draft` refuses a
  per-locale key of these kinds ("save `chapter:catpope:2`"); `validate` / the save check use `validateNeutral` (toon
  languages from the toon entry, a scene's chapter audio from the neutral chapter); `list_chapters` reads the neutral
  chapter / scenes (falls back per locale), title in `locale` (fallback chain), status from the season's locale map;
  `search_content` skips per-locale documents shadowed by a neutral one and, with `locale`, the other languages' entries
  of a locale map. Guidance (`scutoons://rules/workflow` "Languages", entity-keys, text presets, the Ask panel's system
  prompt): texts are locale maps, structure once, set one language at `[…, "text", "en"]`.
- **Sandbox** (`make-sandbox.ts`): writes `_/chapter<N>.json`, `_/chapter<N>/<scene>.json`, `_/season1.json` (texts
  de + en from the generator's `Text` maps, ko titles / descriptions; the dialogue has no ko – readers get German, the
  creator warns). **Decision**: before the import it deletes the sandbox's per-locale chapter / scene / season versions
  (+ their asset usages) from the dev DB – the sandbox is generated, nobody edits it per locale, and a stale per-locale
  version behind the neutral one only confuses (Convert page, history, `content:neutral-check`). On the current DB:
  75 removed, 15 neutral versions updated; a fresh DB gets neutral ones only. `content:neutral-check` skips a toon
  without per-locale chapters / seasons ("nothing to compare with": the sandbox) – still 60 catpope answers, the same 7
  explained differences.
- **Fixtures**: `server/tests/fixtures/content/catpope/_/{season1,chapter1}.json` (neutral, made from the old de / en
  files with `neutralizeDocs`; the en credits "Nova Ludwig" dropped like the real conversion); `albums.test.ts`
  validates / resolves them. `LibraryTest`: `seedToon()` is neutral now (the library's main path; summaries carry the
  served `language`); the old per-locale seed is `seedLegacy()`, used only by the neutral-vs-legacy tests (fallback
  path, mixed neutral / legacy). Kept with per-locale keys on purpose: `ReviewsTest`, `EntityStoreTest`, `AssetsTest`
  (they test key handling / the review flow with generic documents; legacy keys stay valid until production is
  converted).
- **Dev checks**: `check:mcp` on the neutral scene key (+ the per-locale key refused, a missing `en` text is a warning
  with its locale; robust when the scene has no script yet; inline group labels instead of the gone `scene-20`);
  `check:creator-chapter`'s VN part picks a scene with a group script and slice events (catpope's first, else the
  sandbox's – today `scene:sandbox:1:regular-default`).

### Checks 2026-10-06 (after 2B)

`npm run check` (root), `check:tools-css`, `check:creator-chapter` (59), `check:creator-cover` (36),
`check:creator-cast` (19), `check:creator-script` (28), `check:mcp` (33), PHPUnit (267 tests), phpstan clean,
`content:neutral-check` (60 compared, the 7 known differences), `content:check` (only the pre-existing scene-14 #127 /
#128 / #201 misfits). Browser (screenshots `2b-*`): season page (texts de / en / ko, authors, a supporter, trailer
poster + chapter), status per language (all / en only), cover editor (own layout, a custom en line stored once),
toptoy new season + chapter + `es`, the viewer reading sandbox chapter 1 in de and en (draft preview).

### Left (later steps)

- ~~Toon settings into the toon entry (step 4)~~ – done; `SharedField`, `setShared` / `overrides`, DocNotices' "create
  <locale>" are gone (the script keeps `setLocal` / `createLocale`).
- Creator message wording (step 5): e.g. "Create season 1 … as drafts in every language", `locale_files_created`.
- After production's conversion: remove the legacy per-locale path (library fallback, per-locale schemas, MCP's
  `legacy` read, `seedLegacy`).

## Step 4 "Toon settings" – done (2026-10-06, branch `chapter-neutral`)

`toon-settings:<toon>:<l>` (title, description, release, status, `id`, deprecated `cover` / `download`) folds into the
toon entry `toon:<toon>` as locale maps. After this the only per-locale kind left is `script` (+ the legacy read-only
`locations` / `routes`).

- **Model** (`ToonIndex`, `TOON_TEXT_FIELDS`): `title`, `description`, `release` (`LocaleText`) and **`languageStatus`**
  (`Record<locale, Status>`). Decision: a new field name, because the entry's own `status` already is the whole toon's
  visibility (admin catalog / list) – a language's page needs both (entry public AND its language status public, as
  before with the settings' `status`). `hasToonTexts(entry)` = the entry has a `title` map ("converted"). `i18n.<l>.name`
  stays as it is (the lists' name, the admin's) – it duplicates `title` (same values everywhere today); folding them is
  an open owner question. Schemas regenerated (`toon-index`); `ToonSettings` / `entities.toonSettings` are `@deprecated`,
  the key stays valid (legacy).
- **Resolution** (`resolveToon` TS = `NeutralDoc::toon` PHP, `resolveNeutral(kind 'toon')`, golden fixtures
  `toon.json` / `toon-partial.json`): the served locale by the `title` like a season; the answer is exactly the old
  per-language document: `id`, `title`, `language`, `release` / `description` when the entry has them, `status` = the
  language's `languageStatus` (missing → the source's → …; none at all → no status = published).
- **Library** (`FORMAT` 6): `toonSettings()` reads the entry's maps when `hasToonTexts`, else the per-locale documents
  (asked → source → languages) – production works before its conversion. `visibleToon` gates on the served language's
  status (404) and sets the fallback flag; `entry` in the answers drops `TOON_TEXT_FIELDS` too (unchanged shape).
  **`/library/toons?locale=`** (new, the viewer passes its locale): a toon whose page is closed in that language is left
  out (same gate as the toon page; announced toons stay "soon"); without a locale the list is as before. Decision: needed
  so a translation that is still a draft doesn't put a dead link on that language's landing page (before, the list
  ignored the language). `legacyOnly()` also strips the entry's texts; `legacyOnly(false)` strips only those (the toon
  settings compared on their own); `legacy:` / `legacy-toon:` in the cache key.
- **Conversion** (`neutralizeToon`, `planToonConversion`; `npm run dev:neutralize`, the Convert page lists "Toon entry"):
  the published entry + the published settings → a new published entry version; when the entry or a language has a
  newer draft, the newest entry + the newest settings → a draft after it. Nothing when the entry already has texts, has no
  version (an entry is never created – its status would open the toon) or there are no settings. Leftovers: `language`
  dropped; `id` reported when it differs from the entry's; a neutral setting the entry has → dropped (`invisible`: the
  reader always took the entry's), one it lacks → moved into the entry; `cover` (the reader's deprecated fallback) /
  unknown fields dropped (visible), `download` dropped (`invisible`); `accepted_warnings` united. `NeutralDifference.
  invisible` (new) keeps such lines out of the creator's notice / Convert page; the script prints them.
- **Validation**: `validateNeutral('index', …)` = `checkToonIndex` + the texts resolved for the source (empty title /
  description, no release: warnings) + the locale maps' shape (release a date, `languageStatus` a status per language)
  + per-language completeness (missing / same as the source).
- **Creator**: the toon settings page edits ONE document, the entry (`toonIndexDoc(toon, languages)` → `texts`):
  title / description / release via `LocaleText` (the header's text language), the status per language like the
  chapter's (source / one / all; a language without its own status shows the source's, muted), the neutral settings with
  `PropField` / media slots. An entry without texts is converted in memory from the newest per-language settings on load
  (`ContentDoc.loadToonSettings`, the "converted" notice; `unconverted` now compares the base with the base it was
  converted on – the entry has one); saving the page stores it. The toon page's own entry doc (languages) has no texts →
  never converts as a side effect; its Convert bar also shows while toon settings exist and the entry has no texts. No
  "create <l> from <source>" any more, no toon settings created with a new season / chapter, the cover editor reads the
  toon title from the entry. Removed: `SharedField`, `setShared` / `overrides` / `resetOverride`, DocNotices'
  create-locale bars, MetaPage `translationsOptional`, `newToon` / `toonName` (defaults), 7 unused messages.
- **New toon**: the admin's "create toon" stores the entry with `title` = the name in de + en and `languageStatus` draft
  (no per-language document; CatalogTest).
- **MCP**: `toon:<toon>` validated with `validateNeutral('index')`; `save_draft` refuses a `toon-settings` key ("save
  toon:<toon>"); `read_entity` marks them legacy / read only; `list_toons` shows `title` + `languageStatus`; guidance and
  the Ask prompt say the toon settings are locale maps in the entry. **Sandbox**: `make-sandbox` writes the texts /
  release / status per language into the sandbox entry, no `toon.json`, and deletes earlier runs' per-locale toon
  settings (3 removed on the dev DB). Server test fixtures: none hold toon settings (LibraryTest's `seedToon()` now puts
  them in the entry; `seedLegacy()` keeps per-language ones for the fallback tests).

### Dev DB 2026-10-06 (after step 4)

Converted: catpope (#380 published from toon#1 de#21 en#36, #381 draft from toon#167 – the owner's unpublished entry
draft with `announce` + another tile stays a draft), toptoy (#382 / #383), yellowmouse (#387 / #388, through the Convert
page in the browser), sandbox (generated, #384). No differences reported (ids equal, no leftovers in any settings).
`npm run dev:neutralize` now: every toon "has its texts already", nothing to store.

**`content:neutral-check`**: 86 answers (catpope's 60 chapter / season / bundle / cast answers + the toon settings:
every bundle per locale for catpope / toptoy / yellowmouse, published + draft, and the toons list without and with each
locale) – the toon settings answers are all identical (entry texts vs per-language settings); the only differences are
the 7 known catpope ones of phases 1–2 (chapter 1 en text-box styling, season `credits`, draft chapter 2 en). Nothing to
fix.

### Checks 2026-10-06 (after step 4)

`npm run check` (exit 0, incl. `check:tools-css`, svelte-check, workspace tests), `check:creator-cast` (19),
`check:creator-chapter` (59), `check:creator-script` (28), `check:creator-cover` (36; the toon titles per language now
from the entry), `check:mcp` (36: + the entry, the refused key, a bad `languageStatus`), PHPUnit (269 tests), phpstan
clean, `content:neutral-check` (above), `content:check` (only the pre-existing scene-14 #127 / #128 / #201 misfits).
Browser (screenshots `4-*`): toon settings page (de / en side by side, status per language, en set to draft + saved),
yellowmouse converted in memory (notice) and through the Convert page, the viewer landing in de (Katzenpapst listed) and
en (not listed while its en status was draft – a published test version, removed again; catpope's #380 published again),
the toon page in de / en (titles from the entry).

Notes: `check:creator-cover`'s reader step (only when the viewer runs) fails – pre-existing: the sandbox is a draft toon
and the check's browser has no draft-preview session, so `/de/sandbox` is 404; with the viewer down the step is skipped
(as in 2B). Open: fold `i18n.<l>.name` into `title`? Remove the legacy toon-settings path (library fallback,
`ToonSettings` schema, MCP's legacy read, the Convert page's toon step) once production is converted.

## Step 5 "Creator wording" – done (2026-10-06, branch `chapter-neutral`)

- Glossary + rules: agents/DESIGN.md "Creator wording" (one term per concept, de = en, no data paths as labels).
- Messages: 110 keys removed (103 unused – among them the `login.*` texts left from before the shared sign-in, the
  per-locale override texts, the old character groups / affiliations –, 7 merged or obsolete: `close` for the three
  close labels, `remove` for `credits_remove`, `text_box` for `ono_text_box`, `status_follow_chapters`, `toon_entry`),
  ~125 added (the field labels that were hard-coded data keys: title / description / release / author(s), thumbnail,
  key visual, group name / layout / swiper settings, text box fields, music / samples, VN event fields, toon settings'
  tile / map, supporters, people), 82 rewritten (no JSON files / store / per-language documents; the Convert page and
  the "converted" bar in plain words; Textbox, Vorlage, Gegenstand, Event, Stimmung, Version, Leser:innen …).
- Code: PropField got `title` (the label's tooltip) and reads bool selects as yes / no with the default in brackets
  ("(no)"), a select's default likewise; align / valign / effect / trigger / layout / target options in words;
  `VIEW_SLOTS` / `OP_WORDS` → `viewSlots()` / `opWord()` (translated); the save-conflict bar names the language only
  for a per-language document (the script).
- Left as they are: the VN event type names (`showMessage`, `joinScene` … – the script's vocabulary, the checks key on
  them), the style guide page (English reference text), font names (lettering / mono), audio trigger modes
  (mono / poly), validation messages from `@scutoons/content` (English, shared with the MCP tools; a few still say
  "chapter file" for the legacy per-locale path).
