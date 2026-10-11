// The world map as many documents (DRAFT, agents/MAP-DOCUMENTS.md – owner 2026-10-12: "alle einen eigenen Doc, den man
// versionieren kann"): every place is `place:<id>`, every route `route:<id>`, and `map` keeps only the map's own settings
// (size, layers). Each is versioned on its own (drafts, publish, history, restore – entity_versions). Everything that
// reads the map still works on ONE `WorldMap`: it is composed from the documents (here for TS, `Domain\Content\WorldMapDocs`
// for PHP – keep both in step) and split again for saving. Pure: no Svelte, no DOM.
//
//   map            MapSettings   { width, height, layers, accepted_warnings? }
//   place:<id>     PlaceDoc      a MapPlace without its id (the key holds it)
//   route:<id>     RouteDoc      a MapPath without its id
//
// Removing a place / route stores a last version with `removed: true` (a tombstone): the id stays taken forever (events,
// routes and readers' save games reference it), the history stays, publishing the tombstone takes it off readers' maps,
// and restoring an older version brings it back. Order on the map = creation order of the documents (the first version's
// id), so adding a place never touches another document.
import { acceptedWarningsOf, ACCEPTED_WARNINGS_FIELD, type AcceptedWarnings } from './accepted.ts';
import { entities, parseEntityKey } from './entities.ts';
import { checkWorldMap, type MapPath, type MapPlace, type MapProblem, type MapValidateOptions, type WorldMap } from './map.ts';

/** The document `map`: the world map's own settings */
export interface MapSettings extends AcceptedWarnings {
	/** world units = px of the terrain artwork; every position is in this space */
	width: number;
	height: number;
	/** drawn bottom → top */
	layers: WorldMap['layers'];
}

/** The document `place:<id>` */
export type PlaceDoc = Omit<MapPlace, 'id'> & AcceptedWarnings & {
	/** taken off the map (a tombstone: the id stays reserved, the history stays) */
	removed?: boolean;
};

/** The document `route:<id>` */
export type RouteDoc = Omit<MapPath, 'id'> & AcceptedWarnings & {
	/** taken off the map (a tombstone) */
	removed?: boolean;
};

export type MapPartKind = 'place' | 'route';

/** One stored part of the map, in creation order */
export interface MapPart<D = PlaceDoc | RouteDoc> {
	id: string;
	doc: D;
}

/** The map's documents, as stored (tombstones included) */
export interface MapDocs {
	settings: MapSettings | null;
	places: MapPart<PlaceDoc>[];
	routes: MapPart<RouteDoc>[];
}

/** the list of the composed map a part kind lives in */
export const MAP_PART_FIELD: Record<MapPartKind, 'locations' | 'routes'> = { place: 'locations', route: 'routes' };

/** is a stored key one of the world map's documents (`map`, `place:<id>`, `route:<id>`)? */
export function isMapDocKey(key: string): boolean {
	const kind = parseEntityKey(key)?.kind;
	return kind === 'map' || kind === 'place' || kind === 'route';
}

const isObj = (v: unknown): v is Record<string, any> => !!v && typeof v === 'object' && !Array.isArray(v);

// ── accepted warnings: a part's own keys are relative to it; the composed map's are `locations.@<id>…` ────────────────

/** `en:locations.@cathedral.hidden#x1` → scope `en:`, rest */
function splitScope(key: string): [string, string] {
	const m = /^([a-z]{2}:)(.*)$/.exec(key);
	return m ? [m[1], m[2]] : ['', key];
}

/** a part's accepted key → the composed map's (`hidden#x1` → `locations.@cathedral.hidden#x1`; `#x1` → `locations.@cathedral#x1`) */
function absoluteAccepted(field: string, id: string, key: string): string {
	const [scope, rest] = splitScope(key);
	return `${scope}${field}.@${id}${rest.startsWith('#') ? '' : '.'}${rest}`;
}

/** the composed map's accepted key → the part it belongs to and its own key, null for a key of the settings */
function relativeAccepted(key: string): { field: string; id: string; key: string } | null {
	const [scope, rest] = splitScope(key);
	const m = /^(locations|routes)\.@([a-z0-9][a-z0-9_-]{0,63})(?:\.(.*)|(#.*))$/.exec(rest);
	if (!m) return null;
	return { field: m[1], id: m[2], key: `${scope}${m[3] ?? m[4]}` };
}

// ── compose / split ─────────────────────────────────────────────────────────────────────────────────────────────

const withoutMeta = <T extends Record<string, any>>(doc: T): T => {
	const { removed: _removed, [ACCEPTED_WARNINGS_FIELD]: _accepted, ...rest } = doc;
	return rest as T;
};

/**
 * The one WorldMap of the documents: the settings, then every place / route that is not removed, in the given order
 * (creation order). The parts' accepted warnings are moved into the map's list with their place in it.
 */
export function composeWorldMap(docs: MapDocs): WorldMap {
	const s = isObj(docs.settings) ? docs.settings : null;
	const accepted = acceptedWarningsOf(s);
	const take = <D extends PlaceDoc | RouteDoc>(field: 'locations' | 'routes', parts: MapPart<D>[]) =>
		parts
			.filter((p) => isObj(p.doc) && !p.doc.removed)
			.map((p) => {
				for (const k of acceptedWarningsOf(p.doc)) accepted.push(absoluteAccepted(field, p.id, k));
				return { id: p.id, ...withoutMeta(p.doc) };
			});
	const locations = take('locations', docs.places) as MapPlace[];
	const routes = take('routes', docs.routes) as MapPath[];
	const map: WorldMap = {
		width: s?.width ?? 0,
		height: s?.height ?? 0,
		layers: Array.isArray(s?.layers) ? s.layers : [],
		locations,
		routes
	};
	if (accepted.length) map[ACCEPTED_WARNINGS_FIELD] = accepted;
	return map;
}

/** The documents of one WorldMap (no tombstones: a part that is not on the map is simply not there) */
export function splitWorldMap(map: WorldMap): { settings: MapSettings; places: MapPart<PlaceDoc>[]; routes: MapPart<RouteDoc>[] } {
	const partAccepted = new Map<string, string[]>();
	const settingsAccepted: string[] = [];
	for (const key of acceptedWarningsOf(map)) {
		const rel = relativeAccepted(key);
		if (!rel) settingsAccepted.push(key);
		else partAccepted.set(`${rel.field}:${rel.id}`, [...(partAccepted.get(`${rel.field}:${rel.id}`) ?? []), rel.key]);
	}
	const part = <D>(field: string, e: { id: string } & Record<string, any>): MapPart<D> => {
		const { id, ...rest } = e;
		const accepted = partAccepted.get(`${field}:${id}`);
		return { id, doc: (accepted?.length ? { ...rest, [ACCEPTED_WARNINGS_FIELD]: accepted } : rest) as D };
	};
	const settings: MapSettings = { width: map.width, height: map.height, layers: map.layers ?? [] };
	if (settingsAccepted.length) settings[ACCEPTED_WARNINGS_FIELD] = settingsAccepted;
	return {
		settings,
		places: (map.locations ?? []).map((p) => part<PlaceDoc>('locations', p)),
		routes: (map.routes ?? []).map((r) => part<RouteDoc>('routes', r))
	};
}

/**
 * The documents of a list of stored entities in creation order (GET /entities?kinds=map,place,route – the creator, the
 * MCP server); other keys and entities without a document are left out. `replace` swaps one key's document (a draft
 * to check before it is saved; null takes it out).
 */
export function mapDocsFromEntities(list: readonly { key: string; content: unknown }[], replace?: { key: string; doc: unknown }): MapDocs {
	const docs: MapDocs = { settings: null, places: [], routes: [] };
	const entries = list.map((e) => ({ key: e.key, content: replace && e.key === replace.key ? replace.doc : e.content }));
	if (replace && !list.some((e) => e.key === replace.key)) entries.push({ key: replace.key, content: replace.doc });
	for (const e of entries) {
		if (!isObj(e.content)) continue;
		const ref = parseEntityKey(e.key);
		if (ref?.kind === 'map') docs.settings = e.content as unknown as MapSettings;
		else if (ref?.kind === 'place') docs.places.push({ id: ref.id!, doc: e.content as PlaceDoc });
		else if (ref?.kind === 'route') docs.routes.push({ id: ref.id!, doc: e.content as RouteDoc });
	}
	return docs;
}

/** A key → document record of a WorldMap (the documents to store) */
export function worldMapDocs(map: WorldMap): Record<string, MapSettings | PlaceDoc | RouteDoc> {
	const { settings, places, routes } = splitWorldMap(map);
	return {
		[entities.map]: settings,
		...Object.fromEntries(places.map((p) => [entities.place(p.id), p.doc])),
		...Object.fromEntries(routes.map((r) => [entities.route(r.id), r.doc]))
	};
}

/** JSON with sorted object keys: two documents are the same when this is (the editors don't keep key order) */
export function stableJson(value: unknown): string {
	return JSON.stringify(value, (_k, v) => (isObj(v) ? Object.fromEntries(Object.keys(v).sort().map((k) => [k, v[k]])) : v));
}

export type MapDocChangeKind = 'added' | 'changed' | 'removed' | 'restored' | 'settings';

export interface MapDocChange {
	key: string;
	kind: MapDocChangeKind;
	/** what to store */
	doc: MapSettings | PlaceDoc | RouteDoc;
}

/**
 * What to store so that the documents become `next`: the settings when they changed, every added / changed place and
 * route, a tombstone for every one that left the map (its last document + `removed: true`), a removed one that is back.
 * `stored` = the documents as they are now (tombstones included).
 */
export function mapDocChanges(stored: MapDocs, next: WorldMap): MapDocChange[] {
	const out: MapDocChange[] = [];
	const want = splitWorldMap(next);
	if (!stored.settings || stableJson(stored.settings) !== stableJson(want.settings)) out.push({ key: entities.map, kind: 'settings', doc: want.settings });
	const diff = <D extends PlaceDoc | RouteDoc>(kind: MapPartKind, before: MapPart<D>[], after: MapPart<D>[]) => {
		const key = (id: string) => (kind === 'place' ? entities.place(id) : entities.route(id));
		const was = new Map(before.map((p) => [p.id, p.doc]));
		const now = new Set(after.map((p) => p.id));
		for (const p of after) {
			const old = was.get(p.id);
			if (!old) out.push({ key: key(p.id), kind: 'added', doc: p.doc });
			else if (old.removed) out.push({ key: key(p.id), kind: 'restored', doc: p.doc });
			else if (stableJson(old) !== stableJson(p.doc)) out.push({ key: key(p.id), kind: 'changed', doc: p.doc });
		}
		for (const p of before) if (!now.has(p.id) && !p.doc.removed) out.push({ key: key(p.id), kind: 'removed', doc: { ...p.doc, removed: true } });
	};
	diff('place', stored.places, want.places);
	diff('route', stored.routes, want.routes);
	return out;
}

/** The documents after storing `changes` (the saved state of an editor) */
export function applyMapDocChanges(stored: MapDocs, changes: readonly MapDocChange[]): MapDocs {
	const out: MapDocs = { settings: stored.settings, places: [...stored.places], routes: [...stored.routes] };
	for (const c of changes) {
		const ref = parseEntityKey(c.key);
		if (ref?.kind === 'map') out.settings = c.doc as MapSettings;
		else if (ref?.kind === 'place' || ref?.kind === 'route') {
			const list = ref.kind === 'place' ? out.places : out.routes;
			const i = list.findIndex((p) => p.id === ref.id);
			const part = { id: ref.id!, doc: c.doc as PlaceDoc & RouteDoc };
			if (i < 0) list.push(part);
			else list[i] = part;
		}
	}
	return out;
}

/** Every id ever used by a kind of part (tombstones included): a new place / route must not take one of them */
export const takenMapIds = (docs: MapDocs, kind: MapPartKind): string[] => (kind === 'place' ? docs.places : docs.routes).map((p) => p.id);

// ── validation across the documents ─────────────────────────────────────────────────────────────────────────────

/** A problem of the composed map, addressed to the document it is in (`place:<id>` + the path inside it) */
export interface MapDocProblem extends MapProblem {
	/** the stored key the problem is in */
	key: string;
	/** the path inside that document */
	docPath: (string | number)[];
}

/** a composed map's path → the document and the path inside it (`['locations', 3, 'x']` → `place:<id>`, `['x']`) */
export function mapDocOfPath(map: WorldMap, path: readonly (string | number)[]): { key: string; path: (string | number)[] } {
	const [field, index, ...rest] = path;
	if ((field === 'locations' || field === 'routes') && typeof index === 'number') {
		const id = (field === 'locations' ? map.locations : map.routes)?.[index]?.id;
		if (typeof id === 'string') return { key: field === 'locations' ? entities.place(id) : entities.route(id), path: rest };
	}
	return { key: entities.map, path: [...path] };
}

/**
 * The world map's problems across its documents: checkWorldMap of the composition (references between places and
 * routes – a route through a removed place points at an unknown one –, duplicate ids, positions inside the map's size),
 * each addressed to its document.
 */
export function checkMapDocs(docs: MapDocs, options: MapValidateOptions = {}): MapDocProblem[] {
	const map = composeWorldMap(docs);
	return checkWorldMap(map, options).map((p) => {
		const at = mapDocOfPath(map, p.path);
		return { ...p, key: at.key, docPath: at.path };
	});
}
