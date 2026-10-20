// The variants board's zoom (agents/research/VARIANTS-CANVAS.md §3.2 "Pan / zoom" – the Timeline's keys) – pure
// (test/board-zoom.test.mjs). ONE zoom for all bands: a scale of the picture height (bandLayout's zoom – rem maths, the
// text never blurs), 25 % … 200 %; + / − a step (a step across 100 % lands on it), Ctrl / ⌘ + wheel as the Timeline
// (e^(−dy / 500)), 0 fits the widest band's width, the % button → 100 %. Zooming keeps what is under the pointer there
// (keepUnder – Board's DOM side measures the boxes: board-view.svelte.js).

export const ZOOM_MIN = 0.25;
export const ZOOM_MAX = 2;
/** one + / − */
export const ZOOM_STEP = 1.25;

export const clampZoom = (z) => Math.min(ZOOM_MAX, Math.max(ZOOM_MIN, Number.isNaN(Number(z)) ? 1 : Number(z)));
/** + (dir 1) / − (dir −1): one step – one that crosses 100 % stops there */
export function zoomStep(z, dir) {
	const next = clampZoom(z * ZOOM_STEP ** Math.sign(dir));
	return (z - 1) * (next - 1) < 0 ? 1 : next;
}
/** Ctrl / ⌘ + wheel: the zoom after a wheel's deltaY (px) */
export const wheelZoom = (z, dy) => clampZoom(z * Math.exp(-dy / 500));
/** "80 %" */
export const percent = (z) => `${Math.round(z * 100)} %`;

/** 0: the largest zoom (a whole percent) at which `widthAt(zoom)` – the widest open band, rem – fits `avail` (rem); the
 *  smallest when nothing fits; 1 without room to measure. widthAt grows with the zoom (the captions' column jumps it at
 *  CAPTIONS_MIN – a bisection still finds the edge) */
export function fitZoom(widthAt, avail) {
	if (!(avail > 0)) return 1;
	if (widthAt(ZOOM_MAX) <= avail) return ZOOM_MAX;
	if (widthAt(ZOOM_MIN) > avail) return ZOOM_MIN;
	let [lo, hi] = [ZOOM_MIN, ZOOM_MAX];
	for (let i = 0; i < 32; i++) {
		const m = (lo + hi) / 2;
		if (widthAt(m) <= avail) lo = m;
		else hi = m;
	}
	// a whole percent: the edge found to ~1e-9, rounded down – one less when the rounding lands past the edge
	const z = Math.max(ZOOM_MIN, Math.floor(lo * 100 + 1e-6) / 100);
	return widthAt(z) <= avail ? z : Math.max(ZOOM_MIN, Math.round(z * 100 - 1) / 100);
}

/** the box nearest to the point (x, y) – distance 0 inside one (the first such) → its index, −1 without boxes: what a zoom
 *  over the background keeps in place (a picture – the gaps between them are fixed rem and don't scale) */
export function nearestBox(boxes, x, y) {
	let best = -1;
	let far = Infinity;
	boxes.forEach((r, i) => {
		const [dx, dy] = [Math.max(r.left - x, 0, x - (r.left + r.width)), Math.max(r.top - y, 0, y - (r.top + r.height))];
		if (dx * dx + dy * dy < far) [best, far] = [i, dx * dx + dy * dy];
	});
	return best;
}

/** zoom about a point: the box under (x, y) – or the nearest one: the point beside it keeps its distance in the box's
 *  scale – before and after the zoom (a card's picture, a strip – px on the screen) → the scroll change { dx, dy } (px)
 *  that puts the same spot of it back under the point */
export function keepUnder(before, after, x, y) {
	const fx = before.width > 0 ? (x - before.left) / before.width : 0;
	const fy = before.height > 0 ? (y - before.top) / before.height : 0;
	return { dx: after.left + fx * after.width - x, dy: after.top + fy * after.height - y };
}
