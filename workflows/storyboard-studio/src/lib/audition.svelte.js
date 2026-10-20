// The audition's state (agents/research/VARIANTS-CANVAS.md §3.1 Q3; the rules: audition.js, node tests). `aud.on`: the
// board is open – or Play switched the main from the board to the Viewport and still runs (Play shows the audition
// among the picked shots' clips; V goes back to the board); Play's end, the board closed by hand (V, the switch, a
// source's Load) or another step ends it. Not remembered. The clips lane takes an override per picked shot (laneOver – timeline.svelte.js timelineOf):
// the marks the board knows (the status bullets) and, while the audition holds, the current shot's pick.
import { untrack } from 'svelte';
import { board } from './board.svelte.js';
import { ch, studio } from './studio.svelte.js';
import { setEditor, steps } from './steps.svelte.js';
import { auditionPick, currentCard, holds, marksOf } from './audition.js';

export const aud = $state({ on: false });
/** the next editor change is Play's (App) – the audition holds through it */
let byPlay = false;
/** Play from the board carries the audition (until it ends) */
let carried = false;

/** Play needs a picture (App): the main to the Viewport – from the board, the audition stays */
export function playSwitch(open) {
	byPlay = !!open;
	setEditor('viewport');
}

/** App: the audition follows the board (`isOpen()`), Play (`isPlaying()`) and the step – it only writes `aud.on`
 *  (GOTCHAS: an $effect that reads state it writes loops) */
export function useAudition(isOpen, isPlaying) {
	let step = steps.current;
	$effect(() => {
		const open = isOpen();
		const playing = isPlaying();
		const now = steps.current;
		untrack(() => {
			const next = holds({ open, was: aud.on, byPlay, carried, playing, stepped: now !== step });
			[step, carried, byPlay] = [now, next.carried, false];
			if (aud.on !== next.on) aud.on = next.on;
		});
	});
}

/** the clips lane's override per picked shot (timeline-lanes.js clipsLane `over`): { [shot]: { marks, pick? } } – read
 *  inside a $derived (the board's records, its current card, the current shot, `aud.on`) */
export function laneOver(ids) {
	// the board keeps its records while it is closed: only this chapter's files count (another chapter's are stale)
	const at = studio.chapter ? `/out/${ch()}/` : null;
	const own = (id) => (at ? (board.records[id] ?? []).filter((r) => r?.url?.startsWith(at)) : []);
	const over = {};
	for (const id of ids) {
		const records = own(id);
		if (records.length) over[id] = { marks: marksOf(records) };
	}
	const shot = studio.sel?.id;
	// the card the board shows as current – none clicked there: the shot's chosen (Board's `cur`)
	const pick = aud.on && shot ? auditionPick({ shot, current: currentCard({ shot, current: board.current, records: own(shot) }), records: own(shot) }) : null;
	if (pick) over[shot] = { ...over[shot], pick };
	return over;
}
