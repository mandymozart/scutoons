// The record player in 3D (FEATURES 10.102 – owner 2026-10-16: "The item should show sleeve with record inside. And on
// starting to play it is pulled out and put on the record player"): the LP item's model (its sleeve with the record
// inside – tools/vinyl3d/build.py `.set.glb`, nodes "sleeve" and "record") stands behind a simple turntable built here
// (plinth, platter, spindle, tonearm). On play the record slides out of the sleeve's open edge (+X), turns flat and lies
// down on the platter, side A up (side B: the other way up – side A faces +Z in the model); it turns at 33⅓ rpm while
// playing, a pause lets it run down; a side change on the platter lifts it, flips it, puts it back. The camera moves
// from the sleeve to the deck. One renderer per stage, three.js loads with the first stage (`loadThree`); `dispose`
// frees everything. Plain JS like model-view.js (no @types/three) – the types are in record-stage.d.ts.
// Callers show their 2D deck without WebGL or with reduced motion (record-card.svelte).
// The reader's records stand as a carousel where the sleeve stood (owner 2026-10-16: "nach hinten ins Kreisel, wie ein
// Karussell"): a swipe turns it, the front one is the selected one; Play on another puts the one on the platter back
// into its sleeve first. Gestures: record-gestures.js; light: record-light.js. Over 400 lines: the stage's one state
// machine (carousel, the ways out / back, flip, camera) reads best in one place.
import { loadSource, loadThree } from '@scutoons/media/model-view';
import { stageGestures } from './record-gestures.js';
import { castShadows, contactShadow, moodyLights } from './record-light.js';

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
// the camera: the records → the deck; each view looks at its target, the point the reader's ride and zoom go around
// (owner 2026-10-16: "pivoting around each target point") – the carousel's centre (behind the front sleeve by the ring's
// radius) or the turntable's
const views = (rho) => ({
	sleeve: { pos: [SLEEVE.x, 0.34, SLEEVE.z + 0.95], look: [SLEEVE.x, 0.15, SLEEVE.z - rho] },
	deck: { pos: [PLINTH.x - 0.1, 0.63, PLINTH.z + 0.74], look: [PLINTH.x, PLINTH.h, PLINTH.z] }
});
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

function buildDeck(THREE, RoundedBoxGeometry) {
	const deck = new THREE.Group();
	const mat = (color, roughness = 0.6, metalness = 0) => new THREE.MeshStandardMaterial({ color, roughness, metalness });
	// bevelled edges (owner 2026-10-16: "Add some bevel edges to the record player"): the plinth's 5 mm round, the
	// platter's rim 3 mm – a few segments each
	const plinth = new THREE.Mesh(new RoundedBoxGeometry(PLINTH.w, PLINTH.h, PLINTH.d, 3, 0.005), mat(0x2b2724, 0.55));
	plinth.position.set(PLINTH.x, PLINTH.h / 2, PLINTH.z);
	const b = 0.003, ph = 0.007, rim = [[0, -ph], [PLATTER.r - b, -ph], [PLATTER.r - b * 0.3, -ph + b * 0.3], [PLATTER.r, -ph + b], [PLATTER.r, ph - b], [PLATTER.r - b * 0.3, ph - b * 0.3], [PLATTER.r - b, ph], [0, ph]];
	const platter = new THREE.Mesh(new THREE.LatheGeometry(rim.map(([x, y]) => new THREE.Vector2(x, y)), 96), mat(0xa8a8a8, 0.35, 0.85));
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

/** the ring's radius for n sleeves standing in a polygon (sleeve 315 mm wide – they must not cut each other) */
const ringRadius = (n) => (n < 2 ? 0 : Math.max(0.03, 0.17 / Math.tan(Math.PI / n)));
const mod = (i, n) => ((i % n) + n) % n;

/**
 * Mounts the stage on a canvas. `records` = the reader's records [{ key, source }] – source: a .glb's URL or a builder
 * (the shared record model, @scutoons/media/vinyl-model); they stand as a carousel left of the turntable (owner
 * 2026-10-16: "wie ein Karussell"), the front one is the selected one (`opts.onselect(key)`). The state comes in by
 * `update` (any time, also before the models are there): `current` = the key of the record that plays – on play it
 * comes out of its sleeve onto the platter; another `current` puts the one on the platter back first.
 */
export function mountRecordStage(canvas, records, opts = {}) {
	let stopped = false, raf = 0, renderer = null, scene = null, env = null, ro = null, unlisten = null, untilt = null;
	const tiltAim = { yaw: 0, pitch: 0 }, tiltNow = { yaw: 0, pitch: 0 };
	const want = { playing: false, side: 'a', arm: -28, lifted: false, current: records[0]?.key ?? null };
	// the reader's look around (owner 2026-10-16: "camera pivoting … zoom in a bit and look at the record cover … smoothly pan
	// back and zoom out on release"; "a camera ride, about 75 degrees max"): the ride on a level arc around the stage's
	// centre (75° in all), the zoom towards the records; the ride springs back on release, the zoom stays until a camera move of the stage's own (or `resetView()`).
	// Which gesture does what: record-gestures.js
	const user = { yaw: 0, pitch: 0, zoom: 0 };
	/** spring velocities: a release snaps back with a little overshoot (owner: "snappy easing … snap if released") */
	const vel = { yaw: 0, pitch: 0, zoom: 0 };
	const aim = { yaw: 0, pitch: 0, zoom: 0 };
	let held = false;
	// a camera move of the stage's own: a smooth tween from where the camera is (no spring, no jump)
	let homing = false;
	const resetView = () => {
		held = false;
		aim.yaw = aim.pitch = aim.zoom = 0;
	};
	// the orbit centre and the reader's ride / zoom – for the lab and tests (owner 2026-10-16: verify the pivot)
	const orbit = { x: 0, y: 0, z: 0, copy(v) { this.x = v.x; this.y = v.y; this.z = v.z; } };
	// `arm`: where the tonearm's head is on the canvas (px) and its turn – the lab drags it
	const probe = { arm: () => null };
	const inspect = () => ({ target: [orbit.x, orbit.y, orbit.z], yaw: user.yaw, zoom: user.zoom, tilt: [tiltNow.yaw, tiltNow.pitch], arm: probe.arm() });
	const handle = { update: (s) => Object.assign(want, s), dispose, resetView, inspect };

	(async () => {
		try {
			const { THREE } = await loadThree();
			const [{ RoomEnvironment }, { RoundedBoxGeometry }] = await Promise.all([
				import('three/addons/environments/RoomEnvironment.js'),
				import('three/addons/geometries/RoundedBoxGeometry.js')
			]);
			if (stopped) return;
			renderer = new THREE.WebGLRenderer({ canvas, antialias: true, alpha: true });
			renderer.setPixelRatio(Math.min(2, window.devicePixelRatio || 1));
			scene = new THREE.Scene();
			const pmrem = new THREE.PMREMGenerator(renderer);
			env = pmrem.fromScene(new RoomEnvironment(), 0.04).texture;
			pmrem.dispose();
			scene.environment = env;
			moodyLights(THREE, renderer, scene, PLINTH, opts.table ?? null);
			const camera = new THREE.PerspectiveCamera(32, 1, 0.01, 20);

			// the records: each set (sleeve + record) on a slot of the ring; a model that fails is left out
			const loaded = await Promise.all(records.map((r) => loadSource(r.source).catch(() => null)));
			if (stopped) return loaded.forEach((set) => set && disposeTree(set));
			const items = [];
			loaded.forEach((set, i) => set && items.push({ key: records[i].key, set }));
			if (!items.length) throw new Error('no record model loaded');
			const n = items.length, step = (2 * Math.PI) / n, rho = ringRadius(n);
			const ring = new THREE.Group();
			ring.position.set(SLEEVE.x, 0, SLEEVE.z - rho);
			scene.add(ring);
			for (const item of items) {
				item.slot = new THREE.Group();
				item.slot.add(item.set);
				item.set.position.set(0, 0, rho);
				const record = item.set.getObjectByName('record');
				item.sleeved = !!record && !!item.set.getObjectByName('sleeve');
				// a record-only model (no "record" node): the whole scene is the record, standing by itself
				item.record = record ?? item.set;
				item.home = item.record.position.clone();
				// the sleeve turned round to show its back (a second tap on the selected record): 0 … 1
				item.turn = 0;
				item.turned = false;
				item.set.userData.stageItem = item;
				if (item.sleeved) {
					const foot = contactShadow(THREE, 0.36, 0.07);
					foot.position.z = rho;
					item.slot.add(foot);
				}
				ring.add(item.slot);
			}
			items.forEach((item, i) => (item.slot.rotation.y = i * step));
			const { deck, arm } = buildDeck(THREE, RoundedBoxGeometry);
			probe.arm = () => {
				const v = arm.localToWorld(new THREE.Vector3(0, 0, ARM_LEN * 0.8)).project(camera);
				return { x: ((v.x + 1) / 2) * canvas.clientWidth, y: ((1 - v.y) / 2) * canvas.clientHeight, turn: armTurnNow, rest: 0, outer: ARM_OUTER, inner: ARM_INNER };
			};
			const VIEWS = views(rho);
			scene.add(deck);
			castShadows(deck, ring);
			const carrier = new THREE.Group(); // position + spin around Y
			const tilt = new THREE.Group(); // 0 = standing (A faces +Z), −π/2 = A up, +π/2 = B up
			carrier.add(tilt);
			scene.add(carrier);
			opts.onload?.();

			const byKey = (key) => items.find((it) => it.key === key) ?? null;
			let front = Math.max(0, items.indexOf(byKey(want.current))); // unbounded: the ring turns the short way
			let angle = -front * step;
			let told = null;
			const tell = () => {
				const key = items[mod(front, n)].key;
				if (key !== told) opts.onselect?.((told = key));
			};
			tell();

			// the record out of its sleeve (it is at the front then) into the carrier, and back
			const inSleeve = new THREE.Vector3();
			const takeOut = (item) => {
				ring.updateMatrixWorld(true);
				item.record.getWorldPosition(inSleeve);
				if (!item.sleeved) inSleeve.set(SLEEVE.x, R, SLEEVE.z);
				item.record.removeFromParent();
				item.record.position.set(0, item.record === item.set ? -R : 0, 0);
				tilt.add(item.record);
				tilt.rotation.x = 0;
				carrier.rotation.y = 0;
				carrier.position.copy(inSleeve);
			};
			const putBack = (item) => {
				item.record.removeFromParent();
				item.record.position.copy(item.home);
				(item.record === item.set ? item.slot : item.set).add(item.record);
			};

			// the stage: 'idle' (all in their sleeves), 'turn' (the ring brings one to the front), 'out' (from the sleeve onto
			// the platter), 'deck', 'flip' (the other side up), 'back' (from the platter into its sleeve)
			let state = 'idle', t0 = 0, then = null, active = null, goal = null, lies = want.side, spin = 0, armTurnNow = 0, lift = 0.06, view = 0;
			// the focus: the stage's own (the deck while a record lies on it, else the records) unless a tap chose one – for a
			// while; `focus` = the tapped one or null
			let rot0 = 0, focus = null, focusUntil = 0;
			const natural = () => (active && (state === 'deck' || state === 'flip') ? 'deck' : 'records');
			const focused = () => focus ?? natural();
			const from = { tilt: 0 };
			const flat = (side) => (side === 'b' ? Math.PI / 2 : -Math.PI / 2);
			const turnTo = (item) => {
				const i = items.indexOf(item), half = Math.floor(n / 2);
				front += mod(i - mod(front, n) + half, n) - half; // the short way round
			};
			// the intro (the sleeve seen first) waits a moment; a record that is already on lies on the platter from the
			// start – the way out of the sleeve runs only when it is put on (owner 2026-10-16: "nur beim Auflegen")
			const startAt = performance.now() + 1200;
			const placed = opts.placed ? byKey(want.current) : null;
			if (placed) {
				turnTo(placed);
				angle = -front * step;
				ring.rotation.y = angle;
				takeOut(placed);
				active = placed;
				state = 'deck';
				view = 1;
				lies = want.side;
				carrier.position.set(PLATTER.x, ON_PLATTER_Y, PLATTER.z);
				tilt.rotation.x = flat(lies);
				spin = want.playing ? -OMEGA : 0;
				armTurnNow = want.playing ? armTurn(want.arm) : 0;
				lift = want.playing && !want.lifted ? 0 : -0.07;
			}
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
			// which part of the stage is under a point: the records or the turntable (what it hits, else the nearer one)
			const ray = new THREE.Raycaster();
			const ndc = new THREE.Vector2();
			const region = (x, y) => {
				const w = canvas.clientWidth || 1, h = canvas.clientHeight || 1;
				ndc.set((x / w) * 2 - 1, -(y / h) * 2 + 1);
				ray.setFromCamera(ndc, camera);
				const hit = ray.intersectObjects([ring, deck, carrier], true)[0];
				let o = hit?.object, item = null, onArm = false;
				while (o && o !== ring && o !== deck && o !== carrier) {
					item = o.userData.stageItem ?? item;
					onArm ||= o === arm;
					o = o.parent;
				}
				if (onArm) return { part: 'arm' };
				if (o === ring) return { part: 'records', key: item?.key ?? null, front: !!item && item === items[mod(front, n)] && item !== active };
				if (o) return { part: 'deck' };
				return { part: null }; // empty space: nothing (owner 2026-10-16: hit the objects, not screen halves)
			};
			const clamp = (v, lo, hi) => Math.min(hi, Math.max(lo, v));
			/** a tap chose a part (null: back to the stage's own focus); a change of the part tweens the camera there */
			const focusOn = (part) => {
				const before = focused();
				focus = part;
				focusUntil = performance.now() + 12000;
				if (focused() !== before) {
					home();
					tiltBase = null; // the tilt recentres on the new target
				}
			};
			// tilting the phone pivots the camera around the focused target (owner 2026-10-16: "tilt the phone to pivot"):
			// gamma / beta relative to how the phone was held (the first reading, again after a focus change), a dead zone of
			// 3°, at most ±18° yaw and ±4° pitch, low-passed in the tick. iOS asks for the permission on the first tap inside
			// the stage (a user gesture; denied = no tilt). Not with reduced motion; a desktop sends no readings – nothing.
			let tiltBase = null;
			const onTilt = (e) => {
				if (e.gamma == null || e.beta == null) return;
				tiltBase ??= { g: e.gamma, b: e.beta };
				const dead = (v) => (Math.abs(v) < 3 ? 0 : v - Math.sign(v) * 3);
				tiltAim.yaw = (clamp(dead(e.gamma - tiltBase.g) * 0.8, -18, 18) * Math.PI) / 180;
				tiltAim.pitch = (clamp(dead(e.beta - tiltBase.b) * 0.25, -4, 4) * Math.PI) / 180;
			};
			const listenTilt = () => {
				window.addEventListener('deviceorientation', onTilt);
				untilt = () => window.removeEventListener('deviceorientation', onTilt);
			};
			if (typeof DeviceOrientationEvent !== 'undefined' && !matchMedia('(prefers-reduced-motion: reduce)').matches) {
				// listening is harmless without the permission (no readings come); where the browser asks for it (iOS), the
				// first tap inside the stage asks
				listenTilt();
				const ask = DeviceOrientationEvent.requestPermission;
				if (typeof ask === 'function') {
					const once = () => {
						canvas.removeEventListener('pointerup', once);
						ask.call(DeviceOrientationEvent).catch(() => {});
					};
					canvas.addEventListener('pointerup', once);
				}
			}
			const busy = () => state === 'turn' || state === 'back' || state === 'out';
			// a camera move of the stage's own (a focus change, a record put on / swapped) tweens back into its position –
			// the reader's zoom stays otherwise (owner 2026-10-16: "no automatic snap-back")
			const home = () => {
				held = false;
				homing = true;
				aim.yaw = aim.pitch = aim.zoom = 0;
			};
			// the selected record (a tap): a second tap on it turns its sleeve round; it can be carried onto the turntable
			let focusedKey = null, carried = null;
			const carryAt = new THREE.Vector3();
			const carryPlane = new THREE.Plane(new THREE.Vector3(0, 1, 0), -0.12);
			const armPlane = new THREE.Plane(new THREE.Vector3(0, 1, 0), -ARM_Y), armAt = new THREE.Vector3();
			let armHeld = null; // the arm's turn while the reader swings it
			unlisten = stageGestures(canvas, {
				region,
				inStage: (x, y) => y < (1 - (opts.lift ?? 0)) * (canvas.clientHeight || 1),
				// a tap on the turntable / the records focuses it (a raycast – empty space never comes here)
				tap: (hit) => {
					const was = focused() === 'records' && !carried;
					if (hit.part === 'deck') {
						focusOn('deck');
						focusedKey = null;
						return;
					}
					focusOn('records');
					const item = hit.key ? byKey(hit.key) : null;
					if (!item || item === active || busy()) return;
					if (item !== items[mod(front, n)]) {
						turnTo(item);
						tell();
						focusedKey = item.key;
					} else if (was && focusedKey === item.key) item.turned = !item.turned;
					else focusedKey = item.key;
				},
				swipe: (by) => {
					if (busy()) return;
					front += by;
					tell();
					focusOn('records');
				},
				// the zoom goes in and out around the focused centre and stays (out to the default distance at most)
				zoom: (by) => {
					homing = false;
					aim.zoom = clamp(aim.zoom + by, 0, 0.8);
				},
				hold: () => (held = true),
				// the ride springs back on release, the zoom stays
				release: () => {
					held = false;
					aim.yaw = aim.pitch = 0;
				},
				// the tonearm (owner 2026-10-16: "the tonearm is draggable"): a press on it swings it around its pivot, raised,
				// between rest and the inner groove; set down on the record = play from there (the needle's place → the song's
				// position), back at rest = pause (opts.onarm)
				armTo: (x, y) => {
					ndc.set((x / (canvas.clientWidth || 1)) * 2 - 1, -(y / (canvas.clientHeight || 1)) * 2 + 1);
					ray.setFromCamera(ndc, camera);
					if (!ray.ray.intersectPlane(armPlane, armAt)) return;
					const a = Math.atan2(armAt.x - PIVOT.x, armAt.z - PIVOT.z);
					armHeld = clamp(a, Math.min(ARM_INNER, 0), Math.max(ARM_INNER, 0));
				},
				armGrab: (on) => {
					if (on) armHeld = armTurnNow;
					else {
						const a = armHeld;
						armHeld = null;
						if (a === null) return;
						// past the lead-in (the outer groove): on the record
						const k = (a - ARM_OUTER) / (ARM_INNER - ARM_OUTER);
						const onIt = Math.abs(a) >= Math.abs(ARM_OUTER) * 0.85;
						opts.onarm?.(onIt ? { play: true, progress: clamp(k, 0, 1) } : { play: false });
					}
				},
				carry: (on) => {
					const item = items[mod(front, n)];
					if (!on || busy() || item === active || !item.sleeved || !opts.onplay) {
						carried = null;
						return false;
					}
					carried = item;
					item.turned = false;
					item.set.getWorldPosition(carryAt);
					return true;
				},
				carryTo: (x, y) => {
					ndc.set((x / (canvas.clientWidth || 1)) * 2 - 1, -(y / (canvas.clientHeight || 1)) * 2 + 1);
					ray.setFromCamera(ndc, camera);
					if (!ray.ray.intersectPlane(carryPlane, carryAt)) return;
					// it stays over the table between the records and the turntable (low on the screen = near the camera)
					carryAt.x = clamp(carryAt.x, SLEEVE.x - 0.1, PLATTER.x + 0.12);
					carryAt.z = clamp(carryAt.z, SLEEVE.z - 0.1, PLATTER.z + 0.12);
				},
				// let go over the turntable (where the finger is, or the record above the platter – the finger is on it)
				drop: (part) => {
					const item = carried;
					carried = null;
					const over = Math.hypot(carryAt.x - PLATTER.x, carryAt.z - PLATTER.z) < PLATTER.r + 0.08;
					if (item && (part === 'deck' || over)) opts.onplay?.(item.key);
				}
			});
			ro = new ResizeObserver(resize);
			ro.observe(canvas);
			resize();
			const look = new THREE.Vector3();
			const local = new THREE.Vector3();
			const offset = new THREE.Vector3();
			const yAxis = new THREE.Vector3(0, 1, 0);
			const side = new THREE.Vector3();
			const hover = ON_PLATTER_Y + 0.16;
			/** the way between the sleeve and the platter at time s of it (out; back = the same way backwards) */
			const along = (s) => {
				const slide = ease(s / SLIDE), carry = ease((s - SLIDE) / CARRY), drop = ease((s - SLIDE - CARRY) / DROP);
				carrier.position.x = carry > 0 ? lerp(OUT_X, PLATTER.x, carry) : lerp(inSleeve.x, OUT_X, slide);
				carrier.position.z = lerp(inSleeve.z, PLATTER.z, carry);
				carrier.position.y = drop > 0 ? lerp(hover, ON_PLATTER_Y, drop) : lerp(inSleeve.y, hover, Math.min(1, carry * 1.6));
				tilt.rotation.x = lerp(0, flat(lies), carry);
				return { slide, carry };
			};
			const WAY = SLIDE + CARRY + DROP;

			const tick = (now) => {
				if (stopped) return;
				const dt = Math.min(0.05, (now - last) / 1000);
				last = now;
				const s = (now - t0) / 1000;
				// what should lie on the platter: the playing record (a paused one stays where it is)
				goal = want.playing ? byKey(want.current) : active;
				if ((state === 'idle' || state === 'deck') && goal && goal !== active && now >= startAt && Math.abs(spin) < 0.4) {
					state = 'turn';
					home();
					focus = null;
					carried = null;
					turnTo(active ?? goal);
					then = active ? 'back' : 'out';
				} else if (state === 'turn' && Math.abs(angle + front * step) < 0.01 && items.every((it) => it.turn < 0.01 && it.set.position.y < 0.001)) {
					state = then;
					t0 = now;
					if (state === 'out') {
						tell(); // the front one is the one that plays now
						active = goal;
						lies = want.side;
						takeOut(active);
					} else {
						rot0 = Math.atan2(Math.sin(carrier.rotation.y), Math.cos(carrier.rotation.y));
						ring.updateMatrixWorld(true);
						if (active.sleeved) active.set.localToWorld(inSleeve.copy(active.home));
						else inSleeve.set(SLEEVE.x, R, SLEEVE.z);
					}
				} else if (state === 'out') {
					// slide out of the open edge → carry over the platter while turning flat → lay it down
					const { slide, carry } = along(s);
					// the camera follows the record from where it is (records → deck), no jump
					const to = carry > 0 ? 0.35 + 0.65 * carry : 0.35 * slide;
					view += (Math.max(view, to) - view) * Math.min(1, dt * 5);
					if (s >= WAY) state = 'deck';
				} else if (state === 'back') {
					// lift it off, stand it up, carry it back and slide it into its sleeve – then the next one comes out
					const { carry } = along(WAY - s);
					carrier.rotation.y = rot0 * carry;
					if (s >= WAY) {
						putBack(active);
						active = null;
						state = 'idle';
					}
				} else if (state === 'deck' && want.side !== lies && Math.abs(spin) < 0.4 && active === goal) {
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
				// the carousel turns to its front record; the selected sleeve turns round on a second tap (back to the front when
				// it leaves the front or a record is put on); a carried one follows the finger, lifted, and goes home after
				angle += (-front * step - angle) * Math.min(1, dt * 6);
				ring.rotation.y = angle;
				for (const it of items) {
					if (it === active && !it.sleeved) continue; // a record-only model: the set is the record, on its way
					if (it.turned && (it !== items[mod(front, n)] || busy())) it.turned = false;
					it.turn += ((it.turned ? 1 : 0) - it.turn) * Math.min(1, dt * 5);
					it.set.rotation.y = Math.PI * ease(it.turn);
					if (it === carried) it.slot.worldToLocal(local.copy(carryAt)).setY(0.04);
					else local.set(0, 0, rho);
					it.set.position.lerp(local, Math.min(1, dt * (it === carried ? 14 : 8)));
					if (it.set.position.y < 1e-4) it.set.position.y = 0;
				}
				// 33⅓ rpm while playing on the platter; runs up / down in about a second – a change brakes it first
				const changing = (state === 'deck' && (want.side !== lies || goal !== active)) || state === 'flip';
				const onRecord = state === 'deck' && want.playing && !changing && active === goal;
				spin += ((onRecord ? -OMEGA : 0) - spin) * Math.min(1, dt * (changing ? 6 : 2.5));
				if (Math.abs(spin) < 1e-3 && !onRecord) spin = 0;
				carrier.rotation.y += spin * dt;
				// the arm: on the record while it plays on the platter, else at rest; lifted between songs
				// the reader swings it: it follows (raised a little); else on the record while it plays, at rest otherwise
				if (armHeld !== null) armTurnNow += (armHeld - armTurnNow) * Math.min(1, dt * 18);
				else armTurnNow += ((onRecord ? armTurn(want.arm) : 0) - armTurnNow) * Math.min(1, dt * 3);
				lift += ((armHeld !== null ? -0.04 : onRecord && !want.lifted ? 0 : -0.07) - lift) * Math.min(1, dt * 6);
				arm.rotation.set(lift, armTurnNow, 0, 'YXZ');
				// the camera: the records while none lies on the platter (or a tap / swipe asked for them – a while), the deck
				// once one is on; the way out moves it along
				if (focus && !carried && now > focusUntil) focusOn(null);
				if (state !== 'out') {
					// carrying a record: halfway, so the records and the turntable are both in the picture
					const to = carried ? 0.55 : focused() === 'deck' ? 1 : 0;
					view += (to - view) * Math.min(1, dt * 2.5);
				}
				const v = ease(view);
				camera.position.set(...VIEWS.sleeve.pos.map((p, i) => lerp(p, VIEWS.deck.pos[i], v)));
				look.set(...VIEWS.sleeve.look.map((p, i) => lerp(p, VIEWS.deck.look[i], v)));
				// the reader's look around: while held it follows the finger closely; released, an under-damped spring snaps it
				// home (≈ 0.35 s, a small overshoot, settled in about half a second)
				if (homing && ['yaw', 'pitch', 'zoom'].every((k) => Math.abs(aim[k] - user[k]) < 1e-3)) homing = false;
				for (const key of ['yaw', 'pitch', 'zoom']) {
					if (homing && !held) {
						user[key] += (aim[key] - user[key]) * Math.min(1, dt * 3);
						vel[key] = 0;
					} else if (held) {
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
				// the phone's tilt, low-passed (tilt.js below)
				tiltNow.yaw += (tiltAim.yaw - tiltNow.yaw) * Math.min(1, dt * 4);
				tiltNow.pitch += (tiltAim.pitch - tiltNow.pitch) * Math.min(1, dt * 4);
				const yaw = user.yaw + tiltNow.yaw, pitch = user.pitch + tiltNow.pitch;
				if (yaw || pitch || user.zoom) {
					// the tilt orbits around what is in focus (the records or the turntable), the zoom comes closer to it
					offset.copy(camera.position).sub(look);
					offset.applyAxisAngle(yAxis, yaw);
					side.crossVectors(offset, yAxis).normalize();
					offset.applyAxisAngle(side, -pitch);
					offset.multiplyScalar(1 - user.zoom * 0.72);
					camera.position.copy(look).add(offset);
				}
				camera.lookAt(look);
				orbit.copy(look);
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
		unlisten?.();
		untilt?.();
		if (scene) disposeTree(scene);
		env?.dispose();
		renderer?.dispose();
	}
	return handle;
}
