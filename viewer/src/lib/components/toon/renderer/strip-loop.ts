/**
 * The endless band's geometry and motion (strip-endless.svelte; BACKLOG 10.100, TEXT-ACROSS-SLICES.md §8) – pure, no
 * DOM. The frames sit side by side in one track moved by a single translate; nothing is cloned and no node moves: a
 * frame that has left the window on one side is put one turn T (the frames' summed width) further – `left: k·T`, not a
 * transform (that would be a stacking context covering crossing boxes) – while it and its boxes' overhang are off screen
 * (the looper technique of Embla, no library code). Everything lands on the device's pixel grid: with T fractional
 * (2317.78 px at 1440 × 900) each full turn shifted the picture 1–3 device px, and a wrap decided on the unrounded
 * position left a 0.25 px sliver of ink at the screen's edge (measured, 3 engines). All values are px – the band writes
 * them as rem.
 */

/** the frames in a row at the band's height – starts, widths and the turn on the device's pixel grid */
export interface StripLayout {
	/** the band's height, px */
	height: number;
	/** each frame's left in the unwrapped track, px */
	starts: number[];
	/** each frame's width (to the next frame's start), px */
	widths: number[];
	/** one turn: the frames side by side, px – a whole number of device px, so k·T is too */
	turn: number;
}

/** how far a frame's boxes reach past its left / right edge, px (0 = inside the frame) */
export interface Overhang {
	left: number;
	right: number;
}

/** loops in the whole band · loops in a narrowed window · whole and still */
export type StripMode = 'loop' | 'narrowed' | 'still';

export interface StripFit {
	mode: StripMode;
	/** the window's width, px: the band's, or narrowed so any frame with its overhang fits off screen */
	window: number;
	/** the window's left in the band (narrowed: centred on ink), px */
	inset: number;
}

export interface Placement {
	/** the track's translate, px – on the device grid, within one turn */
	x: number;
	/** per frame: how many turns it is put further (`left: k·T`) */
	turns: number[];
	/** per frame: its left edge in the window, px */
	lefts: number[];
	/**
	 * per frame: its overlay plane (`--plane`), 1 … N from the window's left – between two slices the later one on screen
	 * paints on top, at the wrap seam too (frame N on the left of frame 1); inside the band this is DOM order
	 */
	planes: number[];
}

/** a position over time: a glide after a fling, or an ease to a frame edge */
export interface Motion {
	/** where it comes to rest */
	readonly end: number;
	/** an ease (keys, focus): a further key press steps on from its end; a glide is simply stopped */
	readonly ease: boolean;
	/** where it is at `now` (ms, a rAF stamp or performance.now()) and whether it has come to rest */
	at(now: number): { x: number; done: boolean };
}

/** a pointer position, for the release velocity */
export interface Sample {
	/** ms */
	t: number;
	/** px */
	x: number;
}

/** the glide's time constant, ms: x₀ + vτ(1 − e^(−t/τ)) */
export const GLIDE_TAU = 325;
/** the release velocity comes from the pointer's last 100 ms */
export const VELOCITY_WINDOW = 100;
/** a pointer resting this long (ms) before it lets go doesn't fling */
const RESTED = 50;
/** sparse moves (a janky phone, a dropped frame: 50–60 ms apart): the move before the last still counts this close, ms */
const PAIR_GAP = 150;
/** slower releases (px/ms) don't glide */
const MIN_SPEED = 0.05;
/** a glide ends with less than this left, px (its last step); a position this close to a frame edge is on it */
const REST = 0.5;
/** keys and focus ease to a frame edge in this time, ms (ease-out cubic) */
export const EASE_MS = 300;

/** round to the device's pixel grid */
export const snap = (px: number, dpr: number) => Math.round(px * dpr) / dpr;
const mod = (a: number, n: number) => ((a % n) + n) % n;

/** the frames from their width / height ratios at the band's height: every start, width and the turn on the device grid */
export function frameLayout(ratios: number[], height: number, dpr: number): StripLayout {
	const starts: number[] = [];
	let sum = 0;
	for (const ratio of ratios) {
		starts.push(snap(sum * height, dpr));
		sum += ratio > 0 ? ratio : 0;
	}
	const turn = snap(sum * height, dpr);
	const widths = starts.map((start, i) => (starts[i + 1] ?? turn) - start);
	return { height, starts, widths, turn };
}

/** the widest frame with its boxes' overhang, px – what must fit off screen for it to wrap unseen */
export function widest(layout: StripLayout, overhang: Overhang[] = []): number {
	return layout.widths.reduce((max, w, i) => Math.max(max, w + (overhang[i]?.left ?? 0) + (overhang[i]?.right ?? 0)), 0);
}

/**
 * Does the band loop at this width? In screen heights: the window (band width ÷ band height) must be at most
 * Σratio − max(ratio + overhang) – the four ch3–5 groups 2.77 − 0.46 = 2.31 : 1, so portrait phones and 1440 × 900
 * (1.72) loop. Wider (a landscape phone ≈ 2.6, a 1366 × 625 laptop window 2.44, 21:9) the window narrows to
 * T − max(W + overhang), centred on ink, and still loops; T within the band (or a single frame): whole and still.
 */
export function stripFit(layout: StripLayout, width: number, dpr: number, overhang: Overhang[] = []): StripFit {
	const room = layout.turn - widest(layout, overhang);
	if (layout.turn <= width || room <= 0) return { mode: 'still', window: width, inset: 0 };
	if (width <= room) return { mode: 'loop', window: width, inset: 0 };
	const window = Math.floor(room * dpr) / dpr;
	return { mode: 'narrowed', window, inset: snap((width - window) / 2, dpr) };
}

/**
 * Where the track and each frame go for `offset` (px the picture has moved right; any number – it repeats every turn).
 * A frame is placed so its right extent (edge + overhang) lies within [0, T) of the window's left: it wraps as that
 * extent crosses the window's left edge, when its left extent is ≥ T − max(W + overhang) ≥ the window, so off screen
 * both times. Decided on the rounded track position – the one drawn. Still: centred, nothing wraps.
 */
export function placeFrames(offset: number, layout: StripLayout, fit: StripFit, dpr: number, overhang: Overhang[] = []): Placement {
	const { starts, widths, turn } = layout;
	if (fit.mode === 'still' || turn <= 0) {
		const x = snap((fit.window - turn) / 2, dpr);
		return { x, turns: starts.map(() => 0), lefts: starts.map((start) => start + x), planes: starts.map((_, i) => i + 1) };
	}
	let x = snap(mod(snap(offset, dpr), turn), dpr);
	if (x >= turn) x -= turn;
	const turns: number[] = [];
	const lefts: number[] = [];
	starts.forEach((start, i) => {
		const reach = widths[i] + (overhang[i]?.right ?? 0);
		const k = Math.round((mod(start + x + reach, turn) - reach - start - x) / turn);
		turns.push(k);
		lefts.push(start + k * turn + x);
	});
	const order = lefts.map((_, i) => i).sort((a, b) => lefts[a] - lefts[b]);
	const planes: number[] = [];
	order.forEach((i, rank) => (planes[i] = rank + 1));
	return { x, turns, lefts, planes };
}

/** a resize keeps the place: the same share of a turn (stop a running motion first, or it writes the old scale back) */
export function keepPlace(offset: number, oldTurn: number, newTurn: number): number {
	return oldTurn > 0 && newTurn > 0 ? (offset * newTurn) / oldTurn : 0;
}

/**
 * The release velocity, px/ms: the pointer's way over its last 100 ms – with sparse moves (only the last one in there)
 * from the move before it when that is < 150 ms earlier (a swipe with moves 50 ms apart lost its glide – measured). 0
 * when it rested before letting go or there is no earlier move.
 */
export function releaseVelocity(samples: Sample[], now: number): number {
	const last = samples[samples.length - 1];
	if (!last || now - last.t >= RESTED) return 0;
	const earlier = samples.slice(0, -1).filter((s) => s.t < last.t);
	const first = earlier.find((s) => now - s.t <= VELOCITY_WINDOW) ?? earlier.findLast((s) => last.t - s.t <= PAIR_GAP);
	return first ? (last.x - first.x) / (last.t - first.t) : 0;
}

/**
 * A fling's glide from `from` at `velocity` px/ms, started at `start` ms: x₀ + vτ(1 − e^(−t/τ)), time-based (the same way
 * at 60 and 120 Hz, a blocked main thread keeps the distance), no snap. |v| ≤ T/τ – one turn per fling at most. t is
 * clamped ≥ 0: rAF stamps can precede the start (Chromium / Firefox), and the first frame stepped backwards. null =
 * too slow to glide.
 */
export function glide(from: number, velocity: number, start: number, turn: number): Motion | null {
	const cap = Math.max(0, turn) / GLIDE_TAU;
	const v = Math.min(cap, Math.max(-cap, velocity));
	if (!(Math.abs(v) >= MIN_SPEED)) return null;
	const distance = v * GLIDE_TAU;
	return {
		end: from + distance,
		ease: false,
		at(now) {
			const rest = distance * Math.exp(-Math.max(0, now - start) / GLIDE_TAU);
			return Math.abs(rest) < REST ? { x: from + distance, done: true } : { x: from + distance - rest, done: false };
		}
	};
}

/** an ease-out from `from` to `to` (keys, focus), started at `start` ms, time-based like the glide */
export function ease(from: number, to: number, start: number, ms = EASE_MS): Motion {
	return {
		end: to,
		ease: true,
		at(now) {
			const k = ms > 0 ? Math.min(1, Math.max(0, now - start) / ms) : 1;
			return { x: from + (to - from) * (1 - (1 - k) ** 3), done: k >= 1 };
		}
	};
}

/**
 * The next frame edge (a frame's left at the window's left) from `base` – the running ease's end, so presses add up:
 * +1 (→) brings the next frame in (the picture moves left), −1 (←) the previous one
 */
export function stepTarget(base: number, direction: 1 | -1, layout: StripLayout): number {
	if (layout.turn <= 0) return base;
	const edges = edgesNear(base, layout);
	const next = direction > 0 ? edges.reverse().find((e) => e < base - REST) : edges.find((e) => e > base + REST);
	return next ?? base;
}

/** the frame edges (offsets with a frame's left at the window's left) of the turns around `base`, ascending */
function edgesNear(base: number, layout: StripLayout): number[] {
	const { starts, turn } = layout;
	const k0 = Math.floor(-base / turn);
	const edges: number[] = [];
	// 0 − …, not −(…): no −0 for the first frame's edge
	for (let k = k0 - 1; k <= k0 + 2; k++) for (const start of starts) edges.push(0 - (start + k * turn));
	return edges.sort((a, b) => a - b);
}

/** the frame edge nearest `base` (a tie goes right: the earlier frame) */
export function nearestEdge(base: number, layout: StripLayout): number {
	if (layout.turn <= 0 || !layout.starts.length) return base;
	return edgesNear(base, layout).reduce((best, e) => (Math.abs(e - base) <= Math.abs(best - base) ? e : best));
}

/** the frame at the window's left for `offset`: the one whose edge is nearest (any offset – it repeats every turn) */
export function indexAt(offset: number, layout: StripLayout): number {
	const { starts, turn } = layout;
	if (turn <= 0 || !starts.length) return 0;
	const x = mod(-nearestEdge(offset, layout), turn);
	const i = starts.findIndex((start) => Math.abs(start - x) < REST);
	return i < 0 ? 0 : i;
}

/** a whole drag: its sideways way (px, right = +) and how long it took (ms) */
export interface Swipe {
	dx: number;
	ms: number;
}

/** a release at least this fast (px/ms) goes on to the next frame edge the way it points */
export const SWIPE_SPEED = 0.3;
/** a swipe shorter than this (ms) goes on to the next frame edge the way it went – Swiper's short swipe */
export const SHORT_SWIPE_MS = 300;

/**
 * Where a paged band comes to rest after a drag (the home slider – components/home/slides-pager.ts): the next frame
 * edge the way the release points when it was a fling (≥ 0.3 px/ms) or a short swipe (< 300 ms – a quick flick moves on
 * even when it went less than half a frame, and even when it rested before letting go), else the nearest edge. A
 * wheel burst (no swipe, no velocity) settles on the nearest edge.
 */
export function settleTarget(offset: number, layout: StripLayout, velocity = 0, swipe?: Swipe): number {
	const way = Math.abs(velocity) >= SWIPE_SPEED ? Math.sign(velocity) : swipe && swipe.ms < SHORT_SWIPE_MS ? Math.sign(swipe.dx) : 0;
	// the picture moving right (+) brings the previous frame in
	return way ? stepTarget(offset, way > 0 ? -1 : 1, layout) : nearestEdge(offset, layout);
}

/** equal pages, `perView` side by side with `gap` px between them, in a window `width` px wide */
export interface PageLayout {
	/** one page plus its gap, px – on the device grid, rounded down so the last page in view is never clipped */
	pitch: number;
	layout: StripLayout;
	/** loops with a page more than fit in view; with fewer it is whole and still */
	fit: StripFit;
}

/**
 * The home slider's pages on the band's geometry: `count` frames one pitch wide (the gap inside each, after its page),
 * so the gap after the last page in view falls outside the window. The layout options (per view, gap) are the
 * component's – this only does the arithmetic.
 */
export function pageLayout(count: number, perView: number, width: number, gap: number, dpr: number): PageLayout {
	const per = Math.max(1, Math.floor(perView));
	const pitch = Math.max(0, Math.floor(((width + gap) / per) * dpr) / dpr);
	const layout = frameLayout(Array.from({ length: Math.max(0, count) }, () => 1), pitch, dpr);
	const fit: StripFit = count > per && pitch > 0 ? stripFit(layout, width, dpr) : { mode: 'still', window: width, inset: 0 };
	return { pitch, layout, fit };
}

/**
 * Keyboard focus on a frame that isn't whole in the window: the offset that brings it in by the shortest way (its left
 * edge first when it is wider than the window); null when it shows
 */
export function revealTarget(offset: number, frame: number, placement: Placement, layout: StripLayout, window: number): number | null {
	const left = placement.lefts[frame];
	const right = left + layout.widths[frame];
	if (left >= -REST && right <= window + REST) return null;
	return left < 0 || right - left >= window ? offset - left : offset - (right - window);
}
