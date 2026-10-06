import type { Checkpoint, ReaderHistory } from '@scutoons/content';
import { advanceCheckpoint, chapterCheckpointUri, historyCount, normalizeHistory, recordHistory } from '@scutoons/content';
import { setContext, getContext, onDestroy } from 'svelte';
import { fetchProgress, putProgress } from '$lib/api.ts';
import { getCurrentSession } from './session.svelte.ts';

const GAME_KEY = Symbol('game');
const STORAGE_PREFIX = 'scutoons_game_';
const SYNC_DEBOUNCE_MS = 2000;

interface GameState {
	toonId: string;
	variables: Record<string, string | number>;
	progress: Checkpoint[];
	/** what the reader met and how often (text visits; events + items in step 3) – @scutoons/content history.ts; older states: none = empty */
	history: ReaderHistory;
	created_at: string;
	updated_at: string;
}

export type GameContext = ReturnType<typeof createGameContext>;

let _currentContext: GameContext | undefined;

export function createGameContext(toonId: string) {
	const storageKey = `${STORAGE_PREFIX}${toonId}`;

	const state = $state<GameState>({
		toonId,
		variables: {},
		progress: [],
		history: {},
		created_at: new Date().toISOString(),
		updated_at: new Date().toISOString()
	});

	function loadState() {
		if (typeof localStorage === 'undefined') return;

		const saved = localStorage.getItem(storageKey);
		if (!saved) return;

		try {
			const parsed = JSON.parse(saved);
			state.variables = parsed.variables || {};
			state.progress = Array.isArray(parsed.progress) ? parsed.progress : [];
			state.history = normalizeHistory(parsed.history);
			state.created_at = parsed.created_at || state.created_at;
			state.updated_at = parsed.updated_at || state.updated_at;
		} catch (error) {
			console.error('Failed to load game state:', error);
		}
	}

	function saveState() {
		if (typeof localStorage === 'undefined') return;

		state.updated_at = new Date().toISOString();
		localStorage.setItem(storageKey, JSON.stringify(state));
		schedulePush();
	}

	// ---------- account progress sync (/progress/<toonId>/, last-write-wins by updated_at) ----------

	let pushTimer: ReturnType<typeof setTimeout> | undefined;
	let synced = false;

	function snapshot() {
		return $state.snapshot(state);
	}

	function schedulePush() {
		if (!getCurrentSession()?.isLoggedIn) return;
		clearTimeout(pushTimer);
		pushTimer = setTimeout(push, SYNC_DEBOUNCE_MS);
	}

	/** API timestamps are ms since 1970 (numbers); older states may carry ISO strings. */
	function toMillis(value: unknown): number {
		if (typeof value === 'number') return value;
		return Date.parse(String(value ?? '')) || 0;
	}

	function applyRemote(remoteState: any, remoteTime: number) {
		state.variables = remoteState.variables || {};
		state.progress = Array.isArray(remoteState.progress) ? remoteState.progress : [];
		state.history = normalizeHistory(remoteState.history);
		state.created_at = remoteState.created_at || state.created_at;
		state.updated_at = new Date(remoteTime).toISOString();
		localStorage.setItem(storageKey, JSON.stringify(state));
	}

	async function push() {
		if (!getCurrentSession()?.isLoggedIn) return;
		try {
			const result = await putProgress(toonId, snapshot(), toMillis(state.updated_at));
			// another device wrote a newer state in between -> take it (last write wins)
			if (result?.status === 'stale' && result.state) applyRemote(result.state, toMillis(result.updated_at));
		} catch (error) {
			console.warn('Progress sync (push) failed:', error);
		}
	}

	/** Pull once per login: newer server state replaces local, newer local state is pushed. */
	async function sync() {
		if (synced || !getCurrentSession()?.isLoggedIn) return;
		synced = true;
		try {
			const remote = await fetchProgress(toonId).catch(() => null);
			const remoteState = remote?.state;
			const remoteTime = toMillis(remote?.updated_at ?? remoteState?.updated_at);
			const localHasData = Object.keys(state.variables).length > 0 || state.progress.length > 0 || Object.keys(state.history).length > 0;
			// a device that never saved anything always takes the account state
			const localTime = localHasData ? toMillis(state.updated_at) : 0;

			if (remoteState && remoteTime > localTime) {
				applyRemote(remoteState, remoteTime);
			} else if (localHasData && localTime > remoteTime) {
				await push();
			}
		} catch (error) {
			synced = false;
			console.warn('Progress sync failed:', error);
		}
	}

	const unsubscribe =
		getCurrentSession()?.subscribe((event) => {
			if (event === 'logout') {
				synced = false;
				clearTimeout(pushTimer);
			} else {
				sync();
			}
		}) ?? (() => {});

	function getVariable(key: string): string | number | undefined {
		return state.variables[key];
	}

	function setVariable(key: string, value: string | number): void {
		state.variables[key] = value;
		saveState();
	}

	function addVariable(key: string, amount: number): void {
		const current = state.variables[key];
		// a number written as text ("330", set from a choice or a setVariable) counts as that number
		const currentNum = typeof current === 'number' ? current : Number.isFinite(Number(current)) && current !== '' ? Number(current) : 0;
		state.variables[key] = currentNum + amount;
		saveState();
	}

	function hasCheckpoint(uri: string) {
		if (!Array.isArray(state.progress)) return false;
		return state.progress.some((p) => p.uri === uri);
	}

	function addCheckpoint(uri: string) {
		state.progress.push({
			uri: uri,
			isComplete: false,
			created_at: new Date().toISOString(),
			updated_at: new Date().toISOString()
		});
		saveState();
	}

	function completeCheckpoint(uri: string) {
		const checkpoint = state.progress.find((p) => p.uri === uri);
		if (!checkpoint) return;
		checkpoint.isComplete = true;
		checkpoint.updated_at = new Date().toISOString();
		saveState();
	}

	/**
	 * Reading progress of a chapter (the toon page's start button, agents/DESIGN.md "Start button"): `position` = how far
	 * (0..1, slices seen / the chapter's slices; 1 = its last slice was reached = finished); `fresh` = the first call of
	 * this visit. Stored like every checkpoint: localStorage, and for a member `/api/progress/<toon>` (debounced).
	 */
	function readChapter(chapter: number, position: number, fresh: boolean) {
		if (!Array.isArray(state.progress)) state.progress = [];
		const uri = chapterCheckpointUri(toonId, chapter);
		const index = state.progress.findIndex((p) => p.uri === uri);
		const next = advanceCheckpoint(index >= 0 ? $state.snapshot(state.progress[index]) : undefined, uri, position, fresh, new Date().toISOString());
		if (!next) return;
		if (index >= 0) state.progress[index] = next;
		else state.progress.push(next);
		saveState();
	}

	/** how many times the reader met `key` (the history, @scutoons/content history.ts) */
	function visits(key: string): number {
		return historyCount(state.history, key);
	}

	/** one more time for `key` (stored + synced like the variables) */
	function recordVisit(key: string): void {
		recordHistory(state.history, key, Date.now());
		saveState();
	}

	function clear() {
		if (typeof localStorage === 'undefined') return;
		localStorage.removeItem(storageKey);
		state.variables = {};
		state.progress = [];
		state.history = {};
	}

	loadState();
	sync();

	function destroy() {
		unsubscribe();
		clearTimeout(pushTimer);
	}

	return {
		get state() {
			return state;
		},
		getVariable,
		setVariable,
		addVariable,
		saveState,
		loadState,
		hasCheckpoint,
		addCheckpoint,
		completeCheckpoint,
		readChapter,
		visits,
		recordVisit,
		clear,
		sync,
		destroy
	};
}

export function setGameContext(toonId: string) {
	const context = createGameContext(toonId);
	setContext(GAME_KEY, context);
	_currentContext = context;
	onDestroy(() => {
		context.destroy();
		if (_currentContext === context) _currentContext = undefined;
	});
	return context;
}

export function getGameContext() {
	return getContext<GameContext>(GAME_KEY);
}

export function getCurrentGameContext() {
	return _currentContext;
}
