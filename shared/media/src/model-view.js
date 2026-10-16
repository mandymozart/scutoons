// A 3D model (.glb) shown in the tools (owner 2026-10-14: "a preview of the 3d asset in the asset manager and in the
// slot where it is placed"): a live view (turns slowly, drag to turn) and still thumbnails for the picker's cards.
// three.js loads only when a model is shown (dynamic import). Plain JS – the types are in model-view.d.ts.
// Also the viewer's (`@scutoons/media/model-view`, this file only – FEATURES 10.102): the item view's model and the
// record player's stage (viewer record-stage.js) load three.js through `loadThree`, so it is one chunk for all.

let threeLib = null;
async function three() {
	threeLib ??= Promise.all([import('three'), import('three/addons/loaders/GLTFLoader.js')]).then(([THREE, { GLTFLoader }]) => ({ THREE, GLTFLoader }));
	return threeLib;
}
/** three.js and its GLTFLoader, loaded once on first use */
export const loadThree = three;

/** WebGL can draw here (a throw-away context, asked once) – without it a page shows its 2D picture */
let webgl = null;
export function webglAvailable() {
	if (webgl === null) {
		try {
			const c = document.createElement('canvas');
			const gl = c.getContext('webgl2') || c.getContext('webgl');
			webgl = !!gl;
			gl?.getExtension('WEBGL_lose_context')?.loseContext();
		} catch {
			webgl = false;
		}
	}
	return webgl;
}

/** loads a .glb, centred on the origin, feet on y = 0, scaled to a height of 1 */
async function loadModel(url) {
	const { THREE, GLTFLoader } = await three();
	const gltf = await new GLTFLoader().loadAsync(url);
	const root = gltf.scene;
	root.updateMatrixWorld(true);
	const box = new THREE.Box3().setFromObject(root);
	const size = box.getSize(new THREE.Vector3());
	const s = 1 / Math.max(size.x, size.y, size.z, 1e-6);
	const pivot = new THREE.Group();
	root.position.set(-(box.min.x + box.max.x) / 2, -box.min.y, -(box.min.z + box.max.z) / 2);
	pivot.add(root);
	pivot.scale.setScalar(s);
	let tris = 0, bones = 0;
	root.traverse((o) => {
		if (o.isMesh) tris += (o.geometry.index ? o.geometry.index.count : o.geometry.attributes.position.count) / 3;
		if (o.isBone) bones++;
	});
	return { pivot, height: (size.y * s), info: { tris: Math.round(tris), bones } };
}

function stage(THREE) {
	const scene = new THREE.Scene();
	scene.add(new THREE.HemisphereLight(0xffffff, 0x777777, 2.2));
	const key = new THREE.DirectionalLight(0xffffff, 2);
	key.position.set(2, 3, 3);
	scene.add(key);
	const camera = new THREE.PerspectiveCamera(30, 1, 0.01, 50);
	return { scene, camera };
}
/** the camera looks at the model's middle from the front-right, far enough for a 1-high model */
function frame(camera, height, turn = 0.5) {
	const d = 2.6;
	camera.position.set(Math.sin(turn) * d, height * 0.55, Math.cos(turn) * d);
	camera.lookAt(0, height * 0.5, 0);
}
function disposeTree(obj) {
	obj.traverse((o) => {
		o.geometry?.dispose?.();
		for (const m of [].concat(o.material ?? [])) {
			for (const v of Object.values(m)) if (v?.isTexture) v.dispose();
			m.dispose?.();
		}
	});
}

/** a live view on a canvas: turns slowly until dragged; returns a handle to stop it */
export function mountModelView(canvas, url, opts = {}) {
	let stopped = false, raf = 0, renderer = null, model = null, ro = null;
	const handle = { dispose };
	(async () => {
		try {
			const { THREE } = await three();
			if (stopped) return;
			renderer = new THREE.WebGLRenderer({ canvas, antialias: true, alpha: true });
			renderer.setPixelRatio(Math.min(2, window.devicePixelRatio || 1));
			const { scene, camera } = stage(THREE);
			const m = await loadModel(url);
			if (stopped) { disposeTree(m.pivot); return; }
			model = m.pivot;
			scene.add(model);
			opts.onload?.(m.info);
			let yaw = 0, dragging = false, lastX = 0, idle = true;
			canvas.addEventListener('pointerdown', (e) => { dragging = true; idle = false; lastX = e.clientX; canvas.setPointerCapture(e.pointerId); });
			canvas.addEventListener('pointermove', (e) => { if (!dragging) return; yaw += (e.clientX - lastX) * 0.01; lastX = e.clientX; });
			canvas.addEventListener('pointerup', () => (dragging = false));
			const resize = () => {
				const w = canvas.clientWidth || 1, h = canvas.clientHeight || 1;
				renderer.setSize(w, h, false);
				camera.aspect = w / h;
				camera.updateProjectionMatrix();
			};
			ro = new ResizeObserver(resize);
			ro.observe(canvas);
			resize();
			frame(camera, m.height);
			const tick = () => {
				if (stopped) return;
				if (idle && opts.spin !== false) yaw += 0.006;
				model.rotation.y = yaw;
				renderer.render(scene, camera);
				raf = requestAnimationFrame(tick);
			};
			tick();
		} catch (e) {
			if (!stopped) opts.onerror?.(e);
		}
	})();
	function dispose() {
		stopped = true;
		cancelAnimationFrame(raf);
		ro?.disconnect();
		if (model) disposeTree(model);
		renderer?.dispose();
	}
	return handle;
}

// thumbnails: ONE offscreen renderer for all cards (browsers allow only a few WebGL contexts), one at a time, cached
const thumbs = new Map();
let queue = Promise.resolve();
let shared = null;
/** a still picture of a model (PNG data URL), size × size */
export function modelThumbnail(url, size = 256) {
	const key = `${url}|${size}`;
	if (!thumbs.has(key)) {
		const job = queue.then(async () => {
			const { THREE } = await three();
			shared ??= { renderer: new THREE.WebGLRenderer({ antialias: true, alpha: true, preserveDrawingBuffer: true }), ...stage(THREE) };
			const { renderer, scene, camera } = shared;
			renderer.setSize(size, size, false);
			camera.aspect = 1;
			camera.updateProjectionMatrix();
			const m = await loadModel(url);
			scene.add(m.pivot);
			frame(camera, m.height);
			renderer.render(scene, camera);
			const png = renderer.domElement.toDataURL('image/png');
			scene.remove(m.pivot);
			disposeTree(m.pivot);
			return png;
		});
		queue = job.catch(() => undefined); // a failed model doesn't stop the next ones
		thumbs.set(key, job.catch((e) => { thumbs.delete(key); throw e; }));
	}
	return thumbs.get(key);
}
