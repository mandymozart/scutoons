// The SCUTOONS world map (agents/MAP.md): ONE map for the whole universe (entity `map`, platform-wide, language-neutral,
// names as `{ de, en }` inside), the reader's map state (unlocked places, revealed routes, positions – world-wide game
// state under the reserved id `scutoons`), the old per-toon answer shape for older viewer builds, the migration from
// catpope's `locations.json` / `routes.json`, and the validation. Pure: no Svelte, no DOM.
import type { LocaleText } from './credits.ts';
import type { AssetRequest } from './characters.ts';
import type { Map as LegacyToonMap, MapLocation, MapRoute } from './content.ts';
import { toonAsset } from './paths.ts';
import type { AcceptedWarnings } from './accepted.ts';

// ── The entity `map` ────────────────────────────────────────────────────────────────────────────────────────────

/** The SCUTOONS world map (entity `map`, seed `scutoons/_/map.json`) */
export interface WorldMap extends AcceptedWarnings {
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
	/** CDN-root path, e.g. `catpope/map/map-layer-terrain.svg` */
	image: string;
	/** shown only from / up to this zoom (e.g. a street-detail layer for the city) */
	minZoom?: number;
	maxZoom?: number;
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
	/**
	 * hidden until the story's `unlockLocation` event unlocks it – default: visible from the start (owner 2026-10-11: places
	 * are unlocked by events only; the old slice / chapter link `unlock` went with migration 2026-10-11-unlock-location-events)
	 */
	hidden?: boolean;
	/** the chapter the place's card links to */
	chapter?: { toon: string; number: number };
	/**
	 * how the reader's map frames the place when it centres on it (owner 2026-10-10: "pro Location einen Zoom festlegen …
	 * als Mittelpunkt die Location selber, gegebenenfalls noch mit einem Offset"; set in the map builder). Without it: the
	 * place's point at `PLACE_ZOOM` (`placeView`).
	 */
	view?: PlaceView;
}

/** A place's own framing on the reader's map (agents/MAP.md §8.4) */
export interface PlaceView {
	/** the zoom when the map centres on the place, `MAP_MIN_ZOOM`..`MAP_MAX_ZOOM` (default `PLACE_ZOOM` – owner 2026-10-11: "kleiner geht schon") */
	zoom?: number;
	/** the view centre's shift from the place's point, world units (default none: the place itself is the centre) */
	offset?: { x: number; y: number };
}

/** A slice of a chapter in reading order: groups (scenes inlined) → slices */
export interface ChapterSliceRef {
	image: string;
	/** index of the group in the chapter */
	group: number;
	/** index of the slice in its group */
	index: number;
	/** index among all slices of the chapter */
	overall: number;
}

/** Every slice of a chapter in reading order (`groups` with their scenes inlined, as the reader gets them) */
export function chapterSlices(groups: readonly { slices?: readonly { image?: string }[] }[] | null | undefined): ChapterSliceRef[] {
	const out: ChapterSliceRef[] = [];
	(groups ?? []).forEach((g, group) => (g?.slices ?? []).forEach((s, index) => out.push({ image: s?.image ?? '', group, index, overall: out.length })));
	return out;
}

export interface MapImage {
	/** CDN-root path, e.g. `map/locations/cathedral.webp` */
	src: string;
	/** drawn width in screen px at zoom 1 (default 48); the image keeps its ratio */
	size?: number;
	/** the ground point inside the image, % of its box (default 50 / 90 = bottom centre) – sits on the place's (x, y) */
	anchor?: { x: number; y: number };
	/**
	 * the same diorama as the unpainted plaster model (owner 2026-10-11: "zwei Zustände – als Gipsmodell und als bemaltes
	 * fertiges Diorama"): the unlock reveal fades from it to the painted `src` (agents/MAP.md §8.5a). CDN-root path, the
	 * same framing as `src` (size and ground point are shared)
	 */
	plaster?: string;
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
/** the zoom when the map centres on a place – its default and its minimum (owner 2026-10-10: "Mindeststufe 2 und Zoomstufe 2") */
export const PLACE_ZOOM = 2;
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
	/**
	 * with `?toon=`: where the map opens for that toon (its start view, else the centre of its places) and `color` = the sea's
	 * colour (owner 2026-10-01: the toon's colour – its tile colour, else its star's registry colour, `toonTileColor`)
	 */
	view?: { center: { x: number; y: number }; zoom: number; color?: string };
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

/**
 * Where the reader's map centres on a place, and at which zoom: the place's point + its `view.offset`, at its `view.zoom`
 * clamped to `MAP_MIN_ZOOM`..`MAP_MAX_ZOOM` (default `PLACE_ZOOM` = 2; a smaller own zoom is fine). Junk values count as unset.
 */
export function placeView(place: Pick<MapPlace, 'x' | 'y' | 'view'>): { center: Point; zoom: number } {
	const v = place.view;
	const o = v?.offset;
	const shift = o && Number.isFinite(o.x) && Number.isFinite(o.y) ? o : { x: 0, y: 0 };
	const z = v?.zoom;
	const zoom = typeof z === 'number' && Number.isFinite(z) ? Math.min(MAP_MAX_ZOOM, Math.max(MAP_MIN_ZOOM, z)) : PLACE_ZOOM;
	return { center: { x: place.x + shift.x, y: place.y + shift.y }, zoom };
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

/**
 * How a route is drawn: one look for every route (owner 2026-10-01: the mode of transport is not part of the story;
 * a stored legacy `style` is ignored)
 */
export function routeStroke(_style?: MapPathStyle): { width: number; dash: string; color: string; outline?: string; tension: number; bow: number } {
	return { width: 6, dash: '', color: '#000', tension: 0.3, bow: 0 };
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
 * first wins), positions in screen px (`y` = the label's bottom), an approximate label box per item; labels never cover
 * the obstacles (the markers). Returns the ids to label.
 */
export function visibleLabels(
	items: readonly { id: string; x: number; y: number; width: number; height: number }[],
	/** boxes no label may cover (the markers): `{ x, y }` = centre, `size` = width = height */
	obstacles: readonly { x: number; y: number; size: number }[] = []
): Set<string> {
	const shown: { x0: number; y0: number; x1: number; y1: number }[] = obstacles.map((o) => ({ x0: o.x - o.size / 2, y0: o.y - o.size / 2, x1: o.x + o.size / 2, y1: o.y + o.size / 2 }));
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
/** A route of the map document or of the reader's answer */
export type RouteLike = { id: string; hidden?: boolean; waypoints: readonly (Waypoint | ReaderWaypoint)[] };

/** Up to which waypoint index a route is drawn (-1 = not at all) */
export function routeRevealedTo(route: RouteLike, state: MapState): number {
	const last = (route.waypoints?.length ?? 0) - 1;
	if (!route.hidden) return last;
	const r = state.routes[route.id];
	return r ? Math.min(r.revealed, last) : -1;
}

/** A place unlocked but not seen on the opened map yet (the cyan halo, the toggle's badge) */
export const isPlaceNew = (id: string, state: MapState): boolean => !!state.places[id] && state.places[id].seen === undefined;

/**
 * Up to which waypoint a route is drawn for a reader: a visible route entirely; a hidden one up to its revealed index, else
 * (before the events, M3) up to its last waypoint whose place the reader can see. Invisible places on the way are skipped
 * when drawing (`drawnWaypoints`).
 */
export function routeDrawnTo(route: RouteLike, state: MapState, visible: (placeId: string) => boolean): number {
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
	const image = toonAsset(toon, terrain);
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
		map.routes.push({ id: route.id, name, toons: [toon], waypoints: (route.locations ?? []).map((location) => ({ location })) });
	}
	return { map, view: { center: { x: toonMap.center.x, y: toonMap.center.y } } };
}

// ── Wanted images: every place without an image (agents/MAP.md §5, owner 2026-10-01) ─────────────────────────────────

const NL = String.fromCharCode(10);

/** The place illustration spec (MAP.md §5.1) */
export const PLACE_IMAGE_SPEC = '256 × 256 px transparent WebP, ≤ 40 KB (60 KB at most), ground point 50 % / 90 % (bottom centre), no text, no characters';
export const PLACE_IMAGE_SIZE = '256 × 256';

/** Descriptions of catpope's first places for the brief (MAP.md §5.4); others use their names */
const PLACE_DESCRIPTIONS: Record<string, string> = {
	cathedral: 'a white baroque cathedral with two towers and cobalt-blue ornaments on the roofs, a small stone square in front',
	apartment: 'a narrow five-storey old-town apartment house, pale plaster, one lit window, a tiny balcony with plants',
	airport: 'a small modern airport terminal with a control tower and a short runway strip with one parked white jet',
	'medival-street': 'a short medieval cobblestone alley with half-timbered houses and two hanging lanterns',
	station: 'a 19th-century railway station hall with an arched glass roof and a clock above the entrance',
	'mountain-path': 'a winding stony path up a steep green mountain slope with a wooden signpost without text',
	monastery: 'a remote mountain monastery with a bell tower and terraced gardens on a rocky ledge',
	library: 'an old monastery library building with tall arched windows and stacks of books visible inside',
	graveyard: 'a small walled graveyard with crooked headstones, a cypress tree and a wrought-iron gate'
};

/** The brief of a place illustration, for an image generator or a designer (MAP.md §5.2; attach the style references listed there) */
export function placeImageBrief(place: Pick<MapPlace, 'id' | 'name'>): string {
	const description = PLACE_DESCRIPTIONS[place.id] ?? `${mapName(place.name, 'en')} (${mapName(place.name, MAP_SOURCE_LANGUAGE)})`;
	return [
		'Attached are images from the SCUTOONS universe as STYLE references only (rendering, lighting, materials, colour palette). Do not include the characters.',
		`Create a single small map illustration of a place in exactly that rendering style: ${description}.`,
		'It is a tiny self-contained diorama for a world map: three-quarter view from above (about 30 degrees), the place sits on a small irregular patch of its own ground, nothing else around it.',
		'Soft realistic 3D render, same light direction and softness as the references, slightly simplified shapes so it reads at 48 px. Centred horizontally, the point where the place meets the ground at 90 % of the image height, ~6 % padding left and right, 1024 x 1024 px, transparent background, no text, no letters on signs, no logo, no people or animals, no shadow outside the ground patch.'
	].join(NL);
}

/** The CDN path a place's illustration should get: `<toon>/map/places/<id>.webp` (`scutoons/…` when it has no single toon) */
export function placeImagePath(place: Pick<MapPlace, 'id' | 'toons'>): string {
	const folder = place.toons?.length === 1 ? place.toons[0] : 'scutoons';
	return `${folder}/map/places/${place.id}.webp`;
}

export interface MapImageRequest {
	/** the place */
	place: string;
	name: LocaleText;
	/** the toon whose folder the file goes to (`scutoons` for a shared place) */
	toon: string;
	/** the target path from the CDN root */
	path: string;
	size: string;
	spec: string;
	brief: string;
}

/**
 * The wanted place illustrations, derived from the map (never a hand-kept list, so it can't drift): every place without
 * an `image`, in the map's order. Shown in the creator's "Wanted images" as "Map places".
 */
export function mapImageRequests(map: Pick<WorldMap, 'locations'> | null | undefined): MapImageRequest[] {
	return (map?.locations ?? [])
		.filter((p) => !p.image?.src)
		.map((p) => ({ place: p.id, name: p.name, toon: p.toons?.length === 1 ? p.toons[0] : 'scutoons', path: placeImagePath(p), size: PLACE_IMAGE_SIZE, spec: PLACE_IMAGE_SPEC, brief: placeImageBrief(p) }));
}

/** The prefix of the derived map-place requests' ids (never stored: `mapMediaRequests`) */
export const MAP_PLACE_REQUEST_PREFIX = 'map-place:';

/**
 * The wanted place illustrations as toon-media asset requests (`view: 'media'`, `toon`, `size`; owner 2026-10-01): shown in
 * the creator's "Wanted images" next to the stored media requests, derived from the map each time (never stored, so the
 * list can't drift); `note` = "Map place: <name> · spec", `brief` = the full brief.
 */
export function mapMediaRequests(map: Pick<WorldMap, 'locations'> | null | undefined, locale = MAP_SOURCE_LANGUAGE): AssetRequest[] {
	return mapImageRequests(map).map((r) => ({
		id: `${MAP_PLACE_REQUEST_PREFIX}${r.place}`,
		character: '',
		persona: '',
		look: '',
		mood: '',
		view: 'media',
		path: r.path,
		toon: r.toon,
		size: r.size,
		note: `${mapName(r.name, locale) || r.place} · ${r.spec}`,
		brief: r.brief,
		requested_by: null,
		requested_at: '',
		status: 'open'
	}));
}

/** Text for the image supplier: one block per place (name, path, spec, brief) */
export function mapImageRequestsText(requests: MapImageRequest[]): string {
	return requests
		.map((r) => [`${mapName(r.name, MAP_SOURCE_LANGUAGE)} / ${mapName(r.name, 'en')} (${r.place})`, `  ${r.path}  ${r.size}`, `  ${r.spec}`, ...r.brief.split(NL).map((l) => `  ${l}`)].join(NL))
		.join(NL + NL);
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
		// the old slice / chapter link (owner 2026-10-11: places are unlocked by the unlockLocation event only)
		if (p.unlock !== undefined) add('error', [...path, 'unlock'], `${what}: \`unlock\` is gone – an unlockLocation event on a slice or in a script unlocks the place (migration 2026-10-11-unlock-location-events)`);
		if (p.chapter !== undefined) {
			if (!isObj(p.chapter) || typeof p.chapter.toon !== 'string' || !Number.isInteger(p.chapter.number) || p.chapter.number < 0)
				add('error', [...path, 'chapter'], `${what}: the chapter link must be { toon, number }`);
			else if (Array.isArray(p.toons) && !p.toons.includes(p.chapter.toon)) add('warning', [...path, 'chapter'], `${what}: links a chapter of ${p.chapter.toon}, which is not in its toons`);
		}
		if (p.view !== undefined) {
			const v = p.view;
			if (!isObj(v)) add('error', [...path, 'view'], `${what}: the view must be { zoom?, offset? }`);
			else {
				if (v.zoom !== undefined && (!isNum(v.zoom) || v.zoom < MAP_MIN_ZOOM || v.zoom > MAP_MAX_ZOOM))
					add('error', [...path, 'view', 'zoom'], `${what}: the zoom must be a number from ${MAP_MIN_ZOOM * 100} % to ${MAP_MAX_ZOOM * 100} % (${MAP_MIN_ZOOM}–${MAP_MAX_ZOOM})`);
				const o = v.offset;
				if (o !== undefined && (!isObj(o) || !isNum(o.x) || !isNum(o.y))) add('error', [...path, 'view', 'offset'], `${what}: the view offset must be { x, y } in world units`);
				else if (o !== undefined && isNum(p.x) && isNum(p.y) && !inside(p.x + o.x, p.y + o.y))
					add('warning', [...path, 'view', 'offset'], `${what}: the view centre (${p.x + o.x}, ${p.y + o.y}) is outside the map`);
			}
		}
		if (p.image !== undefined) {
			if (!isObj(p.image) || typeof p.image.src !== 'string' || !p.image.src) add('error', [...path, 'image'], `${what}: the image needs a path`);
			else {
				if (!/\.(webp|png|svg)$/i.test(p.image.src)) add('warning', [...path, 'image', 'src'], `${what}: the image should be a transparent WebP`);
				if (p.image.plaster !== undefined && (typeof p.image.plaster !== 'string' || !p.image.plaster)) add('error', [...path, 'image', 'plaster'], `${what}: the plaster model needs a path (or none)`);
				else if (typeof p.image.plaster === 'string' && !/\.(webp|png|svg)$/i.test(p.image.plaster)) add('warning', [...path, 'image', 'plaster'], `${what}: the plaster model should be a transparent WebP`);
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
