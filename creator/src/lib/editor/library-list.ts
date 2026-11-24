// The Library's list as the page draws it from the server's answer (GET /api/library, server/docs/API.md "The extended
// Library"): which query a place asks, the type select's choices, Find, the order and the groups shown, the levels of a
// thing stepped into. The server resolves what an entry is; this only arranges what it answered. Tests: test/library-list.test.ts.
import type { FileKind, FileOrigin, LibraryEntry, LibraryLevel, LibraryQuery, ThingType } from '@scutoons/api';
import { wordsIn } from '../shell/launcher.ts';

/** the places (LIBRARY.md "places"): This item · This season's cast · This toon · Everything · Unused · Recently added */
export type ShelfPlace = 'item' | 'cast' | 'toon' | 'all' | 'unused' | 'recent';
export const SHELF_PLACES: readonly ShelfPlace[] = ['item', 'cast', 'toon', 'all', 'unused', 'recent'];
export type ShelfSort = 'name' | 'recent' | 'uses';
export const SHELF_SORTS: readonly ShelfSort[] = ['name', 'recent', 'uses'];
export type ShelfGrouping = 'none' | 'type';
export const SHELF_GROUPINGS: readonly ShelfGrouping[] = ['none', 'type'];

/** one choice of the type select: things of these types, files of these kinds (LIBRARY.md "Answered" 1 and 5) */
export interface TypeChoice {
	value: string;
	group: 'things' | 'files' | null;
	types?: readonly ThingType[];
	kinds?: readonly FileKind[];
}

/** "All types", then the eleven kinds of thing (instruments with rigs), then the files' kinds */
export const TYPE_CHOICES: readonly TypeChoice[] = [
	{ value: 'all', group: null },
	...(['toon', 'character', 'scene', 'show', 'place', 'item', 'module'] as const).map((t) => ({ value: t, group: 'things' as const, types: [t] })),
	{ value: 'instrument', group: 'things', types: ['instrument', 'rig'] },
	...(['generator', 'kanon', 'person'] as const).map((t) => ({ value: t, group: 'things' as const, types: [t] })),
	...(['image', 'model', 'audio', 'video', 'animation', 'file'] as const).map((k) => ({ value: k, group: 'files' as const, kinds: [k] }))
];

/** what the page knows when it asks: the toon, the open item, the season whose cast is meant */
export interface PlaceContext {
	toon: string | null;
	open: string | null;
	/** `season:<toon>:<n>` */
	season: string | null;
}

/** whether a place can be asked here (This item needs an open item, the toon's places a toon, a cast its season) */
export function placeAvailable(place: ShelfPlace, ctx: PlaceContext): boolean {
	if (place === 'item') return !!ctx.open && !ctx.open.startsWith('toon:');
	if (place === 'cast') return !!seasonNumber(ctx.season) && !!ctx.toon;
	if (place === 'toon') return !!ctx.toon;
	return true;
}

/** the place a front item starts on: what it uses, else its toon, else everything */
export const startPlace = (ctx: PlaceContext): ShelfPlace => (placeAvailable('item', ctx) ? 'item' : ctx.toon ? 'toon' : 'all');

/** `season:catpope:1` → 1 */
export function seasonNumber(season: string | null): number | null {
	const n = Number(season?.split(':')[2]);
	return season?.startsWith('season:') && Number.isInteger(n) && n > 0 ? n : null;
}

/** the toon a season belongs to: `season:catpope:1` → catpope */
export const seasonToon = (season: string): string => season.split(':')[1] ?? '';

/** the server's query for a place, the type choice, From and a pick's kinds */
export function placeQuery(place: ShelfPlace, ctx: PlaceContext, choice: string, from: FileOrigin | null, pickKinds: readonly FileKind[] | null, locale: string): LibraryQuery {
	const q: LibraryQuery = { locale, limit: 5000 };
	if (place === 'item' && ctx.open) Object.assign(q, { toon: ctx.toon, place: `item:${ctx.open}` });
	else if (place === 'cast' && ctx.season) Object.assign(q, { toon: seasonToon(ctx.season), place: `cast:${seasonNumber(ctx.season)}` });
	else if (place === 'toon') Object.assign(q, { toon: ctx.toon, place: 'toon' });
	else if (place === 'all') q.place = 'all';
	else Object.assign(q, { toon: ctx.toon, place });
	if (pickKinds?.length) q.kind = pickKinds;
	else {
		const c = TYPE_CHOICES.find((t) => t.value === choice);
		if (c?.types) q.type = c.types;
		if (c?.kinds) q.kind = c.kinds;
	}
	if (from) q.from = from;
	return q;
}

/** a file entry's path (`image:characters/a.webp` → `characters/a.webp`); a thing: null */
export const filePath = (e: Pick<LibraryEntry, 'address' | 'kind'>): string | null => (e.kind ? e.address.slice(e.address.indexOf(':') + 1) : null);

/** the entries Find keeps: every word in the name or the address */
export const findIn = (entries: readonly LibraryEntry[], query: string): LibraryEntry[] => (query.trim() ? entries.filter((e) => wordsIn(`${e.name} ${e.address}`, query)) : [...entries]);

const block = (e: LibraryEntry) => e.type ?? e.kind ?? '';

/**
 * The order shown: the server's blocks (things by type, then files by kind) stay, each sorted by name, by the newest
 * change or by the most uses; equal ones keep the server's order.
 */
export function sortEntries(entries: readonly LibraryEntry[], sort: ShelfSort): LibraryEntry[] {
	const blocks = new Map<string, number>();
	for (const e of entries) if (!blocks.has(block(e))) blocks.set(block(e), blocks.size);
	const within = (a: LibraryEntry, b: LibraryEntry): number => {
		if (sort === 'recent') return (b.updated ?? '').localeCompare(a.updated ?? '');
		if (sort === 'uses') return b.usedIn - a.usedIn;
		return a.name.localeCompare(b.name, undefined, { sensitivity: 'base', numeric: true });
	};
	return entries
		.map((e, i) => ({ e, i }))
		.sort((a, b) => blocks.get(block(a.e))! - blocks.get(block(b.e))! || within(a.e, b.e) || a.i - b.i)
		.map(({ e }) => e);
}

/** the runs of one type or kind in the order shown (the grouping "By type") */
export function typeRuns(entries: readonly LibraryEntry[]): { key: string; entries: LibraryEntry[] }[] {
	const runs: { key: string; entries: LibraryEntry[] }[] = [];
	for (const e of entries) {
		const key = block(e);
		if (runs.at(-1)?.key === key) runs.at(-1)!.entries.push(e);
		else runs.push({ key, entries: [e] });
	}
	return runs;
}

/** a level's identity: a mood shares its look's address and adds its own name */
export const levelKey = (l: Pick<LibraryLevel, 'address' | 'mood'>): string => (l.mood ? `${l.address}#${l.mood}` : l.address);

/**
 * The columns of a thing stepped into: the levels under the thing, then under each chosen level (a character: personas,
 * the chosen persona's looks, the chosen look's moods), each with the level's kind; a chosen key no longer there ends it.
 */
export function levelColumns(levels: readonly LibraryLevel[], at: string, chosen: readonly string[]): { level: LibraryLevel['level']; rows: LibraryLevel[] }[] {
	const columns: { level: LibraryLevel['level']; rows: LibraryLevel[] }[] = [];
	let parent = at;
	let parentKey = at;
	for (let i = 0; ; i++) {
		const rows = levels.filter((l) => l.parent === parent && levelKey(l) !== parentKey);
		if (!rows.length) break;
		columns.push({ level: rows[0].level, rows });
		const pick = rows.find((l) => levelKey(l) === chosen[i]);
		if (!pick || pick.mood) break;
		parent = pick.address;
		parentKey = levelKey(pick);
	}
	return columns;
}
