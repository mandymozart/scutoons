// Where the reader's world map opens (owner 2026-10-10: "Die Karte bitte auf der Zoom-Stufe starten. An der Position.
// Immer beim ersten Öffnen da, wo man gerade ist. Danach merkt sie sich den State der letzten Interaction. Außer es wird
// explicit ein neuer Focus und Frame gesendet als Event. Erst einmal aber auch beim Erreichen eines neuen Checkpoints."):
//
// - first opening of a toon's map (on this device): centred on where the reader is (`readerHere`) – the place's own view
//   (owner 2026-10-10: "pro Location einen Zoom … als Mittelpunkt die Location selber, gegebenenfalls noch mit einem
//   Offset"; set in the creator's map builder), without one its point at zoom 2 ("Orte ohne diese Angaben: Mindeststufe 2
//   und Zoomstufe 2 und der Punkt" – `placeView`, never below 2); nothing reached yet: the toon's start view;
// - after that: the view the reader left it at (centre + zoom, stored per toon in localStorage `scutoons_map_view`);
// - except (a) an explicit focus (`focusMap` – for the story's focusMap event, agents/MAP.md §3): its frame wins on the
//   next opening and becomes the remembered view; (b) the reader reached a new place (a new "here"): the next opening
//   centres on it (its view) – once; moving the map again is remembered.
// Pure decisions (`readerHere`, `openingView`) + small storage helpers (try/catch: a blocked storage just forgets).
import { PLACE_ZOOM, placeView, type MapPlace, type MapState, type Point, type ReaderPlace } from '@scutoons/content';

export const MAP_VIEW_KEY = 'scutoons_map_view';

/** where the reader is on a toon's map: a place id and how the map frames it (`placeView`: point + offset, zoom ≥ 2) */
export interface MapHere {
	id: string;
	/** the view centre (the place's point + its view offset) */
	x: number;
	y: number;
	zoom: number;
}

/** what is remembered per toon */
export interface StoredMapView {
	/** the world point at the screen's centre */
	x: number;
	y: number;
	zoom: number;
	/** the "here" this view already knows (a different one = a new place reached → centre on it once) */
	here?: string | null;
	/** an explicit focus waiting for the next opening (its frame wins once) */
	focus?: true;
}

/**
 * Where the reader is on a toon's map: the toon's position (set by the story, `positions[toon]`) when it is a place on
 * the map; else the place of this toon the reader reached last – the most recently unlocked one, ties (several unlocked
 * at once, e.g. from the reading progress) by story order (the unlock's chapter, then its reading point). Null: nothing
 * reached yet.
 */
export function readerHere(places: readonly ReaderPlace[], toon: string, state: MapState): MapHere | null {
	const at = (p: ReaderPlace | undefined): MapHere | null => {
		if (!p || !Number.isFinite(p.x) || !Number.isFinite(p.y)) return null;
		const { center, zoom } = placeView(p);
		return { id: p.id, ...center, zoom };
	};
	const position = state.positions?.[toon];
	if (position) {
		const id = 'location' in position ? position.location : position.waypoint;
		const placed = at(places.find((p) => p.id === id));
		if (placed) return placed;
	}
	let best: ReaderPlace | undefined;
	let bestKey: [number, number, number] = [-Infinity, -Infinity, -Infinity];
	for (const p of places) {
		const unlocked = state.places?.[p.id]?.unlocked;
		if (unlocked === undefined || !p.toons?.includes(toon)) continue;
		const u = p.unlock && p.unlock.toon === toon ? p.unlock : null;
		const key: [number, number, number] = [unlocked, u ? u.chapter : -1, u ? (u.at ?? 0) : -1];
		if (key[0] > bestKey[0] || (key[0] === bestKey[0] && (key[1] > bestKey[1] || (key[1] === bestKey[1] && key[2] > bestKey[2])))) {
			best = p;
			bestKey = key;
		}
	}
	return at(best);
}

export type OpeningView =
	| { kind: 'focus' | 'here' | 'stored'; center: Point; zoom: number; /** what to remember now */ store: StoredMapView }
	/** nothing known: the toon's start view (map-overlay.svelte `startView`) */
	| { kind: 'start'; store: null };

/**
 * The view a toon's map opens with: an explicit focus waiting → it; a new "here" (or the first opening with one) → its
 * view (centre + zoom); else the remembered view; else the start view. `store` = the record to keep (the "here" it knows).
 */
export function openingView(stored: StoredMapView | null, here: MapHere | null): OpeningView {
	const hereId = here?.id ?? null;
	if (stored?.focus) return { kind: 'focus', center: { x: stored.x, y: stored.y }, zoom: stored.zoom, store: { x: stored.x, y: stored.y, zoom: stored.zoom, here: hereId } };
	if (here && (!stored || stored.here !== here.id)) return { kind: 'here', center: { x: here.x, y: here.y }, zoom: here.zoom, store: { x: here.x, y: here.y, zoom: here.zoom, here: here.id } };
	if (stored) return { kind: 'stored', center: { x: stored.x, y: stored.y }, zoom: stored.zoom, store: { ...stored, here: hereId } };
	return { kind: 'start', store: null };
}

/** a stored value as a view (junk → null) */
export function parseStoredView(value: unknown): StoredMapView | null {
	const v = value as Partial<StoredMapView> | null;
	if (!v || typeof v !== 'object' || ![v.x, v.y, v.zoom].every((n) => typeof n === 'number' && Number.isFinite(n)) || !(v.zoom! > 0)) return null;
	return { x: v.x!, y: v.y!, zoom: v.zoom!, ...(typeof v.here === 'string' ? { here: v.here } : {}), ...(v.focus === true ? { focus: true as const } : {}) };
}

// ── storage (per toon, this device) ──────────────────────────────────────────────────────────────────────────────

function readAll(): Record<string, unknown> {
	try {
		const raw = globalThis.localStorage?.getItem(MAP_VIEW_KEY);
		const all = raw ? JSON.parse(raw) : {};
		return all && typeof all === 'object' && !Array.isArray(all) ? all : {};
	} catch {
		return {};
	}
}

export function loadMapView(toon: string): StoredMapView | null {
	return parseStoredView(readAll()[toon]);
}

export function saveMapView(toon: string, view: StoredMapView): void {
	try {
		const all = readAll();
		all[toon] = view;
		globalThis.localStorage?.setItem(MAP_VIEW_KEY, JSON.stringify(all));
	} catch {
		/* storage blocked or full: the map just forgets */
	}
}

/**
 * An explicit focus for a toon's map (the story's focusMap event, agents/MAP.md §3 – M3 calls this, then opens the map):
 * the next opening shows this frame and remembers it. `target`: a world point, or a place (`{ id, x, y, view? }` – framed by
 * its view: point + offset, its zoom ≥ 2). `zoom` (explicit) wins; else a place's zoom; else the remembered zoom, else
 * `PLACE_ZOOM`. Listeners (an open map) apply it at once.
 */
export function focusMap(toon: string, target: Point | Pick<MapPlace, 'id' | 'x' | 'y' | 'view'>, zoom?: number): void {
	const before = loadMapView(toon);
	const place = 'id' in target ? placeView(target) : null;
	const center = place?.center ?? target;
	const z = typeof zoom === 'number' && Number.isFinite(zoom) && zoom > 0 ? zoom : (place?.zoom ?? before?.zoom ?? PLACE_ZOOM);
	saveMapView(toon, { x: center.x, y: center.y, zoom: z, here: before?.here ?? null, focus: true });
	globalThis.dispatchEvent?.(new CustomEvent(MAP_FOCUS_EVENT, { detail: { toon } }));
}
export const MAP_FOCUS_EVENT = 'scutoons:map-focus';
