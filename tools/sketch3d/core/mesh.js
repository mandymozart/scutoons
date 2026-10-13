// Silhouette grid (mask.js) → a closed low-poly mesh. No ML, no service: the silhouette is inflated ("union of
// inscribed balls": every inside cell carries a ball as big as its distance to the outline; the surface is the hull of
// those balls, squashed in depth) – round limbs, a round head, a flatter torso. An optional side view cuts the depth
// (visual hull). A vase / bottle / tree trunk can be turned instead (lathe). The scalar field is meshed with surface
// nets, smoothed (Taubin), reduced to the wanted triangle count and coloured per face (flat = the low-poly look).
import * as THREE from 'three';
import { decimate } from './decimate.js';
import { edt } from './mask.js';

/** signed distance on the grid: + inside (distance to the outline), − outside; the outline sits half a cell out */
function signedDist(mask, w, h) {
	const din = edt(mask, w, h);
	const dout = edt(Uint8Array.from(mask, (v) => 1 - v), w, h);
	return Float32Array.from(din, (d, i) => (mask[i] ? d - 0.5 : -(dout[i] - 0.5)));
}

/** the inflation height per cell: max over inside cells q of sqrt(r_q² − |p − q|²), r_q = q's distance to the outline */
export function inflate(mask, w, h) {
	const sd = signedDist(mask, w, h);
	const hgt = new Float32Array(w * h);
	for (let qy = 0; qy < h; qy++) for (let qx = 0; qx < w; qx++) {
		const r = sd[qy * w + qx];
		if (r <= 0) continue;
		// only cells on or near the medial axis matter; skipping cells dominated by a neighbour's ball saves ~90 %
		let dominated = false;
		for (const [dx, dy] of [[1, 0], [-1, 0], [0, 1], [0, -1]]) {
			const x = qx + dx, y = qy + dy;
			if (x >= 0 && y >= 0 && x < w && y < h && sd[y * w + x] >= r + 0.999) { dominated = true; break; }
		}
		if (dominated) continue;
		const R = Math.ceil(r);
		for (let y = Math.max(0, qy - R); y <= Math.min(h - 1, qy + R); y++) for (let x = Math.max(0, qx - R); x <= Math.min(w - 1, qx + R); x++) {
			const d2 = r * r - (x - qx) ** 2 - (y - qy) ** 2;
			if (d2 > 0) { const v = Math.sqrt(d2); if (v > hgt[y * w + x]) hgt[y * w + x] = v; }
		}
	}
	return { sd, hgt };
}

/** surface nets on a scalar field (inside = f > 0) → { positions (grid coords), faces } */
export function surfaceNets(f, nx, ny, nz) {
	const at = (x, y, z) => f[x + nx * (y + ny * z)];
	const cellVert = new Int32Array(nx * ny * nz).fill(-1);
	const pos = [];
	const corner = new Float32Array(8);
	const EDGES = [];
	for (let i = 0; i < 8; i++) for (let b = 0; b < 3; b++) if (!(i & (1 << b))) EDGES.push([i, i | (1 << b)]);
	for (let z = 0; z < nz - 1; z++) for (let y = 0; y < ny - 1; y++) for (let x = 0; x < nx - 1; x++) {
		let m = 0;
		for (let i = 0; i < 8; i++) {
			corner[i] = at(x + (i & 1), y + ((i >> 1) & 1), z + ((i >> 2) & 1));
			if (corner[i] > 0) m |= 1 << i;
		}
		if (m === 0 || m === 255) continue;
		let sx = 0, sy = 0, sz = 0, n = 0;
		for (const [a, b] of EDGES) {
			const fa = corner[a], fb = corner[b];
			if ((fa > 0) === (fb > 0)) continue;
			const t = fa / (fa - fb);
			sx += (a & 1) + t * ((b & 1) - (a & 1));
			sy += ((a >> 1) & 1) + t * (((b >> 1) & 1) - ((a >> 1) & 1));
			sz += ((a >> 2) & 1) + t * (((b >> 2) & 1) - ((a >> 2) & 1));
			n++;
		}
		cellVert[x + nx * (y + ny * z)] = pos.length / 3;
		pos.push(x + sx / n, y + sy / n, z + sz / n);
	}
	const faces = [];
	const cv = (x, y, z) => cellVert[x + nx * (y + ny * z)];
	for (let z = 1; z < nz - 1; z++) for (let y = 1; y < ny - 1; y++) for (let x = 1; x < nx - 1; x++) {
		const c0 = at(x, y, z) > 0;
		const p = [x, y, z];
		for (let a = 0; a < 3; a++) {
			const q = p.slice(); q[a]++;
			if (q[a] >= [nx, ny, nz][a]) continue;
			if ((at(q[0], q[1], q[2]) > 0) === c0) continue;
			const u = (a + 1) % 3, v = (a + 2) % 3;
			const cell = (du, dv) => { const r = p.slice(); r[u] -= du; r[v] -= dv; return cv(r[0], r[1], r[2]); };
			const A = cell(0, 0), B = cell(1, 0), C = cell(1, 1), D = cell(0, 1);
			if (A < 0 || B < 0 || C < 0 || D < 0) continue;
			if (c0) faces.push(A, B, C, A, C, D);
			else faces.push(A, C, B, A, D, C);
		}
	}
	return { pos: Float32Array.from(pos), faces: Uint32Array.from(faces) };
}

/** Taubin smoothing (shrink-free): λ / μ Laplacian steps over the vertex neighbourhoods */
function taubin(pos, faces, iterations) {
	const n = pos.length / 3;
	const nb = Array.from({ length: n }, () => new Set());
	for (let i = 0; i < faces.length; i += 3) {
		const [a, b, c] = [faces[i], faces[i + 1], faces[i + 2]];
		nb[a].add(b); nb[a].add(c); nb[b].add(a); nb[b].add(c); nb[c].add(a); nb[c].add(b);
	}
	const lists = nb.map((s) => [...s]);
	const tmp = new Float32Array(pos.length);
	const step = (k) => {
		for (let i = 0; i < n; i++) {
			const l = lists[i];
			if (!l.length) { tmp[i * 3] = pos[i * 3]; tmp[i * 3 + 1] = pos[i * 3 + 1]; tmp[i * 3 + 2] = pos[i * 3 + 2]; continue; }
			let x = 0, y = 0, z = 0;
			for (const j of l) { x += pos[j * 3]; y += pos[j * 3 + 1]; z += pos[j * 3 + 2]; }
			x /= l.length; y /= l.length; z /= l.length;
			tmp[i * 3] = pos[i * 3] + k * (x - pos[i * 3]);
			tmp[i * 3 + 1] = pos[i * 3 + 1] + k * (y - pos[i * 3 + 1]);
			tmp[i * 3 + 2] = pos[i * 3 + 2] + k * (z - pos[i * 3 + 2]);
		}
		pos.set(tmp);
	};
	for (let i = 0; i < iterations; i++) { step(0.5); step(-0.53); }
}

/** k-means on colours (rgb 0..1) → the palette and each colour's index */
export function palette(cols, k, iterations = 10) {
	const n = cols.length / 3;
	if (!k || n === 0) return null;
	const C = [];
	for (let i = 0; i < k; i++) { const j = Math.floor(((i + 0.5) / k) * n); C.push([cols[j * 3], cols[j * 3 + 1], cols[j * 3 + 2]]); }
	const idx = new Int32Array(n);
	for (let it = 0; it < iterations; it++) {
		const sum = C.map(() => [0, 0, 0, 0]);
		for (let i = 0; i < n; i++) {
			let best = 0, bd = Infinity;
			for (let c = 0; c < k; c++) {
				const d = (cols[i * 3] - C[c][0]) ** 2 + (cols[i * 3 + 1] - C[c][1]) ** 2 + (cols[i * 3 + 2] - C[c][2]) ** 2;
				if (d < bd) { bd = d; best = c; }
			}
			idx[i] = best;
			const s = sum[best]; s[0] += cols[i * 3]; s[1] += cols[i * 3 + 1]; s[2] += cols[i * 3 + 2]; s[3]++;
		}
		sum.forEach((s, c) => { if (s[3]) C[c] = [s[0] / s[3], s[1] / s[3], s[2] / s[3]]; });
	}
	return { colors: C, idx };
}

/**
 * grid (mask.js gridMask) → { geometry (indexed, world metres, feet on y = 0, facing +z), toGrid(x, y), scale, info }
 * options: height (m), depth (squash of the inflation, 1 = round), mode 'inflate' | 'lathe', side (a second grid: the
 * side view, the figure looking right), smooth (Taubin rounds), tris (target triangle count)
 */
export function buildMesh(grid, { height = 1.8, depth = 0.8, mode = 'inflate', side = null, smooth = 4, tris = 1500 } = {}) {
	const { w, h, mask } = grid;
	const { sd, hgt } = inflate(mask, w, h);
	// the body's centre line: the mask's mean x
	let sx = 0, n = 0;
	for (let y = 0; y < h; y++) for (let x = 0; x < w; x++) if (mask[y * w + x]) { sx += x; n++; }
	const cx = sx / n;

	let maxH = 0;
	for (let i = 0; i < w * h; i++) if (hgt[i] > maxH) maxH = hgt[i];
	// lathe: the radius per row = the farthest figure cell from the centre line
	const lathe = mode === 'lathe' ? Float32Array.from({ length: h }, (_, y) => {
		let r = 0;
		for (let x = 0; x < w; x++) if (mask[y * w + x]) r = Math.max(r, Math.abs(x - cx) + 0.5);
		return r;
	}) : null;
	if (lathe) maxH = Math.max(...lathe);

	// the side view, scaled to the same height: rows match, its x becomes z (centred on its bounding box)
	let sideSd = null, sw = 0, scx = 0;
	if (side) {
		sw = side.w;
		// blurred: the distance to a pixelated slanted outline is a staircase – it showed as terraces on a dress
		sideSd = blur(signedDist(side.mask, side.w, side.h), side.w, side.h, 3);
		let a = side.w, b = -1;
		for (let y = 0; y < side.h; y++) for (let x = 0; x < side.w; x++) if (side.mask[y * side.w + x]) { a = Math.min(a, x); b = Math.max(b, x); }
		scx = (a + b) / 2;
		maxH = Math.max(maxH * depth, (b - a) / 2 + 1);
	}
	// with a side view, every row's cross-section is an ellipse: as wide as the front view's run there, as deep as the
	// side view at that height (around its middle) – the trunk takes the side's depth, a limb (a run of its own) stays
	// round. Then trimmed to the exact side outline. (Only front ∩ side gave flat slabs on the chest and the dress.)
	let runC = null, runA = null, rowZc = null, rowB = null;
	if (side) {
		runC = new Float32Array(w * h); runA = new Float32Array(w * h);
		for (let y = 0; y < h; y++) {
			let s0 = -1;
			for (let x = 0; x <= w; x++) {
				const on = x < w && mask[y * w + x];
				if (on && s0 < 0) s0 = x;
				if (!on && s0 >= 0) {
					const c = (s0 + x - 1) / 2, a = (x - s0) / 2;
					const trunk = s0 <= cx && cx <= x - 1;
					for (let k = s0; k < x; k++) { runC[y * w + k] = c; runA[y * w + k] = trunk ? -a : a; } // − marks the trunk
					s0 = -1;
				}
			}
		}
		rowZc = new Float32Array(h); rowB = new Float32Array(h);
		for (let y = 0; y < h; y++) {
			let a = Infinity, b = -Infinity;
			if (y < side.h) for (let x = 0; x < side.w; x++) if (side.mask[y * side.w + x]) { a = Math.min(a, x); b = Math.max(b, x); }
			rowZc[y] = a <= b ? (a + b) / 2 - scx : 0;
			rowB[y] = a <= b ? (b - a) / 2 + 0.5 : 0;
		}
	}
	const ellipse = (x, y, z) => {
		const i = y * w + x;
		const trunk = runA[i] < 0, a = Math.abs(runA[i]);
		const b = trunk ? rowB[y] : Math.min(rowB[y], a);
		if (!a || !b) return -1;
		const r = Math.hypot((x - runC[i]) / a, (z - rowZc[y]) / b);
		return (1 - r) * Math.min(a, b);
	};

	const zr = Math.ceil(maxH * Math.max(depth, 1)) + 3;
	const nx = w, ny = h, nz = zr * 2 + 1;
	const f = new Float32Array(nx * ny * nz);
	for (let k = 0; k < nz; k++) {
		const z = k - zr;
		for (let y = 0; y < ny; y++) for (let x = 0; x < nx; x++) {
			const i = y * w + x;
			let v;
			if (lathe) v = lathe[y] > 0 ? lathe[y] - Math.hypot(x - cx, z) : -1;
			else if (side) v = mask[i] ? ellipse(x, y, z) : sd[i];
			else v = mask[i] ? hgt[i] * depth - Math.abs(z) : sd[i];
			if (sideSd) {
				const sxp = Math.round(scx + z);
				const sv = sxp >= 0 && sxp < sw && y < side.h ? sideSd[y * sw + sxp] : -10;
				v = Math.min(v, sv);
			}
			// close the volume at the grid's border
			if (x === 0 || y === 0 || k === 0 || x === nx - 1 || y === ny - 1 || k === nz - 1) v = Math.min(v, -1);
			f[x + nx * (y + ny * k)] = v;
		}
	}
	// the per-row ellipses step by a cell from row to row (rings): smooth the field up / down (and a little sideways)
	if (side) {
		const tmp = new Float32Array(f.length);
		const pass = (stride, n, at) => {
			for (let i = 0; i < f.length; i++) {
				const c = at(i);
				const lo = c > 0 ? f[i - stride] : f[i], hi = c < n - 1 ? f[i + stride] : f[i];
				tmp[i] = (lo + 2 * f[i] + hi) / 4;
			}
			f.set(tmp);
		};
		for (let r = 0; r < 4; r++) pass(nx, ny, (i) => Math.floor(i / nx) % ny);
		pass(1, nx, (i) => i % nx);
	}
	const { pos, faces } = surfaceNets(f, nx, ny, nz);
	if (!faces.length) throw new Error('empty mesh');
	taubin(pos, faces, smooth);

	// grid → metres: feet on y = 0, the centre line on x = 0, image right = the figure's left = +x, front = +z
	const s = height / (grid.sole - grid.top + 1);
	const soleY = grid.sole + 0.5;
	for (let i = 0; i < pos.length; i += 3) {
		const gx = pos[i], gy = pos[i + 1], gz = pos[i + 2];
		pos[i] = (gx - cx) * s;
		pos[i + 1] = (soleY - gy) * s;
		pos[i + 2] = (gz - zr) * s;
	}
	const before = faces.length / 3;
	const low = before > tris ? decimate(pos, faces, tris) : { positions: pos, faces };
	let geo = new THREE.BufferGeometry();
	geo.setAttribute('position', new THREE.BufferAttribute(low.positions, 3));
	geo.setIndex(new THREE.BufferAttribute(low.faces, 1));
	// the orientation of surface nets' quads depends on the axis convention – make the normals point out
	geo = orientOutward(geo);
	geo.computeBoundingBox();
	const toGrid = (X, Y) => [X / s + cx, soleY - Y / s];
	// the side profile's middle at a height (world z; 0 without a side view): where the trunk's joints sit in depth
	const sideMid = (Y) => {
		if (!side) return 0;
		const gy = Math.max(0, Math.min(side.h - 1, Math.round(soleY - Y / s)));
		let a = Infinity, b = -Infinity;
		for (let x = 0; x < side.w; x++) if (side.mask[gy * side.w + x]) { a = Math.min(a, x); b = Math.max(b, x); }
		return a <= b ? ((a + b) / 2 - scx) * s : 0;
	};
	return { geometry: geo, toGrid, sideMid, scale: s, cx, info: { cells: `${nx}×${ny}×${nz}`, rawTris: before, tris: geo.index ? geo.index.count / 3 : geo.attributes.position.count / 3 } };
}

/** flips every face if the mesh's signed volume is negative (inside out) */
function orientOutward(geo) {
	const p = geo.attributes.position, idx = geo.index;
	const get = (i) => [p.getX(i), p.getY(i), p.getZ(i)];
	let vol = 0;
	const cnt = idx ? idx.count : p.count;
	for (let t = 0; t < cnt; t += 3) {
		const [a, b, c] = [0, 1, 2].map((o) => get(idx ? idx.getX(t + o) : t + o));
		vol += a[0] * (b[1] * c[2] - b[2] * c[1]) - a[1] * (b[0] * c[2] - b[2] * c[0]) + a[2] * (b[0] * c[1] - b[1] * c[0]);
	}
	if (vol < 0 && idx) {
		const arr = idx.array;
		for (let t = 0; t < arr.length; t += 3) { const tmp = arr[t + 1]; arr[t + 1] = arr[t + 2]; arr[t + 2] = tmp; }
		idx.needsUpdate = true;
	}
	return geo;
}

/**
 * indexed geometry → non-indexed with one flat colour per face (sampled from the front image at the face's centre),
 * optionally reduced to a palette of `colors` colours; every other attribute (skin) is carried over.
 */
export function flatColored(geo, grid, toGrid, { colors = 0, mono = null } = {}) {
	const flat = geo.index ? geo.toNonIndexed() : geo.clone();
	const p = flat.attributes.position;
	const nf = p.count / 3;
	const cols = new Float32Array(nf * 3);
	for (let fI = 0; fI < nf; fI++) {
		if (mono) { cols.set(mono, fI * 3); continue; }
		let X = 0, Y = 0;
		for (let o = 0; o < 3; o++) { X += p.getX(fI * 3 + o); Y += p.getY(fI * 3 + o); }
		const [gx, gy] = toGrid(X / 3, Y / 3);
		const x = Math.max(0, Math.min(grid.w - 1, Math.round(gx))), y = Math.max(0, Math.min(grid.h - 1, Math.round(gy)));
		const g = y * grid.w + x;
		cols.set([grid.color[g * 3], grid.color[g * 3 + 1], grid.color[g * 3 + 2]], fI * 3);
	}
	const pal = !mono && colors ? palette(cols, colors) : null;
	const out = new Float32Array(p.count * 3);
	for (let fI = 0; fI < nf; fI++) {
		const c = pal ? pal.colors[pal.idx[fI]] : [cols[fI * 3], cols[fI * 3 + 1], cols[fI * 3 + 2]];
		// the image colours are sRGB; three wants linear vertex colours
		const lin = c.map(srgbToLinear);
		for (let o = 0; o < 3; o++) out.set(lin, (fI * 3 + o) * 3);
	}
	flat.setAttribute('color', new THREE.BufferAttribute(out, 3));
	flat.computeVertexNormals();
	return flat;
}

const srgbToLinear = (c) => (c < 0.04045 ? c * 0.0773993808 : Math.pow(c * 0.9478672986 + 0.0521327014, 2.4));

/** a few rounds of a 3×3 box blur */
function blur(a, w, h, rounds) {
	let src = a, dst = new Float32Array(a.length);
	for (let r = 0; r < rounds; r++) {
		for (let y = 0; y < h; y++) for (let x = 0; x < w; x++) {
			let s = 0, n = 0;
			for (let dy = -1; dy <= 1; dy++) for (let dx = -1; dx <= 1; dx++) {
				const xx = x + dx, yy = y + dy;
				if (xx >= 0 && yy >= 0 && xx < w && yy < h) { s += src[yy * w + xx]; n++; }
			}
			dst[y * w + x] = s / n;
		}
		[src, dst] = [dst, src];
	}
	return src;
}
