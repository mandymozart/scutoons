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

/** a model: a .glb's URL, or a builder (THREE) => Promise<Object3D> – a shared base model built in code (vinyl-model.js) */
export async function loadSource(source) {
	const { THREE, GLTFLoader } = await three();
	return typeof source === 'function' ? source(THREE) : (await new GLTFLoader().loadAsync(source)).scene;
}

/** loads a model, centred on the origin, feet on y = 0, scaled to a height of 1 */
async function loadModel(source) {
	const { THREE } = await three();
	const root = await loadSource(source);
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
function frame(camera, height, turn = 0.5, d = 2.6) {
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

/** pulling the record out of its sleeve in the live view (null: the model has no record in a sleeve) */
function recordPull(THREE, model, camera, canvas) {
	const record = model.getObjectByName('record');
	if (!record || !model.getObjectByName('sleeve')) return null;
	model.updateMatrixWorld(true);
	const home = record.position.clone();
	// the set shifts back by half the way out: sleeve and record stay in the middle of the view
	const set = record.parent, setHome = set.position.clone();
	// all the way out: the record's diameter beyond where it stands (it peeks a little already), in its parent's units
	const size = new THREE.Box3().setFromObject(record).getSize(new THREE.Vector3());
	const scale = record.parent.getWorldScale(new THREE.Vector3());
	const max = (size.x / scale.x) * 0.92;
	const ray = new THREE.Raycaster(), ndc = new THREE.Vector2(), plane = new THREE.Plane(), at = new THREE.Vector3();
	let out = 0, aim = 0, grab = null;
	const point = (e) => {
		const r = canvas.getBoundingClientRect();
		ndc.set(((e.clientX - r.left) / r.width) * 2 - 1, -((e.clientY - r.top) / r.height) * 2 + 1);
		ray.setFromCamera(ndc, camera);
	};
	const along = () => (ray.ray.intersectPlane(plane, at) ? record.parent.worldToLocal(at).x : null);
	return {
		start(e) {
			point(e);
			// the record must be what the finger is on (not the sleeve in front of it)
			const hit = ray.intersectObject(model, true)[0];
			let o = hit?.object;
			while (o && o !== record) o = o.parent;
			if (!o) return;
			// the plane of the disc through the grabbed point: the finger moves the record in it
			plane.setFromNormalAndCoplanarPoint(new THREE.Vector3(0, 0, 1).transformDirection(record.parent.matrixWorld), hit.point);
			const x = along();
			if (x !== null) grab = x - out;
		},
		move(e) {
			if (grab === null) return false;
			point(e);
			const x = along();
			if (x !== null) out = aim = Math.min(max, Math.max(0, x - grab));
			return true;
		},
		end() {
			if (grab === null) return;
			grab = null;
			aim = out < max / 2 ? 0 : max;
		},
		/** moves the record; true while it is (or was) out */
		tick() {
			if (grab === null) out += (aim - out) * 0.18;
			if (Math.abs(out) < 1e-5 && record.position.x === home.x) return false;
			record.position.x = home.x + out;
			set.position.x = setHome.x - (out / 2) * set.scale.x;
			return true;
		},
		/** 0 = in its sleeve … 1 = all the way out */
		progress: () => out / max
	};
}

/** a live view on a canvas: turns slowly until dragged; returns a handle to stop it. `source`: a .glb URL or a builder */
export function mountModelView(canvas, source, opts = {}) {
	let stopped = false, raf = 0, renderer = null, model = null, ro = null;
	const handle = { dispose };
	(async () => {
		try {
			const { THREE } = await three();
			if (stopped) return;
			renderer = new THREE.WebGLRenderer({ canvas, antialias: true, alpha: true });
			renderer.setPixelRatio(Math.min(2, window.devicePixelRatio || 1));
			const { scene, camera } = stage(THREE);
			const m = await loadModel(source);
			if (stopped) { disposeTree(m.pivot); return; }
			model = m.pivot;
			scene.add(model);
			opts.onload?.(m.info);
			let yaw = 0, dragging = false, lastX = 0, idle = true;
			// a record in its sleeve (nodes "record" + "sleeve", the shared record model / a .set.glb) can be pulled out
			// (owner 2026-10-16: "if one grabs the actual vinyl it can be dragged out"): a drag that starts on the record
			// slides it along the sleeve's open edge (+X), following the finger, at most all the way out; let go less than
			// half out and it springs back in, more and it slides all the way out and stays. A drag elsewhere turns the model.
			const pull = recordPull(THREE, model, camera, canvas);
			canvas.addEventListener('pointerdown', (e) => {
				dragging = true;
				idle = false;
				lastX = e.clientX;
				pull?.start(e);
				try {
					canvas.setPointerCapture(e.pointerId);
				} catch {
					/* a pointer the browser doesn't track (synthetic): still follow it */
				}
			});
			canvas.addEventListener('pointermove', (e) => {
				if (!dragging) return;
				if (pull?.move(e)) return;
				yaw += (e.clientX - lastX) * 0.01;
				lastX = e.clientX;
			});
			const up = () => {
				dragging = false;
				pull?.end();
			};
			canvas.addEventListener('pointerup', up);
			canvas.addEventListener('pointercancel', up);
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
				// a record pulled out: the camera steps back so it stays in the picture
				if (pull?.tick()) frame(camera, m.height, 0.5, 2.6 * (1 + 0.55 * pull.progress()));
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
