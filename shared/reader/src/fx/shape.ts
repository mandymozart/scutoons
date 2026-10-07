/**
 * A placed item's cut-out shape (agents/FX.md "transparency"): its icon's alpha channel, read once per icon and kept.
 * Taps on fully transparent pixels miss (with a 0.5rem tolerance) and sparkles start on the outline, not the box.
 * The icon is read again with CORS; a host that doesn't allow that gives no shape – then the box counts, as before.
 */

export interface Shape {
	w: number;
	h: number;
	alpha: Uint8ClampedArray;
	/** points on the outline, in shape pixels */
	edge: [number, number][];
}

const MAX = 256;
const cache = new Map<string, Promise<Shape | null>>();

export function loadShape(src: string): Promise<Shape | null> {
	let shape = cache.get(src);
	if (!shape) {
		shape = read(src);
		cache.set(src, shape);
	}
	return shape;
}

async function read(src: string): Promise<Shape | null> {
	if (typeof document === 'undefined') return null;
	try {
		const img = new Image();
		img.crossOrigin = 'anonymous';
		img.src = src;
		await img.decode();
		const k = Math.min(1, MAX / Math.max(img.naturalWidth, img.naturalHeight, 1));
		const w = Math.max(1, Math.round(img.naturalWidth * k));
		const h = Math.max(1, Math.round(img.naturalHeight * k));
		const canvas = document.createElement('canvas');
		canvas.width = w;
		canvas.height = h;
		const g = canvas.getContext('2d', { willReadFrequently: true });
		if (!g) return null;
		g.drawImage(img, 0, 0, w, h);
		const data = g.getImageData(0, 0, w, h).data; // throws for a tainted (no CORS) image
		const alpha = new Uint8ClampedArray(w * h);
		for (let i = 0; i < alpha.length; i++) alpha[i] = data[i * 4 + 3];
		const a = (x: number, y: number) => (x < 0 || y < 0 || x >= w || y >= h ? 0 : alpha[y * w + x]);
		const edge: [number, number][] = [];
		for (let y = 0; y < h; y += 2)
			for (let x = 0; x < w; x += 2)
				if (a(x, y) > 40 && (a(x - 2, y) < 40 || a(x + 2, y) < 40 || a(x, y - 2) < 40 || a(x, y + 2) < 40)) edge.push([x, y]);
		return { w, h, alpha, edge };
	} catch {
		return null;
	}
}

/** where the drawn icon sits in its element (object-fit: contain), unrotated, relative to the element's centre */
interface Frame {
	cx: number;
	cy: number;
	/** the drawn icon's size */
	w: number;
	h: number;
	/** radians */
	a: number;
}

export function frameOf(el: HTMLElement, shape: Pick<Shape, 'w' | 'h'> | null, rotation: number): Frame {
	const r = el.getBoundingClientRect();
	const bw = el.offsetWidth || r.width;
	const bh = el.offsetHeight || r.height;
	let w = bw;
	let h = bh;
	if (shape && bw > 0 && bh > 0) {
		const k = Math.min(bw / shape.w, bh / shape.h);
		w = shape.w * k;
		h = shape.h * k;
	}
	return { cx: r.left + r.width / 2, cy: r.top + r.height / 2, w, h, a: (rotation * Math.PI) / 180 };
}

/** a shape pixel → the screen */
export function toScreen(f: Frame, shape: Pick<Shape, 'w' | 'h'>, x: number, y: number): [number, number] {
	const lx = (x / shape.w - 0.5) * f.w;
	const ly = (y / shape.h - 0.5) * f.h;
	return [f.cx + lx * Math.cos(f.a) - ly * Math.sin(f.a), f.cy + lx * Math.sin(f.a) + ly * Math.cos(f.a)];
}

/** does a tap at (x, y) touch the drawn shape (or come within `tolerance` px of it)? */
export function hitsShape(el: HTMLElement, shape: Shape, rotation: number, x: number, y: number, tolerance: number): boolean {
	const f = frameOf(el, shape, rotation);
	const dx = x - f.cx;
	const dy = y - f.cy;
	const lx = dx * Math.cos(-f.a) - dy * Math.sin(-f.a);
	const ly = dx * Math.sin(-f.a) + dy * Math.cos(-f.a);
	const ix = (lx / f.w + 0.5) * shape.w;
	const iy = (ly / f.h + 0.5) * shape.h;
	const tol = tolerance * (shape.w / Math.max(1, f.w));
	for (let k = 0; k < 9; k++) {
		const angle = (k * Math.PI) / 4;
		const rr = k === 8 ? 0 : tol;
		const px = Math.round(ix + Math.cos(angle) * rr);
		const py = Math.round(iy + Math.sin(angle) * rr);
		if (px >= 0 && py >= 0 && px < shape.w && py < shape.h && shape.alpha[py * shape.w + px] > 32) return true;
	}
	return false;
}
