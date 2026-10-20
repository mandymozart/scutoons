// Posable figurines of the cast (#124, moved from tools/storyboard-render/figures.js – owner 2026-10-09: "3d figurines
// that match the outlines of our characters, a simple bone structure we can pose"; "more detailed figurines"). No
// textures – under extremely hard light the silhouette, the folds and the planes carry everything, so the shapes are
// modelled for that: tapered muscles, fingers, the sphynx ears, robe folds that catch the light, a tiger with real
// anatomy. Units: metres; a figure faces +z, its left is +x. The poses: poses.ts.
import * as THREE from 'three';
import type { Placing } from '../../editor/storyboard-3d-place.ts';

/** a figurine's joints by name (+ the rig's flags and the tiger's tail segments) */
export type Joints = Record<string, any>;
/** a figurine: its kind, the root group, the joints; buildScene adds how it was placed */
export interface Figurine {
	kind: 'human' | 'tiger' | 'crow';
	root: THREE.Group;
	joints: Joints;
	mesh?: THREE.SkinnedMesh;
	place?: Placing;
}
/** the cast entry a figurine is built from */
export interface FigureCast {
	character?: string | null;
	persona?: string;
	new?: string;
}
/** the loaded models a figurine may use instead of the procedural one (scene.ts) */
export interface FigureAssets {
	tiger?: () => Figurine;
	/** the rigged Cat Pope of a persona (null: none loaded for it) */
	catpope?: (persona: string | undefined, opts: { ears: boolean }) => Figurine | null;
}

/** the one material for everything (buildScene sets it: lit white, the Sin City pass makes it black / white) */
export const look: { mat: THREE.Material | null } = { mat: null };

interface At {
	x?: number;
	y?: number;
	z?: number;
	rx?: number;
	ry?: number;
	rz?: number;
	sx?: number;
	sy?: number;
	sz?: number;
}
function add(geo: THREE.BufferGeometry, parent: THREE.Object3D, { x = 0, y = 0, z = 0, rx = 0, ry = 0, rz = 0, sx = 1, sy = 1, sz = 1 }: At = {}): THREE.Mesh {
	const m = new THREE.Mesh(geo, look.mat ?? undefined);
	m.position.set(x, y, z);
	m.rotation.set(rx, ry, rz);
	m.scale.set(sx, sy, sz);
	m.castShadow = m.receiveShadow = true;
	parent.add(m);
	return m;
}
function joint(parent: THREE.Object3D, x = 0, y = 0, z = 0): THREE.Group {
	const j = new THREE.Group();
	j.position.set(x, y, z);
	parent.add(j);
	return j;
}
const ell = (r: number) => new THREE.SphereGeometry(r, 20, 14);
/** a tapered segment hanging down (−y) from its joint: radius r0 at the joint, r1 at the end, a bulge (muscle) at 35 % */
function seg(j: THREE.Object3D, L: number, r0: number, r1: number, bulge = 1.12): THREE.Group {
	const pts: THREE.Vector2[] = [];
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
function folded(profile: [number, number][], folds = 9, depth = 0.08, seed = 0): THREE.LatheGeometry {
	const g = new THREE.LatheGeometry(profile.map(([r, y]) => new THREE.Vector2(r, y)), 64);
	const p = g.attributes.position;
	const v = new THREE.Vector3();
	for (let i = 0; i < p.count; i++) {
		v.fromBufferAttribute(p, i);
		const a = Math.atan2(v.z, v.x);
		const yTop = profile[0][1],
			yBot = profile[profile.length - 1][1];
		const t = (yTop - v.y) / (yTop - yBot || 1); // folds deepen towards the hem
		const k = 1 + depth * t * (Math.sin(a * folds + seed) * 0.7 + Math.sin(a * folds * 2.3 + seed * 2) * 0.3);
		p.setXYZ(i, v.x * k, v.y, v.z * k);
	}
	g.computeVertexNormals();
	return g;
}

// --- humans ------------------------------------------------------------------------------------------------------
function hand(wrist: THREE.Object3D, side: number) {
	const palm = joint(wrist, 0, -0.02, 0);
	add(new THREE.BoxGeometry(0.075, 0.09, 0.03), palm, { y: -0.045 });
	const fingers: THREE.Group[] = [];
	for (let i = 0; i < 4; i++) {
		const f = joint(palm, -0.027 + i * 0.018, -0.09, 0);
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
 * A human figure (PLF / Cat Pope, YPLF, passers-by). robe (the papal robe: cassock with folds, the cape, wide sleeves,
 * the skullcap), cat (the sphynx head), height (m), build (1 = average).
 */
export function human({ robe = false, cat = false, height = 1.8, build = 1 } = {}): Figurine {
	const b = build;
	const root = new THREE.Group();
	const J: Joints = {};
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
	for (const [side, s] of [['L', 1], ['R', -1]] as const) {
		const sh = (J['sh' + side] = joint(J.chest, 0.19 * s * b, 0.2, 0));
		add(ell(0.07), sh, { sx: 1.1, sy: 0.9, sz: 1 }); // deltoid
		const el = (J['el' + side] = seg(sh, 0.29, 0.058, 0.042));
		const wr = (J['wr' + side] = seg(el, 0.26, 0.046, 0.031));
		J['hand' + side] = hand(wr, s);
		const hip = (J['hip' + side] = joint(J.pelvis, 0.095 * s, -0.04, 0));
		const kn = (J['kn' + side] = seg(hip, 0.45, 0.085, 0.052));
		const an = (J['an' + side] = seg(kn, 0.43, 0.058, 0.035));
		add(new THREE.BoxGeometry(0.085, 0.055, 0.25), an, { y: -0.025, z: 0.07 }); // foot
		add(ell(0.045), an, { y: -0.02, z: 0.19, sx: 1, sy: 0.6, sz: 1 }); // toes
	}
	if (robe) {
		// the cassock: from the waist to the ankles, flared, with folds; it follows the legs (poses.ts: J.robe tilts with
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
export function tiger({ scale = 1 } = {}): Figurine {
	const root = new THREE.Group();
	const J: Joints = {};
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
	const leg = (parent: THREE.Object3D, name: string, x: number, y: number, z: number, front: boolean) => {
		const top = (J[name] = joint(parent, x, y, z));
		const l1 = (J[name + '2'] = seg(top, front ? 0.36 : 0.42, front ? 0.13 : 0.16, 0.085, 1.18));
		const l2 = (J[name + '3'] = seg(l1, front ? 0.3 : 0.36, 0.08, 0.06, 1.05));
		const l3 = (J[name + '4'] = seg(l2, front ? 0.14 : 0.2, 0.06, 0.055, 1));
		add(ell(0.09), l3, { y: -0.01, z: 0.05, sx: 1.05, sy: 0.55, sz: 1.35 }); // the paw
		for (let i = 0; i < 4; i++) add(ell(0.03), l3, { x: -0.045 + i * 0.03, y: -0.02, z: 0.15 }); // toes
		// rest pose: the elbow / hock angles of a standing cat
		if (front) {
			l1.rotation.x = 0.25;
			l2.rotation.x = -0.3;
			l3.rotation.x = 0.05;
		} else {
			top.rotation.x = -0.3;
			l1.rotation.x = 0.85;
			l2.rotation.x = -0.6;
		}
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
export function crow({ scale = 1 } = {}): Figurine {
	const root = new THREE.Group();
	const J: Joints = {};
	J.body = joint(root, 0, 0.5, 0);
	add(ell(0.22), J.body, { rx: -0.5, sx: 0.9, sy: 1.3, sz: 0.9 });
	J.head = joint(J.body, 0, 0.3, 0.1);
	add(ell(0.12), J.head, { sx: 0.95, sy: 1, sz: 1.1 });
	add(new THREE.ConeGeometry(0.05, 0.26, 4), J.head, { z: 0.18, rx: Math.PI / 2, sx: 1, sy: 1, sz: 0.7 });
	for (const [side, s] of [['L', 1], ['R', -1]] as const) {
		const w = (J['wing' + side] = joint(J.body, 0.16 * s, 0.14, 0));
		for (let i = 0; i < 6; i++) add(new THREE.BoxGeometry(0.42 - i * 0.03, 0.012, 0.08), w, { x: (0.22 + i * 0.05) * s, z: -0.12 + i * 0.05, ry: (0.3 - i * 0.12) * s, rz: -0.12 * s });
		const leg = joint(J.body, 0.07 * s, -0.24, 0);
		seg(leg, 0.28, 0.022, 0.015, 1);
	}
	for (let i = 0; i < 5; i++) add(new THREE.BoxGeometry(0.05, 0.012, 0.36), J.body, { x: -0.08 + i * 0.04, y: -0.14, z: -0.3, rx: 0.55, ry: (i - 2) * 0.1 });
	root.scale.setScalar(scale);
	return { kind: 'crow', root, joints: J };
}

/** a cast entry → a figurine (a loaded model where one is given, else the procedural one) */
export function figurine(c: FigureCast | undefined, assets: FigureAssets = {}): Figurine {
	const id = `${c?.character ?? ''}/${c?.persona ?? ''}`;
	if (id === 'catpope/mountaintiger') return assets.tiger ? assets.tiger() : tiger();
	if (c?.character === 'crows') return crow();
	if (c?.character === 'catpope' && c?.persona === 'cat') return tiger({ scale: 0.32 }); // the cat: the same quadruped, small
	// PLF in the chapters is the human priest (no cat features) or the sphynx cat; Cat Pope himself (regular, with the
	// ears) only where the story shows him – the mirror at the end of chapter 5 (owner 2026-10-10)
	const rigged = c?.character === 'catpope' && ['regular', 'human', undefined].includes(c?.persona) ? assets.catpope?.(c?.persona, { ears: c?.persona === 'regular' }) : null;
	if (rigged) return rigged;
	if (c?.character === 'catpope') return human({ robe: true, cat: c?.persona === 'regular' });
	if (c?.character === 'boy') return human({ height: 1.35, build: 0.9 });
	if (/YPLF|junge PLF/i.test(c?.new ?? '')) return human({ robe: true, height: 1.68, build: 0.9 }); // the young PLF: human, no cat
	return human({});
}
