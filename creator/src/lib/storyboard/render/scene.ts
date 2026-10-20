// One storyboard shot → its 3D scene (#124, moved from tools/storyboard-render/core.js – owner 2026-10-09: images, not
// SVG; Sin City black and white; extreme hard light, never diffuse; posable figurines). shotCamera (the shot's preset
// camera or its hand camera), buildScene (the set + the figurines at the storyboard's screen places, seen through that
// camera + the one hard light). The look: sin-pass.ts; the frame with texts and marks: render-shot.ts.
//
// Hand overrides (the 3D view writes them):
//   shot.camera = { pos: [x, y, z], target: [x, y, z], fov, roll }   – a free camera (the 3D view)
//              or { dist, height, pitch, yaw, roll, fov }             – adjustments of the preset camera
//   light.azimuth / light.elevation (°): 0 = from the camera's side, 90 = from the right, −90 left, 180 behind
//   figure.turn (°) beside facing
//   figure.ground = false – a figure that floats (placed along its ray, not on the ground) stays floating under another
//              camera (the 3D view writes it when a hand camera would otherwise put it on the ground)
import * as THREE from 'three';
import type { GLTF } from 'three/addons/loaders/GLTFLoader.js';
import type { Light, ShotSpec } from '@scutoons/content/storyboard';
import { D, placeFigure, turnOf, yawToward, type ViewCamera } from '../../editor/storyboard-3d-place.ts';
import { figurine, look, type FigureAssets, type Figurine } from './figures.ts';
import { pose } from './poses.ts';
import { buildSet, type BuiltSet } from './sets.ts';
import { gltfHuman, humanRigOf } from './human-gltf.ts';
import { gltfTiger, tigerMeshOf } from './tiger-gltf.ts';
import { firstModel, type ModelUrls } from './models.ts';

/** the shot's camera (its size / angle preset, or the hand camera) */
export function shotCamera(shot: ShotSpec): { camera: ViewCamera; camY: number; dist: number } {
	const W = shot.w,
		H = shot.h;
	const aspect = W / H;
	const tall = shot.format === 'loop360' || shot.projection === 'polar' ? 16 / 9 : Math.max(16 / 9, H / W);
	const ANG: Record<string, [number, number]> = { eye: [1.55, 0], high: [5, -24], low: [0.45, 12], bird: [22, -78], worm: [0.12, 30], dutch: [1.55, 0] };
	let [camY, pitch] = ANG[shot.angle ?? ''] ?? ANG.eye;
	if (shot.projection === 'top-down') {
		camY = 24;
		pitch = -89.9;
	}
	const move = shot.move ?? '';
	if (shot.format === 'vertical' && ['tilt-down', 'fall', 'descend', 'reveal'].includes(move) && shot.angle !== 'bird') pitch -= 8;
	if (shot.format === 'vertical' && ['tilt-up', 'climb'].includes(move)) pitch += 10;
	const DIST: Record<string, number> = { ecu: 0.9, cu: 1.6, mcu: 2.4, medium: 3.5, full: 6, wide: 11, ews: 24 };
	const c = shot.camera ?? {};
	const dist = c.dist ?? DIST[shot.size ?? ''] ?? 7;
	camY = c.height ?? camY;
	pitch = c.pitch ?? pitch;
	const vfov = c.fov ?? Math.min(150, (2 * Math.atan(Math.tan(25 * D) * (tall / (16 / 9)))) / D);
	let camera: ViewCamera;
	if (shot.projection === 'isometric') {
		const hh = dist * 0.7;
		camera = new THREE.OrthographicCamera(-hh * aspect, hh * aspect, hh, -hh, 0.1, 400);
		camera.position.set(dist * 1.2, dist * 1.2, dist * 1.2);
		camera.lookAt(0, 0.8, 0);
	} else {
		camera = new THREE.PerspectiveCamera(vfov, aspect, 0.05, 600);
		if (c.pos && c.target) {
			camera.position.fromArray(c.pos);
			camera.lookAt(new THREE.Vector3().fromArray(c.target));
			if (c.roll) camera.rotateZ(c.roll * D);
		} else {
			const yaw = (c.yaw ?? 0) * D;
			camera.position.set(Math.sin(yaw) * dist, camY, Math.cos(yaw) * dist);
			camera.rotation.order = 'YXZ';
			camera.rotation.set(pitch * D, yaw, (c.roll ?? (shot.angle === 'dutch' ? 18 : 0)) * D);
		}
	}
	camera.updateMatrixWorld();
	return { camera, camY: camera.position.y, dist };
}

/** the key light (+ rim) and the point they aim at */
export interface SceneLight {
	key: THREE.DirectionalLight | THREE.PointLight | null;
	rim: THREE.DirectionalLight | null;
	target: THREE.Vector3;
}
/** a built scene */
export interface BuiltScene {
	scene: THREE.Scene;
	camera: ViewCamera;
	figs: Figurine[];
	camY: number;
	dist: number;
	skyWhite: boolean;
	K: number;
	light: SceneLight;
	/** frees every geometry, material, texture and skeleton the build made (models stay cached as files) */
	dispose(): void;
}
export interface BuildOptions {
	/** where the figurines' models are (none: the procedural figurines) */
	models?: ModelUrls;
	/** 0 … 1, the fill's softness when the shot doesn't set light.soft */
	soft?: number;
	shadowSize?: number;
}

const CATPOPE_PERSONAS = ['regular', 'human', undefined];

/** loads the models the shot's figures use: the Cat Pope per persona, the tiger */
async function assetsFor(shot: ShotSpec, models: ModelUrls | undefined): Promise<FigureAssets> {
	if (!models) return {};
	const assets: FigureAssets = {};
	const personas = [...new Set(shot.figures.filter((f) => f.cast?.character === 'catpope' && CATPOPE_PERSONAS.includes(f.cast?.persona)).map((f) => f.cast?.persona))];
	const human = new Map<string | undefined, GLTF>();
	for (const p of personas) {
		const gltf = await firstModel(await models('catpope', p), (g) => !!humanRigOf(g));
		if (gltf) human.set(p, gltf);
	}
	if (human.size) assets.catpope = (p, opts) => (human.has(p) ? gltfHuman(human.get(p)!, opts) : null);
	if (shot.figures.some((f) => f.cast?.persona === 'mountaintiger')) {
		const gltf = await firstModel(await models('tiger', 'mountaintiger'), (g) => !!tigerMeshOf(g));
		if (gltf) assets.tiger = () => gltfTiger(gltf);
	}
	return assets;
}

/** the box the framing shows of a posed figure (core.js: whole, the head for cu / ecu, the upper body for mcu) */
function framedBox(model: Figurine, framing: string, restH: number): { box: THREE.Box3; whole: THREE.Box3 } {
	const whole = new THREE.Box3().setFromObject(model.root, true); // precise: a skinned mesh measured as posed
	let head = model.joints.head && !model.joints.rigged ? new THREE.Box3().setFromObject(model.joints.head) : whole;
	if (model.joints.humanRig) {
		// a rigged human: the head = around its head bone, by the body's height
		const hp = model.joints.head.getWorldPosition(new THREE.Vector3());
		const yl = restH;
		head = new THREE.Box3(new THREE.Vector3(hp.x - yl * 0.08, hp.y - yl * 0.03, hp.z - yl * 0.08), new THREE.Vector3(hp.x + yl * 0.08, Math.max(hp.y + yl * 0.14, whole.max.y), hp.z + yl * 0.08));
	} else if (model.joints.rigged) {
		const zl = whole.max.z - whole.min.z,
			yl = whole.max.y - whole.min.y;
		head = new THREE.Box3(new THREE.Vector3(whole.min.x, whole.min.y + yl * 0.45, whole.max.z - zl * 0.24), whole.max.clone());
	}
	let box = whole;
	if (['cu', 'ecu'].includes(framing)) box = head.clone();
	if (framing === 'ecu') {
		const c = head.getCenter(new THREE.Vector3());
		box = new THREE.Box3(c.clone().add(new THREE.Vector3(0, -0.02, 0)), c.clone().add(new THREE.Vector3(0, 0.09, 0)));
	}
	if (framing === 'mcu') box = new THREE.Box3(new THREE.Vector3(whole.min.x, head.max.y - (head.max.y - whole.min.y) * 0.42, whole.min.z), whole.max.clone());
	return { box, whole };
}

/**
 * The scene of a shot: the set, the figurines at the storyboard's screen positions (seen through the shot's camera),
 * the one hard key light (+ rim).
 */
export async function buildScene(shot: ShotSpec, opts: BuildOptions = {}): Promise<BuiltScene> {
	const W = shot.w,
		H = shot.h;
	const K = W / (shot.outW ?? W);
	look.mat = new THREE.MeshLambertMaterial({ color: 0xffffff });
	const emissive = new THREE.MeshBasicMaterial({ color: 0xffffff });
	const scene = new THREE.Scene();
	const L: Light = shot.light ?? {};
	const interior = shot.location?.time === 'interior' || /library|monastery|studio|server/.test(shot.location?.place ?? '') || L.source === 'none';
	const skyWhite = !interior && !['moon', 'none', 'lightning'].includes(L.source ?? '') && shot.location?.time !== 'night';
	const set: BuiltSet = shot.title ? { practical() {} } : buildSet(scene, shot, { emissive, interior }); // a title shot: black, only the title
	const { camera, camY, dist } = shotCamera(shot);

	const assets = await assetsFor(shot, opts.models);
	const figs: Figurine[] = [];
	for (const fig of shot.figures ?? []) {
		const model = figurine(fig.cast, assets);
		model.root.updateMatrixWorld(true);
		const rest = new THREE.Box3().setFromObject(model.root, true); // h = the figure standing: a lying pose is not blown up
		const restH = rest.max.y - rest.min.y;
		const { airborne } = pose(model, fig.pose ?? 'stand');
		model.root.updateMatrixWorld(true);
		const framing = fig.framing ?? 'full';
		const { box, whole } = framedBox(model, framing, restH);
		const size = Math.max(0.03, framing === 'full' || !framing ? Math.max(box.max.y - box.min.y, restH) : box.max.y - box.min.y);
		const closeUp = box !== whole;
		const place = placeFigure(camera, { W, H, K }, fig, size, airborne || closeUp);
		model.root.scale.multiplyScalar(place.scale);
		model.root.rotation.y = yawToward(camera, place.pos, turnOf(fig));
		const centre = box.getCenter(new THREE.Vector3());
		if (place.grounded) centre.set(centre.x, box.min.y, centre.z);
		const offset = centre.multiplyScalar(place.scale).applyAxisAngle(new THREE.Vector3(0, 1, 0), model.root.rotation.y);
		if (place.grounded) offset.set(offset.x, box.min.y * place.scale, offset.z);
		model.root.position.copy(place.pos).sub(offset);
		// how it was placed – the 3D view inverts this exactly when it writes a moved figure back as x / y / h / turn
		model.place = place;
		model.root.userData.figIndex = figs.length;
		model.root.traverse((o) => {
			if ((o as THREE.Mesh).isMesh) o.userData.fig = figs.length + 1;
		});
		scene.add(model.root);
		figs.push(model);
	}

	const light = placeLight(scene, camera, figs, L, dist, set, opts);
	const shared = [look.mat, emissive];
	return {
		scene, camera, figs, camY, dist, skyWhite, K, light,
		dispose() {
			disposeTree(scene);
			for (const m of shared) m.dispose();
			if (light.key?.shadow?.map) light.key.shadow.map.dispose();
			if (light.rim?.shadow?.map) light.rim.shadow.map.dispose();
		}
	};
}

/** frees what a tree holds on the GPU: geometries, materials, their textures, skeletons */
export function disposeTree(root: THREE.Object3D): void {
	const materials = new Set<THREE.Material>();
	root.traverse((o) => {
		const m = o as THREE.Mesh;
		if (!m.isMesh) return;
		m.geometry?.dispose();
		for (const mat of Array.isArray(m.material) ? m.material : [m.material]) if (mat) materials.add(mat);
		(o as THREE.SkinnedMesh).skeleton?.dispose();
	});
	for (const mat of materials) {
		for (const v of Object.values(mat)) if ((v as THREE.Texture)?.isTexture) (v as THREE.Texture).dispose();
		mat.dispose();
	}
}

/**
 * one hard key (directional, or a practical point light) + a low fill + an optional rim. Softened (owner 2026-10-10:
 * "light can be softer. its a bit too hard now"): the key no longer over-driven (faces turning away grade through the
 * hatch layers), a fill so cast shadows are hatched rather than solid black, soft shadow edges. opts.soft 0…1 (0.5).
 */
function placeLight(scene: THREE.Scene, camera: ViewCamera, figs: Figurine[], L: Light, dist: number, set: BuiltSet, opts: BuildOptions): SceneLight {
	const camRight = new THREE.Vector3(1, 0, 0).applyQuaternion(camera.quaternion).setY(0).normalize();
	const camBack = new THREE.Vector3(0, 0, 1).applyQuaternion(camera.quaternion).setY(0).normalize();
	const up = (y: number) => new THREE.Vector3(0, y, 0);
	const KEYDIR: Record<string, THREE.Vector3> = {
		left: camRight.clone().negate(), right: camRight.clone(), front: camBack.clone(), back: camBack.clone().negate(),
		top: up(1), bottom: up(-1),
		'top-left': camRight.clone().negate().add(up(0.8)), 'top-right': camRight.clone().add(up(0.8)),
		'bottom-left': camRight.clone().negate().add(up(-0.6)), 'bottom-right': camRight.clone().add(up(-0.6))
	};
	const ELEV: Record<string, number> = { below: -25, low: 9, eye: 22, high: 50, overhead: 84 };
	let keyDir: THREE.Vector3;
	if (L.azimuth != null || L.elevation != null) {
		const az = (L.azimuth ?? -45) * D,
			el = (L.elevation ?? 30) * D;
		const flat = camBack.clone().multiplyScalar(Math.cos(az)).add(camRight.clone().multiplyScalar(Math.sin(az))).normalize();
		keyDir = flat.multiplyScalar(Math.cos(el)).add(up(Math.sin(el)));
	} else {
		keyDir = (KEYDIR[L.key ?? ''] ?? KEYDIR['top-left']).clone().normalize();
		if (!['top', 'bottom'].includes(L.key ?? '')) {
			const e = (ELEV[L.height ?? ''] ?? 30) * D;
			const flat = keyDir.clone().setY(0).normalize();
			keyDir.copy(flat.multiplyScalar(Math.cos(e)).add(up(Math.sin(e))));
		}
	}
	const target = new THREE.Vector3();
	if (figs.length) {
		for (const m of figs) target.add(m.root.position);
		target.divideScalar(figs.length);
	} else target.copy(camera.position).addScaledVector(new THREE.Vector3(0, 0, -1).applyQuaternion(camera.quaternion), dist);
	const practical = ['candle', 'screen', 'neon', 'fire', 'energy'].includes(L.source ?? '');
	const reach = Math.max(8, dist * 2.5);
	const soft = Math.min(1, Math.max(0, L.soft ?? opts.soft ?? 0.5)); // the shot's light.soft (0 = Sin City hard … 1 = soft)
	let key: SceneLight['key'] = null;
	if (soft > 0) scene.add(new THREE.HemisphereLight(0xffffff, 0x808080, 1.2 * soft)); // the fill: shadows hatched, not black
	if (L.source !== 'none') {
		if (practical) {
			key = new THREE.PointLight(0xffffff, 260 * (reach / 8) ** 2, reach, 1.6);
			key.position.copy(target).addScaledVector(keyDir, Math.max(1.6, dist * 0.35));
			set.practical(key.position);
		} else {
			const k = new THREE.DirectionalLight(0xffffff, 14 - 10 * soft); // facing the key = white, turning away = hatch
			k.position.copy(target).addScaledVector(keyDir, 60);
			k.target.position.copy(target);
			scene.add(k.target);
			const span = Math.max(14, dist * 2.2);
			Object.assign(k.shadow.camera, { left: -span, right: span, top: span, bottom: -span, near: 1, far: 200 });
			key = k;
		}
		key.castShadow = true;
		key.shadow.mapSize.set(opts.shadowSize ?? 4096, opts.shadowSize ?? 4096);
		key.shadow.bias = -0.0004;
		key.shadow.radius = 1 + 5 * soft; // soft edges grade through the hatch layers
		scene.add(key);
	}
	let rim: THREE.DirectionalLight | null = null;
	if (L.rim || L.key === 'back' || L.source === 'none') {
		rim = new THREE.DirectionalLight(0xffffff, L.source === 'none' ? 9 : 7);
		rim.position.copy(target).add(camBack.clone().negate().multiplyScalar(30)).add(up(10));
		rim.target.position.copy(target);
		scene.add(rim, rim.target);
	}
	return { key, rim, target };
}
