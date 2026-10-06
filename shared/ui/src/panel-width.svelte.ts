import { resizable } from './resizable.svelte.ts';

/**
 * A right side panel's width that its left edge drags (owner 2026-10-03: "version history should also be draggable,
 * same as the right side selectors"): from its normal width up to half the window, arrow keys on the edge,
 * double-click resets; remembered per key. The page sets `--panel-width` from `css`; the edge is a `Divider`
 * (agents/DOCKING.md) – the creator's page panels hand it to the AppShell's right dock. `rightEdge`: where the panel ends
 * (default the window's right edge; an inspector column passes its container's). The drag itself: @scutoons/ui
 * `resizable` (COMPONENTS.md step 13).
 */
export function panelWidth(storageKey: string, normalRem = 24, rightEdge: () => number = () => window.innerWidth) {
	const normal = () => normalRem * (parseFloat(getComputedStyle(document.documentElement).fontSize) || 16);
	/** px the place it is docked in leaves it (the AppShell's dock: the content keeps its minimum) */
	let room: (() => number) | null = null;
	const size = resizable({ storageKey, edge: 'left', anchor: rightEdge, min: normal, max: () => Math.min(window.innerWidth / 2, room?.() ?? Infinity) });
	size.restore();
	return {
		/** px set by dragging / keys; null = the normal width */
		get width() {
			return size.width;
		},
		/** the CSS length of the normal width (the dock never squeezes it below this) */
		normal: `min(${normalRem}rem, 92vw)`,
		/** the CSS length for `--panel-width` */
		get css() {
			return size.width !== null ? `${size.width}px` : `min(${normalRem}rem, 92vw)`;
		},
		set room(fn: (() => number) | null) {
			room = fn;
		},
		start: size.start,
		key: size.key,
		reset: size.reset
	};
}
export type PanelWidth = ReturnType<typeof panelWidth>;
