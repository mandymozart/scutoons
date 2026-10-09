// One storyboard shot → one image (owner 2026-10-09: images, not SVG; Sin City black and white; extreme hard light,
// never diffuse; posable figurines; our own shader). Loaded by render.html?spec=<file url>&id=<shot id> in headless
// Edge / Chrome; the page shows exactly the finished image (canvas at 1:1), the runner screenshots it.
//
// Pipeline: the scene (set + figurines + one hard key light, shadows) → 3 passes into render targets: lit (Lambert,
// hard shadow map), normals + depth, accent mask → the "SIN CITY" pass: luminance → pure black / white at a hard
// threshold, ink lines from depth / normal edges drawn inverted against what is behind them (white on black, black on
// white), the accent colour where the mask is. A loop360 is rendered as N yaw slices into a seamless panorama; a polar
// shot warps that panorama into a little planet.
import * as THREE from 'three';
import { figurine, look, pose } from './figures.js';
import { buildSet } from './sets.js';

const D = Math.PI / 180;
const params = new URLSearchParams(location.search);
const { default: SHOTS } = await import(params.get('spec'));
const shot = SHOTS.find((s) => s.id === params.get('id')) ?? SHOTS[0];

const W = shot.w, H = shot.h;
const K = W / (shot.outW ?? W); // supersampling factor (render-shots.mjs renders at 2×)
const canvas = document.createElement('canvas');
canvas.width = W; canvas.height = H;
canvas.style.cssText = `display:block;width:${W}px;height:${H}px`;
document.body.appendChild(canvas);
const out2d = canvas.getContext('2d');

const renderer = new THREE.WebGLRenderer({ antialias: false, preserveDrawingBuffer: true });
renderer.shadowMap.enabled = true;
renderer.shadowMap.type = THREE.BasicShadowMap; // hard-edged shadows, no filtering
renderer.outputColorSpace = THREE.LinearSRGBColorSpace;

// --- materials ---------------------------------------------------------------------------------------------------
look.mat = new THREE.MeshLambertMaterial({ color: 0xffffff });
const accentMat = new THREE.MeshBasicMaterial({ color: 0xffffff });
const blackMat = new THREE.MeshBasicMaterial({ color: 0x000000 });
const normalMat = new THREE.MeshNormalMaterial();
const emissive = new THREE.MeshBasicMaterial({ color: 0xffffff }); // screens, candles, the energy ball: always white

// --- scene -------------------------------------------------------------------------------------------------------
const scene = new THREE.Scene();
const L = shot.light ?? {};
const interior = shot.location?.time === 'interior' || /library|monastery|studio|server/.test(shot.location?.place ?? '') || L.source === 'none';
const skyWhite = !interior && !['moon', 'none', 'lightning'].includes(L.source) && shot.location?.time !== 'night';
const set = buildSet(scene, shot, { emissive, accentMat, interior });

// camera from the shot's size / angle / projection
const aspect = W / H;
const tall = shot.format === 'loop360' || shot.projection === 'polar' ? 16 / 9 : Math.max(16 / 9, H / W);
const vfov = Math.min(150, 2 * Math.atan(Math.tan(25 * D) * (tall / (16 / 9))) / D);
const ANG = { eye: [1.55, 0], high: [5, -24], low: [0.45, 12], bird: [22, -78], worm: [0.12, 30], dutch: [1.55, 0] };
let [camY, pitch] = ANG[shot.angle] ?? ANG.eye;
if (shot.projection === 'top-down') { camY = 24; pitch = -89.9; }
if (shot.format === 'vertical' && ['tilt-down', 'fall', 'descend', 'reveal'].includes(shot.move) && shot.angle !== 'bird') pitch -= 8;
if (shot.format === 'vertical' && ['tilt-up', 'climb'].includes(shot.move)) pitch += 10;
const DIST = { ecu: 0.9, cu: 1.6, mcu: 2.4, medium: 3.5, full: 6, wide: 11, ews: 24 };
const dist = DIST[shot.size] ?? 7;
let camera;
if (shot.projection === 'isometric') {
	const hh = dist * 0.7;
	camera = new THREE.OrthographicCamera(-hh * aspect, hh * aspect, hh, -hh, 0.1, 400);
	camera.position.set(dist * 1.2, dist * 1.2, dist * 1.2);
	camera.lookAt(0, 0.8, 0);
} else {
	camera = new THREE.PerspectiveCamera(vfov, aspect, 0.05, 600);
	camera.position.set(0, camY, dist);
	camera.rotation.order = 'YXZ';
	camera.rotation.set(pitch * D, 0, shot.angle === 'dutch' ? 18 * D : 0);
}
camera.updateMatrixWorld();

// figurines, placed by the storyboard's screen position (x across, y along the shot, h = height in screens)
const figs = [];
const f = (H / 2) / Math.tan((camera.fov ?? 50) * D / 2); // px per unit at distance 1
for (const fig of shot.figures ?? []) {
	const model = figurine(fig.cast);
	const { airborne } = pose(model, fig.pose ?? 'stand');
	const hPx = Math.max(12 * K, (fig.h ?? 0.3) * 844 * K);
	const cx = (fig.x ?? 0.5) * W, cy = (fig.y ?? 0.5) * H;
	const feetPx = cy + hPx / 2;
	// ray through the feet; on the ground (y = 0) unless airborne or the ray misses
	const ndc = new THREE.Vector3((cx / W) * 2 - 1, -(feetPx / H) * 2 + 1, 0.5).unproject(camera);
	const dir = ndc.sub(camera.position).normalize();
	let pos, depth;
	if (!airborne && dir.y < -0.01 && camera.isPerspectiveCamera) {
		const t = -camera.position.y / dir.y;
		pos = camera.position.clone().addScaledVector(dir, t);
		depth = pos.clone().applyMatrix4(camera.matrixWorldInverse).z * -1;
	} else {
		depth = Math.max(1, (model.height * f) / hPx);
		const c = new THREE.Vector3((cx / W) * 2 - 1, -(cy / H) * 2 + 1, 0.5).unproject(camera).sub(camera.position).normalize();
		pos = camera.position.clone().addScaledVector(c, depth).add(new THREE.Vector3(0, -model.height / 2, 0));
	}
	const scale = camera.isPerspectiveCamera ? (hPx * depth) / (model.height * f) : 1;
	model.root.scale.multiplyScalar(scale);
	model.root.position.copy(pos);
	// facing, relative to the camera: front = towards it
	const toCam = Math.atan2(camera.position.x - pos.x, camera.position.z - pos.z);
	const FACE = { front: 0, away: 180, left: -90, right: 90, 'three-quarter-left': -40, 'three-quarter-right': 40 };
	model.root.rotation.y = toCam + (FACE[fig.facing] ?? 0) * D;
	scene.add(model.root);
	figs.push(model);
}

// --- light: one extremely hard key (+ an optional rim), no ambient, no fill --------------------------------------
const camRight = new THREE.Vector3(1, 0, 0).applyQuaternion(camera.quaternion).setY(0).normalize();
const camBack = new THREE.Vector3(0, 0, 1).applyQuaternion(camera.quaternion).setY(0).normalize(); // towards the camera
const KEYDIR = {
	left: camRight.clone().negate(), right: camRight.clone(), front: camBack.clone(), back: camBack.clone().negate(),
	top: new THREE.Vector3(0, 1, 0), bottom: new THREE.Vector3(0, -1, 0),
	'top-left': camRight.clone().negate().add(new THREE.Vector3(0, 0.8, 0)), 'top-right': camRight.clone().add(new THREE.Vector3(0, 0.8, 0)),
	'bottom-left': camRight.clone().negate().add(new THREE.Vector3(0, -0.6, 0)), 'bottom-right': camRight.clone().add(new THREE.Vector3(0, -0.6, 0))
};
const ELEV = { below: -25, low: 9, eye: 22, high: 50, overhead: 84 };
const keyDir = (KEYDIR[L.key] ?? KEYDIR['top-left']).clone().normalize();
if (!['top', 'bottom'].includes(L.key)) {
	const e = (ELEV[L.height] ?? 30) * D;
	const flat = keyDir.clone().setY(0).normalize();
	keyDir.copy(flat.multiplyScalar(Math.cos(e)).add(new THREE.Vector3(0, Math.sin(e), 0)));
}
// the subject: the figures' centre, else in front of the camera
const target = new THREE.Vector3();
if (figs.length) { for (const m of figs) target.add(m.root.position); target.divideScalar(figs.length); }
else target.copy(camera.position).addScaledVector(new THREE.Vector3(0, 0, -1).applyQuaternion(camera.quaternion), dist);
const practical = ['candle', 'screen', 'neon', 'fire', 'energy'].includes(L.source);
const reach = Math.max(8, dist * 2.5);
let key;
if (L.source === 'none') key = null;
else if (practical) {
	key = new THREE.PointLight(0xffffff, 260 * (reach / 8) ** 2, reach, 1.6);
	key.position.copy(target).addScaledVector(keyDir, Math.max(1.6, dist * 0.35));
	set.practical?.(key.position); // a candle / screen object at the light
} else {
	key = new THREE.DirectionalLight(0xffffff, 14); // over-driven on purpose: anything facing the key is white (Lambert divides by π)
	key.position.copy(target).addScaledVector(keyDir, 60);
	key.target.position.copy(target);
	scene.add(key.target);
	const span = Math.max(14, dist * 2.2);
	Object.assign(key.shadow.camera, { left: -span, right: span, top: span, bottom: -span, near: 1, far: 200 });
}
if (key) {
	key.castShadow = true;
	key.shadow.mapSize.set(4096, 4096);
	key.shadow.bias = -0.0004;
	scene.add(key);
}
if (L.rim || L.key === 'back' || L.source === 'none') {
	const rim = new THREE.DirectionalLight(0xffffff, L.source === 'none' ? 9 : 7);
	rim.position.copy(target).add(camBack.clone().negate().multiplyScalar(30)).add(new THREE.Vector3(0, 10, 0));
	rim.target.position.copy(target);
	scene.add(rim, rim.target);
}

// --- passes ------------------------------------------------------------------------------------------------------
function targetRT(w, h) {
	const rt = new THREE.WebGLRenderTarget(w, h, { depthBuffer: true });
	rt.depthTexture = new THREE.DepthTexture(w, h);
	return rt;
}
const SIN = new THREE.ShaderMaterial({
	uniforms: {
		tLit: { value: null }, tNorm: { value: null }, tDepth: { value: null }, tAccent: { value: null },
		texel: { value: new THREE.Vector2() }, threshold: { value: 0.32 }, sky: { value: skyWhite ? 1 : 0 },
		accent: { value: new THREE.Color(shot.accent ?? '#c00000') }, near: { value: camera.near }, far: { value: camera.far }, ortho: { value: camera.isOrthographicCamera ? 1 : 0 }
	},
	vertexShader: 'varying vec2 vUv; void main(){ vUv = uv; gl_Position = vec4(position.xy, 0., 1.); }',
	fragmentShader: `
		uniform sampler2D tLit, tNorm, tDepth, tAccent; uniform vec2 texel; uniform float threshold, sky, near, far, ortho; uniform vec3 accent;
		varying vec2 vUv;
		float lin(float d){ if (ortho > .5) return near + d * (far - near); float z = d * 2. - 1.; return 2. * near * far / (far + near - z * (far - near)); }
		float dep(vec2 uv){ return lin(texture2D(tDepth, uv).r); }
		vec3 nor(vec2 uv){ return texture2D(tNorm, uv).rgb * 2. - 1.; }
		void main(){
			float d0 = texture2D(tDepth, vUv).r;
			bool empty = d0 > 0.99999;
			float l = dot(texture2D(tLit, vUv).rgb, vec3(.299, .587, .114));
			float v = empty ? sky : step(threshold, l);
			// ink: depth jumps (silhouettes) + normal creases, 1.5 px wide
			float dc = dep(vUv), e = 0.;
			vec3 nc = nor(vUv);
			for (int i = 0; i < 4; i++) {
				vec2 o = (i == 0 ? vec2(1.,0.) : i == 1 ? vec2(-1.,0.) : i == 2 ? vec2(0.,1.) : vec2(0.,-1.)) * texel * 2.2;
				float dn = dep(vUv + o);
				e = max(e, step(0.06 * dc, abs(dn - dc)));
				if (!empty) e = max(e, step(0.55, 1. - dot(nc, nor(vUv + o))));
			}
			float c = e > .5 ? 1. - v : v;
			vec3 col = vec3(c);
			float a = texture2D(tAccent, vUv).r;
			if (a > .5 && e < .5) col = v > .5 ? accent : accent * .55;
			gl_FragColor = vec4(col, 1.);
		}`,
	depthTest: false, depthWrite: false
});
const quad = new THREE.Mesh(new THREE.PlaneGeometry(2, 2), SIN);
const post = new THREE.Scene(); post.add(quad);
const postCam = new THREE.OrthographicCamera(-1, 1, 1, -1, 0, 1);

function renderView(cam, w, h) {
	renderer.setSize(w, h, false);
	const lit = targetRT(w, h), nrm = targetRT(w, h), acc = targetRT(w, h);
	scene.background = null;
	renderer.setClearColor(0x000000, 1);
	renderer.setRenderTarget(lit); renderer.render(scene, cam);
	scene.overrideMaterial = normalMat; renderer.setRenderTarget(nrm); renderer.render(scene, cam);
	// accent mask: accent objects white, all else black
	scene.overrideMaterial = null;
	const swap = [];
	scene.traverse((o) => { if (o.isMesh) { swap.push([o, o.material]); o.material = o.userData.accent ? accentMat : blackMat; } });
	renderer.setRenderTarget(acc); renderer.render(scene, cam);
	for (const [o, m] of swap) o.material = m;
	Object.assign(SIN.uniforms, {});
	SIN.uniforms.tLit.value = lit.texture; SIN.uniforms.tNorm.value = nrm.texture; SIN.uniforms.tDepth.value = lit.depthTexture; SIN.uniforms.tAccent.value = acc.texture;
	SIN.uniforms.texel.value.set(1 / w, 1 / h);
	SIN.uniforms.near.value = cam.near; SIN.uniforms.far.value = cam.far;
	renderer.setRenderTarget(null); renderer.render(post, postCam);
	lit.dispose(); nrm.dispose(); acc.dispose();
	return renderer.domElement;
}

// --- views: a still, a 360 panorama, a little planet -------------------------------------------------------------
function panorama(pw, ph) {
	const n = Math.max(4, Math.round(pw / 390));
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
		g.drawImage(renderView(cam, sliceW, ph), i * sliceW, 0);
	}
	return pano;
}

let image;
if (shot.format === 'loop360') {
	// figures stand around the camera: their x (0…1) is their bearing in the 360
	for (const [i, m] of figs.entries()) {
		const fig = shot.figures[i];
		const bearing = -(fig.x ?? 0.5) * 2 * Math.PI;
		const r = 4 + (1 - (fig.h ?? 0.3)) * 8;
		m.root.position.set(Math.sin(-bearing) * -r, 0, -Math.cos(bearing) * r);
		m.root.rotation.y = Math.atan2(-m.root.position.x, -m.root.position.z) + Math.PI;
		m.root.scale.setScalar((fig.h ?? 0.3) * 2.2 * r / 4);
		pose(m, shot.figures[i].pose ?? 'stand');
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
	image = renderView(camera, W, H);
}
if (image) out2d.drawImage(image, 0, 0, W, H);

// --- the storyboard marks: id + format top left, air zones as a bracket on the left edge -------------------------
out2d.font = `600 ${11 * K}px ui-monospace, Consolas, monospace`;
const label = `${shot.id} · ${shot.format} ${shot.aspect}${shot.move ? ` · ${shot.move}` : ''}${shot.want ? ` [${shot.want}]` : ''}`;
const tw = out2d.measureText(label).width;
out2d.fillStyle = '#000'; out2d.fillRect(0, 0, tw + 12 * K, 18 * K);
out2d.fillStyle = '#fff'; out2d.fillText(label, 6 * K, 13 * K);
for (const [a, b] of shot.air ?? []) {
	const y0 = (a / 100) * H, y1 = (b / 100) * H;
	out2d.fillStyle = '#888';
	out2d.fillRect(K, y0 + 2 * K, 3 * K, Math.max(2 * K, y1 - y0 - 4 * K));
}
document.title = 'done';
