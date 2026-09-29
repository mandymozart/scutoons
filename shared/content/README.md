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
| `template.ts` | `escapeHtml()`, `escapeAndJoin()`, `sanitizeUrl()`, `sanitizeNumber()`, `isItemPublic(item, includeDrafts)`, `cdnUrl(base, path)` |
| `paths.ts` | `Path`, `getAt()`, `setAt()`, `deleteAt()`, `clone()`, `deepEqual()`, `pathKey()`, `round2()` |
| `validate.ts` | `validateFile(kind, data, locale, ctx)`, `validateLocales(kind, file, files, primary)`, `forEachSlice()`, `checkEvent()`, `checkGroup()`, `Problem`, `Level`, `ValidateContext`, `textOf()`, `LENGTH_WARNING_RATIO` |

JSON Schemas (generated from the types, draft-07): `@scutoons/content/schema/{chapter,season,toon,toon-index,group,catalog}.schema.json`.

## Example

```ts
import { Status, validateFile, isItemPublic, cdnUrl, type Chapter } from '@scutoons/content';

const chapter: Chapter = await (await fetch('/api/catpope/de/chapter1.json')).json();
const problems = validateFile('chapter', chapter, 'de', { file: 'chapter1.json', characters: ['Cat Pope'] });
const listed = isItemPublic(chapter, /* includeDrafts */ false) && chapter.status === Status.PUBLISHED;
const firstImage = cdnUrl('https://cdn.scutoons.com', `catpope/${chapter.groups[0].slices?.[0].image}`);
```

```php
// server (later): validate a creator write against the exported schema
$schema = json_decode(file_get_contents('…/shared/content/schema/chapter.schema.json'));
```

## Scripts

- `npm run schema -w @scutoons/content` regenerates `schema/*.schema.json` (commit the result).
- `npm run check -w @scutoons/content` = `tsc --noEmit` + fails when a committed schema is out of date.

Known gaps between the data and the types (the schema is generated strictly from the types): chapter 2 `keyvisual` is a
string, `Collectable` lacks `name`/`image`, scene files use `{"type":"end"}` (see agents/MEMORY.md "Known bugs").
