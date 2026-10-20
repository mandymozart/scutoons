// The variants board's view – pan and zoom on its scroller (#125; the port of #62's board-view.svelte.js –
// agents/research/VARIANTS-CANVAS.md §3.2 "Pan / zoom"; the maths: storyboard-board-zoom.ts). The wheel scrolls down
// through the bands (the browser's own), Shift + wheel sideways, Ctrl / ⌘ + wheel zooms about the pointer; a drag on the
// BACKGROUND pans – not on a card, a strip, a button, the actions or a band's head – and the click that ends a pan is
// swallowed (a pan doesn't make a band's shot current). A zoom keeps what is under its point there: the picture / strip
// under it – or the nearest one – is measured before and after.
import { tick } from 'svelte';
import { clampZoom, keepUnder, nearestBox, wheelZoom } from './storyboard-board-zoom.ts';

/** what a zoom keeps under its point: a card's picture or a strip – they scale with the zoom */
const PICS = '.card .pic, .strip';

/** the root font size (px) – sizes in rem from JS (RULES "Sizes in rem") */
export const remPx = () => parseFloat(getComputedStyle(document.documentElement).fontSize) || 16;

function anchorAt(scroller: HTMLElement, x: number, y: number): Element | null {
	const hit = document.elementFromPoint(x, y);
	if (!hit || !scroller.contains(hit)) return null;
	const own = hit.closest(PICS);
	if (own) return own;
	const band = hit.closest('section.band');
	for (const scope of [band, scroller]) {
		const els = [...(scope?.querySelectorAll(PICS) ?? [])];
		const i = nearestBox(els.map((el) => el.getBoundingClientRect()), x, y);
		if (i >= 0) return els[i];
	}
	return band?.querySelector('[data-body]') ?? null;
}

/** zoom to `z` about the client point (x, y) – the scroller's middle without one; `set` writes the zoom */
export async function zoomAt(scroller: HTMLElement | undefined, z: number, now: number, set: (z: number) => void, x?: number, y?: number): Promise<void> {
	const next = clampZoom(z);
	if (!scroller || Math.abs(next - now) < 1e-6) return;
	const box = scroller.getBoundingClientRect();
	const px = x ?? box.left + box.width / 2;
	const py = y ?? box.top + box.height / 2;
	const el = anchorAt(scroller, px, py);
	const before = el?.getBoundingClientRect();
	set(next);
	await tick();
	if (!before || !el?.isConnected) return;
	const { dx, dy } = keepUnder(before, el.getBoundingClientRect(), px, py);
	scroller.scrollLeft += dx;
	scroller.scrollTop += dy;
}

/** a Svelte action on the board's scroller: Ctrl / ⌘ + wheel, Shift + wheel, the background pan. `opts.zoom()` = the
 *  zoom now, `opts.set(z)` writes it, `opts.panning(on)` says a pan runs (the cursor) */
export function panZoom(scroller: HTMLElement, opts: { zoom: () => number; set: (z: number) => void; panning: (on: boolean) => void }) {
	const off = new AbortController();
	let panning = false;
	scroller.addEventListener(
		'wheel',
		(e) => {
			if (e.ctrlKey || e.metaKey) {
				e.preventDefault();
				void zoomAt(scroller, wheelZoom(opts.zoom(), e.deltaY), opts.zoom(), opts.set, e.clientX, e.clientY);
			} else if (e.shiftKey && !e.deltaX && e.deltaY) {
				e.preventDefault();
				scroller.scrollLeft += e.deltaY;
			}
		},
		{ passive: false, signal: off.signal }
	);
	scroller.addEventListener(
		'pointerdown',
		(e) => {
			const target = e.target as Element | null;
			if (e.button !== 0 || e.ctrlKey || e.metaKey || target?.closest('.card, .strip, button, input, select, textarea, [data-testid="card-actions"], .head')) return;
			const box = scroller.getBoundingClientRect();
			if (e.clientX - box.left >= scroller.clientLeft + scroller.clientWidth || e.clientY - box.top >= scroller.clientTop + scroller.clientHeight) return;
			const start = { x: e.clientX, y: e.clientY, left: scroller.scrollLeft, top: scroller.scrollTop };
			const drag = new AbortController();
			const end = () => {
				drag.abort();
				if (!panning) return;
				panning = false;
				opts.panning(false);
				const swallow = (c: Event) => (c.stopPropagation(), c.preventDefault());
				window.addEventListener('click', swallow, { capture: true, once: true });
				setTimeout(() => window.removeEventListener('click', swallow, { capture: true }));
			};
			window.addEventListener(
				'pointermove',
				(m) => {
					// px: pointer arithmetic – a pan starts after 4px (a click stays a click)
					const [dx, dy] = [m.clientX - start.x, m.clientY - start.y];
					if (!panning && Math.hypot(dx, dy) < 4) return;
					if (!panning) opts.panning(true);
					panning = true;
					scroller.scrollLeft = start.left - dx;
					scroller.scrollTop = start.top - dy;
				},
				{ signal: drag.signal }
			);
			window.addEventListener('pointerup', end, { signal: drag.signal });
			window.addEventListener('pointercancel', end, { signal: drag.signal });
			off.signal.addEventListener('abort', () => drag.abort(), { signal: drag.signal });
		},
		{ signal: off.signal }
	);
	return { destroy: () => off.abort() };
}
