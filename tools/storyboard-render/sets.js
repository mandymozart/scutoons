// The sets: a few shapes per place (MAP.md place ids) + props picked from the shot's action text (German keywords –
// the storyboard describes what the image shows). Deliberately simple: under extreme hard light only silhouettes,
// shadows and edges read. Deterministic (seeded by the shot id), so a re-render gives the same frame.
import * as THREE from 'three';
import { look } from './figures.js';

function rng(seed) {
	let h = 2166136261;
	for (const c of String(seed)) h = Math.imul(h ^ c.charCodeAt(0), 16777619);
	return () => ((h = Math.imul(h ^ (h >>> 15), 2246822507) ^ Math.imul(h ^ (h >>> 13), 3266489909)) >>> 0) / 4294967296;
}
function mesh(geo, scene, { x = 0, y = 0, z = 0, rx = 0, ry = 0, rz = 0, mat } = {}) {
	const m = new THREE.Mesh(geo, mat ?? look.mat);
	m.position.set(x, y, z);
	m.rotation.set(rx, ry, rz);
	m.castShadow = m.receiveShadow = true;
	scene.add(m);
	return m;
}
const spruce = (scene, x, z, h) => {
	mesh(new THREE.CylinderGeometry(0.08 * h / 6, 0.12 * h / 6, h * 0.25, 6), scene, { x, y: h * 0.12, z });
	for (let i = 0; i < 4; i++) mesh(new THREE.ConeGeometry(h * (0.28 - i * 0.05), h * 0.38, 9), scene, { x, y: h * (0.32 + i * 0.17), z });
};
const mountain = (scene, x, z, h) => mesh(new THREE.ConeGeometry(h * 0.9, h, 5), scene, { x, y: h / 2 - 1, z, ry: x });
const house = (scene, x, z, r) => {
	const w = 4 + r() * 3, d = 5 + r() * 3, h = 3 + r() * 2;
	mesh(new THREE.BoxGeometry(w, h, d), scene, { x, y: h / 2, z, ry: r() });
	const roof = mesh(new THREE.CylinderGeometry(0.01, w * 0.72, 2.4, 4, 1), scene, { x, y: h + 1.2, z, ry: Math.PI / 4 });
	roof.scale.set(1, 1, d / w);
};
const bareTree = (scene, x, z, h, r) => {
	mesh(new THREE.CylinderGeometry(0.12, 0.22, h * 0.6, 6), scene, { x, y: h * 0.3, z });
	for (let i = 0; i < 5; i++) mesh(new THREE.CylinderGeometry(0.03, 0.08, h * 0.45, 5), scene, { x: x + (r() - 0.5) * 0.6, y: h * (0.6 + r() * 0.2), z, rz: (r() - 0.5) * 1.6, rx: (r() - 0.5) * 0.8 });
};
const leafyTree = (scene, x, z, h) => {
	mesh(new THREE.CylinderGeometry(0.15, 0.25, h * 0.5, 7), scene, { x, y: h * 0.25, z });
	mesh(new THREE.IcosahedronGeometry(h * 0.38, 1), scene, { x, y: h * 0.72, z });
};
const column = (scene, x, z, h) => mesh(new THREE.CylinderGeometry(0.35, 0.42, h, 14), scene, { x, y: h / 2, z });
const shelf = (scene, x, z, ry, r) => {
	mesh(new THREE.BoxGeometry(3, 4.2, 0.5), scene, { x, y: 2.1, z, ry });
	for (let i = 0; i < 5; i++) for (let k = 0; k < 9; k++) if (r() > 0.25) mesh(new THREE.BoxGeometry(0.12 + r() * 0.12, 0.45 + r() * 0.25, 0.32), scene, { x: x + Math.cos(ry) * (-1.3 + k * 0.32), y: 0.5 + i * 0.8, z: z + 0.3 - Math.sin(ry) * (-1.3 + k * 0.32), ry, rz: (r() - 0.5) * 0.2 });
};
const rack = (scene, x, z, ry, emissive) => {
	mesh(new THREE.BoxGeometry(0.9, 2.2, 1.1), scene, { x, y: 1.1, z, ry });
	for (let i = 0; i < 8; i++) mesh(new THREE.BoxGeometry(0.6, 0.03, 0.02), scene, { x, y: 0.3 + i * 0.24, z: z + 0.56, ry, mat: emissive });
};

/** builds the set into the scene; returns { practical(pos) } to put a visible source (candle, screen) at a light */
export function buildSet(scene, shot, { emissive, accentMat, interior }) {
	const r = rng(shot.id);
	const place = shot.location?.place ?? '';
	const text = `${shot.action ?? ''}`.toLowerCase();
	const has = (...w) => w.some((k) => text.includes(k));
	// the ground: snow (lit white) outside, a stone floor inside
	const ground = mesh(new THREE.PlaneGeometry(800, 800), scene, { rx: -Math.PI / 2 });
	ground.castShadow = false;

	if (/mountain|crossroads|forest|path|clearing/.test(place) || (!interior && has('schnee', 'wald', 'berg'))) {
		for (let i = 0; i < 7; i++) mountain(scene, -120 + i * 40 + r() * 20, -160 - r() * 80, 40 + r() * 50);
		const trees = place === 'mountain-village' ? 10 : /clearing/.test(place) ? 26 : 40;
		for (let i = 0; i < trees; i++) {
			const a = r() * Math.PI * 2, d = (/clearing/.test(place) ? 14 : 7) + r() * 40;
			const x = Math.cos(a) * d, z = Math.sin(a) * d - 8;
			if (Math.abs(x) < 2.5 && z > -4) continue; // keep the subject's spot free
			spruce(scene, x, z, 5 + r() * 9);
		}
		if (place === 'mountain-village' || has('dorf', 'haltestelle')) for (let i = 0; i < 6; i++) house(scene, -30 + i * 11 + r() * 4, -22 - r() * 15, r);
	}
	if (interior || /library|monastery/.test(place)) {
		const walls = mesh(new THREE.BoxGeometry(60, 20, 60), scene, { y: 10 });
		walls.material = new THREE.MeshLambertMaterial({ color: 0xffffff, side: THREE.BackSide });
		if (has('säule', 'gewölbe', 'kloster', 'bibliothek', 'bögen', 'bogen') || /library|monastery/.test(place)) for (let i = 0; i < 6; i++) { column(scene, -8, -4 - i * 7, 9); column(scene, 8, -4 - i * 7, 9); }
		if (has('regal', 'bücher', 'buch', 'bibliothek') || place === 'library') for (let i = 0; i < 6; i++) { shelf(scene, -5.5, -3 - i * 4, Math.PI / 2, r); shelf(scene, 5.5, -3 - i * 4, -Math.PI / 2, r); }
		if (has('server', 'rack', 'serverfarm', 'kontrollzentrum')) for (let i = 0; i < 7; i++) for (const x of [-4, -2.5, 2.5, 4]) rack(scene, x, -4 - i * 1.4, x < 0 ? Math.PI / 2 : -Math.PI / 2, emissive);
		if (has('pult', 'lesepult', 'terminal')) mesh(new THREE.BoxGeometry(0.8, 1.1, 0.6), scene, { x: 1.5, y: 0.55, z: -1.5, rx: 0 });
	}
	if (has('kugel', 'sphere')) mesh(new THREE.SphereGeometry(14, 40, 30), scene, { x: 6, y: 16, z: -45 });
	if (has('fels')) mesh(new THREE.DodecahedronGeometry(2.2, 0), scene, { x: 3, y: 1.4, z: -3, ry: 0.5 });
	if (has('mauer')) mesh(new THREE.BoxGeometry(14, 3, 0.8), scene, { x: 0, y: 1.5, z: -9 });
	if (has('treppe', 'stufen')) for (let i = 0; i < 7; i++) mesh(new THREE.BoxGeometry(5, 0.3, 0.6), scene, { y: 0.15 + i * 0.3, z: -5 - i * 0.6 });
	if (has('pforte', ' tür', 'eingang')) { mesh(new THREE.BoxGeometry(0.5, 4, 0.6), scene, { x: -1.4, y: 2, z: -7 }); mesh(new THREE.BoxGeometry(0.5, 4, 0.6), scene, { x: 1.4, y: 2, z: -7 }); mesh(new THREE.BoxGeometry(3.3, 0.5, 0.6), scene, { y: 4.2, z: -7 }); }
	// chapter 4 / 5 places (2026-10-10): crypt, cemetery, fountain, the garden of Eden
	if (/crypt|tomb|vault/.test(place) || has('gruft', 'sarg', 'sarkophag')) {
		for (let i = 0; i < 4; i++) { column(scene, -4, -3 - i * 5, 5); column(scene, 4, -3 - i * 5, 5); }
		for (let i = 0; i < 4; i++) mesh(new THREE.TorusGeometry(4, 0.35, 8, 24, Math.PI), scene, { y: 5, z: -3 - i * 5 });
		if (has('sarg', 'sarkophag', 'gruft')) mesh(new THREE.BoxGeometry(1.1, 0.9, 2.3), scene, { x: -2, y: 0.45, z: -6 });
	}
	if (/cemetery|graveyard/.test(place) || has('friedhof', 'grab', 'grabstein', 'kreuz')) {
		for (let i = 0; i < 18; i++) {
			const x = -12 + (i % 6) * 4.6 + r(), z = -5 - Math.floor(i / 6) * 5 - r() * 2;
			if (Math.abs(x) < 2 && z > -6) continue;
			if (r() > 0.5) { mesh(new THREE.BoxGeometry(0.18, 1.8, 0.18), scene, { x, y: 0.9, z }); mesh(new THREE.BoxGeometry(0.9, 0.18, 0.18), scene, { x, y: 1.35, z }); }
			else mesh(new THREE.BoxGeometry(0.9, 1.1 + r() * 0.6, 0.22), scene, { x, y: 0.6, z, ry: (r() - 0.5) * 0.4, rz: (r() - 0.5) * 0.15 });
		}
		for (let i = 0; i < 5; i++) bareTree(scene, -20 + i * 10 + r() * 3, -22 - r() * 6, 6 + r() * 4, r);
	}
	if (/fountain|well/.test(place) || has('brunnen')) {
		mesh(new THREE.CylinderGeometry(2.4, 2.6, 0.8, 28), scene, { x: 0, y: 0.4, z: -4 });
		mesh(new THREE.CylinderGeometry(0.35, 0.45, 2.4, 14), scene, { x: 0, y: 1.2, z: -4 });
		mesh(new THREE.CylinderGeometry(1.1, 0.4, 0.4, 20), scene, { x: 0, y: 2.4, z: -4 });
	}
	if (/garden|eden|paradise/.test(place) || has('garten', 'eden', 'paradies')) {
		for (let i = 0; i < 22; i++) {
			const a = r() * Math.PI * 2, d = 8 + r() * 30, x = Math.cos(a) * d, z = Math.sin(a) * d - 10;
			if (Math.abs(x) < 3 && z > -6) continue;
			leafyTree(scene, x, z, 4 + r() * 5);
		}
		if (has('sonne', 'sonnenuntergang')) mesh(new THREE.CircleGeometry(18, 40), scene, { y: 8, z: -180, mat: emissive });
		if (has('apfel', 'äpfel', 'frucht')) for (let i = 0; i < 6; i++) { const a = mesh(new THREE.SphereGeometry(0.12, 8, 6), scene, { x: 1.5 + r() * 0.8, y: 2.6 + r() * 0.6, z: -2.5 + r() * 0.5 }); a.userData.accent = true; }
	}
	if (has('beere', 'beeren')) for (let i = 0; i < 9; i++) {
		const b = mesh(new THREE.SphereGeometry(0.09, 8, 6), scene, { x: 1.6 + r() * 0.6, y: 0.9 + r() * 0.5, z: -1.8 + r() * 0.4 });
		b.userData.accent = true;
	}
	if (has('kerze')) for (let i = 0; i < 3; i++) mesh(new THREE.CylinderGeometry(0.05, 0.05, 0.4, 8), scene, { x: -1.2 + i * 0.3, y: 1.3, z: -1.2, mat: emissive });
	// an accent the storyboard names: the accent material on what carries it (berries above; eyes / glow: a small sphere at the subject)
	return {
		practical(pos) {
			const g = mesh(new THREE.SphereGeometry(0.07, 10, 8), scene, { x: pos.x, y: pos.y, z: pos.z, mat: emissive });
			g.castShadow = false;
		}
	};
}
