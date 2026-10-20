// A place's 3D room (agents/research/LOCATION-SCENES.md, issue #74) – the dev lab's renderer, plain JS like
// record-stage.js. The room is a static diorama: the `.glb`'s `view` camera looks in, a drag / the phone's tilt pivots it
// a little (±18° yaw, ±6° pitch), wheel / pinch moves it in. The room's light is baked (vertex colours, unlit), so the
// scene has no lights; the things in it (pickups, residents) get one soft light. Spots = the model's `spot_<id>` nodes.
import { loadThree } from '@scutoons/media/model-view';

const YAW = (18 * Math.PI) / 180;
const PITCH = (6 * Math.PI) / 180;

/** the room in `canvas` – the types: room-stage.d.ts */
export async function mountRoom(canvas, opts) {
	const t0 = performance.now();
	const { THREE, GLTFLoader } = await loadThree();
	const { MeshoptDecoder } = await import('three/addons/libs/meshopt_decoder.module.js');
	const loader = new GLTFLoader().setMeshoptDecoder(MeshoptDecoder);
	const tThree = performance.now();

	const res = await fetch(opts.url);
	if (!res.ok) throw new Error(`${res.status} ${opts.url}`);
	const bytes = await res.arrayBuffer();
	const tFetch = performance.now();
	const gltf = await loader.parseAsync(bytes, opts.url.replace(/[^/]*$/, ''));
	const tParse = performance.now();

	const renderer = new THREE.WebGLRenderer({
		canvas,
		antialias: (window.devicePixelRatio || 1) < 2,
		powerPreference: 'high-performance'
	});
	renderer.setPixelRatio(Math.min(opts.maxPixelRatio ?? 1.5, window.devicePixelRatio || 1));
	const scene = new THREE.Scene();
	scene.background = new THREE.Color(0x000000);
	scene.add(gltf.scene);
	gltf.scene.updateMatrixWorld(true);

	// the room's own camera (`view`), else a look from the front at its box
	let camera = gltf.cameras[0]?.clone();
	const view = gltf.scene.getObjectByName('view') ?? gltf.cameras[0]?.parent;
	if (camera && view) {
		camera.position.setFromMatrixPosition(gltf.cameras[0].matrixWorld);
		camera.quaternion.setFromRotationMatrix(
			new THREE.Matrix4().extractRotation(gltf.cameras[0].matrixWorld)
		);
	} else {
		camera = new THREE.PerspectiveCamera(55, 1, 0.05, 200);
		const box = new THREE.Box3().setFromObject(gltf.scene),
			c = box.getCenter(new THREE.Vector3());
		camera.position.set(c.x, c.y, box.max.z + box.getSize(new THREE.Vector3()).z * 0.6);
		camera.lookAt(c);
	}
	camera.near = 0.05;
	const baseFov = camera.fov;
	camera.far = 300;
	const home = { pos: camera.position.clone(), quat: camera.quaternion.clone() };
	const forward = new THREE.Vector3(0, 0, -1).applyQuaternion(home.quat);

	// things at the spots: a pickup (its 3D model, else its icon as a standing card) or a resident (its model)
	scene.add(new THREE.HemisphereLight(0xfff2e0, 0x202028, 2.4));
	const things = new THREE.Group();
	scene.add(things);
	const textures = new THREE.TextureLoader();
	for (const t of opts.things) {
		const spot = gltf.scene.getObjectByName('spot_' + t.spot);
		if (!spot) continue;
		let obj;
		if (t.model) {
			obj = (await loader.loadAsync(t.model)).scene;
			const box = new THREE.Box3().setFromObject(obj),
				size = box.getSize(new THREE.Vector3());
			if (t.height) obj.scale.setScalar(t.height / Math.max(size.y, 1e-6));
			// feet on the spot
			const b2 = new THREE.Box3().setFromObject(obj);
			obj.position.y -= b2.min.y;
			const pivot = new THREE.Group();
			pivot.add(obj);
			obj = pivot;
		} else {
			const map = await textures.loadAsync(t.icon);
			map.colorSpace = THREE.SRGBColorSpace;
			obj = new THREE.Sprite(new THREE.SpriteMaterial({ map }));
			const h = t.height ?? 0.3;
			obj.scale.set(h * (map.image.width / map.image.height), h, 1);
			obj.position.y = h / 2;
		}
		const holder = new THREE.Group();
		holder.position.setFromMatrixPosition(spot.matrixWorld);
		// everything turns its front (+Z) to the camera: residents look at the reader, pickups show their face
		holder.lookAt(home.pos.x, holder.position.y, home.pos.z);
		holder.add(obj);
		holder.userData.thing = t.id;
		holder.userData.kind = t.kind;
		things.add(holder);
	}
	const tReady = performance.now();

	// look around: drag (mouse / one finger) or the phone's tilt; wheel / pinch moves in along the view
	let yaw = 0,
		pitch = 0,
		zoom = 0,
		aimYaw = 0,
		aimPitch = 0,
		aimZoom = 0;
	const clamp = (v, a) => Math.max(-a, Math.min(a, v));
	const ptr = new Map();
	let drag = null,
		pinch = null,
		moved = false;
	const down = (e) => {
		ptr.set(e.pointerId, { x: e.clientX, y: e.clientY });
		canvas.setPointerCapture(e.pointerId);
		moved = false;
		if (ptr.size === 1) drag = { x: e.clientX, y: e.clientY, yaw: aimYaw, pitch: aimPitch };
		if (ptr.size === 2) {
			const [a, b] = [...ptr.values()];
			pinch = { d: Math.hypot(a.x - b.x, a.y - b.y), zoom: aimZoom };
			drag = null;
		}
	};
	const move = (e) => {
		if (!ptr.has(e.pointerId)) return;
		ptr.set(e.pointerId, { x: e.clientX, y: e.clientY });
		const w = canvas.clientWidth;
		if (pinch && ptr.size === 2) {
			const [a, b] = [...ptr.values()];
			aimZoom = Math.max(
				0,
				Math.min(
					opts.maxZoom ?? 3,
					pinch.zoom + ((Math.hypot(a.x - b.x, a.y - b.y) - pinch.d) / w) * 4
				)
			);
			moved = true;
		} else if (drag) {
			const dx = e.clientX - drag.x,
				dy = e.clientY - drag.y;
			if (Math.hypot(dx, dy) > 6) moved = true;
			aimYaw = clamp(drag.yaw + (dx / w) * YAW * 2, YAW);
			aimPitch = clamp(drag.pitch + (dy / w) * PITCH * 2, PITCH);
		}
	};
	const ray = new THREE.Raycaster(),
		ndc = new THREE.Vector2();
	const up = (e) => {
		ptr.delete(e.pointerId);
		if (ptr.size < 2) pinch = null;
		if (ptr.size === 0) drag = null;
		if (moved || e.type !== 'pointerup') return;
		const r = canvas.getBoundingClientRect();
		ndc.set(((e.clientX - r.left) / r.width) * 2 - 1, -((e.clientY - r.top) / r.height) * 2 + 1);
		ray.setFromCamera(ndc, camera);
		const hit = ray.intersectObjects([things, gltf.scene], true)[0];
		if (!hit) return;
		let o = hit.object;
		while (o && !o.userData.thing) o = o.parent;
		opts.ontap?.({
			thing: o?.userData.thing,
			point: hit.point.toArray().map((v) => +v.toFixed(2))
		});
	};
	const wheel = (e) => {
		e.preventDefault();
		aimZoom = Math.max(0, Math.min(opts.maxZoom ?? 3, aimZoom - e.deltaY * 0.002));
	};
	let tilt = null;
	const orient = (e) => {
		if (e.gamma == null) return;
		tilt ??= { g: e.gamma, b: e.beta };
		const dz = (v) => (Math.abs(v) < 3 ? 0 : v - Math.sign(v) * 3);
		aimYaw = clamp((-dz(e.gamma - tilt.g) * Math.PI) / 180, YAW);
		aimPitch = clamp((dz(e.beta - tilt.b) * Math.PI) / 180 / 2, PITCH);
	};
	canvas.addEventListener('pointerdown', down);
	canvas.addEventListener('pointermove', move);
	canvas.addEventListener('pointerup', up);
	canvas.addEventListener('pointercancel', up);
	canvas.addEventListener('wheel', wheel, { passive: false });
	window.addEventListener('deviceorientation', orient);

	// the loop, with a frame-time meter
	const qYaw = new THREE.Quaternion(),
		qPitch = new THREE.Quaternion(),
		Y = new THREE.Vector3(0, 1, 0),
		X = new THREE.Vector3(1, 0, 0);
	let frames = 0,
		since = performance.now(),
		fps = 0,
		raf = 0,
		gone = false;
	const resize = () => {
		const w = canvas.clientWidth,
			h = canvas.clientHeight;
		renderer.setSize(w, h, false);
		camera.aspect = w / h;
		// a phone in portrait keeps the room's width: at least 60° across (at most 90° high)
		const minH = (60 * Math.PI) / 180;
		camera.fov = Math.min(90, Math.max(baseFov, ((2 * Math.atan(Math.tan(minH / 2) / camera.aspect)) * 180) / Math.PI));
		camera.updateProjectionMatrix();
	};
	const ro = new ResizeObserver(resize);
	ro.observe(canvas);
	resize();
	const stats = (extra = {}) => ({
		ms: {
			three: Math.round(tThree - t0),
			fetch: Math.round(tFetch - tThree),
			parse: Math.round(tParse - tFetch),
			things: Math.round(tReady - tParse)
		},
		mb: +(bytes.byteLength / 1e6).toFixed(2),
		tris: renderer.info.render.triangles,
		calls: renderer.info.render.calls,
		textures: renderer.info.memory.textures,
		pixelRatio: renderer.getPixelRatio(),
		fps,
		...extra
	});
	const frame = () => {
		if (gone) return;
		raf = requestAnimationFrame(frame);
		yaw += (aimYaw - yaw) * 0.15;
		pitch += (aimPitch - pitch) * 0.15;
		zoom += (aimZoom - zoom) * 0.15;
		qYaw.setFromAxisAngle(Y, yaw);
		qPitch.setFromAxisAngle(X, pitch);
		camera.quaternion.copy(qYaw).multiply(home.quat).multiply(qPitch);
		camera.position.copy(home.pos).addScaledVector(forward, zoom);
		renderer.render(scene, camera);
		frames++;
		const now = performance.now();
		if (now - since > 1000) {
			fps = Math.round((frames * 1000) / (now - since));
			frames = 0;
			since = now;
			opts.onstats?.(stats());
		}
	};
	frame();
	opts.onstats?.(stats());

	return {
		/** the lab's handle (window.__room): the scene, the camera, three */
		debug: { scene, camera, home, THREE },
		/** a picked-up thing leaves the room */
		take(id) {
			const o = things.children.find((c) => c.userData.thing === id);
			if (o) things.remove(o);
		},
		/** asks iOS for the tilt (on a tap) */
		async allowTilt() {
			const D = /** @type {any} */ (window.DeviceOrientationEvent);
			if (D?.requestPermission) await D.requestPermission().catch(() => {});
		},
		dispose() {
			gone = true;
			cancelAnimationFrame(raf);
			ro.disconnect();
			window.removeEventListener('deviceorientation', orient);
			canvas.removeEventListener('pointerdown', down);
			canvas.removeEventListener('pointermove', move);
			canvas.removeEventListener('pointerup', up);
			canvas.removeEventListener('pointercancel', up);
			canvas.removeEventListener('wheel', wheel);
			scene.traverse((o) => {
				o.geometry?.dispose?.();
				for (const m of [].concat(o.material ?? [])) {
					for (const v of Object.values(m)) if (v?.isTexture) v.dispose();
					m.dispose?.();
				}
			});
			renderer.dispose();
		}
	};
}
