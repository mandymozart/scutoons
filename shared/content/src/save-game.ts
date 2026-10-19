// The reader's ONE save game (owner 2026-10-06: "Variables are per save game … make them across all toons"; DATA.md
// "The save game"): story variables, the reader history, the reading progress of every toon and the map's discoveries
// in one state under the reserved id `scutoons` – localStorage `scutoons_game_scutoons`, members `/api/progress/scutoons/`.
// Reading progress stays keyed by toon inside it (checkpoint uris `<toon>/chapter/<n>`); story variables are global –
// a variable set in toptoy is the same one catpope reads.
//
// Merge (two devices, a guest's state meeting the account's – here and in the server's ProgressService::mergeSaveGame):
// the map as sets (mergeMapState), checkpoints by uri (the later `updated_at`), history entries per key (the larger
// count, the first `first`, the last `last`) – nothing either side read or met is lost; `variables` are the NEWER
// write's as a whole (last write wins, as before); `created_at` the earlier one.
import type { Checkpoint } from './content.ts';
import { HISTORY_LIMIT, normalizeHistory, type HistoryEntry, type ReaderHistory } from './history.ts';
import { emptyMapState, mergeMapState, normalizeMapState, WORLD_STATE_ID, type MapState } from './map.ts';

export type SaveGameValue = string | number;

export interface SaveGame {
	/** the story variables of every toon (bare names in texts and conditions) */
	variables: Record<string, SaveGameValue>;
	/** reading progress of every toon: chapter checkpoints `<toon>/chapter/<n>` */
	progress: Checkpoint[];
	/** what the reader met and how often (history.ts) – text visits, `event:<id>` runs */
	history: ReaderHistory;
	/** the map's discoveries (MAP.md §2.4) */
	map: MapState;
	/** when the reader started SCUTOONS (`reader.daysSinceStart`, the seed of shuffles / `random()`) */
	created_at: string;
	updated_at: string;
}

/** localStorage key of the save game (and the prefix of the old per-toon game states) */
export const SAVE_GAME_PREFIX = 'scutoons_game_';
export const SAVE_GAME_KEY = `${SAVE_GAME_PREFIX}${WORLD_STATE_ID}`;

export function emptySaveGame(now = new Date().toISOString()): SaveGame {
	return { variables: {}, progress: [], history: {}, map: emptyMapState(), created_at: now, updated_at: now };
}

const isObject = (x: unknown): x is Record<string, unknown> => !!x && typeof x === 'object' && !Array.isArray(x);

/** a placed pickup's memory in its old form, `[<toon>/]picked:<slice image>#<placement id>` (before 2026-10-19) */
const OLD_PICKED = /^((?:[^/]+\/)?picked:)[^#]+#(.+)$/;

/**
 * A placed pickup's key with the placement id alone (`[<toon>/]picked:<placement id>`, research/TEXT-ACROSS-SLICES.md
 * §5 step 3): the one-time helper for the save games in readers' browsers and stale tabs – the stored ones are
 * rewritten by the migration 2026-10-19-pickup-keys. A key in the new form wins over its old twin.
 */
export function pickedKeyOf(key: string): string {
	const m = OLD_PICKED.exec(key);
	return m ? `${m[1]}${m[2]}` : key;
}

/** story variables as stored: strings and finite numbers, anything else dropped; old pickup keys renamed (pickedKeyOf) */
export function normalizeVariables(raw: unknown): Record<string, SaveGameValue> {
	const out: Record<string, SaveGameValue> = {};
	if (!isObject(raw)) return out;
	for (const [k, v] of Object.entries(raw)) {
		if (!(typeof v === 'string' || (typeof v === 'number' && Number.isFinite(v)))) continue;
		const key = pickedKeyOf(k);
		if (key === k || !(key in raw)) out[key] = v;
	}
	return out;
}

/** checkpoints as stored: entries with a string uri (the last one of a uri wins) */
export function normalizeCheckpoints(raw: unknown): Checkpoint[] {
	if (!Array.isArray(raw)) return [];
	const byUri = new Map<string, Checkpoint>();
	for (const c of raw) if (isObject(c) && typeof c.uri === 'string') byUri.set(c.uri, { ...(c as unknown as Checkpoint), isComplete: !!c.isComplete });
	return [...byUri.values()];
}

/** a stored value (or nothing) as a save game: missing parts filled, junk dropped */
export function normalizeSaveGame(raw: unknown, now = new Date().toISOString()): SaveGame {
	const v = isObject(raw) ? raw : {};
	return {
		variables: normalizeVariables(v.variables),
		progress: normalizeCheckpoints(v.progress),
		history: normalizeHistory(v.history),
		map: normalizeMapState(v.map),
		created_at: typeof v.created_at === 'string' && v.created_at ? v.created_at : now,
		updated_at: typeof v.updated_at === 'string' && v.updated_at ? v.updated_at : now
	};
}

const time = (iso: string | undefined) => Date.parse(iso ?? '') || 0;

/** checkpoints of both by uri: the later `updated_at` (a tie: the finished one) */
export function mergeCheckpoints(a: readonly Checkpoint[], b: readonly Checkpoint[]): Checkpoint[] {
	const out = new Map<string, Checkpoint>();
	for (const c of [...a, ...b]) {
		const have = out.get(c.uri);
		if (!have) out.set(c.uri, c);
		else {
			const d = time(c.updated_at) - time(have.updated_at);
			const finished = { ...(d > 0 || (d === 0 && c.isComplete) ? c : have), isComplete: have.isComplete || c.isComplete };
			out.set(c.uri, finished);
		}
	}
	return [...out.values()];
}

/** history entries of both per key: the larger count, the first first, the last last (at most HISTORY_LIMIT, newest kept) */
export function mergeHistory(a: ReaderHistory, b: ReaderHistory): ReaderHistory {
	const out: ReaderHistory = {};
	for (const key of new Set([...Object.keys(a), ...Object.keys(b)])) {
		const p: HistoryEntry | undefined = a[key];
		const q: HistoryEntry | undefined = b[key];
		out[key] = !p ? { ...q } : !q ? { ...p } : { n: Math.max(p.n, q.n), first: Math.min(p.first, q.first), last: Math.max(p.last, q.last) };
	}
	const keys = Object.keys(out);
	if (keys.length > HISTORY_LIMIT) {
		keys.sort((x, y) => out[x].last - out[y].last);
		for (const old of keys.slice(0, keys.length - HISTORY_LIMIT)) delete out[old];
	}
	return out;
}

/**
 * Two save games as one: `newer`'s variables (last write wins), everything else merged (see the file comment). The
 * server's `ProgressService::mergeSaveGame` does the same on every `PUT /api/progress/scutoons/`.
 */
export function mergeSaveGame(newer: unknown, older: unknown): SaveGame {
	const n = normalizeSaveGame(newer);
	const o = normalizeSaveGame(older);
	return {
		variables: isObject(newer) && 'variables' in newer ? n.variables : o.variables,
		progress: mergeCheckpoints(o.progress, n.progress),
		history: mergeHistory(n.history, o.history),
		map: mergeMapState(n.map, o.map),
		created_at: time(o.created_at) && time(o.created_at) < time(n.created_at) ? o.created_at : n.created_at,
		updated_at: time(o.updated_at) > time(n.updated_at) ? o.updated_at : n.updated_at
	};
}

/** an old per-toon game state (`scutoons_game_<toon>`, before 2026-10-06) */
export interface OldToonState {
	toon: string;
	state: unknown;
}

/**
 * The old per-toon game states taken into the save game (a guest's localStorage – no SQL reaches it; members: the
 * migration `2026-10-06-global-save-game`, the same rule): a toon's variables keep their domain – `x` of catpope's
 * state becomes `catpope/x` (variables.ts), so two toons never clash; a key the save game already holds → the NEWEST
 * value wins (the save game itself counts as one of the states, by its `updated_at`); history and checkpoints merge
 * (mergeSaveGame); `created_at` the earliest. Returns the merged save game and the keys that clashed.
 */
export function absorbToonStates(save: SaveGame, olds: readonly OldToonState[]): { save: SaveGame; clashes: string[] } {
	const rows = [
		{ updated: time(save.updated_at), variables: save.variables },
		...olds.map((o) => {
			const s = isObject(o.state) ? o.state : {};
			// an old toon state's variables were that toon's: they keep their domain (`<toon>/x` – variables.ts)
			const own = Object.fromEntries(Object.entries(normalizeVariables(s.variables)).map(([k, v]) => [`${o.toon}/${k}`, v]));
			return { updated: time(typeof s.updated_at === 'string' ? s.updated_at : undefined), variables: own };
		})
	].sort((a, b) => a.updated - b.updated);
	const variables: Record<string, SaveGameValue> = {};
	const clashes = new Set<string>();
	for (const row of rows)
		for (const [k, v] of Object.entries(row.variables)) {
			if (k in variables && variables[k] !== v) clashes.add(k);
			variables[k] = v;
		}
	let merged = save;
	for (const o of olds) {
		const s = normalizeSaveGame(o.state, save.created_at);
		merged = mergeSaveGame({ ...merged, variables: merged.variables }, { ...s, map: emptyMapState() });
	}
	return { save: { ...merged, variables, updated_at: save.updated_at }, clashes: [...clashes].sort() };
}
