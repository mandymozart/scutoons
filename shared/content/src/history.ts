// The reader's history (agents/EXPRESSIONS.md §5, DATA.md "Game state"): ONE store in the toon's game state
// (`scutoons_game_<toon>.history`, synced for members with the variables and the progress) for every "how often / when
// last" a reader has met something. Step 2: text visits (ink sequences / cycles / once-only lists – `{a|b|c}`); step 3
// adds events (`resolved(id)` / `count(id)`, repeat modes) and placed items (respawn) under their own key kinds, with the
// same entry shape. A missing `history` (states from before) is an empty one.

/** what the reader met: how many times, first and last time (ms since 1970) */
export interface HistoryEntry {
	n: number;
	first: number;
	last: number;
}

/** key → entry; keys are `<kind>:<identity>` – `text:` (a list in a text), later `event:` / `item:` */
export type ReaderHistory = Record<string, HistoryEntry>;

/** the most entries kept: beyond it the longest-unseen go (~90 bytes each: a game state stays far below the server's 256 KB) */
export const HISTORY_LIMIT = 1000;

/**
 * The key of one list (`{a|b|c}`, its `index` in the text) of one text: `doc` = the document (`chapter:<toon>:<n>`, a
 * scene key …), `text` = the text's stable identity inside it (a text box id + line, a script event's path) – the same
 * text in every language counts as one (a reader switching languages goes on where they were).
 */
export function textHistoryKey(doc: string, text: string, list: number): string {
	return `text:${doc}/${text}#${list}`;
}

/** a stored history as it can be used: entries with a count, anything else dropped */
export function normalizeHistory(raw: unknown): ReaderHistory {
	const out: ReaderHistory = {};
	if (!raw || typeof raw !== 'object' || Array.isArray(raw)) return out;
	for (const [key, value] of Object.entries(raw as Record<string, unknown>)) {
		const e = value as Partial<HistoryEntry> | null;
		if (!e || typeof e !== 'object' || !Number.isFinite(e.n) || (e.n as number) < 0) continue;
		const last = Number.isFinite(e.last) ? (e.last as number) : 0;
		out[key] = { n: Math.floor(e.n as number), first: Number.isFinite(e.first) ? (e.first as number) : last, last };
	}
	return out;
}

/** how many times the reader met `key` (0 = never) */
export function historyCount(history: ReaderHistory | null | undefined, key: string): number {
	const e = history?.[key];
	return e && Number.isFinite(e.n) ? e.n : 0;
}

/** one more time for `key` at `now` (changes `history`; beyond HISTORY_LIMIT the longest-unseen entries go) */
export function recordHistory(history: ReaderHistory, key: string, now: number): HistoryEntry {
	const e = history[key];
	const next = e ? { n: e.n + 1, first: e.first, last: now } : { n: 1, first: now, last: now };
	history[key] = next;
	const keys = Object.keys(history);
	if (keys.length > HISTORY_LIMIT) {
		keys.sort((a, b) => history[a].last - history[b].last);
		for (const old of keys.slice(0, keys.length - HISTORY_LIMIT)) if (old !== key) delete history[old];
	}
	return next;
}
