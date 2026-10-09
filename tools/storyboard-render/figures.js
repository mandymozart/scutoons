// Posable figurines of the cast (owner 2026-10-09: "3d figurines that match the outlines of our characters, a simple
// bone structure we can pose"). No textures – the light is extremely hard, the silhouette carries everything. Units:
// metres; a figure faces +z, its left is +x. Refine as we go: a character = a builder + its proportions.
import * as THREE from 'three';

const D = Math.PI / 180;

/** one 2-tone material for everything (black below the threshold, white above) – set by render.js */
export const look = { mat: null, accent: null };

function part(geo, parent, { x = 0, y = 0, z = 0, rx = 0, ry = 0, rz = 0 } = {}) {
	const m = new THREE.Mesh(geo, look.mat);
	m.position.set(x, y, z);
	m.rotation.set(rx, ry, rz);
	m.castShadow = true;
	m.receiveShadow = true;
	parent.add(m);
	return m;
}
function joint(parent, x, y, z) {
	const j = new THREE.Group();
	j.position.set(x, y, z);
	parent.add(j);
	return j;
}
/** a limb segment hanging down from its joint (−y), length L, radius r */
function limb(j, L, r) {
	part(new THREE.CapsuleGeometry(r, Math.max(0.001, L - 2 * r), 4, 10), j, { y: -L / 2 });
	return joint(j, 0, -L, 0);
}

/**
 * A human figure (PLF / Cat Pope, YPLF, passers-by). opts: robe (the long papal robe + cape), cat (sphynx head with
 * ears), height (m). Returns { root, joints, height }.
 */
export function human({ robe = false, cat = false, height = 1.8, slim = 1 } = {}) {
	const s = height / 1.8;
	const root = new THREE.Group();
	const J = {};
	J.pelvis = joint(root, 0, 0.95, 0);
	part(new THREE.SphereGeometry(0.16 * slim, 14, 10), J.pelvis, { y: 0.02 }).scale.set(1.15, 0.7, 0.8);
	J.spine = joint(J.pelvis, 0, 0.05, 0);
	part(new THREE.CapsuleGeometry(0.15 * slim, 0.3, 4, 12), J.spine, { y: 0.22 }).scale.set(1.15, 1, 0.75);
	J.chest = joint(J.spine, 0, 0.42, 0);
	part(new THREE.CapsuleGeometry(0.17 * slim, 0.12, 4, 12), J.chest, { y: 0.02 }).scale.set(1.25, 1, 0.75);
	J.neck = joint(J.chest, 0, 0.14, 0);
	part(new THREE.CylinderGeometry(0.05, 0.06, 0.1, 10), J.neck, { y: 0.05 });
	J.head = joint(J.neck, 0, 0.1, 0);
	if (cat) {
		const skull = part(new THREE.SphereGeometry(0.115, 18, 14), J.head, { y: 0.11 });
		skull.scale.set(1.05, 1, 1.05);
		part(new THREE.SphereGeometry(0.07, 14, 10), J.head, { y: 0.06, z: 0.08 }).scale.set(1.1, 0.8, 0.9); // muzzle
		for (const sx of [-1, 1]) part(new THREE.ConeGeometry(0.06, 0.17, 4), J.head, { x: 0.08 * sx, y: 0.24, z: -0.01, rz: -0.38 * sx }); // the big sphynx ears
	} else {
		part(new THREE.SphereGeometry(0.11, 18, 14), J.head, { y: 0.11 }).scale.set(0.9, 1.1, 1);
	}
	for (const [side, sx] of [['L', 1], ['R', -1]]) {
		const sh = (J['sh' + side] = joint(J.chest, 0.21 * sx * slim, 0.06, 0));
		part(new THREE.SphereGeometry(0.065, 10, 8), sh);
		const el = (J['el' + side] = limb(sh, 0.29, 0.05));
		const wr = (J['wr' + side] = limb(el, 0.26, 0.042));
		part(new THREE.BoxGeometry(0.07, 0.17, 0.035), wr, { y: -0.08 }); // the hand (the hands matter: they cast, pray, stamp)
		const hip = (J['hip' + side] = joint(J.pelvis, 0.1 * sx, -0.04, 0));
		const kn = (J['kn' + side] = limb(hip, 0.44, 0.07));
		const an = (J['an' + side] = limb(kn, 0.43, 0.055));
		part(new THREE.BoxGeometry(0.09, 0.06, 0.24), an, { y: -0.02, z: 0.06 });
	}
	if (robe) {
		// the robe hangs from the chest to the ankles (follows the pelvis, not the legs – a cone reads as the robe)
		const r = part(new THREE.CylinderGeometry(0.2, 0.38, 1.12, 20, 1, true), J.pelvis, { y: -0.36 });
		r.material = look.mat;
		part(new THREE.CylinderGeometry(0.17, 0.21, 0.5, 18), J.spine, { y: 0.2 });
		// the cape (mozzetta) over the shoulders
		part(new THREE.CylinderGeometry(0.16, 0.36, 0.26, 20, 1, true), J.chest, { y: -0.02 });
		// wide sleeves
		for (const side of ['L', 'R']) part(new THREE.CylinderGeometry(0.055, 0.1, 0.26, 10, 1, true), J['el' + side], { y: -0.13 });
	}
	root.scale.setScalar(s);
	return { kind: 'human', root, joints: J, height };
}

/** the white mountain tiger: a quadruped. Returns { root, joints, height } (height = shoulder ≈ 1.1 m) */
export function tiger({ height = 1.1 } = {}) {
	const root = new THREE.Group();
	const J = {};
	J.body = joint(root, 0, 0.95, 0);
	part(new THREE.CapsuleGeometry(0.34, 1.2, 6, 16), J.body, { rx: Math.PI / 2 }).scale.set(1, 1, 0.95);
	J.neck = joint(J.body, 0, 0.12, 0.8);
	part(new THREE.CapsuleGeometry(0.2, 0.25, 4, 12), J.neck, { y: 0.08, z: 0.08, rx: 0.9 });
	J.head = joint(J.neck, 0, 0.22, 0.25);
	part(new THREE.SphereGeometry(0.27, 18, 14), J.head).scale.set(1, 0.9, 1.05);
	part(new THREE.BoxGeometry(0.26, 0.14, 0.26), J.head, { y: -0.1, z: 0.24 }); // muzzle
	J.jaw = joint(J.head, 0, -0.14, 0.12);
	part(new THREE.BoxGeometry(0.22, 0.06, 0.28), J.jaw, { z: 0.12 });
	for (const sx of [-1, 1]) {
		part(new THREE.ConeGeometry(0.08, 0.12, 6), J.head, { x: 0.17 * sx, y: 0.24, z: -0.04 });
		part(new THREE.ConeGeometry(0.025, 0.12, 6), J.jaw, { x: 0.07 * sx, y: 0.05, z: 0.22, rx: Math.PI }); // the huge teeth
	}
	for (const [side, sx] of [['F', 1], ['B', -1]]) for (const [lr, lx] of [['L', 1], ['R', -1]]) {
		const top = (J['leg' + side + lr] = joint(J.body, 0.2 * lx, -0.12, 0.6 * sx));
		const mid = (J['knee' + side + lr] = limb(top, 0.45, 0.11));
		const paw = limb(mid, 0.42, 0.08);
		part(new THREE.SphereGeometry(0.1, 10, 8), paw).scale.set(1, 0.6, 1.3);
	}
	J.tail = joint(J.body, 0, 0.1, -0.75);
	part(new THREE.CapsuleGeometry(0.05, 0.9, 4, 8), J.tail, { y: -0.2, z: -0.3, rx: -1.1 });
	root.scale.setScalar(height / 1.1);
	return { kind: 'tiger', root, joints: J, height };
}

/** a crow creature (the KRÄHENWESEN): a big bird, wings as planes. height ≈ 0.9 m standing */
export function crow({ height = 0.9 } = {}) {
	const root = new THREE.Group();
	const J = {};
	J.body = joint(root, 0, 0.5, 0);
	part(new THREE.SphereGeometry(0.22, 16, 12), J.body, { rx: -0.5 }).scale.set(0.9, 1.25, 0.9);
	J.head = joint(J.body, 0, 0.3, 0.1);
	part(new THREE.SphereGeometry(0.12, 14, 10), J.head);
	part(new THREE.ConeGeometry(0.045, 0.22, 8), J.head, { z: 0.16, rx: Math.PI / 2 });
	for (const [side, sx] of [['L', 1], ['R', -1]]) {
		J['wing' + side] = joint(J.body, 0.17 * sx, 0.12, 0);
		part(new THREE.BoxGeometry(0.62, 0.02, 0.3), J['wing' + side], { x: 0.3 * sx, rz: -0.2 * sx });
		const leg = joint(J.body, 0.07 * sx, -0.22, 0);
		limb(leg, 0.28, 0.02);
	}
	part(new THREE.BoxGeometry(0.18, 0.02, 0.32), J.body, { y: -0.12, z: -0.26, rx: 0.6 }); // tail
	root.scale.setScalar(height / 0.9);
	return { kind: 'crow', root, joints: J, height };
}

// --- poses -----------------------------------------------------------------------------------------------------
// Human: degrees. sh / hip = [forward, sideways out]; el / kn = bend; spine = forward lean; root = { pitch, roll, y }.
const HUMAN = {
	stand: { shL: [0, 8], shR: [0, 8] },
	walk: { shL: [-22, 6], shR: [24, 6], elR: 15, hipL: [24, 0], hipR: [-18, 0], knR: 20 },
	run: { spine: 18, shL: [-45, 10], shR: [55, 10], elL: 85, elR: 80, hipL: [55, 0], knL: 65, hipR: [-35, 0], knR: 85, y: 0.06 },
	jump: { shL: [150, 25], shR: [150, 25], hipL: [45, 0], knL: 80, hipR: [20, 0], knR: 60, y: 0.5 },
	fall: { root: { pitch: -55, roll: 25 }, shL: [130, 60], shR: [100, 70], elL: 30, hipL: [40, 20], hipR: [-10, 25], knR: 40, y: 0.8 },
	slip: { root: { pitch: -38 }, shL: [40, 80], shR: [60, 75], hipL: [70, 0], hipR: [5, 0], knR: 25, y: 0.1 },
	crouch: { spine: 35, shL: [45, 10], shR: [45, 10], elL: 60, elR: 60, hipL: [105, 6], hipR: [105, 6], knL: 125, knR: 125, y: -0.47 },
	kneel: { spine: 5, hipL: [95, 0], knL: 95, hipR: [0, 0], knR: 95, shL: [10, 8], shR: [10, 8], y: -0.42 },
	crawl: { root: { pitch: 78 }, shL: [80, 8], shR: [100, 8], elL: 10, hipL: [70, 0], knL: 95, hipR: [95, 0], knR: 90, y: -0.5 },
	sit: { hipL: [90, 8], hipR: [90, 8], knL: 90, knR: 90, shL: [20, 8], shR: [20, 8], elL: 40, elR: 40, y: -0.45 },
	lie: { root: { pitch: -90 }, shL: [0, 15], shR: [0, 15], y: -0.8 },
	reach: { spine: 10, shR: [90, 0], shL: [10, 8], hipL: [15, 0] },
	point: { shR: [88, -18], shL: [0, 8], head: [0, -10] },
	throw: { spine: -10, shR: [170, 20], elR: 70, shL: [60, 10], hipL: [35, 0], knL: 20, hipR: [-20, 0] },
	cast: { spine: 12, shL: [85, -12], shR: [85, -12], elL: 15, elR: 15, hipL: [35, 0], knL: 30, hipR: [-25, 0], knR: 10 },
	'arms-up': { shL: [20, 150], shR: [20, 150], elL: 25, elR: 25 },
	'hug-self': { spine: 12, shL: [55, -35], shR: [55, -35], elL: 125, elR: 125, head: [12, 0] },
	stamp: { shL: [55, -35], shR: [55, -35], elL: 125, elR: 125, hipL: [75, 0], knL: 95 },
	push: { spine: 28, shL: [80, 0], shR: [80, 0], elL: 10, elR: 10, hipL: [25, 0], knL: 25, hipR: [-35, 0] },
	'look-back': { spine: 5, twist: 45, head: [0, 70], shL: [-10, 8], shR: [15, 8] },
	pray: { shL: [45, -25], shR: [45, -25], elL: 110, elR: 110, head: [25, 0] }
};

function applyHuman(f, name) {
	const p = HUMAN[name] ?? HUMAN.stand;
	const J = f.joints;
	for (const k of Object.keys(J)) J[k].rotation.set(0, 0, 0);
	const side = (k, sx) => {
		const v = p[k];
		if (!v) return;
		J[k].rotation.x = -v[0] * D; // forward = −x (the figure faces +z)
		J[k].rotation.z = v[1] * D * sx;
	};
	side('shL', 1);
	side('shR', -1);
	side('hipL', 1);
	side('hipR', -1);
	for (const s of ['L', 'R']) {
		if (p['el' + s]) J['el' + s].rotation.x = -p['el' + s] * D; // the forearm bends forward
		if (p['kn' + s]) J['kn' + s].rotation.x = p['kn' + s] * D; // the shin bends back
	}
	if (p.spine) J.spine.rotation.x = p.spine * D;
	if (p.twist) J.chest.rotation.y = p.twist * D;
	if (p.head) { J.head.rotation.x = p.head[0] * D; J.head.rotation.y = p.head[1] * D; }
	J.pelvis.position.y = 0.95 + (p.y ?? 0);
	J.pelvis.rotation.set((p.root?.pitch ?? 0) * D, 0, (p.root?.roll ?? 0) * D);
	return { airborne: ['jump', 'fall'].includes(name) };
}

const TIGER = {
	stand: {},
	walk: { FL: 20, BR: 20, FR: -15, BL: -15 },
	run: { FL: 50, FR: 30, BL: -50, BR: -35, body: -5, jaw: 20 },
	jump: { FL: 70, FR: 60, BL: -60, BR: -55, body: -20, jaw: 30, y: 0.6 },
	fall: { FL: 90, FR: 70, BL: -20, BR: 30, body: 50, roll: 30, jaw: 35, y: 0.6 },
	crouch: { FL: -30, FR: -30, BL: 40, BR: 40, kneeB: -80, y: -0.35, jaw: 10 },
	lie: { FL: 80, FR: 80, BL: -80, BR: -80, y: -0.55 },
	'arms-up': { FL: 80, FR: 60, body: -35, jaw: 40, y: 0.25 } // rearing up, roaring
};
function applyTiger(f, name) {
	const p = TIGER[name] ?? (['cast', 'push', 'reach', 'point', 'throw'].includes(name) ? TIGER.run : TIGER.stand);
	const J = f.joints;
	for (const k of Object.keys(J)) J[k].rotation.set(0, 0, 0);
	for (const leg of ['FL', 'FR', 'BL', 'BR']) J['leg' + leg].rotation.x = -(p[leg] ?? 0) * D;
	if (p.kneeB) { J.kneeBL.rotation.x = p.kneeB * D; J.kneeBR.rotation.x = p.kneeB * D; }
	J.body.rotation.x = -(p.body ?? 0) * D;
	J.body.rotation.z = (p.roll ?? 0) * D;
	J.body.position.y = 0.95 + (p.y ?? 0);
	J.jaw.rotation.x = (p.jaw ?? 6) * D;
	J.tail.rotation.x = 0.4;
	return { airborne: ['jump', 'fall'].includes(name) };
}

function applyCrow(f, name) {
	const J = f.joints;
	const fly = ['jump', 'fall', 'arms-up', 'run', 'cast', 'throw', 'push'].includes(name);
	J.wingL.rotation.z = fly ? 0.9 : -1.2;
	J.wingR.rotation.z = fly ? -0.9 : 1.2;
	J.body.rotation.x = fly ? 0.6 : 0;
	J.body.position.y = fly ? 0.9 : 0.5;
	return { airborne: fly };
}

export function pose(f, name) {
	if (f.kind === 'tiger') return applyTiger(f, name);
	if (f.kind === 'crow') return applyCrow(f, name);
	return applyHuman(f, name);
}

/** a cast entry → a figurine */
export function figurine(c) {
	const id = `${c?.character ?? ''}/${c?.persona ?? ''}`;
	if (id === 'catpope/mountaintiger') return tiger();
	if (c?.character === 'crows') return crow();
	if (c?.character === 'catpope' && c?.persona === 'cat') return tiger({ height: 0.45 }); // the cat: a small quadruped
	if (c?.character === 'catpope') return human({ robe: true, cat: true });
	if (c?.character === 'boy') return human({ height: 1.35 });
	if (/YPLF|junge PLF/i.test(c?.new ?? '')) return human({ robe: true, cat: false, height: 1.65, slim: 0.92 });
	return human({});
}
