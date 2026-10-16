/**
 * The endless band's drag hint (owner 2026-10-16, Q10, TEXT-ACROSS-SLICES.md §8: "a nudge is fine. Also use FX … It is
 * deactivated when hints are turned off"): once per band and page view, when the band has been in view for a moment
 * and the reader's "Show hints" is on, a hint line sweeps across its first slice and the screen bounces sideways
 * (strip-nudge-cue.ts draws it). This is the when – pure, timers handed in, so the rules are unit-tested:
 * - only while the band is in view (≥ NUDGE_VISIBLE of it) for NUDGE_DWELL ms, "Show hints" on, the story not blocking
 *   the page (`html[data-vn-blocking]`), the band able to move (not whole and still) and not moving (a drag, a glide,
 *   an ease) – busy: wait another dwell
 * - hints arriving late (the member's answer after the band showed) or the story letting go: armed again while in view
 * - once: played, cancelled or pre-empted, it never comes back in this page view
 * - the reader moving the band (a drag, a sideways wheel, ← →) cancels a running nudge and spends a waiting one – who
 *   moved it needs no hint; a finger or button going down on it cancels a running one too, and makes a waiting one
 *   wait a whole dwell again (a vertical swipe over the band scrolls the page – on a phone the band is nearly the whole
 *   screen, the finger is on it as it arrives); while a finger stays on it the band counts as busy
 * - hints switched off, the story blocking or the band leaving the view while it plays: cancelled
 */

/** the share of the band on screen that counts as "in view" */
export const NUDGE_VISIBLE = 0.6;
/** how long the band stays in view before the nudge, ms (scrolling past it shows nothing) */
export const NUDGE_DWELL = 700;
/** the hint line's sweep across the first slice, ms */
export const NUDGE_SWEEP_MS = 1100;
/** the screen's sideways bounce at the sweep's end, ms */
export const NUDGE_BOUNCE_MS = 520;
/** reduced motion: the line stands still and fades in and out, ms – no sweep, no bounce */
export const NUDGE_STILL_MS = 1400;

export type NudgeState = 'idle' | 'armed' | 'playing' | 'done';

/** the band's and the page's side, read when needed */
export interface NudgeEnv {
	/** "Show hints" (account page) is on */
	hints(): boolean;
	/** the story blocks the page */
	blocked(): boolean;
	/** the band can move at all (not whole and still) */
	movable(): boolean;
	/** a drag, glide or ease moves the band */
	busy(): boolean;
	/** plays the cue and calls `end` when it is over by itself; returns its cancel */
	play(end: () => void): () => void;
	setTimer(fn: () => void, ms: number): unknown;
	clearTimer(handle: unknown): void;
}

export interface Nudge {
	readonly state: NudgeState;
	/** the band came into view / left it */
	view(inView: boolean): void;
	/** "Show hints" or the story's blocking changed */
	changed(): void;
	/** a pointer went down on the band (maybe only to scroll the page) */
	poke(): void;
	/** the reader moved the band */
	touch(): void;
	/** the band goes away */
	destroy(): void;
}

export function nudgeController(env: NudgeEnv): Nudge {
	let state: NudgeState = 'idle';
	let inView = false;
	let timer: unknown = null;
	let cancel: (() => void) | null = null;

	const allowed = () => env.hints() && !env.blocked() && env.movable();
	function disarm() {
		if (timer !== null) env.clearTimer(timer);
		timer = null;
	}
	function arm() {
		disarm();
		state = 'armed';
		timer = env.setTimer(fire, NUDGE_DWELL);
	}
	function fire() {
		timer = null;
		if (state !== 'armed') return;
		if (!inView || !allowed()) return void (state = 'idle');
		// moving: the reader is busy with it – once it rests and stays in view
		if (env.busy()) return arm();
		state = 'playing';
		cancel = env.play(() => {
			cancel = null;
			if (state === 'playing') state = 'done';
		});
	}
	/** over for this page view – a running cue stops */
	function finish() {
		disarm();
		const stop = cancel;
		cancel = null;
		state = 'done';
		stop?.();
	}
	function update() {
		if (state === 'done') return;
		if (state === 'playing') {
			if (!inView || !allowed()) finish();
			return;
		}
		if (inView && allowed()) {
			if (state === 'idle') arm();
		} else if (state === 'armed') {
			disarm();
			state = 'idle';
		}
	}

	return {
		get state() {
			return state;
		},
		view(next) {
			inView = next;
			update();
		},
		changed: update,
		poke() {
			if (state === 'playing') finish();
			else if (state === 'armed') arm();
		},
		touch: finish,
		destroy: finish
	};
}

/** a frame's place in the window, px: its left edge and width */
export interface FrameBox {
	l: number;
	w: number;
}

/**
 * The frame the line sweeps across: the band's first slice (frame 0) when at least half of what can show of it is in the
 * window – the band starts there; else (the page arrived at another slice) the frame with the most of it on screen
 */
export function nudgeFrame(frames: FrameBox[], windowWidth: number): number {
	const shown = (f: FrameBox) => {
		const [lo, hi] = shownPart(f, windowWidth);
		return Math.max(0, hi - lo);
	};
	if (!frames.length) return -1;
	if (shown(frames[0]) >= Math.min(frames[0].w, windowWidth) / 2) return 0;
	let best = 0;
	frames.forEach((f, i) => shown(f) > shown(frames[best]) && (best = i));
	return shown(frames[best]) > 0 ? best : -1;
}

/** the part of a frame inside the window, px from its left edge: `[lo, hi]` (hi ≤ lo – none) – the line spans it */
export function shownPart(f: FrameBox, windowWidth: number): [number, number] {
	return [Math.max(0, -f.l), Math.min(f.w, windowWidth - f.l)];
}
