// "Render all handheld shots" (owner 2026-10-19: "a function that renders all shots made this way as videos, which I
// then send to an AI as control frames"): every shot with the handheld camera on and an active take, along its baked
// track (smoothed, trimmed, at fps), with the studio's own renderer (tools/storyboard-render/core.js) – passes:
// beauty (the Sin City look), depth (white = near, inverse depth, one range per take), normals (view space), grey
// (plain lit). Frames go to the API, ffmpeg makes the videos (api.mjs):
// storyboards/_out/<chapter>/handheld/<shot>--<take>--<pass>.mp4 (+ .webm)
import * as THREE from 'three';
import { buildScene, seedOf, sinPass } from '../../../../../tools/storyboard-render/core.js';
import { api } from '../studio.svelte.js';
import { specFor } from './export.js';
import { bake } from './track.js';

export const PASSES = [
	{ id: 'beauty', label: 'Beauty', title: 'The studio look (Sin City lines + hatch) – also the lineart' },
	{ id: 'depth', label: 'Depth', title: 'White = near, black = far (inverse depth, one range per take)' },
	{ id: 'normals', label: 'Normals', title: 'View-space normals (ControlNet normal)' },
	{ id: 'grey', label: 'Grey', title: 'A plain lit grey render (the block-out look)' }
];

// depth: the scene's depth buffer → k / (z + k), k = the distance to the subject at the take's start (0.5 there)
const DEPTH = new THREE.ShaderMaterial({
	uniforms: { tDepth: { value: null }, near: { value: 0.05 }, far: { value: 600 }, k: { value: 3 } },
	vertexShader: 'varying vec2 vUv; void main(){ vUv = uv; gl_Position = vec4(position.xy, 0., 1.); }',
	fragmentShader: `uniform sampler2D tDepth; uniform float near, far, k; varying vec2 vUv;
		void main(){ float d = texture2D(tDepth, vUv).r; if (d > .99999) { gl_FragColor = vec4(0., 0., 0., 1.); return; }
		float z = 2. * near * far / (far + near - (d * 2. - 1.) * (far - near)); float v = k / (z + k); gl_FragColor = vec4(vec3(v), 1.); }`,
	depthTest: false, depthWrite: false
});

function passes(renderer) {
	const sin = sinPass(renderer);
	const normal = new THREE.MeshNormalMaterial();
	const post = new THREE.Scene();
	post.add(new THREE.Mesh(new THREE.PlaneGeometry(2, 2), DEPTH));
	const postCam = new THREE.OrthographicCamera(-1, 1, 1, -1, 0, 1);
	let rt = null;
	return {
		render(id, built, cam, w, h, look) {
			if (id === 'beauty') return sin.render(built.scene, cam, w, h, look);
			renderer.setSize(w, h, false);
			if (id === 'grey') {
				renderer.setRenderTarget(null);
				renderer.setClearColor(look.sky ? 0xd8d8d8 : 0x202020, 1);
				renderer.render(built.scene, cam);
			} else if (id === 'normals') {
				built.scene.overrideMaterial = normal;
				renderer.setRenderTarget(null);
				renderer.setClearColor(0x8080ff, 1);
				renderer.render(built.scene, cam);
				built.scene.overrideMaterial = null;
			} else {
				if (!rt || rt.width !== w || rt.height !== h) { rt?.dispose(); rt = new THREE.WebGLRenderTarget(w, h); rt.depthTexture = new THREE.DepthTexture(w, h); }
				built.scene.overrideMaterial = normal;
				renderer.setRenderTarget(rt);
				renderer.render(built.scene, cam);
				built.scene.overrideMaterial = null;
				const u = DEPTH.uniforms;
				u.tDepth.value = rt.depthTexture; u.near.value = cam.near; u.far.value = cam.far; u.k.value = look.k;
				renderer.setRenderTarget(null);
				renderer.render(post, postCam);
			}
			return renderer.domElement;
		},
		dispose() { sin.dispose(); normal.dispose(); rt?.dispose(); }
	};
}

/**
 * takes: { shotId: active take }; findShot = studio's find(id); opts: { passes: ['beauty', …], fps, ss (1 | 2), webm }
 * → [{ shot, files } | { shot, error }]
 */
export async function renderAll(chapter, takes, findShot, opts, progress = () => {}) {
	const ch = chapter.split('/')[1];
	const renderer = new THREE.WebGLRenderer({ antialias: false, preserveDrawingBuffer: true });
	const p = passes(renderer);
	const out = [];
	try {
		for (const [id, take] of Object.entries(takes)) {
			const spec = specFor(findShot(id), opts.ss ?? 1);
			if (!spec) { out.push({ shot: id, error: 'the shot is gone' }); continue; }
			const built = await buildScene(spec, { shadowSize: 2048 });
			const cam = built.camera;
			const frames = bake(take, opts.fps ?? 24);
			// the depth range: the subject's distance at the start (the light's target, else the figures' centre)
			const subject = built.light?.target ?? new THREE.Vector3();
			const k = Math.max(0.5, subject.distanceTo(new THREE.Vector3().fromArray(frames[0].position)));
			const look = { sky: built.skyWhite ? 1 : 0, accent: spec.accent ?? '#c00000', K: built.K ?? 1, seed: seedOf(spec.id), k };
			for (const [n, f] of frames.entries()) {
				progress(`${id} · frame ${n + 1}/${frames.length}`);
				cam.position.fromArray(f.position);
				cam.quaternion.fromArray(f.quaternion);
				if (cam.isPerspectiveCamera && f.fov) { cam.fov = f.fov; cam.updateProjectionMatrix(); }
				cam.updateMatrixWorld();
				for (const pass of opts.passes) {
					const png = p.render(pass, built, cam, spec.w, spec.h, look).toDataURL('image/png');
					await api('POST', '/api/handheld/frame', { ch, id, take: take.id, pass, n, png });
				}
			}
			built.dispose();
			const files = [];
			for (const pass of opts.passes) {
				progress(`${id} · encoding ${pass}`);
				const r = await api('POST', '/api/handheld/encode', { ch, id, take: take.id, pass, fps: opts.fps ?? 24, formats: opts.webm ? ['mp4', 'webm'] : ['mp4'] });
				if (!r.ok) { out.push({ shot: id, error: `${pass}: ${r.out} – the frames stay in ${r.frames}` }); continue; }
				files.push(...r.files);
			}
			out.push({ shot: id, files });
		}
	} finally {
		p.dispose();
		renderer.dispose();
		renderer.forceContextLoss();
	}
	return out;
}
