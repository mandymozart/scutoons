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
| `characters.ts` | characters model (language-neutral, texts per locale): `CharacterRegistry`/`RegistryCharacter` (`scutoons/_/characters.json`, global: name/description per locale, color, group, own_toon, voiced_by → people.json, mannequin, tags, legacy `image`, `images` mood → avatar/portrait/full CDN paths), `ToonCast`/`CastMember` (`<toon>/_/cast.json`: character, role per locale, order); mood vocabulary `MOODS` (17, `neutral` first = `DEFAULT_MOOD`) + `MOOD_LABELS`, `CHARACTER_IMAGE_VARIANTS`, `CHARACTER_GROUPS`, `MANNEQUINS`; `conventionImagePath()` (`characters/<id>/<id>_<variant>-<mood>.png`), `characterImage()` (mood/variant → neutral → `image`), `resolveCast()` (→ the legacy `characters.json` `Character[]` of one toon + locale), `validateCharacters()`, `validateCast()`, `applyConventionPaths()`, `setCharacterImage()`, `searchCharacters()`, `castsOf()`; store constants `CHARACTERS_FILE`/`CAST_FILE` |
| `paths.ts` | `Path`, `getAt()`, `setAt()`, `deleteAt()`, `clone()`, `deepEqual()`, `pathKey()`, `round2()` |
| `validate.ts` | `validateFile(kind, data, locale, ctx)`, `validateLocales(kind, file, files, primary)`, `forEachSlice()`, `checkEvent()`, `checkGroup()`, `Problem`, `Level`, `ValidateContext`, `textOf()`, `LENGTH_WARNING_RATIO` |

JSON Schemas (generated from the types, draft-07): `@scutoons/content/schema/{chapter,season,toon,toon-index,group,catalog,credits,people,roles,characters,cast}.schema.json`.

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
  `viewer/static/api/<toon>/{de,en}/characters.json` into `seed/scutoons/_/characters.json` (registry, image paths from
  the CDN root) + `seed/<toon>/_/cast.json`, adds the own-toon characters of toons without a characters.json, and checks
  that `resolveCast()` reproduces the old files. Import: `php server/bin/console import-content shared/content/seed
  --only=characters.json,cast.json` (`--sql` = production statements, committed as `seed/characters-seed.mysql.sql`).

- `npm run schema -w @scutoons/content` regenerates `schema/*.schema.json` (commit the result).
- `npm run check -w @scutoons/content` = `tsc --noEmit` + fails when a committed schema is out of date.

**Chapter visibility** has one source: `Season.sequence[].status` of the locale (`published` | `draft` | `coming-soon`).
`Chapter.status` is optional and deprecated (the creator mirrors the season status into it on save, for older viewer
builds); `Chapter.published` is gone. A chapter always belongs to a season: `validateFile` reports a chapter without an
entry in its locale's season (`seasonStatus: null`) as an error.

Known gaps between the data and the types (the schema is generated strictly from the types): chapter 2 `keyvisual` is a
string, `Collectable` lacks `name`/`image`, scene files use `{"type":"end"}` (see agents/MEMORY.md "Known bugs").
