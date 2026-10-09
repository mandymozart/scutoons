// Image → silhouette mask on a coarse grid (+ the colours under it). Two kinds of input:
//  - a cut-out with transparency (the A-pose PNGs / WebPs: transparent with a soft floor shadow) → alpha decides;
//  - a sketch / photo on a light background → dark lines (or a filled shape) are ink; gaps in the outline are closed by
//    a dilation, everything not reachable from the border is inside, then eroded back.
// Pure functions on ImageData-like objects ({ width, height, data: RGBA }) – runs in the browser and in Node.

/** Exact Euclidean distance transform (Felzenszwalb–Huttenlocher): for every cell where `inside` is true, the distance to
 * the nearest cell where it is false (0 elsewhere). */
export function edt(inside, w, h) {
	const INF = 1e20;
	const f = new Float64Array(Math.max(w, h));
	const d = new Float64Array(Math.max(w, h));
	const v = new Int32Array(Math.max(w, h));
	const z = new Float64Array(Math.max(w, h) + 1);
	const g = new Float64Array(w * h);
	for (let i = 0; i < w * h; i++) g[i] = inside[i] ? INF : 0;
	const pass = (n, get, set) => {
		for (let q = 0; q < n; q++) f[q] = get(q);
		let k = 0;
		v[0] = 0; z[0] = -INF; z[1] = INF;
		for (let q = 1; q < n; q++) {
			let s = ((f[q] + q * q) - (f[v[k]] + v[k] * v[k])) / (2 * q - 2 * v[k]);
			while (s <= z[k]) { k--; s = ((f[q] + q * q) - (f[v[k]] + v[k] * v[k])) / (2 * q - 2 * v[k]); }
			k++; v[k] = q; z[k] = s; z[k + 1] = INF;
		}
		k = 0;
		for (let q = 0; q < n; q++) {
			while (z[k + 1] < q) k++;
			d[q] = (q - v[k]) * (q - v[k]) + f[v[k]];
		}
		for (let q = 0; q < n; q++) set(q, d[q]);
	};
	for (let x = 0; x < w; x++) pass(h, (y) => g[y * w + x], (y, val) => { g[y * w + x] = val; });
	for (let y = 0; y < h; y++) pass(w, (x) => g[y * w + x], (x, val) => { g[y * w + x] = val; });
	const out = new Float32Array(w * h);
	for (let i = 0; i < w * h; i++) out[i] = Math.sqrt(g[i]);
	return out;
}

/** cells reachable from the border through `free` cells (4-neighbourhood) */
function floodFromBorder(free, w, h) {
	const seen = new Uint8Array(w * h);
	const stack = [];
	const push = (i) => { if (free[i] && !seen[i]) { seen[i] = 1; stack.push(i); } };
	for (let x = 0; x < w; x++) { push(x); push((h - 1) * w + x); }
	for (let y = 0; y < h; y++) { push(y * w); push(y * w + w - 1); }
	while (stack.length) {
		const i = stack.pop();
		const x = i % w, y = (i / w) | 0;
		if (x > 0) push(i - 1);
		if (x < w - 1) push(i + 1);
		if (y > 0) push(i - w);
		if (y < h - 1) push(i + w);
	}
	return seen;
}

/** connected components (4-neighbourhood) of `m`; keeps those with at least `minShare` of the largest one's size */
function keepBig(m, w, h, minShare = 0.05) {
	const lab = new Int32Array(w * h).fill(-1);
	const sizes = [];
	for (let s = 0; s < w * h; s++) {
		if (!m[s] || lab[s] >= 0) continue;
		const id = sizes.length;
		let n = 0;
		const stack = [s];
		lab[s] = id;
		while (stack.length) {
			const i = stack.pop();
			n++;
			const x = i % w;
			for (const j of [x > 0 ? i - 1 : -1, x < w - 1 ? i + 1 : -1, i - w, i + w]) {
				if (j >= 0 && j < w * h && m[j] && lab[j] < 0) { lab[j] = id; stack.push(j); }
			}
		}
		sizes.push(n);
	}
	const max = Math.max(0, ...sizes);
	const out = new Uint8Array(w * h);
	for (let i = 0; i < w * h; i++) if (lab[i] >= 0 && sizes[lab[i]] >= max * minShare) out[i] = 1;
	return out;
}

/** 'alpha' when the image has real transparency, else 'background' (plain background, cut out by flood fill) */
export function detectMode(img) {
	const { data, width, height } = img;
	let clear = 0;
	for (let i = 3; i < data.length; i += 4 * 7) if (data[i] < 128) clear++;
	if (clear > (width * height / 7) * 0.05) return 'alpha';
	return 'background'; // a sketch (lines only) is chosen by hand: its gaps need closing
}

/** the background colour: the median of the border pixels */
function borderColour({ data, width: w, height: h }) {
	const ch = [[], [], []];
	const take = (i) => { for (let c = 0; c < 3; c++) ch[c].push(data[i * 4 + c]); };
	for (let x = 0; x < w; x += 3) { take(x); take((h - 1) * w + x); }
	for (let y = 0; y < h; y += 3) { take(y * w); take(y * w + w - 1); }
	return ch.map((a) => a.sort((p, q) => p - q)[a.length >> 1]);
}

/** full-resolution mask of the figure(s) */
export function fullMask(img, { mode = 'auto', ink = 0.55, close = 0.006, tol = 10 } = {}) {
	const { data, width: w, height: h } = img;
	if (mode === 'auto') mode = detectMode(img);
	let m = new Uint8Array(w * h);
	if (mode === 'alpha') {
		// the soft floor shadow stays below ~60 % alpha
		for (let i = 0; i < w * h; i++) m[i] = data[i * 4 + 3] > 160 ? 1 : 0;
	} else if (mode === 'background') {
		// a render / photo on a plain background (a character sheet on white): flood from the border over pixels close
		// to the background colour that sit on no edge – a white dress on white stops at its shaded outline
		const bg = borderColour(img);
		const lum = new Float32Array(w * h);
		for (let i = 0; i < w * h; i++) lum[i] = 0.299 * data[i * 4] + 0.587 * data[i * 4 + 1] + 0.114 * data[i * 4 + 2];
		const free = new Uint8Array(w * h);
		for (let y = 1; y < h - 1; y++) for (let x = 1; x < w - 1; x++) {
			const i = y * w + x;
			const d = Math.max(Math.abs(data[i * 4] - bg[0]), Math.abs(data[i * 4 + 1] - bg[1]), Math.abs(data[i * 4 + 2] - bg[2]));
			const grad = Math.max(Math.abs(lum[i + 1] - lum[i - 1]), Math.abs(lum[i + w] - lum[i - w]));
			free[i] = d <= tol && grad <= tol * 0.6 ? 1 : 0;
		}
		for (let x = 0; x < w; x++) { free[x] = 1; free[(h - 1) * w + x] = 1; }
		for (let y = 0; y < h; y++) { free[y * w] = 1; free[y * w + w - 1] = 1; }
		const outside = floodFromBorder(free, w, h);
		m = Uint8Array.from(outside, (v) => 1 - v);
		// open (erode + dilate by 1) – removes the 1-pixel halo of anti-aliasing / soft shadows
		const din = edt(m, w, h);
		const er = Uint8Array.from(din, (d) => (d > 1.5 ? 1 : 0));
		const dout = edt(Uint8Array.from(er, (v) => 1 - v), w, h);
		m = Uint8Array.from(dout, (d) => (d <= 1.5 ? 1 : 0));
	} else {
		// ink = dark pixels; close the outline's gaps (dilate), fill, erode back
		const isInk = new Uint8Array(w * h);
		for (let i = 0; i < w * h; i++) {
			const l = (0.299 * data[i * 4] + 0.587 * data[i * 4 + 1] + 0.114 * data[i * 4 + 2]) / 255;
			isInk[i] = l < ink && data[i * 4 + 3] > 128 ? 1 : 0;
		}
		const r = Math.max(1, Math.round(Math.max(w, h) * close));
		const toInk = edt(Uint8Array.from(isInk, (v) => 1 - v), w, h); // distance of each non-ink pixel to ink
		const dil = Uint8Array.from(toInk, (d) => (d <= r ? 1 : 0));
		const outside = floodFromBorder(Uint8Array.from(dil, (v) => 1 - v), w, h);
		const filled = Uint8Array.from(outside, (v) => 1 - v);
		const toOut = edt(filled, w, h); // erode by r again
		m = Uint8Array.from(toOut, (d) => (d > r ? 1 : 0));
		for (let i = 0; i < w * h; i++) if (isInk[i] && filled[i]) m[i] = 1; // keep the drawn lines themselves
	}
	m = keepBig(m, w, h, 0.15);
	// fill holes (a gap between arm and body stays: it reaches the border)
	const outside = floodFromBorder(Uint8Array.from(m, (v) => 1 - v), w, h);
	for (let i = 0; i < w * h; i++) if (!outside[i]) m[i] = 1;
	return { mask: m, mode };
}

/**
 * The figure on a coarse grid: crop to its bounding box, `rows` cells from the top of the head to the soles (+ a margin),
 * the colour of each cell = the mean colour of the figure pixels in it (cells outside take their nearest inside colour,
 * so faces on the silhouette's edge don't sample the background).
 * → { w, h, mask, color (Float32 rgb 0..1), top, sole, pad, mode }
 */
export function gridMask(img, { rows = 112, mode = 'auto', ink, close, tol, full = null, box = null } = {}) {
	const { mask: all, mode: used } = full ?? fullMask(img, { mode, ink, close, tol });
	const { width: W, height: H, data } = img;
	// one panel of a sheet: only its columns
	const m = box ? Uint8Array.from(all, (v, i) => (v && i % W >= box.x0 && i % W <= box.x1 ? 1 : 0)) : all;
	let x0 = W, x1 = -1, y0 = H, y1 = -1;
	for (let y = 0; y < H; y++) for (let x = 0; x < W; x++) if (m[y * W + x]) {
		if (x < x0) x0 = x; if (x > x1) x1 = x; if (y < y0) y0 = y; if (y > y1) y1 = y;
	}
	if (x1 < 0) throw new Error('no figure found in the image');
	const cell = (y1 - y0 + 1) / rows; // source pixels per grid cell
	const pad = 3;
	const w = Math.ceil((x1 - x0 + 1) / cell) + pad * 2, h = rows + pad * 2;
	const mask = new Uint8Array(w * h);
	const color = new Float32Array(w * h * 3);
	const cnt = new Float32Array(w * h), tot = new Float32Array(w * h);
	for (let y = y0; y <= y1; y++) for (let x = x0; x <= x1; x++) {
		const gx = Math.min(w - pad - 1, pad + Math.floor((x - x0) / cell)), gy = Math.min(h - pad - 1, pad + Math.floor((y - y0) / cell));
		const g = gy * w + gx, i = y * W + x;
		tot[g]++;
		if (!m[i]) continue;
		cnt[g]++;
		color[g * 3] += data[i * 4] / 255; color[g * 3 + 1] += data[i * 4 + 1] / 255; color[g * 3 + 2] += data[i * 4 + 2] / 255;
	}
	for (let g = 0; g < w * h; g++) {
		if (cnt[g]) for (let c = 0; c < 3; c++) color[g * 3 + c] /= cnt[g];
		mask[g] = tot[g] && cnt[g] / tot[g] >= 0.5 ? 1 : 0;
	}
	// cells without figure pixels take the nearest coloured cell's colour (a few rounds of growing)
	let has = Uint8Array.from(cnt, (c) => (c > 0 ? 1 : 0));
	for (let round = 0; round < 6; round++) {
		const next = has.slice();
		for (let y = 0; y < h; y++) for (let x = 0; x < w; x++) {
			const g = y * w + x;
			if (has[g]) continue;
			let n = 0; const s = [0, 0, 0];
			for (const [dx, dy] of [[1, 0], [-1, 0], [0, 1], [0, -1]]) {
				const xx = x + dx, yy = y + dy;
				if (xx < 0 || yy < 0 || xx >= w || yy >= h || !has[yy * w + xx]) continue;
				const j = yy * w + xx;
				n++; s[0] += color[j * 3]; s[1] += color[j * 3 + 1]; s[2] += color[j * 3 + 2];
			}
			if (n) { for (let c = 0; c < 3; c++) color[g * 3 + c] = s[c] / n; next[g] = 1; }
		}
		has = next;
	}
	return { w, h, mask, color, top: pad, sole: pad + rows - 1, pad, mode: used, crop: { x0, y0, x1, y1, cell } };
}

/**
 * A character sheet → its figures: the image's mask split at empty columns (front, side, ¾, back views next to each
 * other) → { mode, full, boxes: [{ x0, x1, y0, y1 }] } left to right. A single figure → one box.
 */
export function sheet(img, opts = {}) {
	const full = fullMask(img, opts);
	const { width: W, height: H } = img;
	const col = new Int32Array(W);
	const top = new Int32Array(W).fill(H), bot = new Int32Array(W).fill(-1);
	for (let y = 0; y < H; y++) for (let x = 0; x < W; x++) if (full.mask[y * W + x]) { col[x]++; if (y < top[x]) top[x] = y; bot[x] = y; }
	const boxes = [];
	const minGap = Math.max(2, Math.round(W * 0.01));
	let cur = null, gap = 0;
	for (let x = 0; x < W; x++) {
		if (col[x]) {
			if (!cur) cur = { x0: x, x1: x, y0: H, y1: -1 };
			cur.x1 = x; cur.y0 = Math.min(cur.y0, top[x]); cur.y1 = Math.max(cur.y1, bot[x]);
			gap = 0;
		} else if (cur && ++gap >= minGap) { boxes.push(cur); cur = null; }
	}
	if (cur) boxes.push(cur);
	// drop specks: a panel needs at least a quarter of the tallest one's height
	const tall = Math.max(...boxes.map((b) => b.y1 - b.y0));
	return { mode: full.mode, full, boxes: boxes.filter((b) => b.y1 - b.y0 >= tall * 0.25) };
}
