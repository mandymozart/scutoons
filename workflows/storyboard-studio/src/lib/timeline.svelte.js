// The studio's side of the shared Timeline (@scutoons/ui – the view has no transport, research/TIMELINE.md): the lanes
// for the step, the one playhead (ms on the chapter's axis), the zoom per chapter, and what a seek / select / trim does –
// a seek in the Shots lane picks the shot under the playhead on release (unsaved changes ask once), a seek or trim on the
// Take lane previews that moment in the 3D view, the take's Play moves the same playhead.
import { untrack } from 'svelte';
import { itemAt } from '@scutoons/ui/timeline';
import { allShots, pickShot, studio } from './studio.svelte.js';
import { activeTake, hh, pause, play, preview, saveDoc } from './handheld/handheld.svelte.js';
import { revealOf, shotsLane, takeLane } from './timeline-lanes.js';

export const tl = $state({ playhead: 0, zoom: 1 });

/** the zoom per chapter and step (the Shots step's overview, the Handheld step's close-up on the take) */
const zoomKey = (step) => `storyboard-studio:timeline-zoom:${studio.chapter}:${step}`;
export function loadZoom(step) {
	try { tl.zoom = Math.max(1, Number(localStorage.getItem(zoomKey(step))) || 1); } catch { tl.zoom = 1; }
}
/** remembers the zoom (writes no state, so an $effect calling it does not loop) */
export function keepZoom(z, step) {
	try { if (z > 1) localStorage.setItem(zoomKey(step), String(+z.toFixed(3))); else localStorage.removeItem(zoomKey(step)); } catch { /* not remembered */ }
}

/** the lanes etc. for a step (`withTake`: the Handheld step) – call inside a $derived */
export function timelineOf(withTake) {
	const shots = allShots().map((s) => ({ id: s.id, seconds: s.seconds, scene: s.scene, review: s.shot.review?.status }));
	const { lane, markers, span } = shotsLane(shots);
	const sel = lane.items.find((i) => i.id === studio.sel?.id) ?? null;
	// the take only while the shot's handheld camera is on (as the Handheld panel's takes – the old take bar too)
	const take = withTake && sel && hh.doc?.on && hh.docShot === sel.id ? takeLane(activeTake(), sel.start) : null;
	return { lanes: take ? [lane, take] : [lane], markers, span: Math.max(span, take ? take.items[0].start + take.items[0].length : 0), sel, take, reveal: revealOf(sel, take) };
}

/** the playhead follows the selection – to the shot's start when it is outside the shot (a list / stepper pick; a seek
 * that picked the shot stays where it was released, so ← → on the ruler keep their step) – and the take's preview
 * (Play, a handle) */
export function followPlayhead(view) {
	$effect(() => {
		const id = view().sel?.id;
		if (id) untrack(() => { const v = view(); if (!inItem(v.sel, tl.playhead) && !inTake(v, tl.playhead)) tl.playhead = v.sel.start; });
	});
	$effect(() => {
		const v = view();
		if (v.take && hh.docShot === v.sel?.id) { const at = v.take.items[0].start + hh.playhead * 1000; untrack(() => (tl.playhead = at)); }
	});
}
const inTake = (v, t) => !!v.take && t >= v.take.items[0].start && t <= v.take.items[0].start + v.take.items[0].length;
const inItem = (it, t) => !!it && t >= it.start && t < it.start + it.length; // as itemAt

export function seek(v, t, final) {
	tl.playhead = t;
	if (inTake(v, t)) {
		pause();
		preview((t - v.take.items[0].start) / 1000);
		return;
	}
	if (!final) return;
	const hit = itemAt(v.lanes[0].items, t);
	// a cancelled "Discard the changes?" keeps the shot – the playhead goes back to it (its take's moment, else its start)
	if (hit && hit.id !== studio.sel?.id) pickShot(hit.id).then((ok) => { if (!ok && v.sel) tl.playhead = v.take ? v.take.items[0].start + hh.playhead * 1000 : v.sel.start; });
}

export function select(laneId, id) {
	if (laneId === 'shots') pickShot(id);
}

export function trim(v, laneId, id, kept, final) {
	const take = activeTake();
	if (laneId !== 'take' || !take || take.id !== id) return;
	const moved = kept.in !== Math.round(take.in * 1000) ? kept.in : kept.out;
	take.in = +(kept.in / 1000).toFixed(3);
	take.out = +(kept.out / 1000).toFixed(3);
	pause();
	preview(moved / 1000);
	if (final) saveDoc(true);
}

export const togglePlay = () => (hh.playing ? pause() : play());
