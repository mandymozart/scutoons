// The creator's side of the stage (the layouts and drop zones: @scutoons/ui/editor stage.ts). Pure: test/shell.test.ts.

/** canvas tabs fill their cell edge to edge; every other tab is a page: its ground runs under the bar, its content keeps clear */
export const CANVAS_TABS: readonly string[] = ['flow', 'preview', 'sheet', '3d', 'canvas', 'rust', 'nodes', 'monitor'];
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

/**
 * The room the shown panels take from a view that fills the stage (px, inside `area`): a panel in the lower half narrows
 * its bottom, one in the upper half the side it is on; a band wider than half the area narrows nothing.
 */
export function roomAround(rects: Iterable<{ x: number; y: number; w: number; h: number }>, area: { x: number; y: number; w: number; h: number }): { left: number; right: number; bottom: number } {
	const room = { left: 0, right: 0, bottom: 0 };
	for (const r of rects) {
		if (r.w >= area.w / 2) continue;
		const x = r.x - area.x;
		const y = r.y - area.y;
		if (y > area.h / 2) room.bottom = Math.max(room.bottom, area.h - y);
		else if (x + r.w / 2 > area.w / 2) room.right = Math.max(room.right, area.w - x);
		else room.left = Math.max(room.left, x + r.w);
	}
	return room;
}
