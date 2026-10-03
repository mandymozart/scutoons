/**
 * A right side panel's width that its left edge drags (owner 2026-10-03: "version history should also be draggable,
 * same as the right side selectors"): from its normal width up to half the window, arrow keys on the edge,
 * double-click resets; remembered per key. The page sets `--panel-width` from `css` and puts the edge
 * (`role="separator"`) on the panel's left border with `start` / `key` / `reset`.
 */
export function panelWidth(storageKey: string, normalRem = 24) {
	let width = $state<number | null>(null);
	try {
		const saved = Number(typeof localStorage !== 'undefined' ? localStorage.getItem(storageKey) : 0);
		if (saved > 0) width = saved;
	} catch {
		// not remembered
	}
	const normal = () => normalRem * (parseFloat(getComputedStyle(document.documentElement).fontSize) || 16);
	const clamp = (px: number) => Math.round(Math.min(Math.max(px, normal()), Math.max(normal(), window.innerWidth / 2)));
	function remember() {
		try {
			if (width === null) localStorage.removeItem(storageKey);
			else localStorage.setItem(storageKey, String(width));
		} catch {
			// not remembered
		}
	}
	return {
		/** the CSS length for `--panel-width` */
		get css() {
			return width !== null ? `${width}px` : `min(${normalRem}rem, 92vw)`;
		},
		start(event: PointerEvent) {
			event.preventDefault();
			const handle = event.currentTarget as HTMLElement;
			handle.setPointerCapture(event.pointerId);
			const move = (e: PointerEvent) => (width = clamp(window.innerWidth - e.clientX));
			const end = () => {
				handle.removeEventListener('pointermove', move);
				handle.removeEventListener('pointerup', end);
				handle.removeEventListener('pointercancel', end);
				remember();
			};
			handle.addEventListener('pointermove', move);
			handle.addEventListener('pointerup', end);
			handle.addEventListener('pointercancel', end);
		},
		key(event: KeyboardEvent) {
			const step = event.shiftKey ? 64 : 16;
			const current = width ?? normal();
			if (event.key === 'ArrowLeft') width = clamp(current + step);
			else if (event.key === 'ArrowRight') width = clamp(current - step);
			else if (event.key === 'Home') width = null;
			else return;
			event.preventDefault();
			remember();
		},
		reset() {
			width = null;
			remember();
		}
	};
}
