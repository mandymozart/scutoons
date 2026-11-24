<?php
// GENERATED from server/schema/api/library.schema.json by server/schema/generate.ts – edit the source, run `npm run generate`.
declare(strict_types=1);

namespace Scutoons\Generated;

/**
 * The extended Library (#864, agents/research/redesign/lap2/LIBRARY.md): things and files in one list, one server answer for the creator and the deck (GET /api/library, server/docs/API.md "The extended Library"). The server resolves – types, kinds, names, pictures, places, used in – the apps only read. Edit this file, run `npm run generate` (server/schema/generate.ts writes the TS and PHP types), commit the outputs; library.vectors.json holds the file cases every reader holds to.
 *
 * @phpstan-type ThingType 'toon' | 'character' | 'scene' | 'show' | 'place' | 'item' | 'module' | 'instrument' | 'rig' | 'generator' | 'kanon' | 'person'
 * @phpstan-type ItemSubtype 'recipe' | 'product' | 'edition' | 'album' | 'home-scene'
 * @phpstan-type FileKind 'image' | 'audio' | 'video' | 'model' | 'animation' | 'file'
 * @phpstan-type FileOrigin 'uploaded' | 'imported' | 'generated'
 * @phpstan-type LinkRole 'cast' | 'credits' | 'rig' | 'reference'
 * @phpstan-type LevelKind 'persona' | 'look' | 'mood' | 'season' | 'chapter'
 * @phpstan-type LibraryEntry array{address: string, type?: ThingType, subtype?: ItemSubtype, kind?: FileKind, name: string, picture: string | null, parent?: string, toons: list<string>, usedIn: int, updated: string | null, origin?: FileOrigin}
 * @phpstan-type LibraryCounts array<string, int>
 * @phpstan-type LibraryList array{library: 1, version: string, toon: string | null, place: string, locale: string, entries: list<LibraryEntry>, total: int, next: string | null, counts: LibraryCounts}
 * @phpstan-type LibraryLevel array{address: string, level: LevelKind, name: string, picture: string | null, parent: string, mood?: string}
 * @phpstan-type LibraryLink array{address: string, type?: ThingType, name: string, as: LinkRole}
 * @phpstan-type LibraryStep array{library: 1, version: string, locale: string, at: LibraryEntry, levels: list<LibraryLevel>, files: list<LibraryEntry>, uses: list<LibraryEntry>, usedIn: list<LibraryLink>}
 */
final class LibraryTypes
{
    /** the format's version (`library`) */
    public const FORMAT = 1;
    /** @var list<ThingType> */
    public const THING_TYPE = ['toon', 'character', 'scene', 'show', 'place', 'item', 'module', 'instrument', 'rig', 'generator', 'kanon', 'person'];
    /** @var list<ItemSubtype> */
    public const ITEM_SUBTYPE = ['recipe', 'product', 'edition', 'album', 'home-scene'];
    /** @var list<FileKind> */
    public const FILE_KIND = ['image', 'audio', 'video', 'model', 'animation', 'file'];
    /** @var list<FileOrigin> */
    public const FILE_ORIGIN = ['uploaded', 'imported', 'generated'];
    /** @var list<LinkRole> */
    public const LINK_ROLE = ['cast', 'credits', 'rig', 'reference'];
    /** @var list<LevelKind> */
    public const LEVEL_KIND = ['persona', 'look', 'mood', 'season', 'chapter'];
}
