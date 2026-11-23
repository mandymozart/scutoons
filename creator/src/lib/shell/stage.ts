// The creator's side of the stage (the layouts and drop zones: @scutoons/ui/editor stage.ts). Pure: test/shell.test.ts.

/** canvas tabs fill their cell edge to edge; every other tab is a page: its ground runs under the bar, its content keeps clear */
export const CANVAS_TABS: readonly string[] = ['flow', 'preview'];
export const isCanvas = (tab: string): boolean => CANVAS_TABS.includes(tab);

/** the open item's own view (under the stage) as a full-screen target: not a panel tab */
export const ITEM_VIEW = '@item';

export const SPLIT_MIN = 0.25;
export const SPLIT_MAX = 0.75;
export const clampSplit = (v: number): number => (Number.isFinite(v) ? Math.min(SPLIT_MAX, Math.max(SPLIT_MIN, v)) : 0.5);

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
