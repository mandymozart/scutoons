import { dockRegion } from './dock-region.svelte.ts';

/**
 * A right region's width that its left edge (a `Divider`, agents/DOCKING.md) drags (owner 2026-10-03: "version history
 * should also be draggable, same as the right side selectors"): from its normal width up to half the window (or what
 * the place it is docked in leaves it: `room`), arrow keys on the edge, Home / double-click resets; remembered per key.
 * `foldable`: a panel of the AppShell's right dock – it folds to a rail (`folded`, `fold`; dragging it narrow too; owner
 * 2026-10-06), not remembered: a panel opens unfolded. `rightEdge`: where the region ends (default the window's right
 * edge; the slice inspector passes its container's). Built on `dockRegion` (dock-region.svelte.ts).
 */
export function panelWidth(storageKey: string, normalRem = 24, rightEdge: () => number = () => window.innerWidth, options: { foldable?: boolean } = {}) {
	const normal = () => normalRem * (parseFloat(getComputedStyle(document.documentElement).fontSize) || 16);
	/** px the place it is docked in leaves it (the AppShell's dock: the content keeps its minimum) */
	let room: (() => number) | null = null;
	const region = dockRegion({
		key: () => storageKey,
		foldable: () => !!options.foldable,
		side: 'right',
		anchor: rightEdge,
		normal,
		max: () => Math.min(window.innerWidth / 2, room?.() ?? Infinity)
	});
	region.restore();
	return {
		/** px set by dragging / keys; null = the normal width */
		get width() {
			return region.width;
		},
		/** the CSS length of the normal width (the dock never squeezes it below this) */
		normal: `min(${normalRem}rem, 92vw)`,
		/** the CSS length for `--panel-width` */
		get css() {
			return region.width !== null ? `${region.width}px` : `min(${normalRem}rem, 92vw)`;
		},
		set room(fn: (() => number) | null) {
			room = fn;
		},
		get folded() {
			return region.folded;
		},
		get foldable() {
			return region.foldable;
		},
		fold: region.fold,
		start: region.start,
		key: region.key,
		reset: region.reset
	};
}
export type PanelWidth = ReturnType<typeof panelWidth>;
