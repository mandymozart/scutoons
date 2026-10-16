// The record player in 3D (FEATURES 10.102 – owner 2026-10-16: "The item should show sleeve with record inside. And on
// starting to play it is pulled out and put on the record player"): the LP item's model (its sleeve with the record
// inside – tools/vinyl3d/build.py `.set.glb`, nodes "sleeve" and "record") stands behind a simple turntable built here
// (plinth, platter, spindle, tonearm). On play the record slides out of the sleeve's open edge (+X), turns flat and lies
// down on the platter, side A up (side B: the other way up – side A faces +Z in the model); it turns at 33⅓ rpm while
// playing, a pause lets it run down; a side change on the platter lifts it, flips it, puts it back. The camera moves
// from the sleeve to the deck. One renderer per stage, three.js loads with the first stage (`loadThree`); `dispose`
// frees everything. Plain JS like model-view.js (no @types/three) – the types are in record-stage.d.ts.
// Callers show their 2D deck without WebGL or with reduced motion (record-card.svelte).
import { loadThree } from '@scutoons/media/model-view';

const RPM = 100 / 3;
const OMEGA = (RPM / 60) * 2 * Math.PI; // rad / s, clockwise from above = negative around +Y
const R = 0.151; // the record's radius (m)
const T = 0.001; // half its thickness
// the deck: plinth, platter, where the record lies
const PLINTH = { w: 0.46, h: 0.07, d: 0.36, x: 0.13, z: 0.06 };
const PLATTER = { x: 0.1, z: 0.06, r: 0.152, top: PLINTH.h + 0.016 };
const ON_PLATTER_Y = PLATTER.top + T;
// the tonearm: pivot behind the platter on the right, an arm of ARM_LEN pointing +Z at rest
const PIVOT = { x: PLATTER.x + 0.19, z: PLATTER.z - 0.11 };
const ARM_LEN = 0.235;
const ARM_Y = PLINTH.h + 0.028;
// the sleeve stands behind the deck on the left, its open edge towards it
const SLEEVE = { x: -0.3, z: -0.2 };
const OUT_X = SLEEVE.x + 0.33; // the record's centre when it is all the way out
// the camera: the sleeve close up → the deck
const VIEWS = {
	sleeve: { pos: [SLEEVE.x + 0.2, 0.32, SLEEVE.z + 1.0], look: [SLEEVE.x + 0.15, 0.14, SLEEVE.z + 0.05] },
	deck: { pos: [0.02, 0.62, 0.8], look: [0.02, 0.06, -0.02] }
};
/** the camera ride's whole arc around the stage (owner: "about 75 degrees max") */
const ARC = (75 * Math.PI) / 180;
// slow enough to watch (owner 2026-10-16: "Ich sehe noch nicht, dass die Schallplatte aus der Hülle rausfährt")
const SLIDE = 1.6; // s
const CARRY = 1.6;
const DROP = 0.6;
const FLIP = 1.0;

const ease = (t) => (t <= 0 ? 0 : t >= 1 ? 1 : t * t * (3 - 2 * t));
const lerp = (a, b, t) => a + (b - a) * t;

/** the tonearm's turn (rad, 0 = rest pointing +Z) that puts the needle at radius r of the platter */
function needleTurn(r) {
	const dx = PLATTER.x - PIVOT.x, dz = PLATTER.z - PIVOT.z;
	const dist = (a) => Math.hypot(PIVOT.x + ARM_LEN * Math.sin(a) - PLATTER.x, PIVOT.z + ARM_LEN * Math.cos(a) - PLATTER.z);
	let lo = Math.atan2(dx, dz), hi = 0; // dist grows from the centre-pointing turn up to rest
	for (let i = 0; i < 30; i++) {
		const mid = (lo + hi) / 2;
		if (dist(mid) < r) lo = mid;
		else hi = mid;
	}
	return (lo + hi) / 2;
}
const ARM_OUTER = needleTurn(0.146);
const ARM_INNER = needleTurn(0.064);
/** record-deck's arm angle (deg: ≤ −20 rest, 0 = a song's start, 22 = its end) → the 3D arm's turn */
function armTurn(deg) {
	if (deg < -10) return 0;
	return lerp(ARM_OUTER, ARM_INNER, Math.min(1, Math.max(0, deg / 22)));
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

function buildDeck(THREE) {
	const deck = new THREE.Group();
	const mat = (color, roughness = 0.6, metalness = 0) => new THREE.MeshStandardMaterial({ color, roughness, metalness });
	const plinth = new THREE.Mesh(new THREE.BoxGeometry(PLINTH.w, PLINTH.h, PLINTH.d), mat(0x2b2724, 0.55));
	plinth.position.set(PLINTH.x, PLINTH.h / 2, PLINTH.z);
	const platter = new THREE.Mesh(new THREE.CylinderGeometry(PLATTER.r, PLATTER.r, 0.014, 96), mat(0xa8a8a8, 0.35, 0.85));
	platter.position.set(PLATTER.x, PLATTER.top - 0.009, PLATTER.z);
	const felt = new THREE.Mesh(new THREE.CylinderGeometry(PLATTER.r - 0.004, PLATTER.r - 0.004, 0.002, 96), mat(0x1c1c1c, 0.95));
	felt.position.set(PLATTER.x, PLATTER.top - 0.001, PLATTER.z);
	const spindle = new THREE.Mesh(new THREE.CylinderGeometry(0.0034, 0.0034, 0.022, 16), mat(0xdddddd, 0.25, 1));
	spindle.position.set(PLATTER.x, PLATTER.top + 0.008, PLATTER.z);
	const base = new THREE.Mesh(new THREE.CylinderGeometry(0.024, 0.028, 0.03, 32), mat(0x8c8c8c, 0.4, 0.8));
	base.position.set(PIVOT.x, PLINTH.h + 0.015, PIVOT.z);
	// the arm: turns around Y at the pivot, lifts around X
	const arm = new THREE.Group();
	arm.position.set(PIVOT.x, ARM_Y, PIVOT.z);
	const tube = new THREE.Mesh(new THREE.CylinderGeometry(0.0035, 0.0035, ARM_LEN, 12), mat(0xd6d6d6, 0.3, 0.9));
	tube.rotation.x = Math.PI / 2;
	tube.position.z = ARM_LEN / 2;
	const head = new THREE.Mesh(new THREE.BoxGeometry(0.018, 0.008, 0.03), mat(0x1a1a1a, 0.5));
	head.position.set(0, -0.004, ARM_LEN);
	const weight = new THREE.Mesh(new THREE.CylinderGeometry(0.012, 0.012, 0.03, 20), mat(0x6e6e6e, 0.4, 0.8));
	weight.rotation.x = Math.PI / 2;
	weight.position.z = -0.03;
	arm.add(tube, head, weight);
	deck.add(plinth, platter, felt, spindle, base, arm);
	return { deck, arm };
}

/**
 * Mounts the stage on a canvas; `url` = the item's model (.glb, CDN URL). The state comes in by `update` (any time,
 * also before the model is there).
 */
export function mountRecordStage(canvas, url, opts = {}) {
	let stopped = false, raf = 0, renderer = null, scene = null, env = null, ro = null;
	let onDown = null, onMove = null, onUp = null, onWheel = null;
	const want = { playing: false, side: 'a', arm: -28, lifted: false };
	// the reader's look around (owner 2026-10-16: "camera pivoting … zoom in a bit and look at the record cover … smoothly pan
	// back and zoom out on release or on the next interaction with the track list"; "pivot = moving the camera in a half
	// circle around the record player, a camera ride, about 75 degrees max"): a sideways drag rides the camera on a level
	// arc around the stage's centre (75° in all, ±37.5°), pinch / wheel zooms towards the sleeve; releasing – or
	// `resetView()` – eases back to the stage's own view
	const user = { yaw: 0, pitch: 0, zoom: 0 };
	/** spring velocities: a release snaps back with a little overshoot (owner: "snappy easing … snap if released") */
	const vel = { yaw: 0, pitch: 0, zoom: 0 };
	const aim = { yaw: 0, pitch: 0, zoom: 0 };
	let held = false;
	const resetView = () => {
		held = false;
		aim.yaw = aim.pitch = aim.zoom = 0;
	};
	const handle = { update: (s) => Object.assign(want, s), dispose, resetView };

	(async () => {
		try {
			const { THREE, GLTFLoader } = await loadThree();
			const { RoomEnvironment } = await import('three/addons/environments/RoomEnvironment.js');
			if (stopped) return;
			renderer = new THREE.WebGLRenderer({ canvas, antialias: true, alpha: true });
			renderer.setPixelRatio(Math.min(2, window.devicePixelRatio || 1));
			scene = new THREE.Scene();
			const pmrem = new THREE.PMREMGenerator(renderer);
			env = pmrem.fromScene(new RoomEnvironment(), 0.04).texture;
			pmrem.dispose();
			scene.environment = env;
			scene.environmentIntensity = 0.6;
			scene.add(new THREE.HemisphereLight(0xffffff, 0x666666, 0.5));
			const key = new THREE.DirectionalLight(0xffffff, 1.2);
			key.position.set(0.6, 1.2, 0.9);
			scene.add(key);
			const camera = new THREE.PerspectiveCamera(32, 1, 0.01, 20);

			const gltf = await new GLTFLoader().loadAsync(url);
			if (stopped) return disposeTree(gltf.scene);
			const set = gltf.scene;
			set.position.set(SLEEVE.x, 0, SLEEVE.z);
			scene.add(set);
			set.updateMatrixWorld(true);
			// the record: the node "record" (a model of the record alone: the whole scene), taken out of the set
			let record = set.getObjectByName('record');
			const hasSleeve = !!set.getObjectByName('sleeve');
			const inSleeve = new THREE.Vector3(SLEEVE.x, R, SLEEVE.z);
			if (record) record.getWorldPosition(inSleeve);
			else record = set;
			const carrier = new THREE.Group(); // position + spin around Y
			const tilt = new THREE.Group(); // 0 = standing (A faces +Z), −π/2 = A up, +π/2 = B up
			carrier.add(tilt);
			scene.add(carrier);
			if (record !== set) record.removeFromParent();
			else set.position.set(0, 0, 0);
			record.position.set(0, record === set ? -R : 0, 0);
			tilt.add(record);
			carrier.position.copy(inSleeve);
			const { deck, arm } = buildDeck(THREE);
			scene.add(deck);
			opts.onload?.();

			// the record's state: in the sleeve (or standing by it), on its way, on the platter, flipping
			let state = 'sleeve', t0 = 0, lies = want.side, spin = 0, armTurnNow = 0, lift = 0.06, view = 0;
			const from = { y: 0, tilt: 0 };
			const flat = (side) => (side === 'b' ? Math.PI / 2 : -Math.PI / 2);
			const startAt = performance.now() + 1200; // the card has opened and the sleeve is seen first
			let last = performance.now();

			// the stage fills the card's top as its background (owner 2026-10-16: "covering the entire background of the card
			// so it doesn't get cut off"): a narrow, tall canvas widens the view so the 4:3 framing always fits sideways, and
			// `opts.lift` (0…1 of the height) moves the scene up – the controls lie over the canvas's lower part
			const resize = () => {
				const w = canvas.clientWidth || 1, h = canvas.clientHeight || 1;
				renderer.setSize(w, h, false);
				const aspect = w / h;
				const shown = 1 - (opts.lift ?? 0); // the part of the height the scene should fill
				camera.aspect = aspect;
				const base = (32 * Math.PI) / 180;
				// vertical field so that the 4:3 width fits in w and the scene fits in the shown height
				const fitWidth = 2 * Math.atan((Math.tan(base / 2) * 1.8) / aspect);
				camera.fov = (Math.max(base / shown, fitWidth) * 180) / Math.PI;
				const off = ((opts.lift ?? 0) / 2) * h;
				if (off) camera.setViewOffset(w, h, 0, off, w, h);
				else camera.clearViewOffset();
				camera.updateProjectionMatrix();
			};
			// pointers: one = pivot, two = pinch zoom; the wheel zooms and eases back after a moment without wheeling
			const pointers = new Map();
			let pinchFrom = 0, zoomFrom = 0, wheelTimer = 0;
			const clamp = (v, lo, hi) => Math.min(hi, Math.max(lo, v));
			const dist = () => {
				const [a, b] = [...pointers.values()];
				return Math.hypot(a.x - b.x, a.y - b.y) || 1;
			};
			onDown = (e) => {
				try {
					canvas.setPointerCapture?.(e.pointerId);
				} catch {
					/* a pointer the browser doesn't track (synthetic): still follow it */
				}
				pointers.set(e.pointerId, { x: e.clientX, y: e.clientY });
				held = true;
				if (pointers.size === 2) {
					pinchFrom = dist();
					zoomFrom = aim.zoom;
				}
			};
			onMove = (e) => {
				const p = pointers.get(e.pointerId);
				if (!p) return;
				const dx = e.clientX - p.x, dy = e.clientY - p.y;
				p.x = e.clientX;
				p.y = e.clientY;
				if (pointers.size === 1) {
					// the arc: a full drag across the stage ≈ the whole 75°
					aim.yaw = clamp(aim.yaw - (dx / (canvas.clientWidth || 300)) * ARC, -ARC / 2, ARC / 2);
				} else if (pointers.size === 2) aim.zoom = clamp(zoomFrom + (dist() / pinchFrom - 1) * 0.9, 0, 0.8);
			};
			onUp = (e) => {
				pointers.delete(e.pointerId);
				if (pointers.size === 0) resetView();
			};
			onWheel = (e) => {
				e.preventDefault();
				held = true;
				aim.zoom = clamp(aim.zoom - e.deltaY * 0.0012, 0, 0.8);
				clearTimeout(wheelTimer);
				wheelTimer = setTimeout(resetView, 1400);
			};
			canvas.addEventListener('pointerdown', onDown);
			canvas.addEventListener('pointermove', onMove);
			canvas.addEventListener('pointerup', onUp);
			canvas.addEventListener('pointercancel', onUp);
			canvas.addEventListener('wheel', onWheel, { passive: false });
			ro = new ResizeObserver(resize);
			ro.observe(canvas);
			resize();
			const look = new THREE.Vector3();
			const cover = new THREE.Vector3(...VIEWS.sleeve.look);
			const offset = new THREE.Vector3();
			const yAxis = new THREE.Vector3(0, 1, 0);
			const side = new THREE.Vector3();

			const tick = (now) => {
				if (stopped) return;
				const dt = Math.min(0.05, (now - last) / 1000);
				last = now;
				const s = (now - t0) / 1000;
				if (state === 'sleeve' && want.playing && now >= startAt) {
					state = 'out';
					t0 = now;
					lies = want.side;
				} else if (state === 'out') {
					// slide out of the open edge → carry over the platter while turning flat → lay it down
					const slide = ease(s / SLIDE), carry = ease((s - SLIDE) / CARRY), drop = ease((s - SLIDE - CARRY) / DROP);
					const hover = ON_PLATTER_Y + 0.16;
					carrier.position.x = carry > 0 ? lerp(OUT_X, PLATTER.x, carry) : lerp(inSleeve.x, OUT_X, slide);
					carrier.position.z = lerp(inSleeve.z, PLATTER.z, carry);
					carrier.position.y = drop > 0 ? lerp(hover, ON_PLATTER_Y, drop) : lerp(inSleeve.y, hover, Math.min(1, carry * 1.6));
					tilt.rotation.x = lerp(0, flat(lies), carry);
					view = Math.max(slide * 0.35, 0.35 + 0.65 * carry);
					if (s >= SLIDE + CARRY + DROP) state = 'deck';
				} else if (state === 'deck' && want.side !== lies && Math.abs(spin) < 0.4) {
					state = 'flip';
					t0 = now;
					from.tilt = tilt.rotation.x;
					lies = want.side;
				} else if (state === 'flip') {
					// lift, turn over through standing, lay it down again
					const k = ease(s / FLIP);
					carrier.position.y = ON_PLATTER_Y + Math.sin(Math.PI * k) * (R + 0.03);
					tilt.rotation.x = lerp(from.tilt, flat(lies), k);
					if (s >= FLIP) state = 'deck';
				}
				// 33⅓ rpm while playing on the platter; runs up / down in about a second – a side change brakes it first
				const turning = (state === 'deck' && want.side !== lies) || state === 'flip';
				const target = state === 'deck' && want.playing && !turning ? -OMEGA : 0;
				spin += (target - spin) * Math.min(1, dt * (turning ? 6 : 2.5));
				if (Math.abs(spin) < 1e-3 && target === 0) spin = 0;
				carrier.rotation.y += spin * dt;
				// the arm: on the record while it plays on the platter, else at rest; lifted between songs
				const onRecord = state === 'deck' && want.playing && !turning;
				armTurnNow += ((onRecord ? armTurn(want.arm) : 0) - armTurnNow) * Math.min(1, dt * 3);
				lift += ((onRecord && !want.lifted ? 0 : -0.07) - lift) * Math.min(1, dt * 6);
				arm.rotation.set(lift, armTurnNow, 0, 'YXZ');
				// the camera: sleeve → deck as the record comes out (no sleeve: the deck from the start)
				const v = hasSleeve ? ease(view) : 1;
				camera.position.set(...VIEWS.sleeve.pos.map((p, i) => lerp(p, VIEWS.deck.pos[i], v)));
				look.set(...VIEWS.sleeve.look.map((p, i) => lerp(p, VIEWS.deck.look[i], v)));
				// the reader's look around: while held it follows the finger closely; released, an under-damped spring snaps it
				// home (≈ 0.35 s, a small overshoot, settled in about half a second)
				for (const key of ['yaw', 'pitch', 'zoom']) {
					if (held) {
						user[key] += (aim[key] - user[key]) * Math.min(1, dt * 14);
						vel[key] = 0;
					} else {
						const stiffness = 170, damping = 15;
						vel[key] += ((aim[key] - user[key]) * stiffness - vel[key] * damping) * dt;
						user[key] += vel[key] * dt;
						if (Math.abs(aim[key] - user[key]) < 1e-4 && Math.abs(vel[key]) < 1e-3) {
							user[key] = aim[key];
							vel[key] = 0;
						}
					}
				}
				if (user.yaw || user.pitch || user.zoom) {
					// zooming turns the look towards the cover (the sleeve) and comes closer; drag orbits around that point
					look.lerp(cover, Math.min(1, user.zoom * 1.1));
					offset.copy(camera.position).sub(look);
					offset.applyAxisAngle(yAxis, user.yaw);
					side.crossVectors(offset, yAxis).normalize();
					offset.applyAxisAngle(side, -user.pitch);
					offset.multiplyScalar(1 - user.zoom * 0.72);
					camera.position.copy(look).add(offset);
				}
				camera.lookAt(look);
				renderer.render(scene, camera);
				raf = requestAnimationFrame(tick);
			};
			raf = requestAnimationFrame(tick);
		} catch (e) {
			if (!stopped) opts.onerror?.(e);
		}
	})();

	function dispose() {
		stopped = true;
		cancelAnimationFrame(raf);
		ro?.disconnect();
		if (onDown) {
			canvas.removeEventListener('pointerdown', onDown);
			canvas.removeEventListener('pointermove', onMove);
			canvas.removeEventListener('pointerup', onUp);
			canvas.removeEventListener('pointercancel', onUp);
			canvas.removeEventListener('wheel', onWheel);
		}
		if (scene) disposeTree(scene);
		env?.dispose();
		renderer?.dispose();
	}
	return handle;
}
