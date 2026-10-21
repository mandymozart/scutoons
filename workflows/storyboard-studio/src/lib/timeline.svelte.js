// The studio's side of the shared Timeline (@scutoons/ui – no transport inside, research/TIMELINE.md §1). Owner
// 2026-10-19: "time is relative and only will play a role in the live performance mode or if videos get rendered as shot
// in the webtoon … depending on how long a camera is recorded and or trimmed to, thats the length of that clip for that
// shot". The clips of the PICKED shots in the step's view mode (timeline-lanes.js; the clips from GET /api/clips), the
// one playhead (ms), and the transport: Play runs the sequence – a video clip plays its <video> (ClipView; the clock
// while it plays), a take its camera in the 3D view (preview), a still waits its 5 s; a Take-mode take only its kept
// part. In Shot / Take mode the clip under the playhead makes its shot the current one (a seek on release, Play when it
// crosses – unsaved changes ask, a refusal stops Play); in the video modes the current shot stays. Play ends on a seek,
// a step / chapter / mode / picked-set change, another current shot than Play's, the timeline folding (App). No Space
// key (the score's pedal keys stay free). The trim (Take mode, research/TIMELINE.md §4 – way C): the in / out / slip of
// the shared Timeline and App's I / O keys write the take's in / out; while a trim runs the main view shows the in / out
// pair (`tl.pair`, TrimPair). The board's marks and its current card reach the lane as an override per shot (the
// status bullets, the AUDITION – audition.svelte.js laneOver; research/VARIANTS-CANVAS.md §3.1 Q3).
import { untrack } from 'svelte';
import { api, ch, notify, pickedShots, pickShot, studio, toon } from './studio.svelte.js';
import { activeTake, endPreview, hh, preview, saveDoc } from './handheld/handheld.svelte.js';
import { steps } from './steps.svelte.js';
import { out } from './output.svelte.js';
import { laneOver } from './audition.svelte.js';
import { clipAt, clipsLane, FRAME_MS, isVideo, kept, movedEdge, trimAt, trimPoint } from './timeline-lanes.js';

export const tl = $state({
	playhead: 0,
	zoom: 1,
	playing: false,
	/** GET /api/clips of the chapter: { [shot]: { take, rendered, generated } } */
	clips: {},
	/** when that GET was asked (performance.now()) – the board's marks win only when newer (audition.svelte.js laneOver) */
	clipsAt: 0,
	/** video lengths read from their metadata (url → ms) where the API had none */
	known: {},
	/** the in / out pair in the main view (TrimPair) while a trim runs and a moment after: { n (the gesture), id (the
	 *  shot), take, edge 'in' | 'out' | 'slip', in, out (ms of the take), live (the gesture still runs) } | null */
	pair: null,
	/** a trim gesture runs (its calls with final false came, the final one not yet): the timeline's reveal waits for its
	 *  end (Timeline.svelte) – a trim that picks its take's shot never moves the view under the dragging pointer */
	trimming: false
});
/** the main view's <video> while a video clip shows (ClipView) – the transport's clock (not state: an element) */
export const media = { video: null };

const followsShot = (mode) => mode === 'shot' || mode === 'take';
const inside = (it, t) => !!it && t >= it.start && t < it.start + it.length;
/** where Play / a pick puts the playhead in a clip: its start – a Take-mode take's in */
const startOf = (it) => it.start + (it.trim?.in ?? 0);

/** the current shot's active take as its takes file has it now (a trim shows while it is dragged), ms as clips.mjs */
function liveTake() {
	const take = hh.doc?.on ? activeTake() : null;
	const last = take?.samples?.at(-1)?.t;
	const length = Math.round(last * 1000);
	if (!(length > 0)) return null;
	// as clips.mjs activeTake: a missing in = 0, a missing out = the take's end
	const tin = Math.min(length, Math.max(0, Math.round((Number(take.in) || 0) * 1000)));
	const tout = Math.max(tin, Math.min(length, Math.round((Number.isFinite(Number(take.out ?? NaN)) ? Number(take.out) : last) * 1000)));
	return { id: take.id, in: tin, out: tout, length, kept: tout - tin };
}

/** the timeline in a view mode – call inside a $derived: { mode, lanes, items, span, probe, sel (the current's clip) } */
export function timelineOf(mode) {
	const id = studio.sel?.id;
	const live = id && hh.docShot === id && hh.doc ? { [id]: liveTake() } : {};
	const ids = pickedShots();
	// the board's marks (the status bullets) and the audition (the current shot's Generated clip = the current card)
	const { lane, span, probe } = clipsLane(ids, mode, tl.clips, tl.known, live, laneOver(ids, tl.clipsAt));
	return { mode, lanes: [lane], items: lane.items, span, probe, sel: lane.items.find((i) => i.id === id) ?? null };
}

/** the take moment (ms of the whole take) the 3D view shows for the clip under the playhead – only the current shot's
 *  active take, its takes file loaded; else null */
function takeMoment(cur) {
	const it = cur?.item;
	if (it?.clip.source !== 'take' || it.id !== studio.sel?.id || hh.docShot !== it.id || !hh.doc) return null;
	return activeTake()?.id === it.clip.take?.id ? it.clip.from + cur.off : null;
}

// --- the clips data ---------------------------------------------------------------------------------------------------
let lastError = '';
let loadedFor = '';
async function loadClips() {
	const chapter = studio.chapter;
	if (!chapter) return;
	try {
		const at = performance.now();
		const clips = await api('GET', `/api/clips?toon=${toon()}&ch=${ch()}`);
		if (studio.chapter === chapter) [tl.clips, tl.clipsAt, loadedFor] = [clips ?? {}, at, chapter];
		lastError = '';
	} catch (e) {
		if (studio.chapter === chapter) tl.clips = {};
		// one toast per kind of failure, not one per reload
		if (e?.message !== lastError) notify(`The clips: ${(lastError = e?.message ?? String(e))}`, 'error');
	}
}
const probing = new Set();
/** a video's length from its metadata, once per url (the API could not read it) */
function probe(url) {
	if (probing.has(url)) return;
	probing.add(url);
	const v = document.createElement('video');
	v.preload = 'metadata';
	v.muted = true;
	const done = () => { v.onloadedmetadata = v.onerror = null; v.removeAttribute('src'); v.load(); };
	v.onloadedmetadata = () => { if (Number.isFinite(v.duration) && v.duration > 0) tl.known = { ...tl.known, [url]: Math.round(v.duration * 1000) }; done(); };
	// a file that can't be read keeps its 5 s (ClipView says it can't be played)
	v.onerror = done;
	v.src = url;
}

/** App: the clips follow the chapter, the version (a generator finished, a save), the renders, the current shot and its
 *  takes; the playhead follows the current shot; the 3D view shows the take's moment under the playhead */
export function useTimeline(view, cur) {
	viewOf = view;
	let timer;
	$effect(() => {
		void [studio.chapter, studio.version, out.results, studio.sel?.id, hh.doc?.takes?.length, hh.doc?.active, hh.doc?.on];
		// a new chapter at once; else a little later – a trim's save (saveDoc later) is written first
		clearTimeout(timer);
		timer = setTimeout(loadClips, loadedFor === studio.chapter ? 500 : 0);
		return () => clearTimeout(timer);
	});
	$effect(() => view().probe.forEach(probe));
	// Play ends when what it plays changes under it – the step (its timeline may fold), the chapter, the mode, the picked
	// set – or another shot becomes current than the one Play crossed into (the list, the stepper, a note); before the
	// playhead's effect below, so that one puts the playhead on the new current shot
	const played = $derived(`${steps.current}|${studio.chapter}|${view().mode}|${view().items.map((i) => i.id)}`);
	$effect(() => {
		void played;
		untrack(() => { stop(); endPair(); tl.trimming = false; });
	});
	$effect(() => {
		const id = studio.sel?.id;
		untrack(() => {
			if (id !== asked) stop();
			if (id !== tl.pair?.id) endPair();
		});
	});
	// the current shot, the mode or the picked set changed (a pick in the list, the stepper, a click on a clip), or the
	// current's clip got another length (an audition – another card): the playhead to the current's clip – unless it
	// is on it already (a seek that picked it) or Play runs
	const key = $derived(`${studio.sel?.id}|${view().mode}|${view().items.map((i) => i.id)}|${view().sel?.start}|${view().sel?.length}`);
	$effect(() => {
		void key;
		untrack(() => {
			const s = view().sel;
			if (s && !tl.playing && !inside(s, tl.playhead)) tl.playhead = startOf(s);
		});
	});
	// a take under the playhead: its camera at that moment (as the old take lane – a press, a trim, Play); anything else
	// gives the camera back to the shot. The phone's live pose keeps it (stopLive gives it back). Only when the moment or
	// its take changes – a reload of the clips (a save) never takes the camera back after "Back to the shot camera".
	const moment = $derived(takeMoment(cur()));
	const whose = $derived(moment === null ? '' : `${studio.sel?.id}/${hh.doc?.active}`);
	$effect(() => {
		const [at] = [moment, whose];
		untrack(() => {
			cancelAnimationFrame(retry);
			if (hh.live) return;
			if (at !== null) hold(at / 1000);
			else if (hh.held) endPreview();
		});
	});
	$effect(() => () => { stop(); endPair(); tl.trimming = false; cancelAnimationFrame(retry); });
}

let retry = 0;
/** the take's camera at `s` in the 3D view – while the view rebuilds (a new shot) it takes no pose: again on the next
 *  frames until it holds (at most ≈ 15 s, the view's build limit) */
function hold(s) {
	let n = 0;
	const again = () => {
		preview(s);
		if (!hh.held && !hh.live && ++n < 900) retry = requestAnimationFrame(again);
	};
	again();
}
/** the header's "Back to the shot camera": the shot's own camera until the playhead moves again */
export function backToShot() {
	cancelAnimationFrame(retry);
	endPreview();
}

// --- what the timeline asks -------------------------------------------------------------------------------------------
export function seek(v, t, final) {
	stop();
	endPair();
	tl.playhead = t;
	if (!final || !followsShot(v.mode)) return;
	const it = clipAt(v.items, t)?.item;
	// a cancelled "Discard the changes?" keeps the shot – the playhead goes back to it
	if (it && it.id !== studio.sel?.id) pickShot(it.id).then((ok) => { if (!ok && v.sel) tl.playhead = startOf(v.sel); });
}

/** a click on a clip: its shot becomes the current one (in every mode – the panels edit it) */
export function select(v, laneId, id) {
	stop();
	endPair();
	pickShot(id);
}

let picking = null;
/** a trim (Take mode; the shared Timeline's ontrim, App's I / O): `k` = the kept part { in, out } (ms of the take), `edge`
 *  'in' / 'out' (the playhead on the moved point – the in, or the last kept frame before the out) or 'slip' (both moved,
 *  the length kept; the playhead on the new in) – the 3D view shows the take's camera there; a gesture = calls with
 *  `final` false, then one with true (the release, a cancel; a key: one call), the takes file written on it. Meanwhile the
 *  main view shows the in / out pair. A take of another shot first becomes the current one (its takes file loads – then
 *  the handles trim it). */
export function trim(v, laneId, id, k, final, edge) {
	stop();
	tl.trimming = !final;
	const it = v.items.find((i) => i.id === id);
	if (!it?.trim) return;
	// a caller without the edge (the shared Timeline before its 5th argument): the one that moved
	edge ??= movedEdge(it.trim, k);
	tl.playhead = trimPoint(it, k, edge, v.lanes.find((l) => l.id === laneId)?.step ?? FRAME_MS);
	if (id !== studio.sel?.id) {
		// a cancelled "Discard the changes?" keeps the shot – the playhead goes back to it (as a seek)
		if (picking !== id) pickShot((picking = id)).then((ok) => { if (!ok && v.sel) tl.playhead = startOf(v.sel); }).finally(() => (picking = null));
		return;
	}
	const take = activeTake();
	if (hh.docShot !== id || !take || take.id !== it.clip.take?.id) return;
	take.in = +(k.in / 1000).toFixed(3);
	take.out = +(k.out / 1000).toFixed(3);
	showPair(id, take.id, edge, k, final);
	if (final) saveDoc(true);
}

/** I / O (App's keys): the take under the playhead trimmed at the playhead – I: the frame under it is the first one kept,
 *  O: the last one kept (written at once, like a key on a handle). Only a take in Take mode (`v` null: the timeline is
 *  out of sight) – elsewhere a hint. */
export function trimAtPlayhead(v, edge) {
	const lane = v?.mode === 'take' ? v.lanes[0] : null;
	const at = lane && trimAt(v.items, tl.playhead, edge, lane.step ?? FRAME_MS);
	if (!at) return notify('I / O trim the take under the playhead – in Take mode, the timeline open.');
	trim(v, lane.id, at.item.id, { in: at.in, out: at.out }, true, edge);
}

// --- the in / out pair (TrimPair) -------------------------------------------------------------------------------------
/** one picture again this long after a trim's release (research/TIMELINE.md §4: "shortly after release") */
const PAIR_MS = 600;
let pairTimer = 0;
let gestures = 0;
/** a trim's call: the pair shows its values – the same gesture keeps its number (TrimPair keeps the other side's still),
 *  a new one (another edge, after a release, a key) gets a new one; the release lets it go PAIR_MS later */
function showPair(id, take, edge, k, final) {
	clearTimeout(pairTimer);
	const p = tl.pair;
	const same = p?.live && p.id === id && p.take === take && p.edge === edge;
	tl.pair = { n: same ? p.n : ++gestures, id, take, edge, in: k.in, out: k.out, live: !final };
	if (final) pairTimer = setTimeout(endPair, PAIR_MS);
}
/** back to one picture: a seek, a pick, Play, a step / mode / shot change (a gesture its lane lost on the way included) –
 *  it only writes (effects call it: GOTCHAS "an $effect that reads state it writes loops") */
export function endPair() {
	clearTimeout(pairTimer);
	tl.pair = null;
}

// --- the transport ----------------------------------------------------------------------------------------------------
let viewOf = () => null;
let raf = 0;
let last = 0;
/** a shot Play crossed into, while "Discard the changes?" asks (the clock waits); `asked` = the current shot Play
 *  started on / made current (another one ends Play); `runs` counts the playbacks */
let crossing = null;
let asked = null;
let runs = 0;

export const togglePlay = () => (tl.playing ? stop() : start());

function start() {
	const v = viewOf();
	if (!v?.items.length) return;
	endPair();
	tl.playhead = kept(v.items, tl.playhead >= v.span - 1 ? 0 : tl.playhead);
	tl.playing = true;
	asked = studio.sel?.id ?? null;
	runs++;
	last = performance.now();
	raf = requestAnimationFrame(tick);
}

export function stop() {
	if (!tl.playing) return;
	tl.playing = false;
	cancelAnimationFrame(raf);
	crossing = null;
	hh.playing = false;
}

/** the clock: a video clip's own time while it plays (or loads), else the frame time */
function clockAt(it, t, dt) {
	const el = media.video;
	if (!isVideo(it.clip.source) || el?.dataset.url !== it.clip.url || el.error) return t + dt;
	if (el.ended) return it.start + it.length;
	if (!el.paused || el.readyState < 2) return it.start + Math.min(el.currentTime * 1000, it.length);
	return t + dt;
}

function tick(now) {
	if (!tl.playing) return;
	const dt = now - last;
	last = now;
	raf = requestAnimationFrame(tick);
	if (crossing) return;
	const v = viewOf();
	const c = v && clipAt(v.items, tl.playhead);
	if (!c) return stop();
	const t = kept(v.items, clockAt(c.item, tl.playhead, dt));
	// the phone's pose rests while Play runs (handheld.svelte.js onPose)
	hh.playing = true;
	if (t >= v.span) {
		tl.playhead = v.span;
		return stop();
	}
	tl.playhead = t;
	const it = clipAt(v.items, t)?.item;
	if (!it || !followsShot(v.mode) || it.id === studio.sel?.id) return;
	const run = runs;
	crossing = asked = it.id;
	pickShot(it.id).then((ok) => {
		if (run !== runs) return; // stopped and started again meanwhile
		crossing = null;
		last = performance.now();
		if (ok) return;
		// refused: Play ends on the shot that stays – the main view shows it again (as a refused seek)
		stop();
		const s = viewOf()?.sel;
		if (s) tl.playhead = startOf(s);
	});
}
