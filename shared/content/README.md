# @scutoons/content

The content contract of SCUTOONS: the TypeScript types for toon / season / chapter / group / slice / dialog / audio /
visual novel / items, pure helpers, the validation the creator runs, and a JSON Schema export for the server.
The creator writes these files, the server stores and validates them, the viewer reads them.

No Svelte, no DOM access, no bundler globals (`import.meta.env`), no app state: runs in any app and in Node.

## Public API (`import … from '@scutoons/content'`)

| Module | Exports |
|---|---|
| `enums.ts` | `Renderer`, `Status`, `Language`, `EventCategory`, `Trigger`, `TriggerMode`, `OnomatopoeiaVariant`, `OnomatopoeiaEffect`, `JumpToTargetType` |
| `content.ts` | `Toon`, `ToonIndex`, `Season`, `SequenceItem`, `Chapter`, `Group`, `RefLink`, `Slice`, `Dialog` (+ `DialogVariant`, `DialogFont`), `Onomatopoeia`, `KeyVisual`, map / credits / event types, `LoadedSequenceItem` |
| `audio.ts` | `AudioConfiguration`, `AudioTrack`, `AudioEvents`, `MusicEvent(Track)`, `Sample`, `SampleTrigger`, `AudioEventsConfig` |
| `visual-novel.ts` | `VNEvent` (union of all event types), `VNSliceEvent`, `VNEventTrigger`, `Character`, `Collectable`, choices / setters / jumpTo |
| `items.ts` | `Catalog`, `ItemDef`, `Skill`, `Recipe`, `Entry`, `Avatar`, `AVATAR_SLOTS`, `User`, `Me`, `Access`, `CodeInfo`, `Edition` (API contract: agents/ITEMS.md) |
| `membership.ts` | `Membership`, `MembershipType`, `ValidationResult`, `isValidMembershipType()`, `isMembershipExpired()` |
| `products.ts` | shop contract (`GET /api/products/`, `/api/products/featured/`): `ProductCatalog`, `Product`, `ProductVariant`, `FeaturedProduct`, `LocalizedText`; helpers `isSoldOut()`, `pickVariant()`, `variantPrice()`, `formatPrice()`, `editionPieces()`, `localized()` |
| `template.ts` | `escapeHtml()`, `escapeAndJoin()`, `sanitizeUrl()`, `sanitizeNumber()`, `isItemPublic(item, includeDrafts)` (readable), `isItemListed(item, includeDrafts)` (listed incl. coming-soon), `sequenceStatus(season, number)`, `cdnUrl(base, path)` |
| `languages.ts` | toon languages (set per toon in `ToonIndex.sourceLanguage` / `.languages`): `toonLanguages(entry)` → `{ source, languages }` (defaults for older entries: `languages` → `i18n` keys → `de`), `withToonLanguages()`, `isLanguage()`, `NEUTRAL_LOCALE` (`_`), `TOON_INDEX_FILE` (`index.json`), `DEFAULT_SOURCE_LANGUAGE` |
| `credits.ts` | credits model (language-neutral, labels per locale): `PeopleRegistry`/`CreditPerson` (`scutoons/_/people.json`), `RoleVocabulary`/`CreditRole`/`RoleCategory` (`scutoons/_/roles.json`), `ToonCredits`/`CreditSection`/`CreditEntry`/`CreditRoleEntry` (`<toon>/_/credits.json`); `resolveCredits()` (one locale, ready to render with `CreditsView` of @scutoons/reader), `validatePeople()`, `validateRoleVocabulary()`, `validateCredits()`, `replacePerson()`, `personUsage()`, `slugId()`, `uniqueId()`, `CREDITS_LABELS`, store constants `CREDITS_SCOPE`/`PEOPLE_FILE`/`ROLES_FILE`/`CREDITS_FILE` |
| `characters.ts` | characters model (language-neutral, texts per locale) – Character → Persona → Look → Mood × View: `CharacterRegistry`/`RegistryCharacter` (`scutoons/_/characters.json`: name/description per locale, color, cast `group`, own_toon?, voiced_by → people.json, mannequin, tags, legacy `image`, `affiliations`, `artwork`, `personas` [{ id (`regular` + …), label, introduced?, description?, affiliations? (override), `looks` [{ id, label, default, images: mood → view → CDN path, apose?: { path, anchors: slot → { x, y, size } } }], `transitions` [{ from, to, image }] }]), `GroupVocabulary` (`groups.json`), `MoodVocabulary` (`moods.json`, open-ended; `defaultMoodVocabulary()` = 17 standard + dizzy, tripping), `AssetRequests` (`asset-requests.json`), `ToonCast` (`<toon>/_/cast.json`: `seasons` → [{ character, personas, role? }]); `resolveExpression()` (THE image fallback rule, below), `characterImage()` / `legacyCharacterImage()` / `legacyCharacterLabel()` / `legacyTransition()`, `resolveCast()` (→ the legacy `characters.json` `Character[]` + `personas`), `conventionImagePath(character, view, mood, look?, persona?)`, `conventionTransitionPath()`, `wantedImages()` + `wantedImagesText()` / `wantedImagesCsv()`, `closeFulfilledRequests()`, `avatarBodies()` (every look with an A-pose → the reader's avatar bodies), `castEntries()`, `castIndex()`, `expressionIndex()` (for the VN validation), `validateCharacters()` / `validateGroups()` / `validateMoods()` / `validateCast()` / `validateAssetRequests()`, `searchCharacters()`, `groupMembers()`; store constants `CHARACTERS_FILE`/`GROUPS_FILE`/`MOODS_FILE`/`ASSET_REQUESTS_FILE`/`CAST_FILE` |
| `paths.ts` | `Path`, `getAt()`, `setAt()`, `deleteAt()`, `clone()`, `deepEqual()`, `pathKey()`, `round2()` |
| `validate.ts` | `validateFile(kind, data, locale, ctx)`, `validateLocales(kind, file, files, primary)`, `forEachSlice()`, `checkEvent()`, `checkGroup()`, `Problem`, `Level`, `ValidateContext`, `textOf()`, `LENGTH_WARNING_RATIO` |

JSON Schemas (generated from the types, draft-07): `@scutoons/content/schema/{chapter,season,toon,toon-index,group,catalog,credits,people,roles,characters,groups,cast}.schema.json`.

## Characters: glossary and the image fallback rule

Hierarchy (owner, 2026-09-29): **Character → Persona → Look → Mood × View** – the terms are used the same way in types,
UI (de/en) and docs:

| Term | Meaning |
|---|---|
| **Character** | the being in the SCUTOONS universe (`own_toon` optional). Identity-level: name, description, color, `voiced_by`, `affiliations` (default), tags, artwork |
| **Persona** | a different identity or role of the same being (Cat Pope · Biker) – changes who they are in the story. Nested in the character; everyone has `regular`. May override the affiliations |
| **Look** | the appearance right now: outfit, gear, accessories; can change within a scene (helmet on/off). Rule of thumb: if it could come off in the next panel, it's a look. One default look per persona; `transitions` = an image shown while the look changes |
| **Mood** | the expression. Open-ended vocabulary (`moods.json`); only `neutral` is required (the fallback anchor), the `standard` ones are a checklist |
| **View** | the framing: avatar / portrait / full. The data field stays `variant` (backwards compatibility); every UI label says "View" / "Ansicht". Only `portrait` is authored; `avatar` is a derived 192px downscale, `full` is unused. `apose` (per look, neutral, one image + wear anchors per avatar slot in image px) is the reader's avatar body and never falls back |

Convention for new files (stored paths are always explicit):
`characters/<character>[-<persona>]/<character>[-<persona>][_<look>]_<view>-<mood>.png` (regular persona and default look
left out), transitions `…/<character>[-<persona>]_transition-<from>-to-<to>.png`.

Image fallback (`resolveExpression`, one implementation for the viewer VN components and the creator), regressive, one
step at a time; within each step the requested view first, then the same mood's portrait:

1. persona + look + mood
2. persona + look + `neutral` (a missing mood never changes the look)
3. persona + default look + mood
4. persona + default look + `neutral`
5. `regular` persona + default look + mood
6. `regular` persona + default look + `neutral`
7. the legacy single `image`

Casting is per season (`cast.json` `seasons`); VN refs `{ name (character id), persona?, look?, mood?, position.variant
(view) }` – a ref with only a name is the regular persona; the scene key is character + persona.

Unit tests: `npm test -w @scutoons/content` (`test/characters.test.ts`, part of `npm run check`).

## Example

```ts
import { validateFile, isItemPublic, sequenceStatus, cdnUrl, type Chapter, type Season } from '@scutoons/content';

const season: Season = await (await fetch('/api/catpope/de/season1.json')).json();
const chapter: Chapter = await (await fetch('/api/catpope/de/chapter1.json')).json();
// visibility = the season's status of the chapter, per locale (the chapter file's `status` is a deprecated mirror)
const status = sequenceStatus(season, 1);
const readable = status !== null && isItemPublic({ status }, /* includeDrafts */ false);
const problems = validateFile('chapter', chapter, 'de', { file: 'chapter1.json', characters: ['Cat Pope'], seasonStatus: status });
const firstImage = cdnUrl('https://cdn.scutoons.com', `catpope/${chapter.groups[0].slices?.[0].image}`);
```

```php
// server (later): validate a creator write against the exported schema
$schema = json_decode(file_get_contents('…/shared/content/schema/chapter.schema.json'));
```

## Scripts

- `node --experimental-transform-types --no-warnings shared/content/scripts/migrate-credits.ts [--toon=catpope]`
  (from the repo root) converts the old `viewer/static/api/<toon>/{de,en}/credits.json` + season `credits` + toon
  `authors` into `seed/` (`scutoons/_/people.json`, `scutoons/_/roles.json`, `<toon>/_/credits.json`) and prints a report
  (ambiguous roles, aliases, possible duplicates). Import: `php server/bin/console import-content shared/content/seed`
  (`--sql` = production statements, committed as `seed/credits-seed.mysql.sql`).

- `node --experimental-transform-types --no-warnings shared/content/scripts/migrate-characters.ts` converts the old
  `viewer/static/api/<toon>/{de,en}/characters.json` (→ regular persona, default look) and the owner's additions
  (personas, looks, transitions, groups, casts per season) into `seed/scutoons/_/{characters,groups,moods,asset-requests}.json`
  + `seed/<toon>/_/cast.json`, takes every image found in the convention folders `cdn/root/characters/<character>[-<persona>]/`
  and compares `resolveCast()` with the old files. Import: `php server/bin/console import-content shared/content/seed
  --only=characters.json,groups.json,moods.json,asset-requests.json,cast.json` (`--update` after a seed change; `--sql` =
  production statements, committed as `seed/characters-seed.mysql.sql`).

- `npm run schema -w @scutoons/content` regenerates `schema/*.schema.json` (commit the result).
- `npm run check -w @scutoons/content` = `tsc --noEmit` + fails when a committed schema is out of date + the unit tests (`npm test`).

**Chapter visibility** has one source: `Season.sequence[].status` of the locale (`published` | `draft` | `coming-soon`).
`Chapter.status` is optional and deprecated (the creator mirrors the season status into it on save, for older viewer
builds); `Chapter.published` is gone. A chapter always belongs to a season: `validateFile` reports a chapter without an
entry in its locale's season (`seasonStatus: null`) as an error.

Known gaps between the data and the types (the schema is generated strictly from the types): chapter 2 `keyvisual` is a
string, `Collectable` lacks `name`/`image`, scene files use `{"type":"end"}` (see agents/MEMORY.md "Known bugs").
