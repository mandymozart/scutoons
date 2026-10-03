// The outline of a wobbly (shout / song) bubble: a roughly rectangular shape whose edges tremble – small, uneven bumps and
// dents along all four sides, no straight line. Deterministic: the same key (the text box id) and size give the same path,
// so it never jumps between renders; it is computed for the box's laid-out size in px, so it scales with the box.

/** FNV-1a: a 32-bit seed from a string */
function hash(text: string): number {
	let h = 2166136261;
	for (let i = 0; i < text.length; i++) {
		h ^= text.charCodeAt(i);
		h = Math.imul(h, 16777619);
	}
	return h >>> 0;
}

/** mulberry32: a small seeded random generator (0 … 1) */
function random(seed: number): () => number {
	let s = seed;
	return () => {
		s = (s + 0x6d2b79f5) | 0;
		let t = Math.imul(s ^ (s >>> 15), 1 | s);
		t = (t + Math.imul(t ^ (t >>> 7), 61 | t)) ^ t;
		return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
	};
}

const r1 = (n: number) => Math.round(n * 10) / 10;

/** the bump size in px for a box (also how far the outline may lie inside the box's edge) */
export function wobbleAmplitude(width: number, height: number): number {
	return Math.max(1.5, Math.min(4, Math.min(width, height) * 0.035));
}

/**
 * An SVG path (in px, for a `viewBox="0 0 width height"`) around a `width` × `height` box: points along each side, every
 * one pushed in or out by up to the amplitude, joined by smooth curves. Stays inside the box.
 */
export function wobblyPath(width: number, height: number, key: string): string {
	if (!(width > 0) || !(height > 0)) return '';
	const a = wobbleAmplitude(width, height);
	const rand = random(hash(key || 'wobbly'));
	const step = Math.max(7, Math.min(16, Math.min(width, height) / 3));
	// the rect the bumps go around (inset by the amplitude) and each side's outward normal
	const l = a, t = a, r = width - a, b = height - a;
	const sides: [number, number, number, number, number, number][] = [
		[l, t, r, t, 0, -1],
		[r, t, r, b, 1, 0],
		[r, b, l, b, 0, 1],
		[l, b, l, t, -1, 0]
	];
	const points: [number, number][] = [];
	for (const [x1, y1, x2, y2, nx, ny] of sides) {
		const n = Math.max(3, Math.round(Math.hypot(x2 - x1, y2 - y1) / step));
		for (let i = 0; i < n; i++) {
			const f = i / n;
			// uneven: the offset and a little slip along the side
			const off = (rand() * 2 - 1) * a;
			const slip = (rand() - 0.5) * (0.35 / n);
			const g = Math.min(1, Math.max(0, f + (i === 0 ? 0 : slip)));
			points.push([x1 + (x2 - x1) * g + nx * off, y1 + (y2 - y1) * g + ny * off]);
		}
	}
	const mid = (p: [number, number], q: [number, number]): [number, number] => [(p[0] + q[0]) / 2, (p[1] + q[1]) / 2];
	const count = points.length;
	const start = mid(points[0], points[1]);
	let d = `M${r1(start[0])} ${r1(start[1])}`;
	for (let i = 1; i <= count; i++) {
		const p = points[i % count];
		const m = mid(p, points[(i + 1) % count]);
		d += ` Q${r1(p[0])} ${r1(p[1])} ${r1(m[0])} ${r1(m[1])}`;
	}
	return `${d} Z`;
}
