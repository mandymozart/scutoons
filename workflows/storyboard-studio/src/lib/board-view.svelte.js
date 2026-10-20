// The variants board's view – pan and zoom on its scroller (agents/research/VARIANTS-CANVAS.md §3.2 "Pan / zoom", the
// Timeline's keys; the maths: board-zoom.js). The wheel scrolls down through the bands (the browser's own), Shift +
// wheel sideways, Ctrl / ⌘ + wheel zooms about the pointer; a drag on the BACKGROUND pans – not on a card, a strip, a
// button, the actions or a band's head – and the click that ends a pan is swallowed (a pan doesn't make a band's shot
// current, §3.2). One zoom for all bands (board.zoom → bandLayout: the picture height in rem – text never blurs); a zoom
// keeps what is under its point there: the picture / strip under it – or the nearest one – is measured before and after.
import { tick } from 'svelte';
import { board } from './board.svelte.js';
import { clampZoom, keepUnder, nearestBox, wheelZoom } from './board-zoom.js';

export const view = $state({ panning: false });

/** what a zoom keeps under its point: a card's picture or a strip – they scale with the zoom; the gaps, the heads and
 *  the captions don't (rem) */
const PICS = '.card .pic, .strip';

/** the picture under (x, y) – else the nearest one in the band there (the background, a caption, a sticky head: a
 *  point beside a picture keeps its distance in the picture's scale – off by less than a gap, where the band's body
 *  as the anchor drifted rows away), else on the whole board; a band without pictures yet: its body */
function anchorAt(scroller, x, y) {
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

/** zoom to `z` about the client point (x, y) – the scroller's middle without one */
export async function zoomAt(scroller, z, x, y) {
	const next = clampZoom(z);
	if (!scroller || Math.abs(next - board.zoom) < 1e-6) return;
	const box = scroller.getBoundingClientRect();
	[x, y] = [x ?? box.left + box.width / 2, y ?? box.top + box.height / 2];
	const el = anchorAt(scroller, x, y);
	const before = el?.getBoundingClientRect();
	board.zoom = next;
	await tick();
	if (!before || !el.isConnected) return;
	const { dx, dy } = keepUnder(before, el.getBoundingClientRect(), x, y);
	scroller.scrollLeft += dx;
	scroller.scrollTop += dy;
}

/** a Svelte action on the board's scroller: Ctrl / ⌘ + wheel, Shift + wheel, the background pan */
export function panZoom(scroller) {
	const off = new AbortController();
	const on = (type, fn, opts = {}) => scroller.addEventListener(type, fn, { signal: off.signal, ...opts });
	// not passive: Ctrl + wheel must not zoom the page, Shift + wheel must not scroll it twice
	on('wheel', (e) => {
		if (e.ctrlKey || e.metaKey) {
			e.preventDefault();
			zoomAt(scroller, wheelZoom(board.zoom, e.deltaY), e.clientX, e.clientY);
		} else if (e.shiftKey && !e.deltaX && e.deltaY) {
			e.preventDefault();
			scroller.scrollLeft += e.deltaY;
		}
	}, { passive: false });
	on('pointerdown', (e) => {
		if (e.button !== 0 || e.ctrlKey || e.metaKey || e.target.closest('.card, .strip, button, input, select, textarea, [data-testid="card-actions"], .head')) return;
		// the scrollbars are the browser's (px: their room is what the client box leaves)
		const box = scroller.getBoundingClientRect();
		if (e.clientX - box.left >= scroller.clientLeft + scroller.clientWidth || e.clientY - box.top >= scroller.clientTop + scroller.clientHeight) return;
		const start = { x: e.clientX, y: e.clientY, left: scroller.scrollLeft, top: scroller.scrollTop };
		const drag = new AbortController();
		const end = () => {
			drag.abort();
			if (!view.panning) return;
			view.panning = false;
			// the click this release makes is the pan's, not the band's – swallowed once, right after
			const swallow = (c) => (c.stopPropagation(), c.preventDefault());
			window.addEventListener('click', swallow, { capture: true, once: true });
			setTimeout(() => window.removeEventListener('click', swallow, { capture: true }));
		};
		window.addEventListener('pointermove', (m) => {
			// px: pointer arithmetic – a pan starts after 4px (a click stays a click)
			const [dx, dy] = [m.clientX - start.x, m.clientY - start.y];
			if (!view.panning && Math.hypot(dx, dy) < 4) return;
			view.panning = true;
			scroller.scrollLeft = start.left - dx;
			scroller.scrollTop = start.top - dy;
		}, { signal: drag.signal });
		window.addEventListener('pointerup', end, { signal: drag.signal });
		window.addEventListener('pointercancel', end, { signal: drag.signal });
		// the board unmounting mid-drag ends it – a listener that goes with the drag (not one more per pointerdown)
		off.signal.addEventListener('abort', () => drag.abort(), { signal: drag.signal });
	});
	return { destroy: () => off.abort() };
}

/** the root font size (px) – the studio's fluid rem (App) */
export const remPx = () => parseFloat(getComputedStyle(document.documentElement).fontSize) || 16;
