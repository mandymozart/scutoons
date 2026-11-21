// The shapes of deck-core's history rules (#687) as the wasm hands them over (deck/core/src/history.rs); the rules
// themselves are the core's – these are types only.

/** one saved version as the core reads it */
export interface HistorySaved {
	id: number;
	author: string | null;
	at_ms: number;
	message: string | null;
}

/** versions `first..last` saved together; `before` = the version they changed (null: nothing before it in the list) */
export interface HistoryStep {
	first: number;
	last: number;
	before: number | null;
	author: string | null;
	at_ms: number;
	count: number;
}

export type HistoryLineKind = 'added' | 'removed' | 'changed';

export interface HistoryLine {
	place: string;
	kind: HistoryLineKind;
	before: string;
	after: string;
}

/** a change in words: its first places and how many more there are */
export interface HistoryDescription {
	lines: HistoryLine[];
	more: number;
}

/** the reverted document, how many places went back, the places changed since (kept as they are) */
export interface HistoryRevert<T = unknown> {
	doc: T;
	applied: number;
	kept: string[];
}

/** what going back to a version undoes: the later saves, the other people among them */
export interface HistoryBackTo {
	later: number;
	others: string[];
}

/** deck-core `history` through the wasm */
export interface HistoryView {
	steps(versions: HistorySaved[]): HistoryStep[];
	describe(before: unknown, after: unknown): HistoryDescription;
	revert<T>(before: T, after: T, current: T): HistoryRevert<T>;
	backTo(versions: HistorySaved[], point: number, me: string): HistoryBackTo;
}
