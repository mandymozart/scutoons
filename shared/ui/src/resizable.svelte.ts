/**
 * One drag-to-resize edge (COMPONENTS.md step 13; the app shell's sidebar, the Sheet, the creator's side panels and
 * the script outline each had their own): a `role="separator"` handle on the panel's edge drags its width between `min`
 * and `max`, the arrow keys step it (16 px, Shift 64; towards the panel's growing side), Home and a double-click go back
 * to the normal width (`width = null`); remembered per `storageKey` ('' = not remembered).
 *
 *   const size = resizable({ storageKey: 'x', edge: 'left', anchor: () => window.innerWidth, min: () => 384, max: () => window.innerWidth / 2 });
 *   size.restore();   <div role="separator" onpointerdown={size.start} onkeydown={size.key} ondblclick={size.reset}>
 *
 * `edge`: where the handle sits – 'left' for a panel on the right (the width is `anchor − x`), 'right' for one on the
 * left (`x − anchor`), 'top' for a region along the bottom (its HEIGHT is `anchor − y`; ↑ grows it). `filter` sees every raw drag width before it is clamped and may skip it or end the drag (the app
 * shell's fold to a rail); `onstart` / `onend` wrap a drag. While dragging, `html.sc-resizing` is set (the resize
 * cursor everywhere, no text selection). The handle itself: `Divider.svelte` (agents/DOCKING.md).
 */
export interface ResizableOptions {
	storageKey: string | (() => string);
	edge: 'left' | 'right' | 'top';
	/** px: the panel's fixed side (its right end for edge 'left', its left start for edge 'right', its bottom for 'top') */
	anchor: () => number;
	min: () => number;
	max: () => number;
	/** the normal width the keys start from (default `min`) */
	normal?: () => number;
	filter?: (width: number) => 'skip' | 'end' | void;
	onstart?: () => void;
	onend?: () => void;
}

export function resizable(options: ResizableOptions) {
	let width = $state<number | null>(null);
	const key = () => (typeof options.storageKey === 'function' ? options.storageKey() : options.storageKey);
	const clamp = (px: number) => Math.round(Math.min(Math.max(px, options.min()), Math.max(options.min(), options.max())));

	function remember() {
		const k = key();
		if (!k) return;
		try {
			if (width === null) localStorage.removeItem(k);
			else localStorage.setItem(k, String(width));
		} catch {
			// not remembered
		}
	}

	return {
		/** px set by dragging / keys; null = the normal width */
		get width() {
			return width;
		},
		set width(next: number | null) {
			width = next;
		},
		/** reads the remembered width (storage blocked: the normal width) */
		restore() {
			const k = key();
			if (!k) return;
			try {
				const saved = Number(typeof localStorage !== 'undefined' ? localStorage.getItem(k) : 0);
				if (saved > 0) width = saved;
			} catch {
				// not remembered
			}
		},
		start(event: PointerEvent) {
			event.preventDefault();
			event.stopPropagation();
			const handle = event.currentTarget as HTMLElement;
			handle.setPointerCapture?.(event.pointerId);
			// every drag: the resize cursor everywhere, no text selection (Divider / AppShell CSS)
			document.documentElement.classList.add('sc-resizing');
			if (options.edge === 'top') document.documentElement.classList.add('sc-resizing-rows');
			options.onstart?.();
			const end = () => {
				handle.removeEventListener('pointermove', move);
				handle.removeEventListener('pointerup', end);
				handle.removeEventListener('pointercancel', end);
				document.documentElement.classList.remove('sc-resizing', 'sc-resizing-rows');
				options.onend?.();
				remember();
			};
			const move = (e: PointerEvent) => {
				const raw = options.edge === 'left' ? options.anchor() - e.clientX : options.edge === 'top' ? options.anchor() - e.clientY : e.clientX - options.anchor();
				const verdict = options.filter?.(raw);
				if (verdict === 'skip') return;
				if (verdict === 'end') return end();
				width = clamp(raw);
			};
			handle.addEventListener('pointermove', move);
			handle.addEventListener('pointerup', end);
			handle.addEventListener('pointercancel', end);
		},
		key(event: KeyboardEvent) {
			const step = event.shiftKey ? 64 : 16;
			const current = width ?? (options.normal ?? options.min)();
			const grow = options.edge === 'left' ? 'ArrowLeft' : options.edge === 'top' ? 'ArrowUp' : 'ArrowRight';
			const shrink = options.edge === 'left' ? 'ArrowRight' : options.edge === 'top' ? 'ArrowDown' : 'ArrowLeft';
			if (event.key === grow) width = clamp(current + step);
			else if (event.key === shrink) width = clamp(current - step);
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
export type Resizable = ReturnType<typeof resizable>;
