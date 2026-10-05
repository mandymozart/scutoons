import { resizable } from './resizable.svelte.ts';

/**
 * A right side panel's width that its left edge drags (owner 2026-10-03: "version history should also be draggable,
 * same as the right side selectors"): from its normal width up to half the window, arrow keys on the edge,
 * double-click resets; remembered per key. The page sets `--panel-width` from `css` and puts the edge
 * (`role="separator"`) on the panel's left border with `start` / `key` / `reset`. `rightEdge`: where the panel ends
 * (default the window's right edge; an inspector column passes its container's). The drag itself: @scutoons/ui
 * `resizable` (COMPONENTS.md step 13).
 */
export function panelWidth(storageKey: string, normalRem = 24, rightEdge: () => number = () => window.innerWidth) {
	const normal = () => normalRem * (parseFloat(getComputedStyle(document.documentElement).fontSize) || 16);
	const size = resizable({ storageKey, edge: 'left', anchor: rightEdge, min: normal, max: () => window.innerWidth / 2 });
	size.restore();
	return {
		/** the CSS length for `--panel-width` */
		get css() {
			return size.width !== null ? `${size.width}px` : `min(${normalRem}rem, 92vw)`;
		},
		start: size.start,
		key: size.key,
		reset: size.reset
	};
}
