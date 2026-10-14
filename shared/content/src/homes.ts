// The avatar's home (agents/research/CAVE.md, FEATURES 10.98): home scenes, furniture, the walking avatar, the WG,
// the mover. Terms: a home = an avatar's place on the map + its room; a home scene = what a home looks like (`cave`,
// `tatami_room`); furniture = items standing in a home; a WG = a home several of the reader's avatars share.
import type { Localized } from './items.ts';
import type { QualityVariants } from './media-quality.ts';
import { isPlaceWaypoint, routeDrawnTo, type MapState, type RouteLike } from './map.ts';

/**
 * A home scene's file at the reader's media quality (media-quality.ts `pickQualityVariant`): `src` = 1080p (low and
 * medium), `variants.high` = 4K (owner 2026-10-14: "4K und 1080p ist, glaube ich, ausreichend für die Hintergründe")
 */
export interface HomeSceneMedia {
	src: string;
	variants?: QualityVariants | null;
}

/** A home scene (catalog `home_scenes`): what a home looks like – the 2.5D background (images first, 3D later) */
export interface HomeScene {
	/** stable, snake_case (`cave`, `tatami_room`) */
	id: string;
	name: Localized;
	/** the room's background, a still (MAP.md §5.1 three-quarter view), language-neutral; the video's poster */
	image: HomeSceneMedia;
	/**
	 * An animated background (owner 2026-10-14: "Die Cave-Hintergründe können auch Videos sein. Animierte kurze Clips"):
	 * a short muted loop (.mp4 / .webm) over the still; reduced motion or a failed load shows the still
	 */
	video?: HomeSceneMedia | null;
}

/**
 * A furniture item (`ItemDef.furniture`, category `furniture`): how it stands in a home. `size` = its width in % of the
 * room's width at the front edge (the room = 4 m: metres ÷ 4 × 100 – workflows/home/furniture.md); `image` = the piece
 * as it stands in a home (three-quarter view, its back to the left wall, its foot at the bottom), next to the item's
 * `icon`; `workshop` = recipes `at: "workshop"` run at it (the workbench).
 */
export interface ItemFurniture {
	size: number;
	image: string;
	workshop?: boolean;
}

/** A piece of furniture in a home (reader state): where its foot stands (% of the room), mirrored to face left */
export interface FurniturePlacement {
	home: string;
	x: number;
	y: number;
	flip?: boolean;
}

/** the default home scene while a character has none (owner 2026-10-14: the scene is set per avatar) */
export const DEFAULT_HOME_SCENE = 'cave';
/** at most this many of a reader's avatars in one home – the WG (owner 2026-10-14) */
export const HOME_RESIDENTS_MAX = 5;
/** the mover: takes an avatar's furniture along when it moves into another home (owner 2026-10-14: 50 cucumbers) */
export const MOVER_PRICE = { item: 'cucumber', qty: 50 } as const;
/** a furniture item's size bounds (% of the room's front width) */
export const FURNITURE_SIZE_MIN = 5;
export const FURNITURE_SIZE_MAX = 60;

/** Further back = smaller: 65 % at the back wall's foot (y ≤ 55 % of the room), 100 % at the front edge */
export const furnitureDepth = (y: number): number => 0.65 + 0.35 * Math.min(1, Math.max(0, (y - 55) / 45));

/** A character's home place and scene (the registry fields; the first-story place is the creator's suggestion) */
export function characterHome(character: { home?: string; home_scene?: string } | null | undefined): { place?: string; scene: string } {
	return { place: character?.home || undefined, scene: character?.home_scene || DEFAULT_HOME_SCENE };
}

/** One walked stretch: along `route` from waypoint index `from` to `to` (either direction) */
export interface WalkStep {
	route: string;
	from: number;
	to: number;
	/** the place it arrives at */
	place: string;
}

/**
 * The shortest walk (fewest stretches) from one place to another along the routes the reader can see, between places
 * that are open to them (unlocked, or their home): CAVE.md D1 – no spoilers. null = no path; [] = already there.
 */
export function walkPath(routes: readonly RouteLike[], state: MapState, from: string, to: string, open: (placeId: string) => boolean): WalkStep[] | null {
	if (from === to) return [];
	const edges = new Map<string, WalkStep[]>();
	const add = (a: string, step: WalkStep) => edges.set(a, [...(edges.get(a) ?? []), step]);
	for (const route of routes) {
		const upTo = routeDrawnTo(route, state, open);
		let prev: { id: string; i: number } | null = null;
		route.waypoints.forEach((w, i) => {
			if (i > upTo || !isPlaceWaypoint(w)) return;
			if (!open(w.location)) {
				prev = null;
				return;
			}
			if (prev) {
				add(prev.id, { route: route.id, from: prev.i, to: i, place: w.location });
				add(w.location, { route: route.id, from: i, to: prev.i, place: prev.id });
			}
			prev = { id: w.location, i };
		});
	}
	// breadth first: place → the place it was reached from + the stretch
	const back = new Map<string, { prev: string; step: WalkStep } | null>([[from, null]]);
	const queue = [from];
	while (queue.length && !back.has(to)) {
		const at = queue.shift()!;
		for (const step of edges.get(at) ?? []) {
			if (back.has(step.place)) continue;
			back.set(step.place, { prev: at, step });
			queue.push(step.place);
		}
	}
	if (!back.has(to)) return null;
	const out: WalkStep[] = [];
	for (let link = back.get(to); link; link = back.get(link.prev)) out.unshift(link.step);
	return out;
}

type Add = (level: 'error' | 'warning', path: (string | number)[], message: string) => void;
const isObj = (v: unknown): v is Record<string, any> => typeof v === 'object' && v !== null && !Array.isArray(v);
const isStr = (v: unknown): v is string => typeof v === 'string' && v.length > 0;

/** The home rules of a catalog (inside `validateCatalog`; the server: CatalogValidator) */
export function validateHomes(catalog: { items: any[]; recipes?: any[]; home_scenes?: any[] }, add: Add): void {
	const scenes = new Set<string>();
	(catalog.home_scenes ?? []).forEach((s: any, i: number) => {
		const path = ['home_scenes', i];
		if (!isObj(s) || !isStr(s.id) || !/^[a-z0-9][a-z0-9_]*$/.test(s.id)) return add('error', path, 'a home scene needs an id (snake_case)');
		if (scenes.has(s.id)) add('error', [...path, 'id'], `duplicate home scene "${s.id}"`);
		scenes.add(s.id);
		if (!isObj(s.name) || !Object.values(s.name).some(isStr)) add('error', [...path, 'name'], `${s.id}: a home scene needs a name`);
		if (!isObj(s.image) || !isStr(s.image.src)) add('warning', [...path, 'image'], `${s.id}: no background image yet`);
		if (s.video != null && !(isObj(s.video) && isStr(s.video.src) && /\.(mp4|webm)$/i.test(s.video.src))) add('error', [...path, 'video'], `${s.id}: the video must be { src: an .mp4 / .webm path, variants? }`);
	});
	let workshops = 0;
	catalog.items.forEach((item: any, i: number) => {
		if (!isObj(item)) return;
		const path = ['items', i];
		const f = item.furniture;
		if (f != null) {
			if (!isObj(f) || typeof f.size !== 'number' || f.size < FURNITURE_SIZE_MIN || f.size > FURNITURE_SIZE_MAX) {
				add('error', [...path, 'furniture'], `${item.id}: furniture must be { size: ${FURNITURE_SIZE_MIN}–${FURNITURE_SIZE_MAX} (% of the room's width), image, workshop? }`);
			} else {
				if (!isStr(f.image)) add('warning', [...path, 'furniture', 'image'], `${item.id}: no home image yet – the icon stands in`);
				if (f.workshop === true) workshops++;
			}
			if (item.category !== 'furniture') add('warning', [...path, 'category'], `${item.id}: a furniture item should have the category "furniture"`);
			if (item.stackable) add('error', [...path, 'stackable'], `${item.id}: furniture can't be stackable (each piece stands on its own)`);
		} else if (item.category === 'furniture') add('error', [...path, 'furniture'], `${item.id}: the category "furniture" needs furniture { size, image }`);
		for (const [k, skill] of (Array.isArray(item.skills) ? item.skills : []).entries())
			if (skill?.type === 'enter_home' && item.furniture) add('error', [...path, 'skills', k], `${item.id}: furniture can't enter a home`);
	});
	(catalog.recipes ?? []).forEach((r: any, i: number) => {
		if (!isObj(r) || r.at == null) return;
		if (r.at !== 'workshop') add('error', ['recipes', i, 'at'], `${r.id}: "at" can only be "workshop"`);
		else if (!workshops) add('warning', ['recipes', i, 'at'], `${r.id}: runs at a workshop, but no furniture item is a workshop`);
	});
}
