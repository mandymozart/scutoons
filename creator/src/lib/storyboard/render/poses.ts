// The figurines' poses (#124, moved from tools/storyboard-render/figures.js): a pose name of the storyboard's vocabulary
// → joint angles for the procedural human, the rigged Cat Pope (human-gltf.ts), the tiger (procedural and rigged –
// tiger-gltf.ts) and the crow. pose() says whether the pose is airborne (buildScene never grounds such a figure).
import * as THREE from 'three';
import type { Figurine, Joints } from './figures.ts';

const D = Math.PI / 180;

interface HumanPose {
	shL?: [number, number];
	shR?: [number, number];
	hipL?: [number, number];
	hipR?: [number, number];
	elL?: number;
	elR?: number;
	knL?: number;
	knR?: number;
	spine?: number;
	twist?: number;
	head?: [number, number];
	fist?: number;
	root?: { pitch?: number; roll?: number };
	y?: number;
}
// Human: sh / hip = [forward°, out°]; el / kn bend°; spine lean°; twist°; head [nod°, turn°]; fist = curl the fingers;
// root { pitch, roll } – the whole body (falls, crawls); y = pelvis offset (m).
const HUMAN: Record<string, HumanPose> = {
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

/** the pose's result: an airborne figure is never put on the ground */
export interface Posed {
	airborne: boolean;
}

function applyHuman(f: Pick<Figurine, 'joints'>, name: string): Posed {
	if (f.joints.humanRig) return applyHumanRig(f, name);
	const p = HUMAN[name] ?? HUMAN.stand;
	const J = f.joints;
	for (const k of ['shL', 'shR', 'elL', 'elR', 'wrL', 'wrR', 'hipL', 'hipR', 'knL', 'knR', 'anL', 'anR', 'spine', 'chest', 'head']) J[k].rotation.set(0, 0, 0);
	const side = (k: 'shL' | 'shR' | 'hipL' | 'hipR', s: number) => {
		const v = p[k];
		if (!v) return;
		J[k].rotation.x = -v[0] * D;
		J[k].rotation.z = v[1] * D * s;
	};
	side('shL', 1);
	side('shR', -1);
	side('hipL', 1);
	side('hipR', -1);
	for (const s of ['L', 'R'] as const) {
		J['el' + s].rotation.x = -(p[`el${s}`] ?? 0) * D;
		J['kn' + s].rotation.x = (p[`kn${s}`] ?? 0) * D;
		J['an' + s].rotation.x = -((p[`kn${s}`] ?? 0) - (p[`hip${s}`]?.[0] ?? 0)) * 0.25 * D; // the foot stays roughly flat
		for (const fg of (J['hand' + s]?.fingers ?? []).slice(0, 4)) fg.rotation.x = -(p.fist ?? 0.15) * 95 * D;
	}
	J.spine.rotation.x = (p.spine ?? 0) * D;
	J.chest.rotation.y = (p.twist ?? 0) * D;
	if (p.head) {
		J.head.rotation.x = p.head[0] * D;
		J.head.rotation.y = p.head[1] * D;
	}
	J.pelvis.position.y = 0.98 + (p.y ?? 0);
	J.pelvis.rotation.set((p.root?.pitch ?? 0) * D, 0, (p.root?.roll ?? 0) * D);
	if (J.robe) {
		// the robe follows the legs: tilts with the mean hip swing, spreads with the stride, shortens when folded
		const hl = p.hipL?.[0] ?? 0,
			hr = p.hipR?.[0] ?? 0;
		J.robe.rotation.x = -((hl + hr) / 2) * 0.55 * D;
		const stride = Math.abs(hl - hr) / 90;
		const fold = Math.max(0, Math.min(1, (hl + hr) / 2 / 100));
		J.robe.scale.set(1 + stride * 0.35, 1 - fold * 0.45, 1 + stride * 0.6);
	}
	return { airborne: ['jump', 'fall'].includes(name) };
}

/**
 * A rigged human (the owner's catpope.glb, human-gltf.ts): the pose is computed on stand-in joints with the procedural
 * figurine's conventions, then applied ON TOP of each bone's rest rotation; the arms' A-pose rest (≈ 22° out) is taken
 * off the table's "out" angle, so 'stand' hangs the arms like the figurine's.
 */
const PROXY_KEYS = ['pelvis', 'spine', 'chest', 'neck', 'head', 'robe', 'shL', 'elL', 'wrL', 'shR', 'elR', 'wrR', 'hipL', 'knL', 'anL', 'hipR', 'knR', 'anR'];
function applyHumanRig(f: Pick<Figurine, 'joints'>, name: string): Posed {
	const J = f.joints;
	const proxy: Joints = {};
	for (const k of PROXY_KEYS) proxy[k] = new THREE.Group();
	proxy.pelvis.position.y = 0.98;
	const r = applyHuman({ joints: proxy }, name);
	for (const k of PROXY_KEYS) {
		const bone = J[k] as THREE.Bone | undefined;
		if (!bone) continue;
		const rest = bone.userData.rest as { position: THREE.Vector3; quaternion: THREE.Quaternion };
		const e = (proxy[k] as THREE.Group).rotation.clone();
		if (k === 'shL' || k === 'shR') {
			// the bind pose holds the arm out already: take that off the table's out-angle
			const restOut = new THREE.Euler().setFromQuaternion(rest.quaternion, 'XYZ').z;
			e.z -= restOut;
		}
		bone.quaternion.copy(rest.quaternion).multiply(new THREE.Quaternion().setFromEuler(e));
		bone.position.copy(rest.position);
		if (k === 'pelvis') bone.position.y += proxy.pelvis.position.y - 0.98;
		if (k === 'robe') bone.scale.copy(proxy.robe.scale);
	}
	return r;
}

interface TigerPose {
	FL?: number[];
	FR?: number[];
	BL?: number[];
	BR?: number[];
	spine?: number[];
	neck?: number;
	head?: number;
	jaw?: number;
	tail?: number;
	roll?: number;
	pitch?: number;
	y?: number;
}
// Tiger: each leg [shoulder / hip swing°, knee°, ankle°] (forward +); spine [hips°, waist°, chest°] (arch +); neck°,
// head°, jaw° (open), tail curl°, roll°, y.
const TIGER: Record<string, TigerPose> = {
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
const TIGER_ALIAS: Record<string, string> = { cast: 'run', push: 'run', reach: 'jump', point: 'stand', throw: 'jump', stamp: 'walk', 'look-back': 'stand', kneel: 'crouch', sit: 'crouch', crawl: 'crouch', slip: 'fall', pray: 'crouch', 'hug-self': 'crouch' };
const LEGS = ['FL', 'FR', 'BL', 'BR'] as const;

function applyTiger(f: Figurine, name: string): Posed {
	const p = TIGER[name] ?? TIGER[TIGER_ALIAS[name]] ?? TIGER.stand;
	const airborne = ['jump', 'fall'].includes(name) || p === TIGER.jump || p === TIGER.fall;
	const J = f.joints;
	const [sh = 0, sw = 0, sc = 0] = p.spine ?? [];
	if (J.rigged) {
		// the rigged model: the rest pose has straight legs, so the table's angles apply directly
		for (const leg of LEGS) {
			const [a = 0, k = 0, an = 0] = p[leg] ?? [];
			J[leg].rotation.set(-a * D, 0, 0);
			J[leg + '2'].rotation.set((leg[0] === 'B' ? 1 : -1) * Math.abs(k) * D * 0.8, 0, 0);
			J[leg + '3'].rotation.set(-an * D, 0, 0);
		}
		J.waist.rotation.set(-sw * D, 0, 0);
		J.chest.rotation.set(-sc * D, 0, 0);
		J.hips.rotation.set((p.pitch ?? 0) * D - sh * D, 0, (p.roll ?? 0) * D);
		J.hips.position.copy(J.hips.userData.restPos).add(new THREE.Vector3(0, p.y ?? 0, 0));
		J.neck.rotation.set((p.neck ?? 0) * D, 0, 0);
		J.head.rotation.set((p.head ?? 0) * D, 0, 0);
		J.jaw.rotation.set((p.jaw ?? 4) * D * 0.8, 0, 0);
		J.tail.rotation.set(-0.5 - (p.tail ?? 0) * D, 0, 0);
		J.tail2.rotation.set(0.6, 0, 0);
		return { airborne };
	}
	for (const leg of LEGS) {
		const front = leg[0] === 'F';
		const [a = 0, k = 0, an = 0] = p[leg] ?? [];
		J[leg].rotation.x = (front ? 0 : -0.3) - a * D;
		J[leg + '2'].rotation.x = (front ? 0.25 : 0.85) + k * D;
		J[leg + '3'].rotation.x = (front ? -0.3 : -0.6) - an * D;
	}
	J.waist.rotation.x = -sw * D;
	J.chest.rotation.x = -sc * D;
	J.hips.rotation.set((p.pitch ?? 0) * D - sh * D, 0, (p.roll ?? 0) * D);
	J.hips.position.y = 0.95 + (p.y ?? 0);
	J.neck.rotation.x = (p.neck ?? 0) * D;
	J.head.rotation.x = (p.head ?? 0) * D;
	J.jaw.rotation.x = (p.jaw ?? 4) * D;
	(J.tailSegs as THREE.Group[]).forEach((t, i) => (t.rotation.x = i === 0 ? -2.1 : 0.16 + (p.tail ?? 0) * D * (i % 2 ? 1 : 0.5)));
	return { airborne };
}

function applyCrow(f: Figurine, name: string): Posed {
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

/** poses a figurine by the storyboard's pose name */
export function pose(f: Figurine, name: string): Posed {
	if (f.kind === 'tiger') return applyTiger(f, name);
	if (f.kind === 'crow') return applyCrow(f, name);
	return applyHuman(f, name);
}
