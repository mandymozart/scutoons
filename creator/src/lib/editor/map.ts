// The map builder's model (agents/MAP.md §4): the world map entity `map` as one language-neutral ContentDoc (saved /
// published through the page's ActionBar like every entity), and the edits the builder makes – each one an undo step.
import {
	MAP_SOURCE_LANGUAGE,
	emptyWorldMap,
	entities,
	isPlaceWaypoint,
	slugId,
	uniqueId,
	waypointId,
	type FreeWaypoint,
	type MapPath,
	type MapPlace,
	type Point,
	type WorldMap
} from '@scutoons/content';
import { CDN_SERVER } from '$lib/config.ts';
import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
import { ensure, edit } from '$lib/editor/characters.ts';
import { neutralDoc } from '$lib/editor/languages.ts';
import { getDraft } from '$lib/api.ts';

const N = '_';

export const mapDoc = () => neutralDoc(entities.map);

/** the world map's places (id + name) for the event forms – the newest draft, loaded once per page visit */
let placesOnce: Promise<{ id: string; name: unknown }[]> | null = null;
export function mapPlaces(): Promise<{ id: string; name: unknown }[]> {
	placesOnce ??= getDraft('map')
		.then((r) => ((r.content as WorldMap | null)?.locations ?? []).map((p) => ({ id: p.id, name: p.name })))
		.catch(() => {
			placesOnce = null;
			return [];
		});
	return placesOnce;
}
export const mapOf = (doc: ContentDoc | null | undefined) => (doc?.data[N] ?? null) as WorldMap | null;
export const ensureMap = (doc: ContentDoc) => ensure(doc, emptyWorldMap);
export const editMap = (doc: ContentDoc, change: (map: WorldMap) => void, key?: string) => edit<WorldMap>(doc, change, key);

/** every id of the map's places, routes, layers (new ids must be unique among their kind) */
const ids = (list: { id: string }[] | undefined) => (list ?? []).map((x) => x.id);

/** A new place at a world point; the id comes from the name later (editable until the first publish) */
export function addPlace(doc: ContentDoc, at: Point, toons: string[]): string {
	let id = '';
	editMap(doc, (map) => {
		id = uniqueId('ort', ids(map.locations));
		map.locations.push({ id, name: { [MAP_SOURCE_LANGUAGE]: '' }, x: Math.round(at.x), y: Math.round(at.y), toons: [...toons] });
	});
	return id;
}

export function editPlace(doc: ContentDoc, id: string, change: (place: MapPlace) => void, key?: string) {
	editMap(
		doc,
		(map) => {
			const place = map.locations.find((p) => p.id === id);
			if (place) change(place);
		},
		key
	);
}

/** Renames a place id everywhere in the map (its route waypoints); returns the new id */
export function renamePlace(doc: ContentDoc, from: string, wanted: string, key?: string): string {
	let to = from;
	editMap(doc, (map) => {
		const slug = slugId(wanted) || from;
		to = slug === from ? from : uniqueId(slug, ids(map.locations));
		if (to === from) return;
		const place = map.locations.find((p) => p.id === from);
		if (!place) return;
		place.id = to;
		for (const route of map.routes) for (const w of route.waypoints) if (isPlaceWaypoint(w) && w.location === from) w.location = to;
	}, key);
	return to;
}

/** The routes that pass a place (a place on a route can't be deleted) */
export const routesThrough = (map: WorldMap | null, id: string) =>
	(map?.routes ?? []).filter((r) => r.waypoints.some((w) => isPlaceWaypoint(w) && w.location === id)).map((r) => r.id);

/**
 * A slice unlocks a place (agents/MAP.md, owner 2026-10-01: edited from the map builder AND from the chapter's slice – one
 * storage, the place's `unlock`): hidden, unlocked by that chapter's slice (its image) at the slice's reading point.
 * `null` removes the slice's unlock (the place stays hidden; the builder can set another point).
 */
export function setSliceUnlock(doc: ContentDoc, placeId: string, unlock: { toon: string; chapter: number; slice: string; at: number } | null) {
	editPlace(doc, placeId, (p) => {
		if (!unlock) {
			delete p.unlock;
			return;
		}
		p.hidden = true;
		const scene = p.unlock?.toon === unlock.toon && p.unlock.chapter === unlock.chapter ? p.unlock.scene : undefined;
		p.unlock = { toon: unlock.toon, chapter: unlock.chapter, at: unlock.at, slice: unlock.slice, ...(scene !== undefined ? { scene } : {}) };
	});
}

export function removePlace(doc: ContentDoc, id: string) {
	editMap(doc, (map) => {
		map.locations = map.locations.filter((p) => p.id !== id);
	});
}

/** A new route from waypoints (drawn on the canvas) */
export function addRoute(doc: ContentDoc, waypoints: MapPath['waypoints'], toons: string[]): string {
	let id = '';
	editMap(doc, (map) => {
		id = uniqueId('weg', ids(map.routes));
		map.routes.push({ id, name: { [MAP_SOURCE_LANGUAGE]: '' }, toons: [...toons], waypoints: JSON.parse(JSON.stringify(waypoints)), style: 'road' });
	});
	return id;
}

export function editRoute(doc: ContentDoc, id: string, change: (route: MapPath) => void, key?: string) {
	editMap(
		doc,
		(map) => {
			const route = map.routes.find((r) => r.id === id);
			if (route) change(route);
		},
		key
	);
}

export function renameRoute(doc: ContentDoc, from: string, wanted: string, key?: string): string {
	let to = from;
	editMap(doc, (map) => {
		const slug = slugId(wanted) || from;
		to = slug === from ? from : uniqueId(slug, ids(map.routes));
		const route = map.routes.find((r) => r.id === from);
		if (route) route.id = to;
	}, key);
	return to;
}

export function removeRoute(doc: ContentDoc, id: string) {
	editMap(doc, (map) => {
		map.routes = map.routes.filter((r) => r.id !== id);
	});
}

/** A free waypoint id unique within the route (`wp-1`, `wp-2` …) */
export function freeWaypointId(waypoints: MapPath['waypoints'], places: string[] = []): string {
	const taken = new Set([...waypoints.map(waypointId), ...places]);
	for (let n = 1; ; n++) if (!taken.has(`wp-${n}`)) return `wp-${n}`;
}

/** Inserts a free waypoint after index `after` */
export function insertWaypoint(doc: ContentDoc, route: string, after: number, at: Point) {
	editRoute(doc, route, (r) => {
		const wp: FreeWaypoint = { id: freeWaypointId(r.waypoints), x: Math.round(at.x), y: Math.round(at.y) };
		r.waypoints.splice(after + 1, 0, wp);
	});
}

/** The index of the segment of a polyline nearest to a point (the line's click inserts there) */
export function nearestSegment(points: Point[], p: Point): number {
	let best = 0;
	let bestDistance = Infinity;
	for (let i = 0; i < points.length - 1; i++) {
		const a = points[i];
		const b = points[i + 1];
		const dx = b.x - a.x;
		const dy = b.y - a.y;
		const t = Math.max(0, Math.min(1, ((p.x - a.x) * dx + (p.y - a.y) * dy) / (dx * dx + dy * dy || 1)));
		const d = Math.hypot(a.x + t * dx - p.x, a.y + t * dy - p.y);
		if (d < bestDistance) {
			bestDistance = d;
			best = i;
		}
	}
	return best;
}

/**
 * Image files in a CDN folder (the local CDN and servers with a directory index list them; elsewhere the list is empty
 * and the path is typed). `folder` relative to the CDN root, e.g. `map/locations/`.
 */
export async function cdnFolder(folder: string): Promise<string[]> {
	const base = `${CDN_SERVER.replace(/\/$/, '')}/${folder.replace(/^\/+/, '').replace(/\/?$/, '/')}`;
	try {
		const res = await fetch(base);
		if (!res.ok || !(res.headers.get('content-type') ?? '').includes('html')) return [];
		const html = await res.text();
		const out = new Set<string>();
		for (const m of html.matchAll(/href="([^"?#]+\.(?:webp|png|svg|jpe?g))"/gi)) {
			const name = decodeURIComponent(m[1]).replace(/^.*\//, '');
			out.add(`${folder.replace(/^\/+/, '').replace(/\/?$/, '/')}${name}`);
		}
		return [...out].sort();
	} catch {
		return [];
	}
}

/** Does the CDN have the file (HEAD)? */
const headCache = new Map<string, Promise<boolean>>();
export function cdnHas(path: string): Promise<boolean> {
	if (!headCache.has(path)) {
		headCache.set(
			path,
			fetch(`${CDN_SERVER.replace(/\/$/, '')}/${path.replace(/^\/+/, '')}`, { method: 'HEAD' })
				.then((r) => r.ok)
				.catch(() => false)
		);
	}
	return headCache.get(path)!;
}

/** The file size in bytes (HEAD content-length), null when unknown */
export async function cdnSize(path: string): Promise<number | null> {
	try {
		const r = await fetch(`${CDN_SERVER.replace(/\/$/, '')}/${path.replace(/^\/+/, '')}`, { method: 'HEAD' });
		const n = Number(r.headers.get('content-length'));
		return r.ok && Number.isFinite(n) && n > 0 ? n : null;
	} catch {
		return null;
	}
}

/**
 * The id follows the source-language name until the first publish (agents/MAP.md §4.3), as long as it was never typed
 * by hand (it still is the default `ort` / `weg` or the slug of the previous name)
 */
export const autoId = (id: string, previous: string | undefined, fallback: string) =>
	[fallback, previous ? slugId(previous) : ''].some((base) => !!base && (id === base || (id.startsWith(`${base}-`) && /^[0-9]+$/.test(id.slice(base.length + 1)))));

/** a number field's value: empty = not set */
export const numberOrUnset = (v: string) => (v.trim() === '' ? undefined : Number(v));

/** a toon in a place's / route's list: on → off, off → on */
export function toggleToon(list: string[] | undefined, id: string): string[] {
	const current = list ?? [];
	return current.includes(id) ? current.filter((t) => t !== id) : [...current, id];
}
