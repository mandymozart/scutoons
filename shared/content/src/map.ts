// The SCUTOONS world map (agents/MAP.md): ONE map for the whole universe (entity `map`, platform-wide, language-neutral,
// names as `{ de, en }` inside), the reader's map state (unlocked places, revealed routes, positions – world-wide game
// state under the reserved id `scutoons`), the old per-toon answer shape for older viewer builds, the migration from
// catpope's `locations.json` / `routes.json`, and the validation. Pure: no Svelte, no DOM.
import type { LocaleText } from './credits.ts';
import type { Map as LegacyToonMap, MapLocation, MapRoute } from './content.ts';

// ── The entity `map` ────────────────────────────────────────────────────────────────────────────────────────────

/** The SCUTOONS world map (entity `map`, seed `scutoons/_/map.json`) */
export interface WorldMap {
	/** world units = px of the terrain artwork (2048 × 1046 today); every position is in this space */
	width: number;
	height: number;
	/** drawn bottom → top; image paths relative to the CDN root */
	layers: MapLayer[];
	locations: MapPlace[];
	routes: MapPath[];
}

export interface MapLayer {
	/** e.g. `terrain` */
	id: string;
	/** CDN-root path, e.g. `catpope/images/map/map-layer-terrain.svg` */
	image: string;
	/** shown only from / up to this zoom (e.g. a street-detail layer for the city) */
	minZoom?: number;
	maxZoom?: number;
	/** hidden in the builder only (the reader always draws it) – not stored when false */
	builderHidden?: boolean;
}

export interface MapPlace {
	/** slug, stable forever (events reference it) */
	id: string;
	/** `{ de, en }`, more languages as keys; missing → the source language (de) */
	name: LocaleText;
	/** the place's ground point, world units */
	x: number;
	y: number;
	/** the small illustration (agents/MAP.md §5) */
	image?: MapImage;
	/** the toons whose story uses the place (the builder's "used by", the reader's per-toon view) */
	toons: string[];
	/** hidden until it is unlocked (by `unlock`, later by an event) – default: visible from the start */
	hidden?: boolean;
	/** where the story reaches the place (owner 2026-10-01: places unlock over time) – read before the events exist (M3) */
	unlock?: PlaceUnlock;
	/** the chapter the place's card links to */
	chapter?: { toon: string; number: number };
}

/**
 * When a hidden place unlocks by reading progress: a chapter of a toon, from a reading position on (0..1 of the chapter,
 * the reader's chapter checkpoint – 0 = when the chapter is opened, 1 = its end). `scene` = the screenplay scene that
 * introduces the place (the reference for the event that replaces this rule in the editor, M3/M4).
 */
export interface PlaceUnlock {
	toon: string;
	chapter: number;
	/** reading position in the chapter, 0..1 (default 0) */
	at?: number;
	/** the screenplay's scene number */
	scene?: number;
}

export interface MapImage {
	/** CDN-root path, e.g. `map/locations/cathedral.webp` */
	src: string;
	/** drawn width in screen px at zoom 1 (default 48); the image keeps its ratio */
	size?: number;
	/** the ground point inside the image, % of its box (default 50 / 90 = bottom centre) – sits on the place's (x, y) */
	anchor?: { x: number; y: number };
}

export type MapPathStyle = 'road' | 'trail' | 'flight' | 'sea';

export interface MapPath {
	id: string;
	name: LocaleText;
	toons: string[];
	/** ordered, ≥ 2; a waypoint is a place or a free point */
	waypoints: Waypoint[];
	/** how the line is drawn (default `road`) */
	style?: MapPathStyle;
	/** hidden until an event reveals it (default visible) */
	hidden?: boolean;
}

/** A place of the map (`location` = the place id; its waypoint id is the place id) */
export interface PlaceWaypoint {
	location: string;
}
/** A free point on a route (a bridge, a pass); `id` unique within the route */
export interface FreeWaypoint {
	id: string;
	x: number;
	y: number;
	name?: LocaleText;
}
export type Waypoint = PlaceWaypoint | FreeWaypoint;

/** The toon's start view on the world map (`toon:<toon>` → `map`; language-neutral) */
export interface ToonMapView {
	center?: { x: number; y: number };
	zoom?: number;
}

export const MAP_SOURCE_LANGUAGE = 'de';
export const MAP_STYLES: MapPathStyle[] = ['road', 'trail', 'flight', 'sea'];
export const DEFAULT_PLACE_IMAGE_SIZE = 48;
export const DEFAULT_PLACE_ANCHOR = { x: 50, y: 90 } as const;
export const MAP_MIN_ZOOM = 0.25;
export const MAP_MAX_ZOOM = 8;
/** The reserved game-state id of the world (unlocks are universe-wide; no toon may be called `scutoons`) */
export const WORLD_STATE_ID = 'scutoons';

export const emptyWorldMap = (): WorldMap => ({ width: 2048, height: 1046, layers: [], locations: [], routes: [] });

export const isPlaceWaypoint = (w: Waypoint | ReaderWaypoint | null | undefined): w is PlaceWaypoint =>
	!!w && typeof (w as PlaceWaypoint).location === 'string';

/** The id a waypoint is addressed by (events: `{ route, to: <waypoint id> }`): the place id, else the free point's id */
export const waypointId = (w: Waypoint | ReaderWaypoint): string => (isPlaceWaypoint(w) ? w.location : w.id);

// ── The reader's answer (`GET /library/map`) ────────────────────────────────────────────────────────────────────────

/** A place with its name in the reader's locale */
export type ReaderPlace = Omit<MapPlace, 'name'> & { name: string };
export type ReaderWaypoint = PlaceWaypoint | (Omit<FreeWaypoint, 'name'> & { name?: string });
export type ReaderPath = Omit<MapPath, 'name' | 'waypoints'> & { name: string; waypoints: ReaderWaypoint[] };

/** `/library/map?locale=&toon=`: the world for this reader (places/routes of listed toons only, names resolved) */
export interface ReaderMap {
	width: number;
	height: number;
	layers: MapLayer[];
	locations: ReaderPlace[];
	routes: ReaderPath[];
	/** with `?toon=`: where the map opens for that toon (its start view, else the centre of its places) */
	view?: { center: { x: number; y: number }; zoom: number };
}

/** A name in a locale: the locale, else the source language, else any */
export function mapName(name: LocaleText | string | undefined, locale: string): string {
	if (typeof name === 'string') return name;
	if (!name) return '';
	for (const l of [locale, MAP_SOURCE_LANGUAGE]) if (name[l]?.trim()) return name[l];
	return Object.values(name).find((v) => typeof v === 'string' && v.trim()) ?? '';
}

/** The world map resolved for one locale (what `/library/map` answers, without the toon filter) */
export function resolveWorldMap(map: WorldMap, locale: string): ReaderMap {
	return {
		width: map.width,
		height: map.height,
		layers: map.layers ?? [],
		locations: (map.locations ?? []).map((p) => ({ ...p, name: mapName(p.name, locale) })),
		routes: (map.routes ?? []).map((r) => ({
			...r,
			name: mapName(r.name, locale),
			waypoints: (r.waypoints ?? []).map((w) => (isPlaceWaypoint(w) ? { ...w } : { ...w, ...(w.name ? { name: mapName(w.name, locale) } : {}) })) as ReaderWaypoint[]
		}))
	};
}

// ── Geometry (reader + builder draw routes identically) ──────────────────────────────────────────────────────────────

export interface Point {
	x: number;
	y: number;
}

/** The point of a waypoint (a place waypoint: the place's point), null for an unknown place (e.g. catpope's `garden`) */
export function waypointPoint(places: readonly { id: string; x: number; y: number }[], w: Waypoint | ReaderWaypoint): Point | null {
	if (isPlaceWaypoint(w)) {
		const p = places.find((x) => x.id === w.location);
		return p ? { x: p.x, y: p.y } : null;
	}
	return Number.isFinite(w.x) && Number.isFinite(w.y) ? { x: w.x, y: w.y } : null;
}

/** The drawable points of a route's waypoints `[from, to]` (inclusive; unknown places skipped, like the reader always did) */
export function routePoints(places: readonly { id: string; x: number; y: number }[], waypoints: readonly (Waypoint | ReaderWaypoint)[], from = 0, to = Infinity): Point[] {
	const out: Point[] = [];
	waypoints.forEach((w, i) => {
		if (i < from || i > to) return;
		const p = waypointPoint(places, w);
		if (p) out.push(p);
	});
	return out;
}

/**
 * A smooth SVG path through the points (cubic Bézier, control points along the neighbours' tangents – the curve of the
 * original reader map, `tension` 0.3). `bow` lifts every segment's control points (a flight arcs upwards).
 */
export function bezierPath(points: readonly Point[], tension = 0.3, bow = 0): string {
	if (points.length < 2) return '';
	let d = `M ${round(points[0].x)} ${round(points[0].y)}`;
	for (let i = 1; i < points.length; i++) {
		const prev = points[i - 1];
		const curr = points[i];
		const before = points[i - 2] ?? prev;
		const after = points[i + 1] ?? curr;
		const t1 = { x: curr.x - before.x, y: curr.y - before.y };
		const t2 = { x: after.x - prev.x, y: after.y - prev.y };
		const l1 = Math.hypot(t1.x, t1.y) || 1;
		const l2 = Math.hypot(t2.x, t2.y) || 1;
		const len = Math.hypot(curr.x - prev.x, curr.y - prev.y);
		const lift = bow * len;
		const c1 = { x: prev.x + (t1.x / l1) * len * tension, y: prev.y + (t1.y / l1) * len * tension - lift };
		const c2 = { x: curr.x - (t2.x / l2) * len * tension, y: curr.y - (t2.y / l2) * len * tension - lift };
		d += ` C ${round(c1.x)} ${round(c1.y)}, ${round(c2.x)} ${round(c2.y)}, ${round(curr.x)} ${round(curr.y)}`;
	}
	return d;
}

const round = (n: number) => Math.round(n * 100) / 100;

/** How a route style is drawn (agents/MAP.md §4.4): stroke width in screen px, dash pattern, colours, Bézier tension/bow */
export function routeStroke(style: MapPathStyle | undefined): { width: number; dash: string; color: string; outline?: string; tension: number; bow: number } {
	switch (style) {
		case 'trail':
			return { width: 4, dash: '10 8', color: '#000', tension: 0.3, bow: 0 };
		case 'flight':
			return { width: 3, dash: '1 7', color: '#000', tension: 0.5, bow: 0.25 };
		case 'sea':
			return { width: 4, dash: '12 8', color: '#fff', outline: '#000', tension: 0.3, bow: 0 };
		default:
			return { width: 6, dash: '', color: '#000', tension: 0.3, bow: 0 };
	}
}

/** The bounding box centre of points (null when none) */
export function centerOf(points: readonly Point[]): Point | null {
	if (!points.length) return null;
	const xs = points.map((p) => p.x);
	const ys = points.map((p) => p.y);
	return { x: (Math.min(...xs) + Math.max(...xs)) / 2, y: (Math.min(...ys) + Math.max(...ys)) / 2 };
}

/** The image box of a place in screen px at a zoom: left/top relative to the place's screen point */
export function placeImageBox(image: MapImage, zoom: number, ratio = 1): { width: number; height: number; left: number; top: number } {
	const width = (image.size ?? DEFAULT_PLACE_IMAGE_SIZE) * zoom;
	const height = width / (ratio || 1);
	const anchor = image.anchor ?? DEFAULT_PLACE_ANCHOR;
	return { width, height, left: -(anchor.x / 100) * width, top: -(anchor.y / 100) * height };
}

/**
 * Which labels to show at a zoom without overlapping (agents/MAP.md M1 "labels by zoom"): `items` in priority order (the
 * first wins), positions in screen px, an approximate label box per item. Returns the ids to label.
 */
export function visibleLabels(items: readonly { id: string; x: number; y: number; width: number; height: number }[]): Set<string> {
	const shown: { x0: number; y0: number; x1: number; y1: number }[] = [];
	const out = new Set<string>();
	for (const it of items) {
		const box = { x0: it.x - it.width / 2, y0: it.y - it.height, x1: it.x + it.width / 2, y1: it.y };
		if (shown.some((b) => box.x0 < b.x1 && box.x1 > b.x0 && box.y0 < b.y1 && box.y1 > b.y0)) continue;
		shown.push(box);
		out.add(it.id);
	}
	return out;
}

// ── The reader's map state (game state `scutoons`, world-wide) ───────────────────────────────────────────────────────

/** Where a toon's hero is ("you are here"); `at` = when it was set (ms) – the newer write wins in the merge */
export type MapPosition = ({ location: string } | { route: string; waypoint: string }) & { at: number };

export interface MapState {
	/** place id → when it was unlocked / first seen on the opened map (ms) */
	places: Record<string, { unlocked: number; seen?: number }>;
	/** route id → revealed up to waypoint index (inclusive), travelled up to index; `seen` = the revealed index already shown */
	routes: Record<string, { revealed: number; travelled?: number; seen?: number }>;
	/** toon → where its hero is */
	positions: Record<string, MapPosition>;
}

export const emptyMapState = (): MapState => ({ places: {}, routes: {}, positions: {} });

/** A stored value as a map state (missing parts filled, junk dropped) */
export function normalizeMapState(value: unknown): MapState {
	const v = (value && typeof value === 'object' ? value : {}) as Partial<MapState>;
	const obj = (x: unknown) => (x && typeof x === 'object' && !Array.isArray(x) ? (x as Record<string, any>) : {});
	const state = emptyMapState();
	for (const [id, p] of Object.entries(obj(v.places))) if (p && Number.isFinite(p.unlocked)) state.places[id] = { unlocked: p.unlocked, ...(Number.isFinite(p.seen) ? { seen: p.seen } : {}) };
	for (const [id, r] of Object.entries(obj(v.routes)))
		if (r && Number.isInteger(r.revealed))
			state.routes[id] = { revealed: r.revealed, ...(Number.isInteger(r.travelled) ? { travelled: r.travelled } : {}), ...(Number.isInteger(r.seen) ? { seen: r.seen } : {}) };
	for (const [toon, p] of Object.entries(obj(v.positions)))
		if (p && Number.isFinite(p.at) && (typeof p.location === 'string' || (typeof p.route === 'string' && typeof p.waypoint === 'string'))) state.positions[toon] = { ...p } as MapPosition;
	return state;
}

const minDefined = (a?: number, b?: number) => (a === undefined ? b : b === undefined ? a : Math.min(a, b));
const maxDefined = (a?: number, b?: number) => (a === undefined ? b : b === undefined ? a : Math.max(a, b));

/**
 * Two devices' map states as one (agents/MAP.md §2.4 "Merge, not last-write-wins"): union of places and routes, the
 * earlier `unlocked` / first `seen`, the larger `revealed` / `travelled` / `seen` index; positions: the newer `at`.
 * Commutative and idempotent – a discovery made on any device is never lost. The server applies the same merge to
 * `PUT /api/progress/scutoons/` (`ProgressService::mergeWorldState`).
 */
export function mergeMapState(a: MapState | null | undefined, b: MapState | null | undefined): MapState {
	const x = normalizeMapState(a);
	const y = normalizeMapState(b);
	const out = emptyMapState();
	for (const id of new Set([...Object.keys(x.places), ...Object.keys(y.places)])) {
		const p = x.places[id];
		const q = y.places[id];
		const seen = minDefined(p?.seen, q?.seen);
		out.places[id] = { unlocked: minDefined(p?.unlocked, q?.unlocked)!, ...(seen !== undefined ? { seen } : {}) };
	}
	for (const id of new Set([...Object.keys(x.routes), ...Object.keys(y.routes)])) {
		const p = x.routes[id];
		const q = y.routes[id];
		const travelled = maxDefined(p?.travelled, q?.travelled);
		const seen = maxDefined(p?.seen, q?.seen);
		out.routes[id] = { revealed: maxDefined(p?.revealed, q?.revealed)!, ...(travelled !== undefined ? { travelled } : {}), ...(seen !== undefined ? { seen } : {}) };
	}
	for (const toon of new Set([...Object.keys(x.positions), ...Object.keys(y.positions)])) {
		const p = x.positions[toon];
		const q = y.positions[toon];
		out.positions[toon] = !p ? q : !q ? p : q.at > p.at ? q : p;
	}
	return out;
}

// State operations (set operations: firing one again changes nothing). They return whether something changed, so the
// caller toasts only what is new (agents/MAP.md §3.3). The events that call them (unlockLocation, revealRoute, travel,
// setPosition) come with M3.

export function unlockPlace(state: MapState, id: string, now: number): boolean {
	if (state.places[id]) return false;
	state.places[id] = { unlocked: now };
	return true;
}

export function revealRoute(state: MapState, id: string, upTo: number): boolean {
	const current = state.routes[id];
	if (current && current.revealed >= upTo) return false;
	state.routes[id] = { ...current, revealed: upTo };
	return true;
}

export function setPosition(state: MapState, toon: string, position: Omit<MapPosition, 'at'>, now: number): void {
	state.positions[toon] = { ...position, at: now } as MapPosition;
}

/** Marks places as seen on the opened map; returns the ids that were new */
export function markPlacesSeen(state: MapState, ids: Iterable<string>, now: number): string[] {
	const out: string[] = [];
	for (const id of ids) {
		const p = state.places[id];
		if (p && p.seen === undefined) {
			p.seen = now;
			out.push(id);
		}
	}
	return out;
}

/** Visible on the reader's map: not hidden, or unlocked */
export const isPlaceVisible = (place: Pick<MapPlace, 'id' | 'hidden'>, state: MapState): boolean => !place.hidden || !!state.places[place.id];
/** Up to which waypoint index a route is drawn (-1 = not at all) */
export function routeRevealedTo(route: Pick<MapPath, 'id' | 'hidden' | 'waypoints'>, state: MapState): number {
	const last = (route.waypoints?.length ?? 0) - 1;
	if (!route.hidden) return last;
	const r = state.routes[route.id];
	return r ? Math.min(r.revealed, last) : -1;
}
/** A place unlocked but not seen on the opened map yet (the cyan halo, the toggle's badge) */
export const isPlaceNew = (id: string, state: MapState): boolean => !!state.places[id] && state.places[id].seen === undefined;

/**
 * The hidden places a toon's reading progress has reached (`unlock`: the chapter's checkpoint is finished, or its
 * position ≥ `at`). The reader writes them into the world state (`unlockPlace`), so they stay unlocked everywhere –
 * the progress-based rule until the story events exist (M3).
 */
export function unlocksFromProgress(
	places: readonly Pick<MapPlace, 'id' | 'hidden' | 'unlock'>[],
	toon: string,
	checkpoints: readonly { uri: string; isComplete?: boolean; position?: number }[] | null | undefined
): string[] {
	const out: string[] = [];
	const list = Array.isArray(checkpoints) ? checkpoints : [];
	for (const p of places) {
		const u = p.unlock;
		if (!p.hidden || !u || u.toon !== toon) continue;
		const cp = list.find((c) => c?.uri === `${toon}/chapter/${u.chapter}`);
		if (!cp) continue;
		const position = typeof cp.position === 'number' && Number.isFinite(cp.position) ? cp.position : cp.isComplete ? 1 : 0;
		if (cp.isComplete || position >= (u.at ?? 0)) out.push(p.id);
	}
	return out;
}

/**
 * Up to which waypoint a route is drawn for a reader: a visible route entirely; a hidden one up to its revealed index, else
 * (before the events, M3) up to its last waypoint whose place the reader can see. Invisible places on the way are skipped
 * when drawing (`drawnWaypoints`).
 */
export function routeDrawnTo(route: Pick<MapPath, 'id' | 'hidden' | 'waypoints'>, state: MapState, visible: (placeId: string) => boolean): number {
	const revealed = routeRevealedTo(route, state);
	if (revealed >= 0 || !route.hidden) return revealed;
	let last = -1;
	(route.waypoints ?? []).forEach((w, i) => {
		if (isPlaceWaypoint(w) && visible(w.location)) last = i;
	});
	return last;
}

// ── The old per-toon answer (older viewer builds: `ToonBundle.locations` / `.routes`, `toon.map`) ──────────────────────

/**
 * The toon's places and routes in the shape of the old `locations:<toon>:<l>` / `routes:<toon>:<l>` documents (one release,
 * then dropped). Every place and route of the toon, hidden ones too (old viewers showed every place; they have no unlock
 * state); a route lists its place waypoints only (free points did not exist), unknown ids included, as before. The server computes the same
 * (`Library::legacyMap`); this one is for tests and tools.
 */
export function legacyToonMap(map: WorldMap, toon: string, locale: string): { locations: MapLocation[]; routes: MapRoute[] } {
	const locations: MapLocation[] = (map.locations ?? [])
		.filter((p) => p.toons?.includes(toon))
		.map((p) => ({
			id: p.id,
			label: mapName(p.name, locale),
			toonId: p.chapter?.toon ?? toon,
			type: 'chapter',
			...(p.chapter ? { chapterNumber: String(p.chapter.number) } : {}),
			image: p.image?.src ?? '',
			coordinates: { x: p.x, y: p.y }
		}));
	const routes: MapRoute[] = (map.routes ?? [])
		.filter((r) => r.toons?.includes(toon))
		.map((r) => ({ id: r.id, title: mapName(r.name, locale), locations: (r.waypoints ?? []).filter(isPlaceWaypoint).map((w) => w.location) }));
	return { locations, routes };
}

/** A CDN-root path relative to a toon's folder (the old `toon.map.layers.terrain` was toon-relative) */
export function mapPathInToonFolder(path: string, toon: string): string {
	return path.startsWith(`${toon}/`) ? path.slice(toon.length + 1) : `../${path}`;
}

/** The old `toon.map` (`{ layers: { terrain }, width, height, center }`) from the world map + the toon's start view */
export function legacyToonMapSettings(map: WorldMap, toon: string, view: ToonMapView | null | undefined): LegacyToonMap {
	const terrain = (map.layers ?? []).find((l) => l.id === 'terrain') ?? map.layers?.[0];
	const center = view?.center ?? centerOf((map.locations ?? []).filter((p) => p.toons?.includes(toon))) ?? { x: map.width / 2, y: map.height / 2 };
	return { layers: { terrain: terrain ? mapPathInToonFolder(terrain.image, toon) : '' }, width: map.width, height: map.height, center: { x: center.x, y: center.y } };
}

// ── Migration (agents/MAP.md §2.6) ─────────────────────────────────────────────────────────────────────────────────

/** A route of the old `routes.json` (the en file spelled `titel`) */
export interface LegacyRouteFile {
	id: string;
	title?: string;
	titel?: string;
	locations: string[];
}

/**
 * The world map from one toon's old map files: `toonMap` = its old `toon.map` (layers.terrain toon-relative, width, height,
 * center), `locations` / `routes` = the per-locale files (`{ de: [...], en: [...] }`). Every value survives: ids,
 * coordinates, labels → `name` per locale (verbatim), `toonId` → `toons`, `chapterNumber` → `chapter`, route
 * `title ?? titel` → `name`, `locations` → place waypoints in order (unknown ids kept – the validation reports them).
 * Returns the world map and the toon's start view (`{ center }`).
 */
export function migrateLegacyMap(input: {
	toon: string;
	toonMap: LegacyToonMap;
	locations: Record<string, MapLocation[]>;
	routes: Record<string, LegacyRouteFile[]>;
	source: string;
}): { map: WorldMap; view: ToonMapView } {
	const { toon, toonMap, source } = input;
	const locales = [source, ...Object.keys(input.locations).filter((l) => l !== source)];
	const terrain = toonMap.layers?.terrain ?? '';
	const image = terrain.startsWith('../') ? terrain.slice(3) : `${toon}/${terrain}`;
	const map: WorldMap = { width: toonMap.width, height: toonMap.height, layers: terrain ? [{ id: 'terrain', image }] : [], locations: [], routes: [] };
	for (const loc of input.locations[source] ?? []) {
		const name: LocaleText = {};
		for (const l of locales) {
			const label = (input.locations[l] ?? []).find((x) => x.id === loc.id)?.label;
			if (typeof label === 'string') name[l] = label;
		}
		const place: MapPlace = { id: loc.id, name, x: loc.coordinates.x, y: loc.coordinates.y, toons: [loc.toonId || toon] };
		if (loc.type === 'chapter' && loc.chapterNumber !== undefined && loc.chapterNumber !== '') place.chapter = { toon: loc.toonId || toon, number: Number(loc.chapterNumber) };
		if (loc.image) place.image = { src: loc.image };
		map.locations.push(place);
	}
	for (const route of input.routes[source] ?? []) {
		const name: LocaleText = {};
		for (const l of [source, ...Object.keys(input.routes).filter((x) => x !== source)]) {
			const r = (input.routes[l] ?? []).find((x) => x.id === route.id);
			const title = r?.title ?? r?.titel;
			if (typeof title === 'string') name[l] = title;
		}
		map.routes.push({ id: route.id, name, toons: [toon], waypoints: (route.locations ?? []).map((location) => ({ location })), style: 'road' });
	}
	return { map, view: { center: { x: toonMap.center.x, y: toonMap.center.y } } };
}

// ── Validation (agents/MAP.md §4.7; the cross-entity checks against events come with M3/M4) ─────────────────────────

export interface MapProblem {
	level: 'error' | 'warning' | 'info';
	path: (string | number)[];
	message: string;
}

const ID = /^[a-z0-9][a-z0-9_-]{0,63}$/;
const isNum = (v: unknown): v is number => typeof v === 'number' && Number.isFinite(v);
const isObj = (v: unknown): v is Record<string, any> => !!v && typeof v === 'object' && !Array.isArray(v);

export interface MapValidateOptions {
	/** known toon ids (unknown ones in `toons` are warnings) */
	toons?: string[];
	/** the languages names should have (missing ones are info) */
	languages?: string[];
}

/** Problems of the world map document (structure, ids, references, positions) */
export function checkWorldMap(map: any, options: MapValidateOptions = {}): MapProblem[] {
	const out: MapProblem[] = [];
	const add = (level: MapProblem['level'], path: MapProblem['path'], message: string) => out.push({ level, path, message });
	if (!isObj(map)) return [{ level: 'error', path: [], message: 'the map must be an object { width, height, layers, locations, routes }' }];
	for (const key of ['width', 'height'] as const) if (!isNum(map[key]) || map[key] <= 0) add('error', [key], `${key} must be a positive number (world units = px of the terrain)`);
	const inside = (x: unknown, y: unknown) => isNum(x) && isNum(y) && x >= 0 && y >= 0 && (!isNum(map.width) || x <= map.width) && (!isNum(map.height) || y <= map.height);
	const names = (name: unknown, path: MapProblem['path'], what: string, required: boolean) => {
		if (name === undefined && !required) return;
		if (!isObj(name) || !Object.values(name).some((v) => typeof v === 'string' && v.trim())) {
			add(required ? 'error' : 'warning', path, `${what}: the name is empty`);
			return;
		}
		for (const l of options.languages ?? []) if (!(typeof name[l] === 'string' && name[l].trim())) add('info', [...path, l], `${what}: no ${l} name (the reader shows ${MAP_SOURCE_LANGUAGE})`);
	};
	const toonList = (toons: unknown, path: MapProblem['path'], what: string) => {
		if (toons === undefined || (Array.isArray(toons) && !toons.length)) return add('warning', path, `${what} is used by no toon (the reader shows it to nobody)`);
		if (!Array.isArray(toons) || !toons.every((t) => typeof t === 'string')) return add('error', path, `${what}: toons must be a list of toon ids`);
		if (options.toons) for (const t of toons) if (!options.toons.includes(t)) add('warning', path, `${what}: unknown toon "${t}"`);
	};

	const layers = Array.isArray(map.layers) ? map.layers : [];
	if (!Array.isArray(map.layers)) add('error', ['layers'], 'layers must be a list');
	const layerIds = new Set<string>();
	layers.forEach((l: any, i: number) => {
		if (!isObj(l)) return add('error', ['layers', i], 'a layer must be { id, image }');
		if (typeof l.id !== 'string' || !ID.test(l.id)) add('error', ['layers', i, 'id'], `layer ${i + 1}: the id must be a slug (a–z, 0–9, -)`);
		else if (layerIds.has(l.id)) add('error', ['layers', i, 'id'], `duplicate layer id "${l.id}"`);
		else layerIds.add(l.id);
		if (typeof l.image !== 'string' || !l.image) add('error', ['layers', i, 'image'], `layer ${l.id ?? i + 1}: no image`);
		else if (!/\.(svg|webp|png|jpe?g)$/i.test(l.image)) add('warning', ['layers', i, 'image'], `layer ${l.id}: not an image path`);
		for (const z of ['minZoom', 'maxZoom'] as const) if (l[z] !== undefined && (!isNum(l[z]) || l[z] <= 0)) add('error', ['layers', i, z], `layer ${l.id}: ${z} must be a positive number`);
	});

	const places = Array.isArray(map.locations) ? map.locations : [];
	if (!Array.isArray(map.locations)) add('error', ['locations'], 'locations must be a list');
	const placeIds = new Set<string>();
	places.forEach((p: any, i: number) => {
		const path = ['locations', i];
		if (!isObj(p)) return add('error', path, 'a place must be an object');
		const what = `place "${p.id ?? i + 1}"`;
		if (typeof p.id !== 'string' || !ID.test(p.id)) add('error', [...path, 'id'], `${what}: the id must be a slug (a–z, 0–9, -)`);
		else if (placeIds.has(p.id)) add('error', [...path, 'id'], `duplicate place id "${p.id}"`);
		else placeIds.add(p.id);
		names(p.name, [...path, 'name'], what, true);
		if (!isNum(p.x) || !isNum(p.y)) add('error', [...path, 'x'], `${what}: x and y must be numbers`);
		else if (!inside(p.x, p.y)) add('error', [...path, 'x'], `${what}: the position (${p.x}, ${p.y}) is outside the map`);
		toonList(p.toons, [...path, 'toons'], what);
		if (p.hidden !== undefined && typeof p.hidden !== 'boolean') add('error', [...path, 'hidden'], `${what}: hidden must be true or false`);
		if (p.unlock !== undefined) {
			const u = p.unlock;
			if (!isObj(u) || typeof u.toon !== 'string' || !Number.isInteger(u.chapter) || u.chapter < 0) add('error', [...path, 'unlock'], `${what}: unlock must be { toon, chapter, at?, scene? }`);
			else {
				if (u.at !== undefined && (!isNum(u.at) || u.at < 0 || u.at > 1)) add('error', [...path, 'unlock', 'at'], `${what}: unlock.at is a reading position 0..1`);
				if (u.scene !== undefined && (!Number.isInteger(u.scene) || u.scene < 0)) add('error', [...path, 'unlock', 'scene'], `${what}: unlock.scene is a screenplay scene number`);
				if (Array.isArray(p.toons) && !p.toons.includes(u.toon)) add('warning', [...path, 'unlock'], `${what}: unlocked in ${u.toon}, which is not in its toons`);
				if (!p.hidden) add('info', [...path, 'unlock'], `${what}: visible from the start – the unlock point does nothing`);
			}
		} else if (p.hidden === true) add('warning', [...path, 'hidden'], `${what}: hidden, and nothing unlocks it yet (no unlock point; story events come later)`);
		if (p.chapter !== undefined) {
			if (!isObj(p.chapter) || typeof p.chapter.toon !== 'string' || !Number.isInteger(p.chapter.number) || p.chapter.number < 0)
				add('error', [...path, 'chapter'], `${what}: the chapter link must be { toon, number }`);
			else if (Array.isArray(p.toons) && !p.toons.includes(p.chapter.toon)) add('warning', [...path, 'chapter'], `${what}: links a chapter of ${p.chapter.toon}, which is not in its toons`);
		}
		if (p.image !== undefined) {
			if (!isObj(p.image) || typeof p.image.src !== 'string' || !p.image.src) add('error', [...path, 'image'], `${what}: the image needs a path`);
			else {
				if (!/\.(webp|png|svg)$/i.test(p.image.src)) add('warning', [...path, 'image', 'src'], `${what}: the image should be a transparent WebP`);
				if (p.image.size !== undefined && (!isNum(p.image.size) || p.image.size <= 0)) add('error', [...path, 'image', 'size'], `${what}: the image size must be a positive number (px at zoom 1)`);
				const a = p.image.anchor;
				if (a !== undefined && (!isObj(a) || !isNum(a.x) || !isNum(a.y) || a.x < 0 || a.x > 100 || a.y < 0 || a.y > 100))
					add('error', [...path, 'image', 'anchor'], `${what}: the ground point must be { x, y } in % (0–100)`);
			}
		}
	});

	const routes = Array.isArray(map.routes) ? map.routes : [];
	if (!Array.isArray(map.routes)) add('error', ['routes'], 'routes must be a list');
	const routeIds = new Set<string>();
	routes.forEach((r: any, i: number) => {
		const path = ['routes', i];
		if (!isObj(r)) return add('error', path, 'a route must be an object');
		const what = `route "${r.id ?? i + 1}"`;
		if (typeof r.id !== 'string' || !ID.test(r.id)) add('error', [...path, 'id'], `${what}: the id must be a slug (a–z, 0–9, -)`);
		else if (routeIds.has(r.id)) add('error', [...path, 'id'], `duplicate route id "${r.id}"`);
		else routeIds.add(r.id);
		names(r.name, [...path, 'name'], what, true);
		toonList(r.toons, [...path, 'toons'], what);
		if (r.style !== undefined && !MAP_STYLES.includes(r.style)) add('error', [...path, 'style'], `${what}: style must be one of ${MAP_STYLES.join(', ')}`);
		if (r.hidden !== undefined && typeof r.hidden !== 'boolean') add('error', [...path, 'hidden'], `${what}: hidden must be true or false`);
		const waypoints = Array.isArray(r.waypoints) ? r.waypoints : [];
		if (!Array.isArray(r.waypoints) || waypoints.length < 2) add('error', [...path, 'waypoints'], `${what}: a route needs at least 2 waypoints`);
		const free = new Set<string>();
		waypoints.forEach((w: any, j: number) => {
			const wp = [...path, 'waypoints', j];
			if (isPlaceWaypoint(w)) {
				if (!placeIds.has(w.location)) add('error', wp, `${what}: waypoint ${j + 1} is an unknown place "${w.location}"`);
			} else if (isObj(w)) {
				if (typeof w.id !== 'string' || !ID.test(w.id)) add('error', [...wp, 'id'], `${what}: waypoint ${j + 1} needs an id (a slug)`);
				else if (free.has(w.id) || placeIds.has(w.id)) add('error', [...wp, 'id'], `${what}: duplicate waypoint id "${w.id}"`);
				else free.add(w.id);
				if (!inside(w.x, w.y)) add('error', [...wp, 'x'], `${what}: waypoint ${w.id ?? j + 1} is outside the map`);
				names(w.name, [...wp, 'name'], `${what} waypoint ${w.id ?? j + 1}`, false);
			} else add('error', wp, `${what}: waypoint ${j + 1} must be { location } or { id, x, y }`);
		});
	});
	return out;
}
