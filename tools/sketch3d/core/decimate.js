// Quadric error decimation (Garland–Heckbert) for closed meshes: edge collapses ordered by the quadric error of the
// best point, with a heap and lazy invalidation; a collapse that would flip or squash a face is refused. three's
// SimplifyModifier is O(n²) – tens of seconds for a 20k-triangle body; this takes well under a second.

class Heap {
	constructor() { this.a = []; }
	push(item) { const a = this.a; a.push(item); let i = a.length - 1; while (i > 0) { const p = (i - 1) >> 1; if (a[p][0] <= a[i][0]) break; [a[p], a[i]] = [a[i], a[p]]; i = p; } }
	pop() {
		const a = this.a, top = a[0], last = a.pop();
		if (a.length) {
			a[0] = last;
			let i = 0;
			for (;;) { const l = 2 * i + 1, r = l + 1; let m = i; if (l < a.length && a[l][0] < a[m][0]) m = l; if (r < a.length && a[r][0] < a[m][0]) m = r; if (m === i) break; [a[m], a[i]] = [a[i], a[m]]; i = m; }
		}
		return top;
	}
	get size() { return this.a.length; }
}

/** positions (Float32Array xyz), faces (index triples) → { positions, faces } with about `target` triangles */
export function decimate(positions, faces, target) {
	const nv = positions.length / 3;
	const P = Float64Array.from(positions);
	const F = Int32Array.from(faces);
	const nf = F.length / 3;
	const alive = new Uint8Array(nf).fill(1);
	const vAlive = new Uint8Array(nv).fill(1);
	const ver = new Uint32Array(nv); // bumped when a vertex changes → stale heap entries are skipped
	const vf = Array.from({ length: nv }, () => []);
	for (let f = 0; f < nf; f++) for (let k = 0; k < 3; k++) vf[F[f * 3 + k]].push(f);

	// quadrics: 10 floats per vertex (symmetric 4×4)
	const Q = new Float64Array(nv * 10);
	const addPlane = (q, o, a, b, c, d, w) => {
		q[o] += w * a * a; q[o + 1] += w * a * b; q[o + 2] += w * a * c; q[o + 3] += w * a * d;
		q[o + 4] += w * b * b; q[o + 5] += w * b * c; q[o + 6] += w * b * d;
		q[o + 7] += w * c * c; q[o + 8] += w * c * d; q[o + 9] += w * d * d;
	};
	const normal = (f, out) => {
		const a = F[f * 3] * 3, b = F[f * 3 + 1] * 3, c = F[f * 3 + 2] * 3;
		const ux = P[b] - P[a], uy = P[b + 1] - P[a + 1], uz = P[b + 2] - P[a + 2];
		const vx = P[c] - P[a], vy = P[c + 1] - P[a + 1], vz = P[c + 2] - P[a + 2];
		out[0] = uy * vz - uz * vy; out[1] = uz * vx - ux * vz; out[2] = ux * vy - uy * vx;
		return out;
	};
	const n3 = [0, 0, 0];
	for (let f = 0; f < nf; f++) {
		normal(f, n3);
		const len = Math.hypot(n3[0], n3[1], n3[2]);
		if (!len) continue;
		const a = n3[0] / len, b = n3[1] / len, c = n3[2] / len;
		const p = F[f * 3] * 3;
		const d = -(a * P[p] + b * P[p + 1] + c * P[p + 2]);
		for (let k = 0; k < 3; k++) addPlane(Q, F[f * 3 + k] * 10, a, b, c, d, len / 2);
	}

	const q = new Float64Array(10);
	const err = (x, y, z) =>
		q[0] * x * x + 2 * q[1] * x * y + 2 * q[2] * x * z + 2 * q[3] * x + q[4] * y * y + 2 * q[5] * y * z + 2 * q[6] * y + q[7] * z * z + 2 * q[8] * z + q[9];
	/** the best point for collapsing u–v: solve the quadric, else the best of the ends and the middle */
	const best = (u, v) => {
		for (let i = 0; i < 10; i++) q[i] = Q[u * 10 + i] + Q[v * 10 + i];
		const [a, b, c, d, e, f, g, h, i2] = [q[0], q[1], q[2], q[4], q[5], q[7], q[3], q[6], q[8]];
		// | a b c | x = -| g |
		// | b d e |      | h |
		// | c e f |      | i2|
		const det = a * (d * f - e * e) - b * (b * f - e * c) + c * (b * e - d * c);
		const cands = [];
		if (Math.abs(det) > 1e-12) {
			const x = -(g * (d * f - e * e) - b * (h * f - e * i2) + c * (h * e - d * i2)) / det;
			const y = -(a * (h * f - e * i2) - g * (b * f - e * c) + c * (b * i2 - h * c)) / det;
			const z = -(a * (d * i2 - h * e) - b * (b * i2 - h * c) + g * (b * e - d * c)) / det;
			// only if it stays near the edge (an ill-conditioned quadric throws points far away)
			const mx = (P[u * 3] + P[v * 3]) / 2, my = (P[u * 3 + 1] + P[v * 3 + 1]) / 2, mz = (P[u * 3 + 2] + P[v * 3 + 2]) / 2;
			const L = Math.hypot(P[u * 3] - P[v * 3], P[u * 3 + 1] - P[v * 3 + 1], P[u * 3 + 2] - P[v * 3 + 2]);
			if (Math.hypot(x - mx, y - my, z - mz) < L * 2) cands.push([x, y, z]);
		}
		cands.push([P[u * 3], P[u * 3 + 1], P[u * 3 + 2]], [P[v * 3], P[v * 3 + 1], P[v * 3 + 2]], [(P[u * 3] + P[v * 3]) / 2, (P[u * 3 + 1] + P[v * 3 + 1]) / 2, (P[u * 3 + 2] + P[v * 3 + 2]) / 2]);
		let bp = cands[0], be = Infinity;
		for (const c3 of cands) { const e3 = err(c3[0], c3[1], c3[2]); if (e3 < be) { be = e3; bp = c3; } }
		return [Math.max(0, be), bp];
	};

	const heap = new Heap();
	const pushEdge = (u, v) => { const [e, p] = best(u, v); heap.push([e, u, v, ver[u], ver[v], p]); };
	for (let f = 0; f < nf; f++) for (let k = 0; k < 3; k++) {
		const u = F[f * 3 + k], v = F[f * 3 + ((k + 1) % 3)];
		if (u < v) pushEdge(u, v);
	}

	const minQuality = 0.25;
	/** 4√3 · area / Σ edge² – 1 for an equilateral triangle, → 0 for a sliver */
	const quality = (f) => {
		const a = F[f * 3] * 3, b = F[f * 3 + 1] * 3, c = F[f * 3 + 2] * 3;
		normal(f, nq);
		const area2 = Math.hypot(nq[0], nq[1], nq[2]);
		const e = (i, j) => (P[i] - P[j]) ** 2 + (P[i + 1] - P[j + 1]) ** 2 + (P[i + 2] - P[j + 2]) ** 2;
		const s = e(a, b) + e(b, c) + e(c, a);
		return s ? (2 * Math.sqrt(3) * area2) / s : 0;
	};
	const nq = [0, 0, 0];
	let faceCount = nf;
	const na = [0, 0, 0], nb = [0, 0, 0];
	const neighbours = (u) => { const s = new Set(); for (const f of vf[u]) if (alive[f]) for (let k = 0; k < 3; k++) s.add(F[f * 3 + k]); s.delete(u); return s; };
	while (faceCount > target && heap.size) {
		const [, u, v, vu, vv, p] = heap.pop();
		if (!vAlive[u] || !vAlive[v] || ver[u] !== vu || ver[v] !== vv) continue;
		// link condition (keeps the mesh a manifold): u and v may share exactly two neighbours
		const Nu = neighbours(u), Nv = neighbours(v);
		let shared = 0;
		for (const w of Nu) if (Nv.has(w)) shared++;
		if (shared !== 2) continue;
		// refuse if a remaining face around u or v would flip or degenerate
		let ok = true;
		for (const w of [u, v]) for (const f of vf[w]) {
			if (!alive[f]) continue;
			const fu = F[f * 3] === v || F[f * 3 + 1] === v || F[f * 3 + 2] === v;
			const fv = F[f * 3] === u || F[f * 3 + 1] === u || F[f * 3 + 2] === u;
			if (fu && fv) continue; // collapses away
			normal(f, na);
			const save = [P[w * 3], P[w * 3 + 1], P[w * 3 + 2]];
			P[w * 3] = p[0]; P[w * 3 + 1] = p[1]; P[w * 3 + 2] = p[2];
			normal(f, nb);
			P[w * 3] = save[0]; P[w * 3 + 1] = save[1]; P[w * 3 + 2] = save[2];
			const la = Math.hypot(...na), lb = Math.hypot(...nb);
			if (lb < 1e-12 || (na[0] * nb[0] + na[1] * nb[1] + na[2] * nb[2]) / (la * lb) < 0.2) { ok = false; break; }
			// triangle quality after the move (1 = equilateral): refuse slivers – long thin faces read as noise
			const save2 = [P[w * 3], P[w * 3 + 1], P[w * 3 + 2]];
			P[w * 3] = p[0]; P[w * 3 + 1] = p[1]; P[w * 3 + 2] = p[2];
			const qa = quality(f);
			P[w * 3] = save2[0]; P[w * 3 + 1] = save2[1]; P[w * 3 + 2] = save2[2];
			if (qa < minQuality && qa < quality(f)) { ok = false; break; }
		}
		if (!ok) continue;
		// collapse v into u
		P[u * 3] = p[0]; P[u * 3 + 1] = p[1]; P[u * 3 + 2] = p[2];
		for (let i = 0; i < 10; i++) Q[u * 10 + i] += Q[v * 10 + i];
		vAlive[v] = 0;
		for (const f of vf[v]) {
			if (!alive[f]) continue;
			let hasU = false;
			for (let k = 0; k < 3; k++) if (F[f * 3 + k] === u) hasU = true;
			if (hasU) { alive[f] = 0; faceCount--; continue; }
			for (let k = 0; k < 3; k++) if (F[f * 3 + k] === v) F[f * 3 + k] = u;
			vf[u].push(f);
		}
		vf[u] = vf[u].filter((f) => alive[f]);
		ver[u]++; // only the edges at u changed
		for (const w of neighbours(u)) pushEdge(u, w);
	}

	// compact
	const remap = new Int32Array(nv).fill(-1);
	const outP = [], outF = [];
	for (let f = 0; f < nf; f++) {
		if (!alive[f]) continue;
		for (let k = 0; k < 3; k++) {
			const v = F[f * 3 + k];
			if (remap[v] < 0) { remap[v] = outP.length / 3; outP.push(P[v * 3], P[v * 3 + 1], P[v * 3 + 2]); }
			outF.push(remap[v]);
		}
	}
	return { positions: Float32Array.from(outP), faces: Uint32Array.from(outF) };
}
