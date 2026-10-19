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
	/** px: where the region ends, set by the place it is docked in (the AppShell: the panel's right edge – another
	 * panel may sit right of it) */
	let edge: (() => number) | null = null;
	const region = dockRegion({
		key: () => storageKey,
		foldable: () => !!options.foldable,
		side: 'right',
		anchor: () => edge?.() ?? rightEdge(),
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
		set edge(fn: (() => number) | null) {
			edge = fn;
		},
		/** px: the normal width */
		get normalPx() {
			return normal();
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

/**
 * The height of the AppShell's bottom region (`bottom`, agents/DOCKING.md "Bottom region" – the storyboard studio's
 * timeline, owner 2026-10-19): its top edge (a `Divider`, `edge="top"`) drags it taller, from its normal height up to
 * half the window; ↑ / ↓ on the edge, Home / double-click resets; remembered per key. It folds to a rail (`fold`, or
 * dragging it lower than 60 % of the normal height); the fold is the page's to remember (not here). The shell sets
 * `edge` (its bottom).
 */
export function panelHeight(storageKey: string, normalRem = 10) {
	const normal = () => normalRem * (parseFloat(getComputedStyle(document.documentElement).fontSize) || 16);
	let edge: (() => number) | null = null;
	const region = dockRegion({
		key: () => storageKey,
		foldable: () => true,
		side: 'bottom',
		anchor: () => edge?.() ?? window.innerHeight,
		normal,
		max: () => window.innerHeight / 2
	});
	region.restore();
	return {
		/** px set by dragging / keys; null = the normal height */
		get height() {
			return region.width;
		},
		/** the CSS length of the height */
		get css() {
			return region.width !== null ? `${region.width}px` : `${normalRem}rem`;
		},
		set edge(fn: (() => number) | null) {
			edge = fn;
		},
		get folded() {
			return region.folded;
		},
		fold: region.fold,
		/** for the Divider (`width` = the height) */
		get width() {
			return region.width;
		},
		start: region.start,
		key: region.key,
		reset: region.reset
	};
}
export type PanelHeight = ReturnType<typeof panelHeight>;
