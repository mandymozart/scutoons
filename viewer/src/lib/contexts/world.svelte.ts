// The reader's world state (agents/MAP.md §2.4): the map discoveries of the whole universe – unlocked places, revealed
// routes, where each toon's hero is – in ONE game state under the reserved id `scutoons` (owner 2026-10-01: "if a reader
// gets to some point in one story it unlocks an amazing potential for storytelling"). Guests: localStorage
// `scutoons_game_scutoons`; members: `/api/progress/scutoons/` too. Unlike the per-toon game state (last write wins) the
// map merges as sets (`mergeMapState`): a discovery made on another device, or as a guest before signing in, is never
// lost. The server merges the same way on every PUT, so a stale write still adds its discoveries.
//
// Who writes: today the reading – a seen slice (`sliceSeen`: the places whose `unlock.slice` is that slice's image) and
// the chapter progress (`unlocksFromProgress`, for places whose chapter has no slices yet); the story events
// (unlockLocation, revealRoute, travel, setPosition – MAP.md §3) call the same operations in M3.
import { getContext, setContext } from 'svelte';
import {
	WORLD_STATE_ID,
	emptyMapState,
	markPlacesSeen,
	mergeMapState,
	placesForSlice,
	normalizeMapState,
	revealRoute,
	setPosition,
	unlockPlace,
	type MapPlace,
	type MapPosition,
	type MapState
} from '@scutoons/content';
import { fetchProgress, putProgress } from '$lib/api.ts';
import type { SessionContext } from './session.svelte.ts';

const KEY = Symbol('world');
const STORAGE_KEY = `scutoons_game_${WORLD_STATE_ID}`;
const SYNC_DEBOUNCE_MS = 2000;

interface WorldState {
	map: MapState;
	created_at: string;
	updated_at: string;
}

export type WorldContext = ReturnType<typeof createWorldContext>;

export function createWorldContext(session: SessionContext | null) {
	const now = () => new Date().toISOString();
	const state = $state<WorldState>({ map: emptyMapState(), created_at: now(), updated_at: now() });

	function load() {
		try {
			const saved = typeof localStorage === 'undefined' ? null : localStorage.getItem(STORAGE_KEY);
			if (!saved) return;
			const parsed = JSON.parse(saved);
			state.map = normalizeMapState(parsed?.map);
			state.created_at = parsed?.created_at || state.created_at;
			state.updated_at = parsed?.updated_at || state.updated_at;
		} catch {
			/* a broken entry starts empty */
		}
	}

	function store() {
		try {
			localStorage.setItem(STORAGE_KEY, JSON.stringify($state.snapshot(state)));
		} catch {
			/* storage full / blocked: the state lives for this visit */
		}
	}

	function save() {
		state.updated_at = now();
		store();
		schedulePush();
	}

	// ── account sync (set merge) ──────────────────────────────────────────────────────────────────────────

	let pushTimer: ReturnType<typeof setTimeout> | undefined;
	let synced = false;
	const loggedIn = () => !!session?.isLoggedIn;
	const millis = (value: unknown) => (typeof value === 'number' ? value : Date.parse(String(value ?? '')) || 0);

	function schedulePush() {
		if (!loggedIn()) return;
		clearTimeout(pushTimer);
		pushTimer = setTimeout(push, SYNC_DEBOUNCE_MS);
	}

	/** the server's answer (merged there) merged into ours: nothing either side discovered is lost */
	function adopt(remote: any) {
		const merged = mergeMapState(state.map, normalizeMapState(remote?.map));
		if (JSON.stringify(merged) === JSON.stringify($state.snapshot(state.map))) return false;
		state.map = merged;
		store();
		return true;
	}

	async function push() {
		if (!loggedIn()) return;
		try {
			const result = await putProgress(WORLD_STATE_ID, $state.snapshot(state), millis(state.updated_at));
			if (result?.state) adopt(result.state);
		} catch (error) {
			console.warn('World state sync (push) failed:', error);
		}
	}

	/** Once per login: the account's state and this device's (a guest's discoveries too) become one, then pushed */
	async function sync() {
		if (synced || !loggedIn()) return;
		synced = true;
		try {
			const remote = await fetchProgress(WORLD_STATE_ID).catch(() => null);
			const before = JSON.stringify($state.snapshot(state.map));
			adopt(remote?.state);
			const remoteMap = JSON.stringify(normalizeMapState(remote?.state?.map));
			// push when this device knew something the account didn't
			if (JSON.stringify(mergeMapState(normalizeMapState(remote?.state?.map), JSON.parse(before))) !== remoteMap) await push();
		} catch (error) {
			synced = false;
			console.warn('World state sync failed:', error);
		}
	}

	const unsubscribe =
		session?.subscribe((event) => {
			if (event === 'logout') {
				synced = false;
				clearTimeout(pushTimer);
			} else sync();
		}) ?? (() => {});

	// ── operations (set operations: repeating one changes nothing) ─────────────────────────────────────────

	/** Unlocks places; returns the ids that were new */
	function unlock(ids: Iterable<string>): string[] {
		const at = Date.now();
		const added = [...ids].filter((id) => unlockPlace(state.map, id, at));
		if (added.length) save();
		return added;
	}

	/** Marks places seen on the opened map (their halo and the toggle's badge go away) */
	function seen(ids: Iterable<string>): string[] {
		const marked = markPlacesSeen(state.map, ids, Date.now());
		if (marked.length) save();
		return marked;
	}

	function reveal(route: string, upTo: number): boolean {
		const changed = revealRoute(state.map, route, upTo);
		if (changed) save();
		return changed;
	}

	function position(toon: string, where: Omit<MapPosition, 'at'>) {
		setPosition(state.map, toon, where, Date.now());
		save();
	}

	// ── slice unlocks: the reading tracker reports every slice passed; the map's places (loaded when the browser is idle,
	// `toon/map/map.svelte`) say which slice unlocks which place. Slices seen before they arrived wait here (this visit). ──

	type UnlockRules = readonly Pick<MapPlace, 'id' | 'hidden' | 'unlock'>[];
	let rules: UnlockRules | null = null;
	const waiting: { toon: string; chapter: number; image: string }[] = [];

	/** A slice of a toon's chapter was seen: unlock the places it unlocks; returns the ids that were new */
	function sliceSeen(toon: string, chapter: number, image: string): string[] {
		if (!image) return [];
		if (!rules) {
			waiting.push({ toon, chapter, image });
			return [];
		}
		return unlock(placesForSlice(rules, toon, chapter, image));
	}

	/** The world's places (with their `unlock`), set once the map data is loaded */
	function setUnlockRules(places: UnlockRules) {
		rules = places;
		const ids = waiting.splice(0).flatMap((s) => placesForSlice(places, s.toon, s.chapter, s.image));
		if (ids.length) unlock(ids);
	}

	load();

	return {
		get map() {
			return state.map;
		},
		/** a place unlocked and not seen on the opened map yet (the toggle's badge) */
		get hasNew() {
			return Object.values(state.map.places).some((p) => p.seen === undefined);
		},
		unlock,
		sliceSeen,
		setUnlockRules,
		seen,
		reveal,
		position,
		sync,
		destroy() {
			unsubscribe();
			clearTimeout(pushTimer);
		}
	};
}

export function setWorldContext(session: SessionContext | null) {
	const context = createWorldContext(session);
	setContext(KEY, context);
	context.sync();
	return context;
}

export function getWorldContext(): WorldContext | undefined {
	return getContext<WorldContext | undefined>(KEY);
}
