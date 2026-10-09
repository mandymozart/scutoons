// One storyboard shot → one image (owner 2026-10-09: images, not SVG; Sin City black and white; extreme hard light,
// never diffuse; posable figurines; our own shader; "more line art, but keep intense bw to see lighting clearly").
// Split in three so the studio can move a camera live: buildScene (set + figurines + the one hard light), sinPass
// (the look: lit + mask → black / white, ink only where a thing ends, organic width, the accent), renderShot (both,
// plus panoramas, the little planet, the storyboard marks). render.js (headless) and the studio use the same code.
//
// Hand overrides (the studio writes them):
//   shot.camera = { pos: [x, y, z], target: [x, y, z], fov, roll }   – a free camera (the 3D view)
//              or { dist, height, pitch, yaw, roll, fov }             – adjustments of the preset camera
//   light.azimuth / light.elevation (°): 0 = from the camera's side, 90 = from the right, −90 left, 180 behind
//   figure.turn (°) beside facing
import * as THREE from 'three';
import { GLTFLoader } from 'three/addons/loaders/GLTFLoader.js';
import { figurine, look, pose } from './figures.js';
import { gltfTiger } from './tiger-gltf.js';
import { buildSet } from './sets.js';
import { presetOf, presetStyle } from './text-presets.js';

const D = Math.PI / 180;
const FACE = { front: 0, away: 180, left: -90, right: 90, 'three-quarter-left': -40, 'three-quarter-right': 40 };
let tigerGltf = null;

/** the shot's camera */
export function shotCamera(shot) {
	const W = shot.w, H = shot.h;
	const aspect = W / H;
	const tall = shot.format === 'loop360' || shot.projection === 'polar' ? 16 / 9 : Math.max(16 / 9, H / W);
	const ANG = { eye: [1.55, 0], high: [5, -24], low: [0.45, 12], bird: [22, -78], worm: [0.12, 30], dutch: [1.55, 0] };
	let [camY, pitch] = ANG[shot.angle] ?? ANG.eye;
	if (shot.projection === 'top-down') { camY = 24; pitch = -89.9; }
	if (shot.format === 'vertical' && ['tilt-down', 'fall', 'descend', 'reveal'].includes(shot.move) && shot.angle !== 'bird') pitch -= 8;
	if (shot.format === 'vertical' && ['tilt-up', 'climb'].includes(shot.move)) pitch += 10;
	const DIST = { ecu: 0.9, cu: 1.6, mcu: 2.4, medium: 3.5, full: 6, wide: 11, ews: 24 };
	const c = shot.camera ?? {};
	const dist = c.dist ?? DIST[shot.size] ?? 7;
	camY = c.height ?? camY;
	pitch = c.pitch ?? pitch;
	const vfov = c.fov ?? Math.min(150, 2 * Math.atan(Math.tan(25 * D) * (tall / (16 / 9))) / D);
	let camera;
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

/**
 * The scene of a shot: the set, the figurines at the storyboard's screen positions (seen through the shot's camera),
 * the one hard key light (+ rim). → { scene, camera, figs, camY, dist, skyWhite, K, light, dispose }
 */
export async function buildScene(shot, opts = {}) {
	const W = shot.w, H = shot.h;
	const K = W / (shot.outW ?? W);
	look.mat = new THREE.MeshLambertMaterial({ color: 0xffffff });
	const accentMat = new THREE.MeshBasicMaterial({ color: 0xffffff });
	const emissive = new THREE.MeshBasicMaterial({ color: 0xffffff });
	const scene = new THREE.Scene();
	const L = shot.light ?? {};
	const interior = shot.location?.time === 'interior' || /library|monastery|studio|server/.test(shot.location?.place ?? '') || L.source === 'none';
	const skyWhite = !interior && !['moon', 'none', 'lightning'].includes(L.source) && shot.location?.time !== 'night';
	const set = buildSet(scene, shot, { emissive, accentMat, interior });
	const { camera, camY, dist } = shotCamera(shot);

	const figs = [];
	const assets = { gltfTiger };
	if ((shot.figures ?? []).some((f) => f.cast?.persona === 'mountaintiger')) {
		tigerGltf ??= await new GLTFLoader().loadAsync(opts.tigerUrl ?? new URL('./models/mountain-tiger.glb', import.meta.url).href);
		assets.tiger = tigerGltf;
	}
	const fpx = (H / 2) / Math.tan((camera.fov ?? 50) * D / 2);
	for (const fig of shot.figures ?? []) {
		const model = figurine(fig.cast, assets);
		const { airborne } = pose(model, fig.pose ?? 'stand');
		model.root.updateMatrixWorld(true);
		const whole = new THREE.Box3().setFromObject(model.root);
		const framing = fig.framing ?? 'full';
		let head = model.joints.head && !model.joints.rigged ? new THREE.Box3().setFromObject(model.joints.head) : whole;
		if (model.joints.rigged) {
			const zl = whole.max.z - whole.min.z, yl = whole.max.y - whole.min.y;
			head = new THREE.Box3(new THREE.Vector3(whole.min.x, whole.min.y + yl * 0.45, whole.max.z - zl * 0.24), whole.max.clone());
		}
		let box = whole;
		if (['cu', 'ecu'].includes(framing)) box = head.clone();
		if (framing === 'ecu') { const c = head.getCenter(new THREE.Vector3()); box = new THREE.Box3(c.clone().add(new THREE.Vector3(0, -0.02, 0)), c.clone().add(new THREE.Vector3(0, 0.09, 0))); }
		if (framing === 'mcu') box = new THREE.Box3(new THREE.Vector3(whole.min.x, head.max.y - (head.max.y - whole.min.y) * 0.42, whole.min.z), whole.max.clone());
		const size = Math.max(0.03, box.max.y - box.min.y);
		const closeUp = box !== whole;
		const hPx = Math.max(12 * K, (fig.h ?? 0.3) * 844 * K);
		const cx = (fig.x ?? 0.5) * W, cy = (fig.y ?? 0.5) * H;
		const ray = (px, py) => new THREE.Vector3((px / W) * 2 - 1, -(py / H) * 2 + 1, 0.5).unproject(camera).sub(camera.position).normalize();
		const dir = ray(cx, cy + hPx / 2);
		let pos, depth, grounded = false;
		if (!airborne && !closeUp && dir.y < -0.01 && camera.isPerspectiveCamera) {
			pos = camera.position.clone().addScaledVector(dir, -camera.position.y / dir.y);
			depth = -pos.clone().applyMatrix4(camera.matrixWorldInverse).z;
			grounded = true;
		} else {
			depth = Math.max(0.8, (size * fpx) / hPx);
			pos = camera.position.clone().addScaledVector(ray(cx, cy), depth);
		}
		const scale = camera.isPerspectiveCamera ? (hPx * depth) / (size * fpx) : hPx / (H / (camera.top - camera.bottom));
		model.root.scale.multiplyScalar(scale);
		const toCam = Math.atan2(camera.position.x - pos.x, camera.position.z - pos.z);
		model.root.rotation.y = toCam + (fig.turn ?? FACE[fig.facing] ?? 0) * D;
		const centre = box.getCenter(new THREE.Vector3());
		if (grounded) centre.set(centre.x, box.min.y, centre.z);
		const offset = centre.multiplyScalar(scale).applyAxisAngle(new THREE.Vector3(0, 1, 0), model.root.rotation.y);
		if (grounded) offset.set(offset.x, box.min.y * scale, offset.z);
		model.root.position.copy(pos).sub(offset);
		model.root.userData.figIndex = figs.length;
		model.root.traverse((o) => { if (o.isMesh) o.userData.fig = figs.length + 1; });
		scene.add(model.root);
		figs.push(model);
	}

	const light = placeLight(scene, camera, figs, L, dist, set, opts);
	return {
		scene, camera, figs, camY, dist, skyWhite, K, light,
		dispose() { scene.traverse((o) => { if (o.isMesh) o.geometry?.dispose(); }); if (light.key?.shadow?.map) light.key.shadow.map.dispose(); }
	};
}

/** one extremely hard key (directional, or a practical point light) + an optional rim; no ambient, no fill */
function placeLight(scene, camera, figs, L, dist, set, opts) {
	const camRight = new THREE.Vector3(1, 0, 0).applyQuaternion(camera.quaternion).setY(0).normalize();
	const camBack = new THREE.Vector3(0, 0, 1).applyQuaternion(camera.quaternion).setY(0).normalize();
	const KEYDIR = {
		left: camRight.clone().negate(), right: camRight.clone(), front: camBack.clone(), back: camBack.clone().negate(),
		top: new THREE.Vector3(0, 1, 0), bottom: new THREE.Vector3(0, -1, 0),
		'top-left': camRight.clone().negate().add(new THREE.Vector3(0, 0.8, 0)), 'top-right': camRight.clone().add(new THREE.Vector3(0, 0.8, 0)),
		'bottom-left': camRight.clone().negate().add(new THREE.Vector3(0, -0.6, 0)), 'bottom-right': camRight.clone().add(new THREE.Vector3(0, -0.6, 0))
	};
	const ELEV = { below: -25, low: 9, eye: 22, high: 50, overhead: 84 };
	let keyDir;
	if (L.azimuth != null || L.elevation != null) {
		const az = (L.azimuth ?? -45) * D, el = (L.elevation ?? 30) * D;
		const flat = camBack.clone().multiplyScalar(Math.cos(az)).add(camRight.clone().multiplyScalar(Math.sin(az))).normalize();
		keyDir = flat.multiplyScalar(Math.cos(el)).add(new THREE.Vector3(0, Math.sin(el), 0));
	} else {
		keyDir = (KEYDIR[L.key] ?? KEYDIR['top-left']).clone().normalize();
		if (!['top', 'bottom'].includes(L.key)) {
			const e = (ELEV[L.height] ?? 30) * D;
			const flat = keyDir.clone().setY(0).normalize();
			keyDir.copy(flat.multiplyScalar(Math.cos(e)).add(new THREE.Vector3(0, Math.sin(e), 0)));
		}
	}
	const target = new THREE.Vector3();
	if (figs.length) { for (const m of figs) target.add(m.root.position); target.divideScalar(figs.length); }
	else target.copy(camera.position).addScaledVector(new THREE.Vector3(0, 0, -1).applyQuaternion(camera.quaternion), dist);
	const practical = ['candle', 'screen', 'neon', 'fire', 'energy'].includes(L.source);
	const reach = Math.max(8, dist * 2.5);
	let key = null;
	if (L.source !== 'none') {
		if (practical) {
			key = new THREE.PointLight(0xffffff, 260 * (reach / 8) ** 2, reach, 1.6);
			key.position.copy(target).addScaledVector(keyDir, Math.max(1.6, dist * 0.35));
			set.practical?.(key.position);
		} else {
			key = new THREE.DirectionalLight(0xffffff, 14); // over-driven: anything facing the key is white
			key.position.copy(target).addScaledVector(keyDir, 60);
			key.target.position.copy(target);
			scene.add(key.target);
			const span = Math.max(14, dist * 2.2);
			Object.assign(key.shadow.camera, { left: -span, right: span, top: span, bottom: -span, near: 1, far: 200 });
		}
		key.castShadow = true;
		key.shadow.mapSize.set(opts.shadowSize ?? 4096, opts.shadowSize ?? 4096);
		key.shadow.bias = -0.0004;
		scene.add(key);
	}
	let rim = null;
	if (L.rim || L.key === 'back' || L.source === 'none') {
		rim = new THREE.DirectionalLight(0xffffff, L.source === 'none' ? 9 : 7);
		rim.position.copy(target).add(camBack.clone().negate().multiplyScalar(30)).add(new THREE.Vector3(0, 10, 0));
		rim.target.position.copy(target);
		scene.add(rim, rim.target);
	}
	return { key, rim, target };
}

/**
 * The look as a reusable pass on one renderer: render(scene, camera, w, h, { sky, accent, K, seed }) → the renderer's
 * canvas. Lit + a mask (R = accent, G = object id) → pure black / white at a hard threshold; ink only where one thing
 * ends (object ids; depth jumps between set pieces), organic width, drawn inverted against what is behind it.
 */
export function sinPass(renderer) {
	renderer.shadowMap.enabled = true;
	renderer.shadowMap.type = THREE.BasicShadowMap;
	renderer.outputColorSpace = THREE.LinearSRGBColorSpace;
	const maskMats = {};
	const normalMat = new THREE.MeshNormalMaterial();
	const SIN = new THREE.ShaderMaterial({
		uniforms: {
			tLit: { value: null }, tMask: { value: null }, tDepth: { value: null }, tNorm: { value: null },
			texel: { value: new THREE.Vector2() }, res: { value: new THREE.Vector2() }, threshold: { value: 0.32 }, sky: { value: 1 },
			ink: { value: 1.6 }, inkVar: { value: 2.2 }, seed: { value: 0 },
			accent: { value: new THREE.Color('#c00000') }, near: { value: 0.05 }, far: { value: 600 }, ortho: { value: 0 },
			shade: { value: 0 }, color: { value: 0 }, wobble: { value: 0 }, inner: { value: 0 }, hatch: { value: 8 }, K: { value: 1 }
		},
		vertexShader: 'varying vec2 vUv; void main(){ vUv = uv; gl_Position = vec4(position.xy, 0., 1.); }',
		// THE LOOK. Base: pure black / white from a hard luminance threshold (the one hard key did the work) – the Sin City
		// storyboard. Options (owner 2026-10-10, after "Moebius-style post-processing" by Maxime Heckel – black and white
		// by default, colour optional):
		//   shade 0 = solid black shadows · 1 = crosshatching (more layers the darker: diagonal <= .80, vertical <= .55,
		//   horizontal <= .35, stripes every `hatch` px) · 2 = tones (0 / .25 / .5 / .75 steps)
		//   inner = Sobel lines inside things too (depth x 25 + the normals' luma; for figurines off by default – the owner
		//   wants no lines at joints) · wobble = hand-drawn displacement of the line sampling (0 = clean line art)
		//   color = keep the lit colour (the tiger's texture, accents) instead of black / white
		// Outlines: where one THING ends (object ids) and depth jumps between set pieces, organic width, inverted against
		// what is behind them.
		fragmentShader: `
			uniform sampler2D tLit, tMask, tDepth, tNorm; uniform vec2 texel, res; uniform float threshold, sky, near, far, ortho, ink, inkVar, seed, shade, color, wobble, inner, hatch, K; uniform vec3 accent;
			varying vec2 vUv;
			float lin(float d){ if (ortho > .5) return near + d * (far - near); float z = d * 2. - 1.; return 2. * near * far / (far + near - z * (far - near)); }
			float dep(vec2 uv){ return lin(texture2D(tDepth, uv).r); }
			float oid(vec2 uv){ return floor(texture2D(tMask, uv).g * 255. + .5); }
			float luma(vec3 c){ return dot(c, vec3(.2125, .7154, .0721)); }
			float hash(vec2 p){ return fract(sin(dot(p, vec2(127.1, 311.7)) + seed) * 43758.5453); }
			float vnoise(vec2 p){ vec2 i = floor(p), f = fract(p); f = f * f * (3. - 2. * f);
				return mix(mix(hash(i), hash(i + vec2(1., 0.)), f.x), mix(hash(i + vec2(0., 1.)), hash(i + vec2(1., 1.)), f.x), f.y); }
			// Sobel over depth (relative to the distance, weighted) and the normals' luma – the Moebius inner lines
			float sobel(vec2 uv, vec2 st){
				float gx = 0., gy = 0., nx = 0., ny = 0.;
				float dc = max(dep(uv), .001);
				for (int j = 0; j < 3; j++) for (int i = 0; i < 3; i++) {
					float fx = float(i) - 1., fy = float(j) - 1.;
					float wx = fx * (fy == 0. ? 2. : 1.), wy = fy * (fx == 0. ? 2. : 1.);
					vec2 o = vec2(fx, fy) * st;
					float d = dep(uv + o) / dc;
					float n = luma(texture2D(tNorm, uv + o).rgb);
					gx += wx * d; gy += wy * d; nx += wx * n; ny += wy * n;
				}
				return sqrt(gx * gx + gy * gy) * 3. + sqrt(nx * nx + ny * ny);
			}
			void main(){
				vec2 px = vUv * res;
				vec2 wob = wobble > 0. ? vec2(sin(px.y * .08), cos(px.x * .08)) * hash(floor(px / 6.)) * wobble * texel : vec2(0.);
				vec2 uv = vUv + wob;
				bool empty = texture2D(tDepth, vUv).r > 0.99999;
				vec3 lit = texture2D(tLit, vUv).rgb;
				float l = luma(lit);
				float v = empty ? sky : step(threshold, l);
				float s = v;
				if (!empty && shade > .5 && shade < 1.5) {
					float sp = hatch * K, w = max(1., K);
					s = 1.;
					if (l <= .80 && mod(px.x + px.y, sp) <= w) s = 0.;
					if (l <= .55 && mod(px.x, sp) <= w) s = 0.;
					if (l <= .35 && mod(px.y, sp) <= w) s = 0.;
					if (l <= .12) s = 0.;
				} else if (!empty && shade > 1.5) {
					s = l <= .2 ? 0. : l <= .35 ? .25 : l <= .5 ? .5 : l <= .7 ? .75 : 1.;
				}
				float n = vnoise(px / 46.) * .65 + vnoise(px / 13.) * .35;
				float width = ink + inkVar * n * n;
				float dc = dep(uv), ic = oid(uv), e = 0.;
				for (int r = 1; r <= 6; r++) {
					float rr = float(r);
					if (rr > width) break;
					for (int k = 0; k < 8; k++) {
						float a = float(k) * .7853982;
						vec2 q = uv + vec2(cos(a), sin(a)) * texel * rr;
						float id = oid(q);
						if (id != ic) { e = 1.; break; }
						float dn = dep(q);
						if (ic < .5 && abs(dn - dc) > .06 * min(dc, dn)) { e = 1.; break; }
					}
					if (e > .5) break;
				}
				if (!empty && e < .5 && (inner > .5 || (ic < .5 && shade > .5))) {
					if (sobel(uv, texel * max(1., K) * 1.5) > .9) e = 1.;
				}
				vec3 col;
				if (color > .5) {
					col = empty ? vec3(sky) : lit * (shade > .5 ? s : (v > .5 ? 1. : .18));
					if (e > .5) col = vec3(0.);
				} else {
					float c = s;
					if (e > .5) c = 1. - step(.5, s); // ink inverted against what is under it – also on a hatched or toned shadow
					col = vec3(c);
				}
				float acc = texture2D(tMask, vUv).r;
				if (acc > .5 && e < .5 && color < .5) col = v > .5 ? accent : accent * .55;
				gl_FragColor = vec4(col, 1.);
			}`,
		depthTest: false, depthWrite: false
	});
	const post = new THREE.Scene();
	post.add(new THREE.Mesh(new THREE.PlaneGeometry(2, 2), SIN));
	const postCam = new THREE.OrthographicCamera(-1, 1, 1, -1, 0, 1);
	let rt = null;
	const targets = (w, h) => {
		if (rt && rt.w === w && rt.h === h) return rt;
		rt?.lit.dispose(); rt?.mask.dispose(); rt?.norm.dispose();
		const make = () => { const t = new THREE.WebGLRenderTarget(w, h, { depthBuffer: true }); t.depthTexture = new THREE.DepthTexture(w, h); return t; };
		return (rt = { w, h, lit: make(), mask: make(), norm: make() });
	};
	return {
		/** p: { sky, accent, K, seed, shade: 'solid' | 'hatch' | 'tone', color, wobble (px), inner } */
		render(scene, cam, w, h, p = {}) {
			renderer.setSize(w, h, false);
			const { lit, mask, norm } = targets(w, h);
			renderer.setClearColor(0x000000, 1);
			renderer.setRenderTarget(lit); renderer.render(scene, cam);
			if ((p.shade && p.shade !== 'solid') || p.inner) {
				scene.overrideMaterial = normalMat;
				renderer.setRenderTarget(norm); renderer.render(scene, cam);
				scene.overrideMaterial = null;
			}
			const swap = [];
			scene.traverse((o) => {
				if (!o.isMesh || o.userData.helper) return;
				swap.push([o, o.material]);
				const k = `${o.userData.accent ? 1 : 0}/${o.userData.fig ?? 0}`;
				o.material = (maskMats[k] ??= new THREE.MeshBasicMaterial({ color: new THREE.Color(o.userData.accent ? 1 : 0, (o.userData.fig ?? 0) / 255, 0) }));
			});
			renderer.setRenderTarget(mask); renderer.render(scene, cam);
			for (const [o, m] of swap) o.material = m;
			const u = SIN.uniforms;
			u.tLit.value = lit.texture; u.tMask.value = mask.texture; u.tDepth.value = lit.depthTexture; u.tNorm.value = norm.texture;
			u.texel.value.set(1 / w, 1 / h); u.res.value.set(w, h);
			u.near.value = cam.near; u.far.value = cam.far; u.ortho.value = cam.isOrthographicCamera ? 1 : 0;
			u.sky.value = p.sky ?? 1; u.ink.value = 1.6 * (p.K ?? 1); u.inkVar.value = 2.2 * (p.K ?? 1); u.seed.value = p.seed ?? 0; u.K.value = p.K ?? 1;
			u.shade.value = { hatch: 1, tone: 2 }[p.shade] ?? 0; u.color.value = p.color ? 1 : 0; u.wobble.value = (p.wobble ?? 0) * (p.K ?? 1); u.inner.value = p.inner ? 1 : 0;
			u.accent.value.set(p.accent ?? '#c00000');
			renderer.setRenderTarget(null); renderer.render(post, postCam);
			return renderer.domElement;
		},
		dispose() { rt?.lit.dispose(); rt?.mask.dispose(); rt?.norm.dispose(); SIN.dispose(); normalMat.dispose(); for (const m of Object.values(maskMats)) m.dispose(); }
	};
}

/** where each figure landed on the image: its projected box in output px */
export function figureBoxes(figs, camera, W, H, K = 1) {
	return figs.map((m, index) => {
		const b = new THREE.Box3().setFromObject(m.root);
		let x0 = Infinity, y0 = Infinity, x1 = -Infinity, y1 = -Infinity;
		for (const cx of [b.min.x, b.max.x]) for (const cy of [b.min.y, b.max.y]) for (const cz of [b.min.z, b.max.z]) {
			const p = new THREE.Vector3(cx, cy, cz).project(camera);
			const px = ((p.x + 1) / 2) * W, py = ((1 - p.y) / 2) * H;
			x0 = Math.min(x0, px); y0 = Math.min(y0, py); x1 = Math.max(x1, px); y1 = Math.max(y1, py);
		}
		return { index, box: { x0: x0 / K, y0: y0 / K, x1: x1 / K, y1: y1 / K } };
	});
}

export const seedOf = (id) => [...String(id)].reduce((a, c) => a + c.charCodeAt(0), 0) % 97;

/** one shot → { canvas, figures } (opts: renderer, canvas, tigerUrl, marks, bubbles, shadowSize) */
export async function renderShot(shot, opts = {}) {
	const W = shot.w, H = shot.h;
	const canvas = opts.canvas ?? document.createElement('canvas');
	canvas.width = W; canvas.height = H;
	const out2d = canvas.getContext('2d');
	const renderer = opts.renderer ?? new THREE.WebGLRenderer({ antialias: false, preserveDrawingBuffer: true });
	const pass = sinPass(renderer);
	const built = await buildScene(shot, opts);
	const { scene, camera, figs, camY, K, skyWhite } = built;
	const look = { sky: skyWhite ? 1 : 0, accent: shot.accent ?? '#c00000', K, seed: seedOf(shot.id), ...(shot.look ?? {}) };

	const panorama = (pw, ph) => {
		const n = Math.max(4, Math.round(pw / (390 * K)));
		const sliceW = Math.round(pw / n);
		const pano = document.createElement('canvas');
		pano.width = sliceW * n; pano.height = ph;
		const g = pano.getContext('2d');
		const cam = new THREE.PerspectiveCamera(1, sliceW / ph, 0.05, 600);
		const hfov = 360 / n;
		cam.fov = 2 * Math.atan(Math.tan(hfov * D / 2) / cam.aspect) / D;
		cam.updateProjectionMatrix();
		cam.position.set(0, camY, 0);
		cam.rotation.order = 'YXZ';
		for (let i = 0; i < n; i++) {
			cam.rotation.set(0, -i * hfov * D, 0);
			cam.updateMatrixWorld();
			g.drawImage(pass.render(scene, cam, sliceW, ph, look), i * sliceW, 0);
		}
		return pano;
	};

	let image;
	if (shot.format === 'loop360') {
		for (const [i, m] of figs.entries()) {
			const fig = shot.figures[i];
			const bearing = -(fig.x ?? 0.5) * 2 * Math.PI;
			const r = 4 + (1 - (fig.h ?? 0.3)) * 8;
			m.root.position.set(Math.sin(-bearing) * -r, 0, -Math.cos(bearing) * r);
			m.root.rotation.y = Math.atan2(-m.root.position.x, -m.root.position.z) + Math.PI + (fig.turn ?? 0) * D;
			m.root.scale.setScalar((fig.h ?? 0.3) * 2.2 * r / 4);
			pose(m, fig.pose ?? 'stand');
		}
		image = panorama(W, H);
	} else if (shot.projection === 'polar') {
		const pano = panorama(1560 * K, 520 * K);
		const src = pano.getContext('2d').getImageData(0, 0, pano.width, pano.height).data;
		const out = out2d.createImageData(W, H);
		const cx = W / 2, cy = H * 0.62, R = Math.min(W, H) * 0.62;
		for (let y = 0; y < H; y++) for (let x = 0; x < W; x++) {
			const dx = x - cx, dy = y - cy, r = Math.hypot(dx, dy) / R;
			const a = (Math.atan2(dx, dy) / (2 * Math.PI) + 1) % 1;
			const sy = Math.min(pano.height - 1, Math.max(0, Math.round((1 - Math.min(1, r)) * (pano.height - 1))));
			const sx = Math.min(pano.width - 1, Math.round(a * (pano.width - 1)));
			const si = (sy * pano.width + sx) * 4, oi = (y * W + x) * 4;
			out.data[oi] = src[si]; out.data[oi + 1] = src[si + 1]; out.data[oi + 2] = src[si + 2]; out.data[oi + 3] = 255;
		}
		out2d.putImageData(out, 0, 0);
	} else {
		image = pass.render(scene, camera, W, H, look);
	}
	if (image) out2d.drawImage(image, 0, 0, W, H);
	const figures = figureBoxes(figs, camera, W, H, K);
	let clean = null;
	if (opts.clean) { clean = document.createElement('canvas'); clean.width = W; clean.height = H; clean.getContext('2d').drawImage(canvas, 0, 0); }
	if (opts.bubbles !== false && (shot.dialog ?? []).length) await loadFonts();
	if (opts.bubbles !== false) drawDialog(out2d, shot, figures.map((f) => ({ ...f, box: { x0: f.box.x0 * K, y0: f.box.y0 * K, x1: f.box.x1 * K, y1: f.box.y1 * K } })), W, H, K);
	if (opts.marks !== false) drawMarks(out2d, shot, W, H, K);
	built.dispose();
	pass.dispose();
	return { canvas, clean, figures };
}

/** the storyboard marks: id + format top left, air zones as a bar on the left edge */
export function drawMarks(g, shot, W, H, K = 1) {
	g.font = `600 ${11 * K}px ui-monospace, Consolas, monospace`;
	const label = `${shot.id} · ${shot.format} ${shot.aspect}${shot.move ? ` · ${shot.move}` : ''}${shot.want ? ` [${shot.want}]` : ''}`;
	const tw = g.measureText(label).width;
	g.fillStyle = '#000'; g.fillRect(0, 0, tw + 12 * K, 18 * K);
	g.fillStyle = '#fff'; g.fillText(label, 6 * K, 13 * K);
	for (const [a, b] of shot.air ?? []) {
		const y0 = (a / 100) * H, y1 = (b / 100) * H;
		g.fillStyle = '#888';
		g.fillRect(K, y0 + 2 * K, 3 * K, Math.max(2 * K, y1 - y0 - 4 * K));
	}
}

/**
 * The texts in the SCUTOONS text presets (text-presets.js, agents/DESIGN.md Part 2 – owner 2026-10-09: the first
 * throw follows the design dogma): Narration (paper box, 2px ink rule, italic IBM Plex Mono), Scene (the same in
 * capitals), Speech bubble (paper ellipse, 2px ink outline, a tail – towards the nearest figure's head unless the text
 * sets `tail`), Onomatopoeia (Bangers, white with an ink outline, −7°), Story (bare mono). Font size = % of the shot's
 * width like the reader's cqw; the box at the storyboard's rect; the real words – the room for the text is judged in
 * the frame. figures: [{ box: {x0,y0,x1,y1} }] in canvas px.
 */
export function drawDialog(g, shot, figures, W, H, K = 1) {
	const wrap = (text, maxW) => {
		const out = [];
		for (const para of String(text).split('\n')) {
			let line = '';
			for (const w of para.split(/\s+/)) {
				const t = line ? `${line} ${w}` : w;
				if (g.measureText(t).width > maxW && line) { out.push(line); line = w; } else line = t;
			}
			out.push(line);
		}
		return out;
	};
	const INK = '#000', PAPER = '#fff';
	for (const d of shot.dialog ?? []) {
		if (!d.text) continue;
		const preset = presetOf(d);
		const st = presetStyle(preset, d);
		const r = d.rect ?? { x: 5, y: 5, width: 60 };
		const x = (r.x / 100) * W, y = (r.y / 100) * H, w = ((r.width ?? 60) / 100) * W;
		const size = ((d.size ?? st.size) / 100) * W;
		const text = st.uppercase ? String(d.text).toUpperCase() : String(d.text);
		g.save();
		if (preset === 'onomatopoeia') {
			g.font = `${size}px Bangers, Impact, sans-serif`;
			const lines = wrap(text, w);
			const lh = size * 1.05;
			g.translate(x + w / 2, y + (lines.length * lh) / 2);
			g.rotate((st.rotation * Math.PI) / 180);
			g.textAlign = 'center'; g.textBaseline = 'middle'; g.lineJoin = 'round';
			lines.forEach((l, i) => {
				const ly = (i - (lines.length - 1) / 2) * lh;
				g.lineWidth = size * st.strokeWidth * 2; g.strokeStyle = INK; g.strokeText(l, 0, ly);
				g.fillStyle = st.color; g.fillText(l, 0, ly);
			});
			g.restore();
			continue;
		}
		const italic = preset === 'narration' || preset === 'scene';
		g.font = `${italic ? 'italic ' : ''}400 ${size}px 'IBM Plex Mono', ui-monospace, monospace`;
		const pad = size * 0.5;
		const lines = wrap(text, w - pad * 2);
		const lh = size * 1.15;
		const th = Math.max(((r.height ?? 0) / 100) * H, lines.length * lh + pad * 2);
		const rule = 2 * K;
		g.lineWidth = rule; g.strokeStyle = INK; g.fillStyle = PAPER;
		if (preset === 'speech-bubble') {
			const cx = x + w / 2, cy = y + th / 2, rx = w / 2 + pad * 0.6, ry = th / 2 + pad * 0.8;
			// the tail: the text's `tail`, else towards the nearest figure's head
			let tx, ty;
			const heads = figures.map((f) => ({ x: (f.box.x0 + f.box.x1) / 2, y: f.box.y0 + (f.box.y1 - f.box.y0) * 0.12 }));
			if (d.tail && d.tail !== 'none') {
				ty = d.tail.startsWith('top') ? y - size * 1.8 : y + th + size * 1.8;
				tx = d.tail.endsWith('left') ? x + w * 0.22 : d.tail.endsWith('right') ? x + w * 0.78 : cx;
			} else if (heads.length) {
				const h = heads.sort((a, b) => Math.hypot(a.x - cx, a.y - cy) - Math.hypot(b.x - cx, b.y - cy))[0];
				const dx = h.x - cx, dy = h.y - cy, l = Math.hypot(dx, dy) || 1;
				const reach = Math.min(l * 0.7, Math.max(rx, ry) + size * 2.2);
				tx = cx + (dx / l) * reach; ty = cy + (dy / l) * reach;
			} else { tx = x + w * 0.22; ty = y + th + size * 1.8; }
			const a0 = Math.atan2((ty - cy) / ry, (tx - cx) / rx);
			g.beginPath();
			g.ellipse(cx, cy, rx, ry, 0, a0 + 0.16, a0 - 0.16 + Math.PI * 2);
			g.lineTo(tx, ty);
			g.closePath();
			g.fill(); g.stroke();
			g.fillStyle = INK; g.textAlign = 'center'; g.textBaseline = 'middle';
			lines.forEach((l, i) => g.fillText(l, cx, cy + (i - (lines.length - 1) / 2) * lh));
		} else {
			const boxed = preset !== 'story';
			const tw = Math.min(w, Math.max(...lines.map((l) => g.measureText(l).width)) + pad * 2);
			if (boxed) { g.fillRect(x, y, w, th); g.strokeRect(x + rule / 2, y + rule / 2, w - rule, th - rule); }
			g.fillStyle = INK; g.textAlign = 'left'; g.textBaseline = 'middle';
			const top = preset === 'story' ? y + pad + lh / 2 : y + th / 2 - ((lines.length - 1) * lh) / 2;
			lines.forEach((l, i) => g.fillText(l, x + pad, top + i * lh));
			void tw;
		}
		g.restore();
	}
}

/** the reader's fonts (self-hosted in @scutoons/tokens) – a canvas draws text only with fonts already loaded */
export async function loadFonts() {
	if (typeof document === 'undefined' || !document.fonts) return;
	// never wait forever (a headless page can stall on a font while a model loads): at most 3 s, then draw with what is there
	const all = Promise.all(["400 16px 'IBM Plex Mono'", "italic 400 16px 'IBM Plex Mono'", '16px Bangers'].map((f) => document.fonts.load(f).catch(() => null)));
	await Promise.race([all, new Promise((ok) => setTimeout(ok, 3000))]);
}
