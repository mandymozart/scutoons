// The variants board's zoom (#125; the port of #62's board-zoom.js – agents/research/VARIANTS-CANVAS.md §3.2 "Pan /
// zoom"). Pure (test/storyboard-board-zoom.test.ts). ONE zoom for all bands: a scale of the picture height (bandLayout's
// zoom – rem maths, the text never blurs), 25 % … 200 %; + / − a step (a step across 100 % lands on it), Ctrl / ⌘ +
// wheel (e^(−dy / 500)), 0 fits the widest band's width, the % button → 100 %. Zooming keeps what is under the pointer
// there (keepUnder – the DOM side measures the boxes: storyboard-board-view.ts).

export const ZOOM_MIN = 0.25;
export const ZOOM_MAX = 2;
/** one + / − */
export const ZOOM_STEP = 1.25;

export const clampZoom = (z: number) => Math.min(ZOOM_MAX, Math.max(ZOOM_MIN, Number.isNaN(Number(z)) ? 1 : Number(z)));
/** + (dir 1) / − (dir −1): one step – one that crosses 100 % stops there */
export function zoomStep(z: number, dir: number): number {
	const next = clampZoom(z * ZOOM_STEP ** Math.sign(dir));
	return (z - 1) * (next - 1) < 0 ? 1 : next;
}
/** Ctrl / ⌘ + wheel: the zoom after a wheel's deltaY (px) */
export const wheelZoom = (z: number, dy: number) => clampZoom(z * Math.exp(-dy / 500));
/** "80 %" */
export const percent = (z: number) => `${Math.round(z * 100)} %`;

/** 0: the largest zoom (a whole percent) at which `widthAt(zoom)` – the widest open band, rem – fits `avail` (rem); the
 *  smallest when nothing fits; 1 without room to measure (a bisection – the captions' column jumps the width) */
export function fitZoom(widthAt: (zoom: number) => number, avail: number): number {
	if (!(avail > 0)) return 1;
	if (widthAt(ZOOM_MAX) <= avail) return ZOOM_MAX;
	if (widthAt(ZOOM_MIN) > avail) return ZOOM_MIN;
	let [lo, hi] = [ZOOM_MIN, ZOOM_MAX];
	for (let i = 0; i < 32; i++) {
		const m = (lo + hi) / 2;
		if (widthAt(m) <= avail) lo = m;
		else hi = m;
	}
	const z = Math.max(ZOOM_MIN, Math.floor(lo * 100 + 1e-6) / 100);
	return widthAt(z) <= avail ? z : Math.max(ZOOM_MIN, Math.round(z * 100 - 1) / 100);
}

export interface Box {
	left: number;
	top: number;
	width: number;
	height: number;
}

/** the box nearest to the point (x, y) – distance 0 inside one → its index, −1 without boxes */
export function nearestBox(boxes: readonly Box[], x: number, y: number): number {
	let best = -1;
	let far = Infinity;
	boxes.forEach((r, i) => {
		const dx = Math.max(r.left - x, 0, x - (r.left + r.width));
		const dy = Math.max(r.top - y, 0, y - (r.top + r.height));
		if (dx * dx + dy * dy < far) [best, far] = [i, dx * dx + dy * dy];
	});
	return best;
}

/** zoom about a point: the box under (x, y) before and after the zoom (px on the screen) → the scroll change { dx, dy }
 *  that puts the same spot of it back under the point */
export function keepUnder(before: Box, after: Box, x: number, y: number): { dx: number; dy: number } {
	const fx = before.width > 0 ? (x - before.left) / before.width : 0;
	const fy = before.height > 0 ? (y - before.top) / before.height : 0;
	return { dx: after.left + fx * after.width - x, dy: after.top + fy * after.height - y };
}
