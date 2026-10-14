// The reader's ONE save game (owner 2026-10-06: "Variables are per save game … make them across all toons"; DATA.md
// "The save game", @scutoons/content save-game.ts): story variables, the history, every toon's reading progress and the
// map's discoveries (agents/MAP.md §2.4) in one state under the reserved id `scutoons`. Guests: localStorage
// `scutoons_game_scutoons`; members: `/api/progress/scutoons/` too. Two states MERGE (`mergeSaveGame`, the server does
// the same on every PUT): map, checkpoints and history are never lost, variables are the newer write's.
// The old per-toon game states (`scutoons_game_<toon>`, before 2026-10-06) are taken in once on load and removed
// (`absorbToonStates`: a clash → the newest state's value) – the guests' side of the migration 2026-10-06-global-save-game
// (no SQL reaches a browser; nothing an author did is involved, so no creator feedback – HANDOVER's exception).
// Per toon is only the view on it (contexts/game.svelte.ts: the toon of the page, its checkpoints).
//
// Map writers: the story's events – unlockLocation (map-reveal.ts; owner 2026-10-11: places are unlocked by events only,
// the old slice / reading-progress link went), later revealRoute, travel, setPosition (MAP.md §3) – set operations.
import { getContext, setContext } from 'svelte';
import {
	SAVE_GAME_KEY,
	SAVE_GAME_PREFIX,
	WORLD_STATE_ID,
	absorbToonStates,
	advanceCheckpoint,
	chapterCheckpointUri,
	emptySaveGame,
	historyCount,
	mergeSaveGame,
	normalizeSaveGame,
	recordHistory,
	resetMapState,
	type OldToonState,
	type SaveGame,
	markPlacesSeen,
	moveResident,
	revealRoute,
	setPosition,
	setAvatarPosition,
	unlockPlace,
	type MapPosition,
	type ReadingSpotRef
} from '@scutoons/content';
import { fetchProgress, putProgress } from '$lib/api.ts';
import { MAP_VIEW_KEY } from '$lib/components/toon/map/map-view.ts';
import type { SessionContext } from './session.svelte.ts';

const KEY = Symbol('world');
const SYNC_DEBOUNCE_MS = 2000;

export type WorldContext = ReturnType<typeof createWorldContext>;

export function createWorldContext(session: SessionContext | null) {
	const now = () => new Date().toISOString();
	const state = $state<SaveGame>(emptySaveGame());

	function replace(next: SaveGame) {
		state.variables = next.variables;
		state.progress = next.progress;
		state.history = next.history;
		state.map = next.map;
		state.created_at = next.created_at;
		state.updated_at = next.updated_at;
	}

	function load() {
		if (typeof localStorage === 'undefined') return;
		try {
			const saved = localStorage.getItem(SAVE_GAME_KEY);
			if (saved) replace(normalizeSaveGame(JSON.parse(saved)));
		} catch {
			/* a broken entry starts empty */
		}
		absorbOldStates();
	}

	/** the old per-toon game states of this device, taken in once and removed (see the file comment) */
	function absorbOldStates() {
		const olds: (OldToonState & { key: string })[] = [];
		try {
			for (let i = 0; i < localStorage.length; i++) {
				const key = localStorage.key(i);
				if (!key || !key.startsWith(SAVE_GAME_PREFIX) || key === SAVE_GAME_KEY) continue;
				let parsed: unknown = null;
				try {
					parsed = JSON.parse(localStorage.getItem(key) ?? 'null');
				} catch {
					/* broken: dropped with the key */
				}
				olds.push({ key, toon: key.slice(SAVE_GAME_PREFIX.length), state: parsed });
			}
		} catch {
			return;
		}
		if (!olds.length) return;
		const { save: merged, clashes } = absorbToonStates($state.snapshot(state) as SaveGame, olds);
		replace(merged);
		if (clashes.length) console.info(`Save game: took in ${olds.length} old toon states – the newest one's value of ${clashes.join(', ')}`);
		save();
		for (const o of olds) {
			try {
				localStorage.removeItem(o.key);
			} catch {
				/* stays – taken in again next time (the same values) */
			}
		}
	}

	function store() {
		try {
			localStorage.setItem(SAVE_GAME_KEY, JSON.stringify($state.snapshot(state)));
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

	/** a device that never saved anything: the account's state is taken as it is */
	const empty = (s: SaveGame) =>
		!Object.keys(s.variables).length && !s.progress.length && !Object.keys(s.history).length && !Object.keys(s.map.places).length && !Object.keys(s.map.routes).length;

	/**
	 * The server's state (`remoteTime` ms) and ours as one: the newer one's variables, everything else merged. Returns
	 * whether ours knew something the server's didn't (then it is pushed).
	 */
	function adopt(remote: unknown, remoteTime: number): boolean {
		const local = $state.snapshot(state) as SaveGame;
		if (!remote || typeof remote !== 'object') return !empty(local);
		const localTime = empty(local) ? 0 : millis(local.updated_at);
		const remoteState = { ...(remote as object), updated_at: new Date(remoteTime).toISOString() };
		const merged = localTime > remoteTime ? mergeSaveGame(local, remoteState) : mergeSaveGame(remoteState, local);
		if (JSON.stringify(merged) !== JSON.stringify(local)) {
			replace(merged);
			store();
		}
		return JSON.stringify(normalizeSaveGame(remoteState)) !== JSON.stringify(merged);
	}

	async function push() {
		if (!loggedIn()) return;
		try {
			const result = await putProgress(WORLD_STATE_ID, $state.snapshot(state), millis(state.updated_at));
			if (result?.state) adopt(result.state, millis(result.updated_at));
		} catch (error) {
			console.warn('Save game sync (push) failed:', error);
		}
	}

	/** Once per login: the account's save game and this device's (a guest's too) become one, then pushed if needed */
	async function sync() {
		if (synced || !loggedIn()) return;
		synced = true;
		try {
			const remote = await fetchProgress(WORLD_STATE_ID).catch(() => null);
			if (adopt(remote?.state, millis(remote?.updated_at ?? remote?.state?.updated_at))) await push();
		} catch (error) {
			synced = false;
			console.warn('Save game sync failed:', error);
		}
	}

	const unsubscribe =
		session?.subscribe((event) => {
			if (event === 'logout') {
				synced = false;
				clearTimeout(pushTimer);
			} else sync();
		}) ?? (() => {});

	// ── story variables, history, reading progress (every toon) ───────────────────────────────────────────────

	function getVariable(key: string): string | number | undefined {
		return state.variables[key];
	}

	function setVariable(key: string, value: string | number): void {
		state.variables[key] = value;
		save();
	}

	function addVariable(key: string, amount: number): void {
		const current = state.variables[key];
		// a number written as text ("330", set from a choice or a setVariable) counts as that number
		const currentNum = typeof current === 'number' ? current : Number.isFinite(Number(current)) && current !== '' ? Number(current) : 0;
		state.variables[key] = currentNum + amount;
		save();
	}

	/** how many times the reader met `key` (the history, @scutoons/content history.ts) */
	function visits(key: string): number {
		return historyCount(state.history, key);
	}

	/** one more time for `key` (stored + synced) */
	function recordVisit(key: string): void {
		recordHistory(state.history, key, Date.now());
		save();
	}

	function hasCheckpoint(uri: string) {
		return state.progress.some((p) => p.uri === uri);
	}

	function addCheckpoint(uri: string) {
		state.progress.push({ uri, isComplete: false, created_at: now(), updated_at: now() });
		save();
	}

	function completeCheckpoint(uri: string) {
		const checkpoint = state.progress.find((p) => p.uri === uri);
		if (!checkpoint) return;
		checkpoint.isComplete = true;
		checkpoint.updated_at = now();
		save();
	}

	/**
	 * Reading progress of a toon's chapter (the toon page's start button, agents/DESIGN.md "Start button"): `position` =
	 * how far (0..1); `fresh` = the first call of this visit; `spot` = the group / slice there ("continue" opens it,
	 * BACKLOG 10.69).
	 */
	function readChapter(toon: string, chapter: number, position: number, fresh: boolean, spot?: ReadingSpotRef | null) {
		const uri = chapterCheckpointUri(toon, chapter);
		const index = state.progress.findIndex((p) => p.uri === uri);
		const next = advanceCheckpoint(index >= 0 ? $state.snapshot(state.progress[index]) : undefined, uri, position, fresh, now(), spot);
		if (!next) return;
		if (index >= 0) state.progress[index] = next;
		else state.progress.push(next);
		save();
	}

	// ── map operations (set operations: repeating one changes nothing) ─────────────────────────────────────────

	/** Unlocks places; returns the ids that were new */
	function unlock(ids: Iterable<string>): string[] {
		const at = Date.now();
		const added = [...ids].filter((id) => unlockPlace(state.map, id, at));
		if (added.length) save();
		return added;
	}

	/** The reader (`READER_RESIDENT`) or a character moves to a region (`migrate`, agents/MAP.md §8.10); false = lives there already */
	function migrate(who: string, region: string): boolean {
		const moved = moveResident(state.map, who, region, Date.now());
		if (moved) save();
		return moved;
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

	/**
	 * Starts the map over – no place unlocked, as if read anew (the account's "Reset the map" next to the draft preview,
	 * owner 2026-10-11: to see the unlock animations again). Only the map: checkpoints, variables, history and the
	 * inventory stay. `resetAt` makes every device drop its older copy on the next sync (mergeMapState).
	 */
	function resetMap(): void {
		state.map = resetMapState(Date.now());
		try {
			localStorage.removeItem(MAP_VIEW_KEY); // the remembered map view: the map opens on its start view again
		} catch {
			/* storage blocked */
		}
		save();
	}

	function position(toon: string, where: Omit<MapPosition, 'at'>) {
		setPosition(state.map, toon, where, Date.now());
		save();
	}

	/** The reader's own avatar walked somewhere (the walking mode – research/CAVE.md D1) */
	function avatarTo(where: Omit<MapPosition, 'at'>) {
		setAvatarPosition(state.map, where, Date.now());
		save();
	}

	/** the world's places with their names (the reader's language) once the map data is there – texts read `map.total` / `map.here` */
	let places = $state<readonly { id: string; name?: string }[] | null>(null);
	/** The world's places (id + name), set once the map data is loaded (toon/map/map.svelte) */
	function setPlaces(loaded: readonly { id: string; name?: unknown }[]) {
		places = loaded.map((p) => ({ id: p.id, name: typeof p.name === 'string' ? p.name : undefined }));
	}

	load();

	return {
		/** the whole save game (reactive) */
		get state() {
			return state;
		},
		get map() {
			return state.map;
		},
		getVariable,
		setVariable,
		addVariable,
		visits,
		recordVisit,
		hasCheckpoint,
		addCheckpoint,
		completeCheckpoint,
		readChapter,
		save,
		/** the world's places (id + name) once the map data is loaded, else null */
		get places() {
			return places;
		},
		/** a place unlocked and not seen on the opened map yet (the toggle's badge) */
		get hasNew() {
			return Object.values(state.map.places).some((p) => p.seen === undefined);
		},
		unlock,
		migrate,
		setPlaces,
		seen,
		reveal,
		resetMap,
		position,
		avatarTo,
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
