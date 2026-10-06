# @scutoons/content

The content contract of SCUTOONS: the TypeScript types for toon / season / chapter / group / slice / dialog / audio /
visual novel / items, pure helpers, the validation the creator runs, and a JSON Schema export for the server.
The creator writes these files, the server stores and validates them, the viewer reads them.

No Svelte, no DOM access, no bundler globals (`import.meta.env`), no app state: runs in any app and in Node.

## Public API (`import … from '@scutoons/content'`)

| Module | Exports |
|---|---|
| `enums.ts` | `Renderer`, `Status`, `Language`, `EventCategory`, `Trigger`, `TriggerMode`, `TextBoxEffect` (a `showTextBox` event's effect), `JumpToTargetType` |
| `content.ts` | `Toon`, `ToonIndex`, `Season`, `SequenceItem`, `Chapter`, `Group`, `RefLink`, `Slice` (`image` + required `width` / `height` in px + optional `video` `SliceVideo` { src, loop?, autoplay? } – the image is the video's poster), `DEFAULT_SLICE` (390 × 625), `Dialog` (+ `DialogVariant`, `DialogFont`), `Onomatopoeia`, `KeyVisual`, map / credits / event types, `LoadedSequenceItem` |
| `audio.ts` | `AudioConfiguration`, `AudioTrack`, `AudioEvents`, `MusicEvent(Track)`, `Sample`, `SampleTrigger`, `AudioEventsConfig` |
| `visual-novel.ts` | `VNEvent` (union of all event types), `VNSliceEvent`, `VNEventTrigger`, `Character`, choices / setters / jumpTo |
| `items.ts` | `Catalog`, `ItemDef`, `Skill`, `Recipe`, `Entry`, `Avatar`, `AVATAR_SLOTS`, `User`, `Me`, `Access`, `CodeInfo`, `Edition` (API contract: agents/ITEMS.md) |
| `membership.ts` | `Membership`, `MembershipType`, `ValidationResult`, `isValidMembershipType()`, `isMembershipExpired()` |
| `products.ts` | shop contract (`GET /api/products/`, `/api/products/featured/`): `ProductCatalog`, `Product`, `ProductVariant`, `FeaturedProduct`, `LocalizedText`; helpers `isSoldOut()`, `pickVariant()`, `variantPrice()`, `formatPrice()`, `editionPieces()`, `localized()` |
| `template.ts` | `escapeHtml()`, `escapeAndJoin()`, `sanitizeUrl()`, `sanitizeNumber()`, `isItemPublic(item, includeDrafts)` (readable), `isItemListed(item, includeDrafts)` (listed incl. coming-soon), `sequenceStatus(season, number)`, `cdnUrl(base, path)` |
| `languages.ts` | toon languages (set per toon in `ToonIndex.sourceLanguage` / `.languages`): `toonLanguages(entry)` → `{ source, languages }` (defaults for older entries: `languages` → `i18n` keys → `de`), `withToonLanguages()`, `isLanguage()`, `NEUTRAL_LOCALE` (`_`), `TOON_INDEX_FILE` (`index.json`), `DEFAULT_SOURCE_LANGUAGE` |
| `credits.ts` | credits model (language-neutral, labels per locale): `PeopleRegistry`/`CreditPerson` (`scutoons/_/people.json`), `RoleVocabulary`/`CreditRole`/`RoleCategory` (`scutoons/_/roles.json`), `ToonCredits`/`CreditSection`/`CreditEntry`/`CreditRoleEntry` (`<toon>/_/credits.json`); `resolveCredits()` (one locale, ready to render with `CreditsView` of @scutoons/reader), `validatePeople()`, `validateRoleVocabulary()`, `validateCredits()`, `replacePerson()`, `personUsage()`, `slugId()`, `uniqueId()`, `CREDITS_LABELS`, store constants `CREDITS_SCOPE`/`PEOPLE_FILE`/`ROLES_FILE`/`CREDITS_FILE` |
| `characters.ts` | characters model (language-neutral, texts per locale) – Character → Persona → Look → Mood × View: `CharacterRegistry`/`RegistryCharacter` (`scutoons/_/characters.json`: id, name/description {locale}, color, own_toon?, voiced_by? (people.json), tags?, legacy `image`, artwork?, `apose?` { looks: [{ id (`swim` = the standard look; `naked` removed 2026-10-05), label, default, path?, slots?: [{ id, slot, x, y, size, label? }] }] } (the reader's avatar body, character level), `personas` [{ id (`regular` always + e.g. `human`, `cat`, `biker`), label, introduced?, description?, color?, `looks` [{ id, label, default (exactly one), images: mood → view → CDN path }], transitions? [{ from, to, image, label? }] }]), `MoodVocabulary` (`moods.json`, open-ended; `defaultMoodVocabulary()` = 17 standard + dizzy, tripping), `AssetRequests` (`asset-requests.json`), `ToonCast` (`<toon>/_/cast.json`: `seasons` → [{ character, personas, role? }]); `resolveExpression()` (THE image fallback rule, below), `characterImage()` / `legacyCharacterImage()` / `legacyCharacterLabel()` / `legacyCharacterColor()` / `legacyTransition()`, `resolveCast()` (→ the legacy `characters.json` `Character[]` + `personas`), `LEGACY_CHARACTER_IDS` + `resolveCharacterId()` + `normalizeSceneRef()` (old ids plf / cat / mountain-tiger → catpope personas, old ref form), `conventionImagePath()`, `conventionTransitionPath()`, `aposeConventionPath()`, `avatarBodies()` (→ `[{ character, look, label, path, slots, default }]`, retired looks left out – `isRetiredAposeLook()` / `RETIRED_APOSE_LOOKS`), `bodySlots()`, `remapAvatarSlots()`, `wantedImages()` + `wantedImagesText()` / `wantedImagesCsv()`, `closeFulfilledRequests()`, `castEntries()`, `castIndex()`, `expressionIndex()` (for the VN validation), `validateCharacters()` / `validateMoods()` / `validateCast()` / `validateAssetRequests()`, `searchCharacters()`; store constants `CHARACTERS_FILE`/`MOODS_FILE`/`ASSET_REQUESTS_FILE`/`CAST_FILE` |
| `paths.ts` | `Path`, `getAt()`, `setAt()`, `deleteAt()`, `clone()`, `deepEqual()`, `pathKey()`, `round2()` |
| `validate.ts` | `validateFile(kind, data, locale, ctx)`, `validateLocales(kind, file, files, primary)`, `forEachSlice()`, `checkEvent()`, `checkGroup()`, `Problem`, `Level`, `ValidateContext`, `textOf()`, `LENGTH_WARNING_RATIO` |

JSON Schemas (generated from the types, draft-07): `@scutoons/content/schema/{chapter,season,toon,toon-index,group,catalog,credits,people,roles,characters,moods,asset-requests,cast}.schema.json`.

## Slices (owner 2026-09-30)

- The standard slice is **390 × 625 px** (the Naver webtoon size, `DEFAULT_SLICE`), delivered and served at **1x only as
  WebP**: `<toon>/s<n>/c<n>/<sXcY>-slice-N.webp` (the chapter's folder, agents/ASSETS.md "Folder convention";
  390 px wide, never scaled up). Other heights are fine, the width stays 390.
- Every slice carries `width` / `height` (px of the image): the reader reserves the space before the lazy image loads
  (no layout shift, the audio intersection triggers fire slice by slice). `validateFile` reports a missing size as an
  **error** and a width ≠ 390 as a warning; the creator reads the size from the CDN whenever an image path is set.
- A slice can be a video (`video: { src, loop?, autoplay? }`, H.264 MP4 ≤ 3 MB, new name per version): muted, inline,
  looping, plays in view, poster = `image` (the only image with reduced motion and in older viewers).
- `sensitive: true` (`SensitiveFlag`, `sensitive.ts`; the same in every language, not a boolean = error): while the reader's
  "Hide sensitive content" is on (`hidesSensitive(user)`; guests always) the reader blurs the image (a video shows its
  poster), keeps the slice's samples silent and shows a small note (agents/ITEMS.md "Sensitive slices").

## Characters: glossary and the image fallback rule

Hierarchy (owner, 2026-09-30): **Character → Persona → Look → Mood × View** – the terms are used the same way in types,
UI (de/en) and docs:

| Term | Meaning |
|---|---|
| **Character** | the being in the SCUTOONS universe (`own_toon` optional). Name, description, color, voice, tags, artwork and the A-pose |
| **Persona** | a different identity or role of the same being (Cat Pope · human / cat / mountaintiger / Biker) – changes who they are in the story. Nested in the character; everyone has `regular`; may have its own color |
| **Look** | the appearance right now: outfit, gear, accessories; can change within a scene (helmet on/off). Rule of thumb: if it could come off in the next panel, it's a look. One default look per persona; `transitions` = an image shown while the look changes |
| **Mood** | the expression. Open-ended vocabulary (`moods.json`); only `neutral` is required (the fallback anchor), the `standard` ones are a checklist |
| **View** | the framing: avatar / portrait / full. The data field stays `variant` (backwards compatibility); every UI label says "View" / "Ansicht". Only `portrait` is authored; `avatar` is a derived 192px downscale, `full` is unused |
| **A-pose** | `character.apose.looks` (character level, `swim` = the standard look; the `naked` look was removed 2026-10-05 – a leftover is ignored and a validation warning): one image + its wear slots (slot instances `{ id, slot, x, y, size, label? }` in image px; a duo has a slot type twice) – the reader's avatar body. Never a fallback; outfitting happens only in the reader's inventory |

Convention for new files (stored paths are always explicit):
`characters/<character>[-<persona>]/<character>[-<persona>][_<look>]_<view>-<mood>.webp` (regular persona and default look
left out), transitions `…/<character>[-<persona>]_transition-<from>-to-<to>.webp`, A-pose
`characters/<character>/<character>_apose-<look>.webp` (or .png). The supplier delivers PNGs under the same names; the
creator's asset import writes the WebP
next to every portrait / avatar / transition PNG; a portrait that came without an avatar gets one from the Luma generation workflows, so the registry points to the WebP files (agents/PERFORMANCE.md P3: 600² portrait ~350 KB → ~55 KB).
The PNGs stay on the CDN for older viewer builds; the missing-image substitutes stay PNG.

Image fallback (`resolveExpression`, one implementation for the viewer VN components and the creator), regressive, one
step at a time; within each step the requested view first, then the same mood's portrait:

1. persona + look + mood
2. persona + look + `neutral` (a missing mood never changes the look)
3. persona + default look + mood
4. persona + default look + `neutral`
5. `regular` persona + default look + mood
6. `regular` persona + default look + `neutral`
7. the legacy single `image`

Casting is per season (`cast.json` `seasons`). VN refs: `{ character, persona, look?, mood?, variant? }` (`variant` = the
view); the old form `{ name, position: { variant } }` and the old ids plf / cat / mountain-tiger still resolve
(`normalizeSceneRef`); the scene key is character + persona.

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

Production is the content's source of truth (CUTOVER 10.30): the committed seed (`seed/`, `legacy/`) and the one-off
migration scripts (`migrate-credits.ts`, `migrate-characters.ts`, `migrate-map.ts`, `migrate-scene-refs.ts`,
`slice-sizes.ts`) are gone; local dev loads production's export (`npm run dev:db`, server/docs/DATABASE.md). The unit
tests read a small fixture in `server/tests/fixtures/content/`.

- `node --experimental-transform-types --no-warnings shared/content/scripts/make-sandbox.ts [--out=<dir>] [--no-import] [--dry-run]`
  generates the **sandbox toon** "Cat Pope's walk" (dev + creator only, never in production: toon, season chapters
  all `draft`) from the newest registry + moods of the local dev database into the interchange folder `--out` (default
  `logs/dev/sandbox`, gitignored, rewritten completely): one chapter per encounter (every other character, then Cat
  Pope's own personas), one VN group per persona × look; everyone shows every mood of every look, Cat Pope mimics it
  (or is `sad` when his regular look lacks it), look changes use the registry transitions, Cat Pope tries each look.
  Then it imports it (`php server/bin/console import-content logs/dev/sandbox --toon=sandbox --update`; `--no-import`
  skips that). `npm run dev:db` runs it after loading the production seed; `npm run dev:db sandbox` runs it alone.
  Re-run after adding characters, personas, looks, moods or transitions. It is also the VN test bed. A third language
  `ko` (2026-10-02, for the creator's i18n switch): the `de` files copied with Korean toon / season / chapter titles and
  descriptions (`PARTIAL` in the script; the dialogue is still German, like a translation in progress); `toons.json`
  lists `ko` for the sandbox.
- `node --experimental-transform-types --no-warnings shared/content/scripts/import-screenplay.ts --dir=<dir> --toon= --locale=`
  writes a screenplay as the toon's `script` entity file (default out `logs/dev/screenplay`); then
  `npm run dev:db -- console import-content ../logs/dev/screenplay --toon=<toon> --only=script [--update]`.
- `npm run schema -w @scutoons/content` regenerates `schema/*.schema.json` (commit the result).
- `npm run check -w @scutoons/content` = `tsc --noEmit` + fails when a committed schema is out of date + the unit tests (`npm test`).

**Chapter visibility** has one source: `Season.sequence[].status` of the locale (`published` | `draft` | `coming-soon`).
`Chapter.status` is optional and deprecated (the creator mirrors the season status into it on save, for older viewer
builds); `Chapter.published` is gone. A chapter always belongs to a season: `validateFile` reports a chapter without an
entry in its locale's season (`seasonStatus: null`) as an error.

Known gaps between the data and the types (the schema is generated strictly from the types): chapter 2 `keyvisual` is a
string, scene files use `{"type":"end"}` (see agents/MEMORY.md "Known bugs").
