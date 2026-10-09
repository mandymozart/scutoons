// Auto-rig for a character in A-pose (workflows/characters/base-views.md: front view, symmetrical, arms 30–40° away
// from the body, legs slightly apart). The joints are read off the silhouette – head, neck, the armpits (where the gap
// between arm and body starts), the hand tips, the crotch, the soles – and placed like the storyboard figurines'
// (tools/storyboard-render/figures.js human(): pelvis, spine, chest, neck, head, sh/el/wr L/R, hip/kn/an L/R; L = the
// figure's left = +x, it faces +z). Rest pose = arms and legs hanging straight down, the A-pose is the bind pose (the
// shoulder / hip bones carry the A angle as their rest rotation), so figures.js's pose table applies as it is.
// Every vertex is bound to the bones of its body region only (arm / leg / trunk, from the silhouette) by inverse distance to
// the bone segments, up to 4 bones, so joints bend softly.
import * as THREE from 'three';

/** runs of figure cells in row y: [[x0, x1], …] */
function runs(grid, y) {
	const out = [];
	let s = -1;
	for (let x = 0; x <= grid.w; x++) {
		const m = x < grid.w && grid.mask[y * grid.w + x];
		if (m && s < 0) s = x;
		if (!m && s >= 0) { out.push([s, x - 1]); s = -1; }
	}
	return out;
}

/** 'T.neck' | 'arm.L.el' | 'leg.R.kn' … → where that joint lives in a landmarks object */
function jointRef(lm, key) {
	const [g, a, b] = key.split('.');
	if (g === 'T' && lm.T[a]) return { obj: lm.T, k: a };
	if ((g === 'arm' || g === 'leg') && lm[g][a]?.[b]) return { obj: lm[g][a], k: b };
	return null;
}

/** every movable joint of a landmarks object: [{ key, p: [gx, gy] }] (the mirror of 'arm.L.el' is 'arm.R.el') */
export function jointList(lm) {
	const out = Object.entries(lm.T).map(([k, p]) => ({ key: 'T.' + k, p }));
	for (const g of ['arm', 'leg']) for (const s of ['L', 'R']) for (const [k, p] of Object.entries(lm[g][s])) if (Array.isArray(p)) out.push({ key: g + '.' + s + '.' + k, p });
	return out;
}

/**
 * Landmarks in grid coordinates (x right in the image, y down) + a region label per cell.
 * Image left = the figure's RIGHT (it faces us): side 'R' looks at x < cx, 'L' at x > cx.
 */
export function landmarks(grid, cx, joints = null) {
	const { w, h, mask, top, sole } = grid;
	const H = sole - top + 1;
	const at = (x, y) => x >= 0 && y >= 0 && x < w && y < h && mask[y * w + Math.round(x)];
	const trunkRun = (y) => {
		const r = runs(grid, y);
		return r.find(([a, b]) => a <= cx && cx <= b) ?? r.sort((p, q) => Math.abs((p[0] + p[1]) / 2 - cx) - Math.abs((q[0] + q[1]) / 2 - cx))[0];
	};
	const warn = [];

	// crotch: from knee height up, while there is a gap near the centre line (a tail hanging between the legs fills
	// only a column or two); the first row without one
	const k = Math.max(1, Math.round(H * 0.035));
	const gapNear = (y) => { for (let x = Math.round(cx - k); x <= Math.round(cx + k); x++) if (x >= 0 && x < w && !mask[y * w + x]) return true; return false; };
	let crotch = -1;
	const knee = Math.round(sole - H * 0.25);
	if (gapNear(knee)) { crotch = knee; while (crotch > top + H * 0.3 && gapNear(crotch - 1)) crotch--; }
	// no gap at the knees → the 'robe' bone (figures.js swings it with the legs); the legs are only the feet below the hem
	let robe = false;
	if (crotch < 0) {
		// no gap at the knees: a robe / dress (or legs pressed together) – the hem is where the feet show apart, else
		// the floor
		robe = true;
		let hem = Math.round(sole - H * 0.02);
		if (gapNear(hem)) while (hem > knee && gapNear(hem - 1)) hem--;
		crotch = Math.min(hem, Math.round(sole - H * 0.03));
		warn.push('no gap between the legs: rigged as a robe (one robe bone, the legs only below the hem)');
	}

	// neck: the narrowest trunk row between 6 % and 30 % of the height (below ears / hair tips, above the arms' join);
	// the middle of the narrowest stretch
	const width = (y) => { const r = trunkRun(y); return r ? r[1] - r[0] + 1 : Infinity; };
	let neckW = Infinity;
	for (let y = Math.round(top + H * 0.06); y < top + H * 0.3; y++) neckW = Math.min(neckW, width(y));
	const narrow = [];
	for (let y = Math.round(top + H * 0.06); y < top + H * 0.3; y++) if (width(y) <= neckW + 0.5) narrow.push(y);
	const neck = narrow.length ? narrow[Math.floor(narrow.length / 2)] : Math.round(top + H * 0.15);

	// armpits: the first row below the neck with a separate run beside the trunk
	const armpit = {};
	for (const [side, dir] of [['L', 1], ['R', -1]]) {
		for (let y = neck + 1; y < crotch; y++) {
			const t = trunkRun(y);
			if (!t) continue;
			const beside = runs(grid, y).some(([a, b]) => (dir > 0 ? a > t[1] : b < t[0]));
			if (beside) { armpit[side] = y; break; }
		}
		if (armpit[side] == null) { armpit[side] = Math.round(neck + H * 0.12); warn.push('arm ' + side + ' not separate from the body – armpit guessed'); }
	}

	// arms: the cells below the armpit outside the trunk's edge there (flood fill); the tip = the farthest from the shoulder
	const label = new Uint8Array(w * h); // 0 out, 1 trunk, 2 arm L, 3 arm R, 4 leg L, 5 leg R, 6 robe
	for (let i = 0; i < w * h; i++) if (mask[i]) label[i] = 1;
	const arm = {};
	for (const [side, dir, lab] of [['L', 1, 2], ['R', -1, 3]]) {
		const y0 = armpit[side];
		const t = trunkRun(y0) ?? [cx - 1, cx + 1];
		const edge = dir > 0 ? t[1] : t[0];
		const r = runs(grid, Math.min(h - 1, y0 + 1)).filter(([a, b]) => (dir > 0 ? a > edge : b < edge));
		const armW = r.length ? r[0][1] - r[0][0] + 1 : H * 0.05;
		let sh = [edge - dir * armW * 0.45, y0 - armW * 0.55]; // replaced by the fitted axis below
		const centres = [];
		// the arm's cells: its run followed row by row from the armpit down – a run beside the trunk (below the crotch:
		// not the innermost run, that is the leg) overlapping the row above – until it ends or touches the body
		const seen = new Uint8Array(w * h);
		let prev = r.length ? (dir > 0 ? r[0] : r[r.length - 1]) : null;
		let tip = sh, far = 0;
		for (let y = y0 + 1; prev && y < h; y++) {
			const side = runs(grid, y).filter(([a, b]) => (dir > 0 ? (a + b) / 2 > cx : (a + b) / 2 < cx) && !(a <= cx && cx <= b));
			side.sort((p, q) => Math.abs((p[0] + p[1]) / 2 - cx) - Math.abs((q[0] + q[1]) / 2 - cx)); // innermost first
			const body = y < crotch ? trunkRun(y) : side[0];
			const next = side.find((s) => s !== body && s[0] <= prev[1] + 1 && s[1] >= prev[0] - 1);
			if (!next) break;
			for (let x = next[0]; x <= next[1]; x++) {
				seen[y * w + x] = 1;
				const d = Math.hypot(x - sh[0], y - sh[1]);
				if (d > far) { far = d; tip = [x, y]; }
			}
			centres.push([(next[0] + next[1]) / 2, y]);
			prev = next;
		}
		// the arm's axis (least squares x = a + b·y over the row centres, without the hand), extended up to the
		// shoulder: 40 % of the way from the neck to the armpit, inside the silhouette's outer edge there
		const fit = centres.slice(0, Math.max(3, Math.round(centres.length * 0.7)));
		if (fit.length >= 3) {
			const n = fit.length, my = fit.reduce((t, c) => t + c[1], 0) / n, mx = fit.reduce((t, c) => t + c[0], 0) / n;
			let num = 0, den = 0;
			for (const [x, y] of fit) { num += (x - mx) * (y - my); den += (y - my) ** 2; }
			const b = den ? num / den : 0;
			const shY = neck + (y0 - neck) * 0.4;
			let shX = mx + b * (shY - my);
			const row = runs(grid, Math.round(shY));
			const outer = row.length ? (dir > 0 ? row[row.length - 1][1] : row[0][0]) : shX;
			shX = dir > 0 ? Math.min(shX, outer - armW * 0.5) : Math.max(shX, outer + armW * 0.5);
			sh = [shX, shY];
			far = Math.hypot(tip[0] - sh[0], tip[1] - sh[1]);
		}
		// the shoulder cap belongs to the arm too: between neck and armpit (not below – a dress flaring out under a narrow
		// waist would be dragged along), beyond the trunk's edge, and near the arm's axis (a cape's far hem stays)
		const ax = tip[0] - sh[0], ay = tip[1] - sh[1], al = Math.hypot(ax, ay) || 1;
		for (let y = neck; y <= y0; y++) for (let x = 0; x < w; x++) {
			const i = y * w + x;
			const off = Math.abs((x - sh[0]) * ay - (y - sh[1]) * ax) / al; // distance to the axis line
			if (mask[i] && off <= armW * 0.9 && (dir > 0 ? x > edge - armW * 0.15 : x < edge + armW * 0.15)) seen[i] = 1;
		}
		// the hand: splayed fingers are runs of their own – a run beside the body (not the trunk, not the leg) that touches
		// the arm's cells in the row above is the arm's too, down to a little below the traced arm's end
		const lastY = centres.length ? centres[centres.length - 1][1] : y0;
		for (let y = y0 + 1; y <= Math.min(h - 1, lastY + armW * 1.5); y++) {
			const side = runs(grid, y).filter(([a, b]) => (dir > 0 ? (a + b) / 2 > cx : (a + b) / 2 < cx) && !(a <= cx && cx <= b));
			side.sort((p, q) => Math.abs((p[0] + p[1]) / 2 - cx) - Math.abs((q[0] + q[1]) / 2 - cx));
			const body = y < crotch ? trunkRun(y) : side[0];
			for (const s of side) {
				if (s === body) continue;
				let touches = false;
				for (let x = Math.max(0, s[0] - 1); x <= Math.min(w - 1, s[1] + 1) && !touches; x++) touches = !!seen[(y - 1) * w + x];
				if (touches) for (let x = s[0]; x <= s[1]; x++) seen[y * w + x] = 1;
			}
		}
		for (let i = 0; i < w * h; i++) {
			if (!seen[i]) continue;
			label[i] = lab;
			const x = i % w, y = (i / w) | 0;
			if (y > y0) { const d = Math.hypot(x - sh[0], y - sh[1]); if (d > far) { far = d; tip = [x, y]; } }
		}
		const L = (k) => [sh[0] + (tip[0] - sh[0]) * k, sh[1] + (tip[1] - sh[1]) * k];
		arm[side] = { sh, el: L(0.42), wr: L(0.78), tip, width: armW };
		if (far < H * 0.15) warn.push(`arm ${side} looks short (${Math.round(far)} cells) – check the A-pose`);
	}

	// legs: below the crotch, each side of the centre line
	const leg = {};
	const legRun = (y, dir) => runs(grid, y).filter(([a, b]) => label[y * w + a] === 1 && (dir > 0 ? (a + b) / 2 > cx : (a + b) / 2 < cx))
		.sort((p, q) => Math.abs((p[0] + p[1]) / 2 - cx) - Math.abs((q[0] + q[1]) / 2 - cx))[0];
	for (const [side, dir, lab] of [['L', 1, 4], ['R', -1, 5]]) {
		const below = legRun(Math.min(sole, crotch + 2), dir);
		const lw = below ? below[1] - below[0] + 1 : H * 0.08;
		const hipX = below ? (below[0] + below[1]) / 2 : cx + dir * H * 0.05;
		const ankleY = sole - H * 0.045;
		const ar = legRun(Math.round(ankleY), dir);
		const anX = ar ? (ar[0] + ar[1]) / 2 : hipX;
		const hip = robe ? [cx + (anX - cx) * 0.85, sole - H * 0.48] : [hipX, crotch - lw * 0.35];
		leg[side] = { hip, kn: [(hipX + anX) / 2, (hip[1] + ankleY) / 2], an: [anX, ankleY], toe: [anX, sole + 0.5], width: lw };
		for (let y = crotch; y < h; y++) for (let x = 0; x < w; x++) if (label[y * w + x] === 1 && (dir > 0 ? x >= cx : x < cx)) label[y * w + x] = lab;
	}

	const pelvisY = Math.min(leg.L.hip[1], leg.R.hip[1]) - H * 0.02;
	// the robe: everything of the trunk between the pelvis and the hem
	if (robe) for (let y = Math.round(pelvisY); y < crotch; y++) for (let x = 0; x < w; x++) if (label[y * w + x] === 1) label[y * w + x] = 6;
	const shY = (arm.L.sh[1] + arm.R.sh[1]) / 2;
	const T = {
		pelvis: [cx, pelvisY],
		spine: [cx, pelvisY + (shY - pelvisY) * 0.12],
		chest: [cx, pelvisY + (shY - pelvisY) * 0.62],
		neck: [cx, Math.max(neck + 1, shY - (shY - neck) * 0.15)],
		head: [cx, neck],
		top: [cx, top - 0.5]
	};
	if (robe) T.robe = [cx, pelvisY - H * 0.02];
	const lm = { T, arm, leg };
	// joints moved by hand win (grid coordinates)
	for (const [key, p] of Object.entries(joints ?? {})) { const at = jointRef(lm, key); if (at) at.obj[at.k] = p.slice(); }
	return { T, arm, leg, label, crotch, neck, armpit, robe, warn };
}

/**
 * geometry (indexed, world metres) + the mesh's grid mapping → a SkinnedMesh-ready rig:
 * { bones (by name), root, skeleton, geometry (with skinIndex / skinWeight), lm (landmarks), restPelvisY }
 */
export function autoRig(geometry, grid, { cx, scale, toGrid, sideMid = () => 0 }, joints = null) {
	const lm = landmarks(grid, cx, joints);
	const soleY = grid.sole + 0.5;
	const W = ([gx, gy]) => new THREE.Vector3((gx - cx) * scale, (soleY - gy) * scale, 0);

	// joint world positions; image left = the figure's right: arm.R lies at x < cx → world x < 0 ✓
	const P = {
		pelvis: W(lm.T.pelvis), spine: W(lm.T.spine), chest: W(lm.T.chest), neck: W(lm.T.neck), head: W(lm.T.head), top: W(lm.T.top)
	};
	for (const s of ['L', 'R']) {
		P['sh' + s] = W(lm.arm[s].sh); P['el' + s] = W(lm.arm[s].el); P['wr' + s] = W(lm.arm[s].wr); P['tip' + s] = W(lm.arm[s].tip);
		P['hip' + s] = W(lm.leg[s].hip); P['kn' + s] = W(lm.leg[s].kn); P['an' + s] = W(lm.leg[s].an); P['toe' + s] = W(lm.leg[s].toe);
	}

	// depth from the side profile: the trunk joints at its middle, each limb chain at its root's depth
	for (const k of ['pelvis', 'spine', 'chest', 'neck', 'head', 'top']) P[k].z = sideMid(P[k].y);
	for (const s of ['L', 'R']) {
		const za = sideMid(P['sh' + s].y), zl = sideMid(P['hip' + s].y);
		for (const k of ['sh', 'el', 'wr', 'tip']) P[k + s].z = za;
		for (const k of ['hip', 'kn', 'an', 'toe']) P[k + s].z = zl;
	}

	// bones: the trunk straight up; limbs as chains whose rest rotation (about z) is the A angle, children straight down
	const bones = {};
	const root = new THREE.Bone();
	root.name = 'root';
	const mk = (name, parent, local) => {
		const b = new THREE.Bone();
		b.name = name;
		b.position.copy(local);
		(parent ? bones[parent] : root).add(b);
		bones[name] = b;
		return b;
	};
	mk('pelvis', null, P.pelvis.clone());
	mk('spine', 'pelvis', P.spine.clone().sub(P.pelvis));
	mk('chest', 'spine', P.chest.clone().sub(P.spine));
	mk('neck', 'chest', P.neck.clone().sub(P.chest));
	mk('head', 'neck', P.head.clone().sub(P.neck));
	if (lm.robe) { P.robe = W(lm.T.robe); P.hem = W([cx, lm.crotch]); P.robe.z = sideMid(P.robe.y); P.hem.z = sideMid(P.hem.y); mk('robe', 'pelvis', P.robe.clone().sub(P.pelvis)); }
	const limb = (names, parent, pts) => {
		// pts: the chain's joint positions + its end; the first bone turns by the angle of the whole chain
		const dir = pts[pts.length - 1].clone().sub(pts[0]);
		const ang = Math.atan2(dir.x, -dir.y); // 0 = straight down, + = towards +x
		const first = mk(names[0], parent, pts[0].clone().sub(P[parent]));
		first.rotation.z = ang;
		for (let i = 1; i < names.length; i++) mk(names[i], names[i - 1], new THREE.Vector3(0, -pts[i].distanceTo(pts[i - 1]), 0));
		first.userData.restZ = ang;
	};
	for (const s of ['L', 'R']) {
		limb(['sh' + s, 'el' + s, 'wr' + s], 'chest', [P['sh' + s], P['el' + s], P['wr' + s], P['tip' + s]]);
		limb(['hip' + s, 'kn' + s, 'an' + s], 'pelvis', [P['hip' + s], P['kn' + s], P['an' + s], P['toe' + s]]);
	}
	root.updateMatrixWorld(true);

	// the bone segments for skinning (bone → its child's position or the chain's end)
	const wp = (n) => bones[n].getWorldPosition(new THREE.Vector3());
	const SEG = [
		['pelvis', wp('pelvis'), wp('spine'), [1, 4, 5, 6]], ['spine', wp('spine'), wp('chest'), [1]], ['chest', wp('chest'), wp('neck'), [1, 2, 3]],
		['neck', wp('neck'), wp('head'), [1]], ['head', wp('head'), P.top, [1]]
	];
	if (lm.robe) SEG.push(['robe', wp('robe'), P.hem, [6]]);
	for (const [s, a, l] of [['L', 2, 4], ['R', 3, 5]]) {
		SEG.push(['sh' + s, wp('sh' + s), wp('el' + s), [a]], ['el' + s, wp('el' + s), wp('wr' + s), [a]], ['wr' + s, wp('wr' + s), P['tip' + s], [a]]);
		SEG.push(['hip' + s, wp('hip' + s), wp('kn' + s), lm.robe ? [l] : [l, 1]], ['kn' + s, wp('kn' + s), wp('an' + s), [l]], ['an' + s, wp('an' + s), P['toe' + s], [l]]);
	}
	const order = ['root', ...Object.keys(bones)];
	const all = [root, ...Object.values(bones)];
	const index = Object.fromEntries(order.map((n, i) => [n, i]));

	const pos = geometry.attributes.position;
	const n = pos.count;
	const si = new Uint16Array(n * 4), sw = new Float32Array(n * 4);
	const v = new THREE.Vector3(), seg = new THREE.Line3(), q = new THREE.Vector3();
	const regionAt = (X, Y) => {
		const [gx, gy] = toGrid(X, Y);
		const x0 = Math.round(gx), y0 = Math.round(gy);
		for (let r = 0; r <= 3; r++) for (let dy = -r; dy <= r; dy++) for (let dx = -r; dx <= r; dx++) {
			const x = x0 + dx, y = y0 + dy;
			if (x >= 0 && y >= 0 && x < grid.w && y < grid.h && lm.label[y * grid.w + x]) return lm.label[y * grid.w + x];
		}
		return 1;
	};
	for (let i = 0; i < n; i++) {
		v.fromBufferAttribute(pos, i);
		const reg = regionAt(v.x, v.y);
		const cand = [];
		for (const [name, a, b, regions] of SEG) {
			seg.set(a, b);
			seg.closestPointToPoint(v, true, q);
			let d = q.distanceTo(v);
			if (!regions.includes(reg)) continue; // only the bones of the vertex's own region (hands near the thighs!)
			cand.push([index[name], 1 / Math.pow(d + 0.01, 4)]);
		}
		cand.sort((p, r) => r[1] - p[1]);
		const top4 = cand.slice(0, 4);
		const sum = top4.reduce((t, c) => t + c[1], 0);
		top4.forEach(([b, wgt], k) => { si[i * 4 + k] = b; sw[i * 4 + k] = wgt / sum; });
	}
	geometry.setAttribute('skinIndex', new THREE.Uint16BufferAttribute(si, 4));
	geometry.setAttribute('skinWeight', new THREE.Float32BufferAttribute(sw, 4));
	const skeleton = new THREE.Skeleton(all);
	return { bones, root, skeleton, geometry, lm, P, restPelvisY: P.pelvis.y };
}

/** a SkinnedMesh from a rigged, coloured geometry; joints like figures.js's (+ empty hands so its pose() runs) */
export function skinnedFigure(geometry, rig, material) {
	const mesh = new THREE.SkinnedMesh(geometry, material);
	mesh.name = 'body';
	mesh.add(rig.root);
	mesh.bind(rig.skeleton);
	const joints = { ...rig.bones, handL: { fingers: [] }, handR: { fingers: [] } };
	return { kind: 'human', root: mesh, joints, mesh, restPelvisY: rig.restPelvisY };
}
