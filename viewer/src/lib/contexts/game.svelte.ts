import { setContext, getContext } from 'svelte';

const GAME_KEY = Symbol('game');
const STORAGE_PREFIX = 'scutoons_game_';

interface GameState {
	toonId: string;
	variables: Record<string, string | number>;
	progress: Record<string, unknown>;
	lastCheckpoint: string | null;
	created_at: string;
	updated_at: string;
}

export type GameContext = ReturnType<typeof createGameContext>;

export function createGameContext(toonId: string) {
	const storageKey = `${STORAGE_PREFIX}${toonId}`;

	const state = $state<GameState>({
		toonId,
		variables: {},
		progress: {},
		lastCheckpoint: null,
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
			state.progress = parsed.progress || {};
			state.lastCheckpoint = parsed.lastCheckpoint || null;
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
	}

	function getVariable(key: string): string | number | undefined {
		return state.variables[key];
	}

	function setVariable(key: string, value: string | number): void {
		state.variables[key] = value;
		saveState();
	}

	function addVariable(key: string, amount: number): void {
		const current = state.variables[key];
		const currentNum = typeof current === 'number' ? current : 0;
		state.variables[key] = currentNum + amount;
		saveState();
	}

	function clear() {
		if (typeof localStorage === 'undefined') return;
		localStorage.removeItem(storageKey);
		state.variables = {};
		state.progress = {};
		state.lastCheckpoint = null;
	}

	loadState();

	return {
		get state() {
			return state;
		},
		getVariable,
		setVariable,
		addVariable,
		saveState,
		loadState,
		clear
	};
}

export function setGameContext(toonId: string) {
	const context = createGameContext(toonId);
	setContext(GAME_KEY, context);
	return context;
}

export function getGameContext() {
	return getContext<GameContext>(GAME_KEY);
}
