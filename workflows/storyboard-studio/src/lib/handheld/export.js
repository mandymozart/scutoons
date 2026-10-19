// The handheld takes as downloads (owner 2026-10-19: "Also als Download zusätzlich zum Rendern als Video"): per shot a
// GLB with the animated camera (optionally with the blocked scene) and a JSON of the samples; all handheld shots of the
// chapter as one zip. The track is the take as rendered: smoothed with its value, trimmed to in…out, t from 0, at the
// recorded rate (~60 Hz). The scene's world = glTF's: metres, Y up, the camera looks down its −Z.
// Blender: File → Import → glTF 2.0 (the camera + its action); Unreal: the glTF / Interchange importer → Sequencer.
import * as THREE from 'three';
import { GLTFExporter } from 'three/addons/exporters/GLTFExporter.js';
import { buildScene } from '../../../../../tools/storyboard-render/core.js';
import { shotSpec } from '../../../../../tools/storyboard-render/spec.js';
import { sampleAt, smooth } from './track.js';
import { download, zip } from './zip.js';

/** the take as exported: smoothed, trimmed (the ends interpolated exactly at in / out), t from 0 */
export function trimmed(take) {
	const all = smooth(take.samples, take.smooth ?? 0);
	if (!all.length) return [];
	const tin = Math.max(0, take.in ?? 0), tout = Math.min(all[all.length - 1].t, take.out ?? Infinity);
	const inner = all.filter((s) => s.t > tin + 1e-4 && s.t < tout - 1e-4);
	return [sampleAt(all, tin), ...inner, sampleAt(all, tout)].map((s) => ({ ...s, t: +(s.t - tin).toFixed(4) }));
}

/** the JSON for other tools */
export function takeJson(shotId, take, spec) {
	const samples = trimmed(take);
	return JSON.stringify({
		format: 'scutoons-handheld-camera/1',
		shot: shotId, take: take.id, created: take.created,
		space: { units: 'm', up: '+Y', camera: 'looks down its -Z, +Y up (three.js / glTF)', handedness: 'right' },
		aspect: spec ? spec.outW / spec.outH : null, width: spec?.outW ?? null, height: spec?.outH ?? null,
		smooth: take.smooth ?? 0, scale: take.scale ?? 1, duration: samples.length ? samples[samples.length - 1].t : 0,
		samples: samples.map((s) => ({ t: s.t, position: s.position.map((n) => +n.toFixed(5)), quaternion: s.quaternion.map((n) => +n.toFixed(6)), fov: s.fov }))
	}, null, 1);
}

/** the GLB: one perspective camera named <shot>-camera with an animation <shot>-<take>; withScene = the blocked set + figures too */
export async function takeGlb(shotId, take, spec, withScene = false) {
	const samples = trimmed(take);
	const fov = samples[0]?.fov ?? 50;
	const camera = new THREE.PerspectiveCamera(fov, spec ? spec.outW / spec.outH : 9 / 16, 0.05, 600);
	camera.name = `${shotId}-camera`;
	camera.position.fromArray(samples[0].position);
	camera.quaternion.fromArray(samples[0].quaternion);
	const times = samples.map((s) => s.t);
	const clip = new THREE.AnimationClip(`${shotId}-${take.id}`, times[times.length - 1] ?? 0, [
		new THREE.VectorKeyframeTrack(`${camera.name}.position`, times, samples.flatMap((s) => s.position)),
		new THREE.QuaternionKeyframeTrack(`${camera.name}.quaternion`, times, samples.flatMap((s) => s.quaternion))
	]);
	let root = new THREE.Scene();
	let built = null;
	if (withScene && spec) {
		built = await buildScene(spec, { shadowSize: 512 });
		root = built.scene;
		root.remove(built.camera);
	}
	root.add(camera);
	try {
		const glb = await new GLTFExporter().parseAsync(root, { binary: true, animations: [clip], onlyVisible: true });
		return new Uint8Array(glb);
	} finally {
		root.remove(camera);
		built?.dispose();
	}
}

/** hit = studio.svelte.js find(id): the shot's render spec */
export const specFor = (hit, ss = 1) => (hit ? shotSpec(hit.scene, JSON.parse(JSON.stringify(hit.shot)), ss) : null);

/** one shot's downloads */
export async function downloadTake(hit, shotId, take, kind, withScene) {
	const spec = specFor(hit);
	if (kind === 'json') return download(`${shotId}-${take.id}-camera.json`, new Blob([takeJson(shotId, take, spec)], { type: 'application/json' }));
	return download(`${shotId}-${take.id}-camera.glb`, new Blob([await takeGlb(shotId, take, spec, withScene)], { type: 'model/gltf-binary' }));
}

/** every handheld shot of the chapter (takes: { shotId: active take }) → <chapter>-handheld.zip */
export async function downloadAll(chapter, takes, findShot, withScene, progress = () => {}) {
	const files = [];
	const ids = Object.keys(takes);
	for (const [i, id] of ids.entries()) {
		progress(`${i + 1}/${ids.length} ${id}`);
		const spec = specFor(findShot(id));
		files.push({ name: `${id}/${id}-${takes[id].id}-camera.glb`, data: await takeGlb(id, takes[id], spec, withScene) });
		files.push({ name: `${id}/${id}-${takes[id].id}-camera.json`, data: takeJson(id, takes[id], spec) });
	}
	files.push({ name: 'README.txt', data: README });
	download(`${chapter.replace('/', '-')}-handheld.zip`, zip(files));
	return ids.length;
}

const README = `SCUTOONS storyboard studio – handheld camera takes
Per shot: <shot>-<take>-camera.glb (an animated perspective camera, optionally the blocked scene) and
<shot>-<take>-camera.json (samples t [s], position [m], quaternion [x, y, z, w], fov [vertical degrees]).
Space: metres, +Y up, the camera looks down its -Z (three.js / glTF).

Blender: File > Import > glTF 2.0 (.glb). The camera comes with its action; set the scene's frame rate, then
  Object > Animation > Bake Action if you need keys on whole frames. FBX: after the import, File > Export > FBX.
Unreal: import the .glb (glTF / Interchange importer), add the camera to a Level Sequence (Sequencer) –
  or import the FBX exported from Blender as a camera track in Sequencer.
`;
