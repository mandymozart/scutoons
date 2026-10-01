// Entity keys (agents/DATA.md §2): how every toon-related document is addressed in the database and the API.
//
//   toon:<toon>                       the toon's index entry (list entry: status, names, languages)
//   toon-settings:<toon>:<locale>     the toon's settings (title, covers, map, downloads)
//   season:<toon>:<n>:<locale>        chapter:<toon>:<n>:<locale>        scene:<toon>:<chapter>:<name>:<locale>
//   locations:<toon>:<locale>         routes:<toon>:<locale>             (the map)
//   cast:<toon>                       credits:<toon>                     (language-neutral, per toon)
//   characters  moods  asset-requests  people  roles  events           (platform-wide, language-neutral)
//
// An "entity" is a key without its locale (`chapter:catpope:2`); the stored document of one language adds `:<locale>`.
// Language-neutral entities are stored under the entity itself. JSON file names exist only in the import/export
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
	| 'characters'
	| 'moods'
	| 'asset-requests'
	| 'people'
	| 'roles'
	| 'events';

/** kind → [has toon, has number, has scene name, localized] */
const SHAPES: Record<EntityKind, [boolean, boolean, boolean, boolean]> = {
	toon: [true, false, false, false],
	'toon-settings': [true, false, false, true],
	season: [true, true, false, true],
	chapter: [true, true, false, true],
	scene: [true, true, true, true],
	locations: [true, false, false, true],
	routes: [true, false, false, true],
	cast: [true, false, false, false],
	credits: [true, false, false, false],
	characters: [false, false, false, false],
	moods: [false, false, false, false],
	'asset-requests': [false, false, false, false],
	people: [false, false, false, false],
	roles: [false, false, false, false],
	events: [false, false, false, false]
};

export interface EntityRef {
	kind: EntityKind;
	toon?: string;
	/** season / chapter number, the chapter of a scene */
	number?: number;
	/** a scene's name (e.g. `scene-14`) */
	scene?: string;
	/** only on stored keys of localized kinds */
	locale?: string;
}

/** Builders of entities (keys without locale) */
export const entities = {
	toon: (toon: string) => `toon:${toon}`,
	toonSettings: (toon: string) => `toon-settings:${toon}`,
	season: (toon: string, n: number) => `season:${toon}:${n}`,
	chapter: (toon: string, n: number) => `chapter:${toon}:${n}`,
	scene: (toon: string, chapter: number, name: string) => `scene:${toon}:${chapter}:${name}`,
	locations: (toon: string) => `locations:${toon}`,
	routes: (toon: string) => `routes:${toon}`,
	cast: (toon: string) => `cast:${toon}`,
	credits: (toon: string) => `credits:${toon}`,
	characters: 'characters',
	moods: 'moods',
	assetRequests: 'asset-requests',
	people: 'people',
	roles: 'roles',
	events: 'events'
} as const;

/** Whether an entity (or key) of this kind is stored per locale */
export function isLocalizedEntity(entity: string): boolean {
	const shape = SHAPES[entity.split(':')[0] as EntityKind];
	return !!shape?.[3];
}

/** The stored key of an entity in one locale (language-neutral entities and the neutral locale `_`: the entity itself) */
export function entityKeyFor(entity: string, locale: string | null | undefined): string {
	return locale && locale !== NEUTRAL_LOCALE && isLocalizedEntity(entity) ? `${entity}:${locale}` : entity;
}

/** A key or entity split into its parts, null when it is none (`chapter:catpope:2` and `chapter:catpope:2:de` both parse) */
export function parseEntityKey(key: string): EntityRef | null {
	const parts = key.split(':');
	const kind = parts.shift() as EntityKind;
	const shape = SHAPES[kind];
	if (!shape) return null;
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
