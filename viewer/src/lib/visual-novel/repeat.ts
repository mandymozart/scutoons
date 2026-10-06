import { eventHistoryKey, isDue, repeatOf, type RecurrenceEntry, type Repeat } from '@scutoons/content';
import type { GameContext } from '$lib/contexts/game.svelte';
import { pageView } from './page-view.ts';

/**
 * Whether a slice trigger's / a placed item's events run now (agents/EXPRESSIONS.md §5, the one recurrence model –
 * recurrence.ts `isDue`): every time (no `repeat`), once per page view, once for good, once per n minutes / hours /
 * days – decided from the reader's history (`event:<id>`, the game state – synced for members). Events that run as one
 * (the same trigger; an item's taps) share the decision: their newest entry counts. Events without ids (stored before
 * the step-3 creator; validation requires ids for "once for good" / "every n") count in `local` – once per page view at
 * most, as before.
 */
export interface RunMemory {
	/** ran on this page (events without ids) */
	ran: boolean;
	/** the page view `ran` belongs to */
	view?: string;
}

/** the history entries of `ids`, merged: how often the group ran, when last */
function entryOf(ids: readonly string[], game: GameContext): RecurrenceEntry | null {
	const entries = ids.flatMap((id) => game.state.history[eventHistoryKey(id)] ?? []);
	if (!entries.length) return null;
	return { n: Math.max(...entries.map((e) => e.n)), last: Math.max(...entries.map((e) => e.last)) };
}

export function isRunDue(rule: Repeat | undefined, ids: readonly string[], game: GameContext | undefined, local: RunMemory): boolean {
	const r = repeatOf(rule);
	if (r.mode === 'every') return true;
	const view = pageView();
	if (!ids.length || !game) return !(local.ran && local.view === view.id);
	return isDue(r, entryOf(ids, game), Date.now(), view);
}

/** A run starts: every event of it counts once (`count(id)` / `resolved(id)`; the next decision). */
export function recordRun(ids: readonly string[], game: GameContext | undefined, local: RunMemory) {
	local.ran = true;
	local.view = pageView().id;
	for (const id of ids) game?.recordVisit(eventHistoryKey(id));
}

/** the ids of events (those that have one) */
export const idsOf = (events: readonly { id?: string }[] | undefined): string[] =>
	(events ?? []).flatMap((e) => (typeof e?.id === 'string' && e.id ? [e.id] : []));
