// Entity keys (agents/DATA.md §2): how every toon-related document is addressed in the database and the API.
//
//   toon:<toon>                       the toon's index entry (status, names, languages, the language-neutral settings: map, tile …
//                                     and – agents/NEUTRAL.md step 4 – title / description / release / status per language)
//   toon-settings:<toon>:<locale>     legacy: the toon's per-language settings, read while the entry has no texts
//   season:<toon>:<n>                 chapter:<toon>:<n>                 scene:<toon>:<chapter>:<name>
//                                     one language-neutral document each, texts as locale maps (neutral.ts, agents/NEUTRAL.md)
//   season:<toon>:<n>:<locale>  chapter:<toon>:<n>:<locale>  scene:<toon>:<chapter>:<name>:<locale>
//                                     legacy per-locale documents: read while the entity has no neutral document
//   locations:<toon>:<locale>         routes:<toon>:<locale>             (the map)
//   cast:<toon>                       credits:<toon>                     (language-neutral, per toon)
//   script:<toon>:<locale>            the toon's screenplay (creator only: no library endpoint, readers never get it)
//   characters  moods  asset-requests  people  roles  events  map      (platform-wide, language-neutral)
//   place:<id>  route:<id>            one document per place / route of the world map (agents/MAP-DOCUMENTS.md):
//                                     `map` keeps the map's own settings (size, layers); ids are stable forever
//   character:<id>                    one document per character of the registry (`characters` keeps its own fields)
//   (`locations:*` / `routes:*` are the old per-toon map, read-only since the world map `map`, agents/MAP.md)
//
// An "entity" is a key without its locale (`chapter:catpope:2`); the stored document of one language adds `:<locale>`.
// Language-neutral entities are stored under the entity itself – so are chapters, scenes and seasons since 2026-10-06
// (NEUTRAL_DOC_KINDS; their per-locale keys stay readable until production is converted). JSON file names exist only in the import/export
// format (server `Console\ContentFiles`), never in the apps.

import { NEUTRAL_LOCALE } from './languages.ts';

export type EntityKind =
	| 'toon'
	| 'toon-settings'
	| 'season'
	| 'chapter'
	| 'scene'
	| 'locations'
	| 'routes'
	| 'cast'
	| 'credits'
	| 'script'
	| 'characters'
	| 'moods'
	| 'asset-requests'
	| 'people'
	| 'roles'
	| 'events'
	| 'map'
	| 'place'
	| 'region'
	| 'route'
	| 'character'
	| CatalogEntityKind;

/**
 * The catalog entries as versioned documents, `<kind>:<id>` (server CatalogDocs, agents/CATALOG-IN-CREATOR.md): no toon
 * segment (the toon is the entry's `toon` field – an entry may move), language-neutral; publishing writes the catalog.
 */
export const CATALOG_ENTITY_KINDS = ['item', 'recipe', 'album', 'product', 'edition', 'home-scene', 'airlift-carrier'] as const;
export type CatalogEntityKind = (typeof CATALOG_ENTITY_KINDS)[number];
/** a catalog document's id (the server's widest catalog id rule; ids may hold upper case) */
const CATALOG_ID = /^[A-Za-z0-9][A-Za-z0-9_-]{0,99}$/;
const isCatalogKind = (kind: string): kind is CatalogEntityKind => (CATALOG_ENTITY_KINDS as readonly string[]).includes(kind);

/** kind → [has toon, has number, has scene name, localized] (the catalog kinds: an id instead – parseEntityKey) */
const SHAPES: Record<Exclude<EntityKind, CatalogEntityKind>, [boolean, boolean, boolean, boolean]> = {
	toon: [true, false, false, false],
	'toon-settings': [true, false, false, true],
	season: [true, true, false, true],
	chapter: [true, true, false, true],
	scene: [true, true, true, true],
	locations: [true, false, false, true],
	routes: [true, false, false, true],
	cast: [true, false, false, false],
	credits: [true, false, false, false],
	script: [true, false, false, true],
	characters: [false, false, false, false],
	moods: [false, false, false, false],
	'asset-requests': [false, false, false, false],
	people: [false, false, false, false],
	roles: [false, false, false, false],
	events: [false, false, false, false],
	map: [false, false, false, false],
	// one segment: the id (ID_KINDS)
	place: [false, false, false, false],
	region: [false, false, false, false],
	route: [false, false, false, false],
	character: [false, false, false, false]
};

/**
 * Kinds addressed by an id of their own (`place:<id>`, `route:<id>`, `character:<id>` – platform-wide, language-neutral;
 * agents/MAP-DOCUMENTS.md).
 * The id is a slug, stable forever once published (events reference it). The same shape is meant for the catalog's items
 * (`item:<id>`, agents/CATALOG-IN-CREATOR.md).
 */
export const ID_KINDS: readonly EntityKind[] = ['place', 'route', 'region', 'character'];
/** an id segment: a slug (the map's place / route ids, `checkWorldMap`) */
export const ENTITY_ID = /^[a-z0-9][a-z0-9_-]{0,63}$/;

export interface EntityRef {
	kind: EntityKind;
	toon?: string;
	/** season / chapter number, the chapter of a scene */
	number?: number;
	/** a scene's name (e.g. `scene-14`) */
	scene?: string;
	/** only on stored keys of localized kinds */
	locale?: string;
	/** the id of an ID_KINDS key (`place:cathedral` → `cathedral`) or a catalog document's entry id (`item:camera` → camera) */
	id?: string;
}

/** Builders of entities (keys without locale) */
export const entities = {
	toon: (toon: string) => `toon:${toon}`,
	/** @deprecated legacy per-language toon settings (read-only: the toon entry holds them since agents/NEUTRAL.md step 4) */
	toonSettings: (toon: string) => `toon-settings:${toon}`,
	season: (toon: string, n: number) => `season:${toon}:${n}`,
	chapter: (toon: string, n: number) => `chapter:${toon}:${n}`,
	scene: (toon: string, chapter: number, name: string) => `scene:${toon}:${chapter}:${name}`,
	locations: (toon: string) => `locations:${toon}`,
	routes: (toon: string) => `routes:${toon}`,
	cast: (toon: string) => `cast:${toon}`,
	credits: (toon: string) => `credits:${toon}`,
	script: (toon: string) => `script:${toon}`,
	characters: 'characters',
	moods: 'moods',
	assetRequests: 'asset-requests',
	people: 'people',
	roles: 'roles',
	events: 'events',
	/** the SCUTOONS world map's own settings: size, layers (agents/MAP.md, MAP-DOCUMENTS.md) */
	map: 'map',
	/** one place of the world map */
	place: (id: string) => `place:${id}`,
	/** one route of the world map */
	route: (id: string) => `route:${id}`,
	/** one region of the world map (agents/MAP.md §8.10) */
	region: (id: string) => `region:${id}`,
	/** one character of the registry (`characters` keeps the registry's own fields) */
	character: (id: string) => `character:${id}`,
	/** a catalog entry's document (`item:camera`) */
	catalog: (kind: CatalogEntityKind, id: string) => `${kind}:${id}`
} as const;

/**
 * Kinds stored as one language-neutral document under the entity itself (`chapter:catpope:2`, neutral.ts). Their
 * per-locale keys (`chapter:catpope:2:de`) are the legacy form: the library reads them while no neutral document exists.
 */
export const NEUTRAL_DOC_KINDS: readonly EntityKind[] = ['chapter', 'scene', 'season'];

/** Whether a stored key holds a language-neutral document (no locale segment: `toon:x`, `chapter:x:2`; not `chapter:x:2:de`) */
export function isNeutralKey(key: string): boolean {
	const ref = parseEntityKey(key);
	return !!ref && !ref.locale;
}

/** Whether a stored key is the neutral document of a chapter, scene or season (not a legacy per-locale one) */
export function isNeutralDocKey(key: string): boolean {
	const ref = parseEntityKey(key);
	return !!ref && !ref.locale && NEUTRAL_DOC_KINDS.includes(ref.kind);
}

/** Whether an entity (or key) of this kind is stored per locale (for chapter / scene / season: the legacy form) */
export function isLocalizedEntity(entity: string): boolean {
	const shape = SHAPES[entity.split(':')[0] as Exclude<EntityKind, CatalogEntityKind>];
	return !!shape?.[3];
}

/** The stored key of an entity in one locale (language-neutral entities and the neutral locale `_`: the entity itself) */
export function entityKeyFor(entity: string, locale: string | null | undefined): string {
	return locale && locale !== NEUTRAL_LOCALE && isLocalizedEntity(entity) ? `${entity}:${locale}` : entity;
}

/** A key or entity split into its parts, null when it is none (`chapter:catpope:2` and `chapter:catpope:2:de` both parse) */
export function parseEntityKey(key: string): EntityRef | null {
	const parts = key.split(':');
	const first = parts.shift() ?? '';
	if (isCatalogKind(first)) return parts.length === 1 && CATALOG_ID.test(parts[0]) ? { kind: first, id: parts[0] } : null;
	const kind = first as Exclude<EntityKind, CatalogEntityKind>;
	const shape = SHAPES[kind];
	if (!shape) return null;
	if (ID_KINDS.includes(kind)) return parts.length === 1 && ENTITY_ID.test(parts[0]) ? { kind, id: parts[0] } : null;
	const [hasToon, hasNumber, hasScene, localized] = shape;
	const base = Number(hasToon) + Number(hasNumber) + Number(hasScene);
	if (parts.length !== base && !(localized && parts.length === base + 1)) return null;
	const ref: EntityRef = { kind };
	if (hasToon) ref.toon = parts.shift();
	if (hasNumber) {
		// seasons start at 1, chapters at 0 (a season's prologue); digits only, no leading zeros (like the server)
		const raw = parts.shift() ?? '';
		if (!(kind === 'season' ? /^[1-9]\d{0,5}$/ : /^(0|[1-9]\d{0,5})$/).test(raw)) return null;
		ref.number = Number(raw);
	}
	if (hasScene) ref.scene = parts.shift();
	if (localized && parts.length) ref.locale = parts.shift();
	return ref;
}

/** The entity of a key (the key without its locale) */
export function entityOf(key: string): string {
	const ref = parseEntityKey(key);
	return ref?.locale ? key.slice(0, -(ref.locale.length + 1)) : key;
}

// ── Scene references (content format) ─────────────────────────────────────────────────────────────────────────

const SCENE_REF = /^chapter(0|[1-9]\d{0,5})\/([a-z0-9_-]{1,64})\.json$/;

/**
 * A chapter group's scene reference `ref.src` (content format: `chapter<N>/<name>.json`) → the scene entity, null
 * when it isn't one. The reference string is part of the stored chapter documents and stays as it is.
 */
export function sceneEntityOfRef(toon: string, src: string): string | null {
	const match = SCENE_REF.exec(src);
	return match ? entities.scene(toon, Number(match[1]), match[2]) : null;
}

/** The `ref.src` value for a scene of a chapter (content format) */
export function sceneRef(chapter: number, name: string): string {
	return `chapter${chapter}/${name}.json`;
}

/** The scene's name of a `ref.src` (e.g. `scene-14`), null when it isn't one */
export function sceneNameOfRef(src: string): string | null {
	return SCENE_REF.exec(src)?.[2] ?? null;
}
