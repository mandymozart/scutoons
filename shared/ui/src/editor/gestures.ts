/**
 * Gestures – the pure part. A fling: a panel or a tab let go while the pointer still moves fast toward a screen edge, and
 * would pass it shortly, closes (a slow drag keeps the stage's drop zones). A swipe up: a press at the bottom edge (or on
 * the status bar) moved far enough upward opens the user menu.
 */
import type { Rect, Size } from './floats.ts';

/** One pointer position and when (ms) */
export interface Sample {
	x: number;
	y: number;
	t: number;
}

export type FlingEdge = 'left' | 'right' | 'top' | 'bottom';

export interface FlingRules {
	/** the least speed of a fling (px per ms) */
	speed: number;
	/** the samples that count: the last this many ms */
	span: number;
	/** the pointer must have moved this recently before the release (ms), else it rested: no fling */
	rest: number;
	/** where the pointer would be this many ms after the release decides whether it passes the edge */
	ahead: number;
}

export const FLING: FlingRules = { speed: 1, span: 80, rest: 60, ahead: 160 };

/** The samples kept: the newest, and those of the last `keep` ms before it */
export const addSample = (samples: readonly Sample[], s: Sample, keep = 120): Sample[] => [...samples.filter((o) => s.t - o.t <= keep && o.t <= s.t), s];

/** The pointer's speed (px per ms) over the last `span` ms of the samples */
export const velocity = (samples: readonly Sample[], span = FLING.span): { vx: number; vy: number } => {
	const last = samples.at(-1);
	if (!last) return { vx: 0, vy: 0 };
	const first = samples.find((s) => last.t - s.t <= span) ?? last;
	const dt = last.t - first.t;
	return dt > 0 ? { vx: (last.x - first.x) / dt, vy: (last.y - first.y) / dt } : { vx: 0, vy: 0 };
};

/**
 * The edge a release at `now` flings toward, or null: fast enough, mostly toward that edge, the pointer still moving,
 * and where it would be `ahead` ms later lies past the edge
 */
export const flingEdge = (samples: readonly Sample[], now: number, win: Size, rules: FlingRules = FLING): FlingEdge | null => {
	const last = samples.at(-1);
	if (!last || now - last.t > rules.rest) return null;
	const { vx, vy } = velocity(samples, rules.span);
	if (Math.hypot(vx, vy) < rules.speed) return null;
	const x = last.x + vx * rules.ahead;
	const y = last.y + vy * rules.ahead;
	if (Math.abs(vx) >= Math.abs(vy)) {
		if (vx < 0 && x < 0) return 'left';
		if (vx > 0 && x > win.w) return 'right';
		return null;
	}
	if (vy < 0 && y < 0) return 'top';
	if (vy > 0 && y > win.h) return 'bottom';
	return null;
};

/**
 * How far a rectangle flung toward `edge` travels to leave the window wholly (px), drifting across as the pointer moved
 * (`v`, px per ms)
 */
export const throwOffset = (rect: Rect, edge: FlingEdge, v: { vx: number; vy: number }, win: Size): { x: number; y: number } => {
	const along = {
		left: -(rect.x + rect.w),
		right: win.w - rect.x,
		top: -(rect.y + rect.h),
		bottom: win.h - rect.y
	}[edge];
	if (edge === 'left' || edge === 'right') return { x: along, y: v.vx ? (v.vy / v.vx) * along : 0 };
	return { x: v.vy ? (v.vx / v.vy) * along : 0, y: along };
};

/** Where a swipe up may start: within `band` px of the window's bottom */
export const atBottomEdge = (y: number, winH: number, band: number): boolean => y >= winH - band;

/** A press moved by dx, dy is a swipe up: at least `distance` px upward, more up than sideways */
export const isSwipeUp = (dx: number, dy: number, distance: number): boolean => -dy >= distance && Math.abs(dx) < -dy;
