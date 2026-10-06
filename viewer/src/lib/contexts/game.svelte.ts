import { variableKey, type Checkpoint, type ReaderHistory } from '@scutoons/content';
import { setContext, getContext, onDestroy } from 'svelte';
import { getWorldContext, type WorldContext } from './world.svelte.ts';

const GAME_KEY = Symbol('game');

/**
 * A toon page's view on the reader's ONE save game (contexts/world.svelte.ts, DATA.md "The save game", owner 2026-10-06:
 * variables across all toons): story variables, the history and every toon's checkpoints are the save game's; this
 * context only adds the toon of the page (`toonId` – checkpoint uris `<toon>/chapter/<n>`, jump urls, and the domain of
 * a variable name: `coins` is this toon's, `global.coins` every toon's, `toptoy.coins` toptoy's – @scutoons/content
 * variables.ts; the save game stores the keys). Every toon's shell (toon-shell.svelte) gets one, all on one save game.
 */
interface GameView {
	readonly toonId: string;
	/** the save game's variables by storage key (`catpope/coins`, `coins` = global) */
	readonly variables: Record<string, string | number>;
	/** every toon's checkpoints (filter by the uri's toon – progress.ts helpers do) */
	readonly progress: Checkpoint[];
	readonly history: ReaderHistory;
	/** when the reader started SCUTOONS (reader.daysSinceStart, the seed of shuffles / random()) */
	readonly created_at: string;
	readonly updated_at: string;
}

export type GameContext = ReturnType<typeof createGameContext>;

let _currentContext: GameContext | undefined;

export function createGameContext(toonId: string, world: WorldContext) {
	const save = world.state;
	const state: GameView = {
		toonId,
		get variables() {
			return save.variables;
		},
		get progress() {
			return save.progress;
		},
		get history() {
			return save.history;
		},
		get created_at() {
			return save.created_at;
		},
		get updated_at() {
			return save.updated_at;
		}
	};

	return {
		get state() {
			return state;
		},
		/** a variable by its name as written in this toon's content (`coins`, `global.coins`, `toptoy.coins`) */
		getVariable: (name: string) => world.getVariable(variableKey(name, toonId)),
		setVariable: (name: string, value: string | number) => world.setVariable(variableKey(name, toonId), value),
		addVariable: (name: string, amount: number) => world.addVariable(variableKey(name, toonId), amount),
		saveState: world.save,
		hasCheckpoint: world.hasCheckpoint,
		addCheckpoint: world.addCheckpoint,
		completeCheckpoint: world.completeCheckpoint,
		/** reading progress of a chapter of THIS toon (0..1, `fresh` = the first call of this visit) */
		readChapter: (chapter: number, position: number, fresh: boolean) => world.readChapter(toonId, chapter, position, fresh),
		visits: world.visits,
		recordVisit: world.recordVisit,
		sync: world.sync
	};
}

export function setGameContext(toonId: string) {
	const world = getWorldContext();
	if (!world) throw new Error('the save game (world context) is set by the root layout');
	const context = createGameContext(toonId, world);
	setContext(GAME_KEY, context);
	_currentContext = context;
	onDestroy(() => {
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
