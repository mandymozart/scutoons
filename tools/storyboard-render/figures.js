// Posable figurines of the cast (owner 2026-10-09: "3d figurines that match the outlines of our characters, a simple
// bone structure we can pose"; "more detailed figurines"). No textures – under extremely hard light the silhouette,
// the folds and the planes carry everything, so the shapes are modelled for that: tapered muscles, fingers, the
// sphynx ears, robe folds that catch the light, a tiger with real anatomy. Units: metres; a figure faces +z, its left
// is +x. Refine as we go: a character = a builder + proportions + its poses.
import * as THREE from 'three';

const D = Math.PI / 180;

/** the one material for everything (render.js sets it: lit white, the Sin City pass makes it black / white) */
export const look = { mat: null };

function add(geo, parent, { x = 0, y = 0, z = 0, rx = 0, ry = 0, rz = 0, sx = 1, sy = 1, sz = 1 } = {}) {
	const m = new THREE.Mesh(geo, look.mat);
	m.position.set(x, y, z);
	m.rotation.set(rx, ry, rz);
	m.scale.set(sx, sy, sz);
	m.castShadow = m.receiveShadow = true;
	parent.add(m);
	return m;
}
function joint(parent, x = 0, y = 0, z = 0) {
	const j = new THREE.Group();
	j.position.set(x, y, z);
	parent.add(j);
	return j;
}
const ell = (r) => new THREE.SphereGeometry(r, 20, 14);
/** a tapered segment hanging down (−y) from its joint: radius r0 at the joint, r1 at the end, a bulge (muscle) at 35 % */
function seg(j, L, r0, r1, bulge = 1.12) {
	const pts = [];
	const n = 10;
	for (let i = 0; i <= n; i++) {
		const t = i / n;
		const r = (r0 + (r1 - r0) * t) * (1 + (bulge - 1) * Math.sin(Math.PI * Math.min(1, t / 0.7)));
		pts.push(new THREE.Vector2(Math.max(0.001, r), -t * L));
	}
	pts.unshift(new THREE.Vector2(0.001, 0));
	pts.push(new THREE.Vector2(0.001, -L));
	add(new THREE.LatheGeometry(pts, 16), j);
	return joint(j, 0, -L, 0);
}
/** a lathe with vertical folds: profile [[r, y], …] from top to bottom; folds = count, depth = fraction of r */
function folded(profile, folds = 9, depth = 0.08, seed = 0) {
	const g = new THREE.LatheGeometry(profile.map(([r, y]) => new THREE.Vector2(r, y)), 64);
	const p = g.attributes.position;
	const v = new THREE.Vector3();
	for (let i = 0; i < p.count; i++) {
		v.fromBufferAttribute(p, i);
		const a = Math.atan2(v.z, v.x);
		const yTop = profile[0][1], yBot = profile[profile.length - 1][1];
		const t = (yTop - v.y) / (yTop - yBot || 1); // folds deepen towards the hem
		const k = 1 + depth * t * (Math.sin(a * folds + seed) * 0.7 + Math.sin(a * folds * 2.3 + seed * 2) * 0.3);
		p.setXYZ(i, v.x * k, v.y, v.z * k);
	}
	g.computeVertexNormals();
	return g;
}

// --- humans ------------------------------------------------------------------------------------------------------
function hand(wrist, side) {
	const palm = joint(wrist, 0, -0.02, 0);
	add(new THREE.BoxGeometry(0.075, 0.09, 0.03), palm, { y: -0.045 });
	const fingers = [];
	for (let i = 0; i < 4; i++) {
		const f = joint(palm, (-0.027 + i * 0.018), -0.09, 0);
		seg(f, 0.075 - Math.abs(i - 1.5) * 0.008, 0.008, 0.006, 1);
		fingers.push(f);
	}
	const thumb = joint(palm, 0.038 * side, -0.035, 0.012);
	thumb.rotation.z = 0.6 * side;
	seg(thumb, 0.06, 0.01, 0.007, 1);
	fingers.push(thumb);
	return { palm, fingers };
}

/**
 * A human figure (PLF / Cat Pope, YPLF, passers-by). opts: robe (the papal robe: cassock with folds, the cape,
 * wide sleeves, the skullcap), cat (the sphynx head), height (m), build (1 = average). Returns { kind, root, joints }.
 */
export function human({ robe = false, cat = false, height = 1.8, build = 1 } = {}) {
	const b = build;
	const root = new THREE.Group();
	const J = {};
	J.pelvis = joint(root, 0, 0.98, 0);
	add(ell(0.15), J.pelvis, { y: 0, sx: 1.18 * b, sy: 0.75, sz: 0.85 });
	J.spine = joint(J.pelvis, 0, 0.06, 0);
	add(ell(0.13), J.spine, { y: 0.12, sx: 1.05 * b, sy: 1.2, sz: 0.72 }); // abdomen
	J.chest = joint(J.spine, 0, 0.27, 0);
	add(ell(0.16), J.chest, { y: 0.1, sx: 1.12 * b, sy: 1.05, sz: 0.75 }); // ribcage
	add(ell(0.075), J.chest, { x: 0.075, y: 0.13, z: 0.08, sx: 1.2, sy: 0.7, sz: 0.6 }); // pectorals
	add(ell(0.075), J.chest, { x: -0.075, y: 0.13, z: 0.08, sx: 1.2, sy: 0.7, sz: 0.6 });
	J.neck = joint(J.chest, 0, 0.25, 0);
	add(new THREE.CylinderGeometry(0.048, 0.06, 0.12, 14), J.neck, { y: 0.05 });
	J.head = joint(J.neck, 0, 0.11, 0);
	if (cat) {
		// the sphynx: a wide skull, high cheekbones, a short muzzle, the very big upright ears – THE silhouette
		add(ell(0.11), J.head, { y: 0.1, sx: 1.12, sy: 0.95, sz: 1.05 });
		add(ell(0.06), J.head, { x: 0.065, y: 0.07, z: 0.06, sx: 1, sy: 0.75, sz: 0.9 }); // cheekbones
		add(ell(0.06), J.head, { x: -0.065, y: 0.07, z: 0.06, sx: 1, sy: 0.75, sz: 0.9 });
		add(ell(0.055), J.head, { y: 0.045, z: 0.1, sx: 1.05, sy: 0.75, sz: 0.85 }); // muzzle
		add(ell(0.02), J.head, { y: 0.07, z: 0.15, sx: 1.2, sy: 0.7, sz: 0.8 }); // nose
		add(new THREE.BoxGeometry(0.17, 0.02, 0.05), J.head, { y: 0.13, z: 0.09, rx: -0.2 }); // brow – shadows the eyes
		for (const s of [1, -1]) {
			const ear = add(new THREE.ConeGeometry(0.07, 0.2, 3, 1, true), J.head, { x: 0.085 * s, y: 0.27, z: -0.01, rz: -0.32 * s, ry: (s > 0 ? 0.5 : -0.5) + Math.PI / 6, sx: 1, sy: 1, sz: 0.45 });
			ear.material = new THREE.MeshLambertMaterial({ color: 0xffffff, side: THREE.DoubleSide });
		}
		if (robe) add(new THREE.SphereGeometry(0.07, 16, 8, 0, Math.PI * 2, 0, Math.PI / 2), J.head, { y: 0.19, sx: 1, sy: 0.5, sz: 1 }); // the skullcap
	} else {
		add(ell(0.105), J.head, { y: 0.11, sx: 0.88, sy: 1.12, sz: 1 });
		add(ell(0.045), J.head, { y: 0.03, z: 0.05, sx: 1.2, sy: 0.8, sz: 0.9 }); // jaw
		add(new THREE.ConeGeometry(0.018, 0.045, 6), J.head, { y: 0.1, z: 0.1, rx: Math.PI / 2 }); // nose
		for (const s of [1, -1]) add(ell(0.025), J.head, { x: 0.09 * s, y: 0.1, sx: 0.4, sy: 1, sz: 0.7 });
	}
	for (const [side, s] of [['L', 1], ['R', -1]]) {
		const sh = (J['sh' + side] = joint(J.chest, 0.19 * s * b, 0.2, 0));
		add(ell(0.07), sh, { sx: 1.1, sy: 0.9, sz: 1 }); // deltoid
		const el = (J['el' + side] = seg(sh, 0.29, 0.058, 0.042));
		const wr = (J['wr' + side] = seg(el, 0.26, 0.046, 0.031));
		const h = hand(wr, s);
		J['hand' + side] = h;
		const hip = (J['hip' + side] = joint(J.pelvis, 0.095 * s, -0.04, 0));
		const kn = (J['kn' + side] = seg(hip, 0.45, 0.085, 0.052));
		const an = (J['an' + side] = seg(kn, 0.43, 0.058, 0.035));
		add(new THREE.BoxGeometry(0.085, 0.055, 0.25), an, { y: -0.025, z: 0.07 }); // foot
		add(ell(0.045), an, { y: -0.02, z: 0.19, sx: 1, sy: 0.6, sz: 1 }); // toes
	}
	if (robe) {
		// the cassock: from the waist to the ankles, flared, with folds; it follows the legs (render: J.robe tilts with
		// the mean hip swing and shortens when the figure crouches / kneels)
		J.robe = joint(J.pelvis, 0, 0.05, 0);
		add(folded([[0.17, 0], [0.2, -0.15], [0.27, -0.5], [0.34, -0.85], [0.4, -1.0]], 11, 0.12, 1), J.robe);
		add(folded([[0.15, 0.4], [0.17, 0.2], [0.175, 0]], 7, 0.04, 2), J.spine); // the torso in cloth
		add(new THREE.TorusGeometry(0.17, 0.025, 8, 24), J.spine, { y: 0.02, rx: Math.PI / 2 }); // the sash
		// the cape (mozzetta) over the shoulders
		add(folded([[0.08, 0.26], [0.2, 0.2], [0.3, 0.08], [0.34, -0.02]], 8, 0.08, 3), J.chest, { y: 0.02 });
		// wide sleeves, bell-shaped, on the forearms
		for (const side of ['L', 'R']) add(folded([[0.05, 0], [0.07, -0.12], [0.1, -0.24]], 6, 0.12, 4), J['el' + side]);
	}
	root.scale.setScalar(height / 1.8);
	return { kind: 'human', root, joints: J };
}

// --- the tiger -------------------------------------------------------------------------------------------------
/**
 * The white mountain tiger: about 2.8 m nose to rump, 1 m at the shoulder. Spine in three parts (chest, waist, hips)
 * so it can gallop, stalk, rear; three-part legs on the toes; a jaw that opens; an eight-part tail.
 */
export function tiger({ scale = 1 } = {}) {
	const root = new THREE.Group();
	const J = {};
	J.hips = joint(root, 0, 0.95, -0.55);
	add(ell(0.3), J.hips, { y: 0, sx: 1.05, sy: 1.05, sz: 1.15 }); // haunch
	J.waist = joint(J.hips, 0, 0.02, 0.32);
	add(ell(0.27), J.waist, { z: 0.25, sx: 0.95, sy: 0.95, sz: 1.4 }); // belly
	J.chest = joint(J.waist, 0, 0.04, 0.55);
	add(ell(0.34), J.chest, { y: 0.02, sx: 1.0, sy: 1.08, sz: 1.15 }); // ribcage – deep
	for (const s of [1, -1]) add(ell(0.16), J.chest, { x: 0.17 * s, y: 0.22, z: 0.18, sx: 0.7, sy: 1.2, sz: 1 }); // shoulder blades
	J.neck = joint(J.chest, 0, 0.18, 0.3);
	add(new THREE.CylinderGeometry(0.2, 0.27, 0.42, 18), J.neck, { y: 0.06, z: 0.16, rx: 1.0 });
	J.head = joint(J.neck, 0, 0.22, 0.38);
	add(ell(0.21), J.head, { sx: 1.18, sy: 1.0, sz: 1.05 }); // skull – wide
	for (const s of [1, -1]) add(ell(0.13), J.head, { x: 0.19 * s, y: -0.06, z: -0.02, sx: 0.8, sy: 1, sz: 0.9 }); // the cheek ruff
	add(ell(0.13), J.head, { y: -0.05, z: 0.19, sx: 1.05, sy: 0.75, sz: 1.05 }); // muzzle
	add(ell(0.05), J.head, { y: 0.0, z: 0.31, sx: 1.2, sy: 0.7, sz: 0.8 }); // nose
	add(new THREE.BoxGeometry(0.3, 0.05, 0.08), J.head, { y: 0.1, z: 0.13, rx: -0.3 }); // brow
	for (const s of [1, -1]) {
		add(new THREE.SphereGeometry(0.07, 14, 8, 0, Math.PI * 2, 0, Math.PI / 2), J.head, { x: 0.14 * s, y: 0.17, z: -0.05, rx: -0.3, rz: -0.4 * s, sx: 1, sy: 1.1, sz: 0.5 }); // round ears
		add(new THREE.ConeGeometry(0.022, 0.11, 8), J.head, { x: 0.06 * s, y: -0.15, z: 0.25 }).rotation.x = Math.PI; // upper fangs
	}
	J.jaw = joint(J.head, 0, -0.11, 0.06);
	add(ell(0.1), J.jaw, { y: -0.03, z: 0.12, sx: 0.95, sy: 0.45, sz: 1.2 });
	for (const s of [1, -1]) add(new THREE.ConeGeometry(0.018, 0.08, 8), J.jaw, { x: 0.05 * s, y: 0.02, z: 0.2 }); // lower fangs
	// legs: front from the chest, hind from the hips; upper, lower, the long foot (on the toes), the paw
	const leg = (parent, name, x, y, z, front) => {
		const top = (J[name] = joint(parent, x, y, z));
		const l1 = (J[name + '2'] = seg(top, front ? 0.36 : 0.42, front ? 0.13 : 0.16, 0.085, 1.18));
		const l2 = (J[name + '3'] = seg(l1, front ? 0.3 : 0.36, 0.08, 0.06, 1.05));
		const l3 = (J[name + '4'] = seg(l2, front ? 0.14 : 0.2, 0.06, 0.055, 1));
		add(ell(0.09), l3, { y: -0.01, z: 0.05, sx: 1.05, sy: 0.55, sz: 1.35 }); // the paw
		for (let i = 0; i < 4; i++) add(ell(0.03), l3, { x: -0.045 + i * 0.03, y: -0.02, z: 0.15 }); // toes
		// rest pose: the elbow / hock angles of a standing cat
		if (front) { l1.rotation.x = 0.25; l2.rotation.x = -0.3; l3.rotation.x = 0.05; }
		else { top.rotation.x = -0.3; l1.rotation.x = 0.85; l2.rotation.x = -0.6; }
	};
	for (const s of [1, -1]) {
		leg(J.chest, s > 0 ? 'FL' : 'FR', 0.17 * s, -0.12, 0.05, true);
		leg(J.hips, s > 0 ? 'BL' : 'BR', 0.17 * s, -0.05, -0.05, false);
	}
	// tail: eight segments, a lazy S
	let t = (J.tail = joint(J.hips, 0, 0.1, -0.28));
	J.tailSegs = [];
	for (let i = 0; i < 8; i++) {
		t.rotation.x = i === 0 ? -2.1 : 0.16;
		add(new THREE.CylinderGeometry(0.05 - i * 0.004, 0.055 - i * 0.004, 0.13, 10), t, { y: -0.065 });
		J.tailSegs.push(t);
		t = joint(t, 0, -0.13, 0);
	}
	root.scale.setScalar(scale);
	return { kind: 'tiger', root, joints: J };
}

/** a crow creature (KRÄHENWESEN): a big bird, layered wing feathers, a heavy beak */
export function crow({ scale = 1 } = {}) {
	const root = new THREE.Group();
	const J = {};
	J.body = joint(root, 0, 0.5, 0);
	add(ell(0.22), J.body, { rx: -0.5, sx: 0.9, sy: 1.3, sz: 0.9 });
	J.head = joint(J.body, 0, 0.3, 0.1);
	add(ell(0.12), J.head, { sx: 0.95, sy: 1, sz: 1.1 });
	add(new THREE.ConeGeometry(0.05, 0.26, 4), J.head, { z: 0.18, rx: Math.PI / 2, sx: 1, sy: 1, sz: 0.7 });
	for (const [side, s] of [['L', 1], ['R', -1]]) {
		const w = (J['wing' + side] = joint(J.body, 0.16 * s, 0.14, 0));
		for (let i = 0; i < 6; i++) add(new THREE.BoxGeometry(0.42 - i * 0.03, 0.012, 0.08), w, { x: (0.22 + i * 0.05) * s, z: -0.12 + i * 0.05, ry: (0.3 - i * 0.12) * s, rz: -0.12 * s });
		const leg = joint(J.body, 0.07 * s, -0.24, 0);
		seg(leg, 0.28, 0.022, 0.015, 1);
	}
	for (let i = 0; i < 5; i++) add(new THREE.BoxGeometry(0.05, 0.012, 0.36), J.body, { x: -0.08 + i * 0.04, y: -0.14, z: -0.3, rx: 0.55, ry: (i - 2) * 0.1 });
	root.scale.setScalar(scale);
	return { kind: 'crow', root, joints: J };
}

// --- poses -------------------------------------------------------------------------------------------------------
// Human: sh / hip = [forward°, out°]; el / kn bend°; spine lean°; twist°; head [nod°, turn°]; fist = curl the fingers;
// root { pitch, roll } – the whole body (falls, crawls); y = pelvis offset (m).
const HUMAN = {
	stand: { shL: [2, 8], shR: [2, 8], elL: 8, elR: 8 },
	walk: { shL: [-24, 7], shR: [26, 7], elL: 10, elR: 22, hipL: [26, 0], knL: 8, hipR: [-18, 0], knR: 24 },
	run: { spine: 20, shL: [-50, 12], shR: [60, 12], elL: 95, elR: 85, hipL: [62, 0], knL: 70, hipR: [-38, 0], knR: 95, y: 0.05, fist: 1 },
	jump: { spine: -8, shL: [155, 25], shR: [155, 25], elL: 15, elR: 15, hipL: [50, 0], knL: 85, hipR: [22, 0], knR: 65, y: 0.5 },
	fall: { root: { pitch: -60, roll: 28 }, shL: [135, 65], shR: [105, 75], elL: 35, elR: 20, hipL: [45, 22], hipR: [-10, 28], knL: 30, knR: 50, head: [-20, 15], y: 0.8 },
	slip: { root: { pitch: -40 }, shL: [45, 85], shR: [65, 80], hipL: [75, 0], hipR: [5, 0], knR: 30, head: [-15, 0], y: 0.1 },
	crouch: { spine: 38, shL: [48, 12], shR: [48, 12], elL: 65, elR: 65, hipL: [108, 8], hipR: [108, 8], knL: 128, knR: 128, head: [-25, 0], y: -0.48 },
	kneel: { spine: 6, hipL: [92, 0], knL: 95, hipR: [0, 0], knR: 98, shL: [12, 8], shR: [12, 8], elL: 20, elR: 20, y: -0.43 },
	crawl: { root: { pitch: 75 }, shL: [85, 10], shR: [105, 10], elL: 15, elR: 25, hipL: [72, 0], knL: 100, hipR: [98, 0], knR: 90, head: [-55, 0], y: -0.52 },
	sit: { hipL: [92, 8], hipR: [92, 8], knL: 92, knR: 92, shL: [22, 8], shR: [22, 8], elL: 45, elR: 45, y: -0.46 },
	lie: { root: { pitch: -90 }, shL: [0, 18], shR: [0, 18], y: -0.82 },
	reach: { spine: 12, shR: [92, 0], elR: 5, shL: [12, 8], hipL: [16, 0] },
	point: { shR: [90, -18], elR: 0, shL: [2, 8], head: [0, -10], fist: 0.6 },
	throw: { spine: -12, twist: -30, shR: [172, 22], elR: 75, shL: [62, 12], hipL: [38, 0], knL: 22, hipR: [-22, 0], fist: 1 },
	cast: { spine: 14, shL: [88, -14], shR: [88, -14], elL: 18, elR: 18, hipL: [38, 0], knL: 32, hipR: [-28, 0], knR: 12 },
	'arms-up': { shL: [22, 152], shR: [22, 152], elL: 28, elR: 28, head: [-12, 0] },
	'hug-self': { spine: 14, shL: [58, -38], shR: [58, -38], elL: 128, elR: 128, head: [16, 0], fist: 0.7 },
	stamp: { spine: 8, shL: [58, -38], shR: [58, -38], elL: 128, elR: 128, hipL: [78, 0], knL: 100, fist: 0.7 },
	push: { spine: 30, shL: [82, 0], shR: [82, 0], elL: 12, elR: 12, hipL: [28, 0], knL: 28, hipR: [-38, 0] },
	'look-back': { spine: 6, twist: 48, head: [0, 72], shL: [-12, 8], shR: [18, 8] },
	pray: { shL: [48, -28], shR: [48, -28], elL: 112, elR: 112, head: [28, 0] }
};

/** the human pose names (tools/sketch3d poses its rigged models with this table) */
export const humanPoses = Object.keys(HUMAN);

function applyHuman(f, name) {
	const p = HUMAN[name] ?? HUMAN.stand;
	const J = f.joints;
	for (const k of ['shL', 'shR', 'elL', 'elR', 'wrL', 'wrR', 'hipL', 'hipR', 'knL', 'knR', 'anL', 'anR', 'spine', 'chest', 'head']) J[k].rotation.set(0, 0, 0);
	const side = (k, s) => {
		const v = p[k];
		if (!v) return;
		J[k].rotation.x = -v[0] * D;
		J[k].rotation.z = v[1] * D * s;
	};
	side('shL', 1); side('shR', -1); side('hipL', 1); side('hipR', -1);
	for (const s of ['L', 'R']) {
		J['el' + s].rotation.x = -(p['el' + s] ?? 0) * D;
		J['kn' + s].rotation.x = (p['kn' + s] ?? 0) * D;
		J['an' + s].rotation.x = -((p['kn' + s] ?? 0) - (p['hip' + s]?.[0] ?? 0)) * 0.25 * D; // the foot stays roughly flat
		for (const fg of J['hand' + s].fingers.slice(0, 4)) fg.rotation.x = -(p.fist ?? 0.15) * 95 * D;
	}
	J.spine.rotation.x = (p.spine ?? 0) * D;
	J.chest.rotation.y = (p.twist ?? 0) * D;
	if (p.head) { J.head.rotation.x = p.head[0] * D; J.head.rotation.y = p.head[1] * D; }
	J.pelvis.position.y = 0.98 + (p.y ?? 0);
	J.pelvis.rotation.set((p.root?.pitch ?? 0) * D, 0, (p.root?.roll ?? 0) * D);
	if (J.robe) {
		// the robe follows the legs: tilts with the mean hip swing, spreads with the stride, shortens when folded
		const hl = p.hipL?.[0] ?? 0, hr = p.hipR?.[0] ?? 0;
		J.robe.rotation.x = -((hl + hr) / 2) * 0.55 * D;
		const stride = Math.abs(hl - hr) / 90;
		const fold = Math.max(0, Math.min(1, ((hl + hr) / 2) / 100));
		J.robe.scale.set(1 + stride * 0.35, 1 - fold * 0.45, 1 + stride * 0.6);
	}
	return { airborne: ['jump', 'fall'].includes(name) };
}

// Tiger: each leg [shoulder / hip swing°, knee°, ankle°] (forward +); spine [hips°, waist°, chest°] (arch +); neck°,
// head°, jaw° (open), tail curl°, roll°, y.
const TIGER = {
	stand: { neck: 5 },
	walk: { FL: [18, 0, 0], FR: [-14, 10, 0], BL: [-14, 0, 0], BR: [16, 0, 0] },
	run: { FL: [55, -10, 20], FR: [30, 20, 10], BL: [-55, -15, 20], BR: [-35, 0, 10], spine: [-8, 6, -8], neck: -12, head: 10, jaw: 25, tail: 4, y: 0.1 },
	jump: { FL: [80, -5, 30], FR: [70, 0, 25], BL: [-70, -20, 30], BR: [-60, -15, 25], spine: [-10, 0, -15], neck: -20, head: 15, jaw: 38, tail: -6, y: 0.7 },
	fall: { FL: [95, 20, 10], FR: [40, 50, 0], BL: [-30, 40, 10], BR: [20, -20, 0], spine: [20, 10, 25], neck: 30, head: -25, jaw: 45, tail: 18, roll: 35, pitch: 40, y: 0.7 },
	crouch: { FL: [-25, 40, -10], FR: [-20, 40, -10], BL: [25, 30, 0], BR: [25, 30, 0], spine: [6, -4, 10], neck: -15, head: 18, jaw: 8, y: -0.38 },
	lie: { FL: [80, -10, 0], FR: [75, -10, 0], BL: [-75, 0, 0], BR: [-70, 0, 0], neck: -10, y: -0.6 },
	'arms-up': { FL: [95, 20, 30], FR: [70, 40, 20], BL: [-10, 15, 0], BR: [-5, 15, 0], spine: [-30, -15, -25], neck: -35, head: 30, jaw: 50, pitch: -32, tail: -8, y: 0.15 },
	roar: { FL: [10, 0, 0], FR: [-5, 0, 0], neck: -30, head: 35, jaw: 55, spine: [0, -4, -6] }
};
function applyTiger(f, name) {
	const alias = { cast: 'run', push: 'run', reach: 'jump', point: 'stand', throw: 'jump', stamp: 'walk', 'look-back': 'stand', kneel: 'crouch', sit: 'crouch', crawl: 'crouch', slip: 'fall', pray: 'crouch', 'hug-self': 'crouch' };
	const p = TIGER[name] ?? TIGER[alias[name]] ?? TIGER.stand;
	const J = f.joints;
	if (J.rigged) {
		// the rigged model: the rest pose has straight legs, so the table's angles apply directly
		for (const leg of ['FL', 'FR', 'BL', 'BR']) {
			const [a = 0, k = 0, an = 0] = p[leg] ?? [];
			J[leg].rotation.set(-a * D, 0, 0);
			J[leg + '2'].rotation.set((leg[0] === 'B' ? 1 : -1) * Math.abs(k) * D * 0.8, 0, 0);
			J[leg + '3'].rotation.set(-an * D, 0, 0);
		}
		const [sh = 0, sw = 0, sc = 0] = p.spine ?? [];
		J.waist.rotation.set(-sw * D, 0, 0);
		J.chest.rotation.set(-sc * D, 0, 0);
		J.hips.rotation.set((p.pitch ?? 0) * D - sh * D, 0, (p.roll ?? 0) * D);
		J.hips.position.copy(J.hips.userData.restPos).add(new THREE.Vector3(0, p.y ?? 0, 0));
		J.neck.rotation.set((p.neck ?? 0) * D, 0, 0);
		J.head.rotation.set((p.head ?? 0) * D, 0, 0);
		J.jaw.rotation.set((p.jaw ?? 4) * D * 0.8, 0, 0);
		J.tail.rotation.set(-0.5 - (p.tail ?? 0) * D, 0, 0);
		J.tail2.rotation.set(0.6, 0, 0);
		return { airborne: ['jump', 'fall'].includes(name) || p === TIGER.jump || p === TIGER.fall };
	}
	for (const leg of ['FL', 'FR', 'BL', 'BR']) {
		const front = leg[0] === 'F';
		const [a = 0, k = 0, an = 0] = p[leg] ?? [];
		J[leg].rotation.x = (front ? 0 : -0.3) - a * D;
		J[leg + '2'].rotation.x = (front ? 0.25 : 0.85) + k * D;
		J[leg + '3'].rotation.x = (front ? -0.3 : -0.6) - an * D;
	}
	const [sh = 0, sw = 0, sc = 0] = p.spine ?? [];
	J.waist.rotation.x = -sw * D;
	J.chest.rotation.x = -sc * D;
	J.hips.rotation.set((p.pitch ?? 0) * D - sh * D, 0, (p.roll ?? 0) * D);
	J.hips.position.y = 0.95 + (p.y ?? 0);
	J.neck.rotation.x = (p.neck ?? 0) * D;
	J.head.rotation.x = (p.head ?? 0) * D;
	J.jaw.rotation.x = (p.jaw ?? 4) * D;
	for (const [i, t] of J.tailSegs.entries()) t.rotation.x = i === 0 ? -2.1 : (0.16 + (p.tail ?? 0) * D * (i % 2 ? 1 : 0.5));
	return { airborne: ['jump', 'fall'].includes(name) || (p === TIGER.jump || p === TIGER.fall) };
}

function applyCrow(f, name) {
	const J = f.joints;
	const fly = ['jump', 'fall', 'arms-up', 'run', 'cast', 'throw', 'push'].includes(name);
	J.wingL.rotation.z = fly ? 0.7 : -1.25;
	J.wingR.rotation.z = fly ? -0.7 : 1.25;
	J.wingL.rotation.y = fly ? 0 : -0.6;
	J.wingR.rotation.y = fly ? 0 : 0.6;
	J.body.rotation.x = fly ? 0.7 : 0;
	J.body.position.y = fly ? 0.9 : 0.5;
	J.head.rotation.x = fly ? -0.5 : 0;
	return { airborne: fly };
}

export function pose(f, name) {
	if (f.kind === 'tiger') return applyTiger(f, name);
	if (f.kind === 'crow') return applyCrow(f, name);
	return applyHuman(f, name);
}

/** a cast entry → a figurine */
export function figurine(c, assets = {}) {
	const id = `${c?.character ?? ''}/${c?.persona ?? ''}`;
	if (id === 'catpope/mountaintiger') return assets.tiger ? assets.gltfTiger(assets.tiger) : tiger();
	if (c?.character === 'crows') return crow();
	if (c?.character === 'catpope' && c?.persona === 'cat') return tiger({ scale: 0.32 }); // the cat: the same quadruped, small
	if (c?.character === 'catpope') return human({ robe: true, cat: true });
	if (c?.character === 'boy') return human({ height: 1.35, build: 0.9 });
	if (/YPLF|junge PLF/i.test(c?.new ?? '')) return human({ robe: true, cat: true, height: 1.68, build: 0.9 });
	return human({});
}
