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
const SLIDE = 0.9; // s
const CARRY = 1.2;
const DROP = 0.45;
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
	const want = { playing: false, side: 'a', arm: -28, lifted: false };
	const handle = { update: (s) => Object.assign(want, s), dispose };

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
			const startAt = performance.now() + 500; // the card opens first
			let last = performance.now();

			const resize = () => {
				const w = canvas.clientWidth || 1, h = canvas.clientHeight || 1;
				renderer.setSize(w, h, false);
				camera.aspect = w / h;
				camera.updateProjectionMatrix();
			};
			ro = new ResizeObserver(resize);
			ro.observe(canvas);
			resize();
			const look = new THREE.Vector3();

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
		if (scene) disposeTree(scene);
		env?.dispose();
		renderer?.dispose();
	}
	return handle;
}
