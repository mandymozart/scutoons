// Taking the stage (research/redesign/stage.html): at most two panel tabs on a workspace's stage, side by side; none = the
// workspace's own view fills the window. Pure: test/shell.test.ts.

/** the tabs on the stage, left to right (0–2) */
export type Stage = readonly string[];

/** a side of the stage a dropped tab lands on */
export type Side = 'left' | 'right';

/** canvas tabs fill the whole window edge to edge; every other tab is a page: its ground runs under the bar, its content keeps clear */
export const CANVAS_TABS: readonly string[] = ['flow', 'preview'];
export const isCanvas = (tab: string): boolean => CANVAS_TABS.includes(tab);

export const SPLIT_MIN = 0.25;
export const SPLIT_MAX = 0.75;
export const clampSplit = (v: number): number => (Number.isFinite(v) ? Math.min(SPLIT_MAX, Math.max(SPLIT_MIN, v)) : 0.5);

/** ◱: this tab alone on the stage; pressed again (it is there) it goes back */
export function send(stage: Stage, tab: string): Stage {
	return stage.includes(tab) ? back(stage, tab) : [tab];
}

/** a tab dropped on a half: with one tab there the stage splits, with two it replaces that half; never more than two */
export function splitIn(stage: Stage, tab: string, side: Side): Stage {
	if (!stage.length || stage.includes(tab)) return send(stage, tab);
	if (stage.length === 1) return side === 'left' ? [tab, stage[0]] : [stage[0], tab];
	return side === 'left' ? [tab, stage[1]] : [stage[0], tab];
}

/** one tab back into its panel (none named: the last one); the other keeps the whole stage */
export function back(stage: Stage, tab?: string): Stage {
	const out = tab ?? stage.at(-1);
	return stage.filter((t) => t !== out);
}

/** what a tab dragged over the stage at `x` (a share of the width) would do */
export type DropPlan = { kind: 'take' } | { kind: 'split'; side: Side } | { kind: 'replace'; side: Side; out: string };

export function dropPlan(stage: Stage, tab: string, x: number): DropPlan | null {
	if (!stage.length) return { kind: 'take' };
	if (stage.includes(tab)) return null;
	const side: Side = x < 0.5 ? 'left' : 'right';
	if (stage.length === 1) return { kind: 'split', side };
	return { kind: 'replace', side, out: stage[side === 'left' ? 0 : 1] };
}

/** the room the shown panels leave free across a window `W` wide: the right edge of the left column, the left edge of the right one */
export function freeRoom(rects: Iterable<{ x: number; w: number }>, W: number): { left: number; right: number } {
	let left = 0;
	let right = W;
	for (const r of rects) {
		if (r.x + r.w / 2 < W / 2) left = Math.max(left, r.x + r.w);
		else right = Math.min(right, r.x);
	}
	return { left, right: Math.max(left, right) };
}

/** the tabs that left the stage between two states (they fly back into their panels) */
export const leaving = (was: Stage, now: Stage): string[] => was.filter((t) => !now.includes(t));
/** the tabs that came onto the stage */
export const arriving = (was: Stage, now: Stage): string[] => now.filter((t) => !was.includes(t));

/** a stored stage, every entry checked: strings, no repeats, at most two */
export function readStage(raw: unknown): string[] | undefined {
	if (!Array.isArray(raw)) return undefined;
	return [...new Set(raw.filter((t): t is string => typeof t === 'string' && !!t))].slice(0, 2);
}
