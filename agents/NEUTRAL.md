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

1. Foundation: types, keys, schemas, resolve (TS + PHP), neutralize (TS), library read path + legacy fallback, server
   validation/schemas/AssetUsages/Reviews/ContentFiles/sandbox/fixtures, conversion script + equivalence check on the
   2026-10-05 content snapshot.
2. Creator: chapter editor (inline groups + scenes), season editor, cover editor, visibility, new chapter / season /
   language, Convert page, MCP tools.
3. Linked groups (`groups/chains`) merged in; its fields live in the neutral document.
4. Toon settings into the toon entry.
5. Creator messages refactor (owner 2026-10-06): `creator/messages/{de,en}.json` + hints/tooltips in the code still
   name JSON files that no longer exist since the content lives in the database – say what it is today (the document,
   published / draft), shorter and consistent wording, drop unused keys.
6. Full checks, browser tests creator + viewer.
