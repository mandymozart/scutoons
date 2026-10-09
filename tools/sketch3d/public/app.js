// The page: inputs + parameters on the left, the model in the middle (orbit, wireframe, ink, bones, storyboard poses),
// the silhouette with the found joints on the right. ?batch=all|a,b → builds every character's A-pose and saves the
// GLBs (server.mjs --batch).
import * as THREE from 'three';
import { OrbitControls } from 'three/addons/controls/OrbitControls.js';
import { DEFAULTS, analyse, buildBase, finish, loadImageData, toGLB } from '/core/pipeline.js';
import { jointList } from '/core/rig.js';
import { humanPoses, pose } from '/render/figures.js';

const $ = (id) => document.getElementById(id);
const params = new URLSearchParams(location.search);

// ---------------------------------------------------------------- batch (headless): no renderer, just build + save
if (params.has('batch')) {
	const want = params.get('batch');
	const log = (m) => fetch('/api/log', { method: 'POST', body: m });
	const list = await (await fetch('/api/characters')).json();
	for (const c of list.filter((c) => want === 'all' || want.split(',').includes(c.name))) {
		try {
			const im = await loadImageData(c.url);
			const sh = analyse(im, DEFAULTS);
			const b = buildBase({ img: im, sheet: sh, box: sh.boxes[0] }, null, DEFAULTS);
			// joints set by hand for this character (the page saves them as out/<name>-joints.json)
			const jr = await fetch(`/out/${c.name}-joints.json`);
			const rel = jr.ok ? await jr.json() : {};
			const H = b.grid.sole - b.grid.top + 1;
			const r = finish(b, DEFAULTS, Object.fromEntries(Object.entries(rel).map(([k, [x, y]]) => [k, [b.mesh.cx + x * H, b.grid.top + y * H]])));
			const buf = await toGLB(r.object, c.name);
			await fetch(`/api/save?name=${encodeURIComponent(c.name)}`, { method: 'POST', body: buf });
			await log(`${c.name}: ${c.file} → ${r.info.tris} tris, ${(buf.byteLength / 1024).toFixed(0)} KB${r.info.warn.length ? ' – ' + r.info.warn.join('; ') : ''}`);
		} catch (e) {
			await log(`${c.name}: FAILED ${e.message}`);
		}
	}
	await fetch('/api/done', { method: 'POST' });
	throw new Error('batch done'); // stop the module here
}

// ---------------------------------------------------------------- settings (remembered per browser)
const KEY = 'sketch3d.settings';
let S = { ...DEFAULTS };
try { S = { ...S, ...JSON.parse(localStorage.getItem(KEY) ?? '{}') }; } catch { /* private window */ }
const remember = () => { try { localStorage.setItem(KEY, JSON.stringify(S)); } catch { /* ignore */ } };

for (const k of ['tol', 'rows', 'depth', 'smooth', 'tris', 'colors', 'height']) {
	const el = $(k), out = $(k + 'V');
	el.value = S[k]; out.value = S[k];
	el.addEventListener('input', () => { S[k] = Number(el.value); out.value = el.value; remember(); k === 'tol' ? reanalyseSoon() : rebuildSoon(); });
}
for (const k of ['rig', 'mono']) {
	$(k).checked = S[k];
	$(k).addEventListener('change', () => { S[k] = $(k).checked; remember(); rebuildSoon(); });
}
function seg(id, key, onChange) {
	const root = $(id);
	const set = (v) => root.querySelectorAll('button').forEach((b) => b.classList.toggle('on', b.dataset.v === v));
	if (key) set(S[key]);
	root.addEventListener('click', (e) => {
		const b = e.target.closest('button');
		if (!b) return;
		set(b.dataset.v);
		onChange(b.dataset.v);
	});
}
seg('mode', 'mode', (v) => { S.mode = v; remember(); rebuildSoon(); });
seg('input', 'input', (v) => { S.input = v; remember(); reanalyseSoon(); });
let atimer = 0;
function reanalyseSoon() { clearTimeout(atimer); atimer = setTimeout(reanalyse, 250); }

// ---------------------------------------------------------------- inputs: images and character sheets
// Every image is split into its figures (panels); each panel gets a role: Vorne (the silhouette that is inflated +
// rigged), Seite (a true side profile, optional: cuts the depth, gives the joints their depth) or – (ignored, e.g. the
// ¾ and back views of a sheet). Two figures (front + side image, or a front + side sheet) → the second is the side.
let images = [], panels = [], roles = [];
let joints = {}, jointsFor = null; // images: [{ img, sh, url }], panels: [{ i (image), box }]
const status = (t) => { $('status').textContent = t; };
const cleanName = (n) => n.replace(/\.[^.]+$/, '').replace(/_apose.*$/, '').replace(/[^a-z0-9_-]+/gi, '-').toLowerCase();
async function setImages(srcs, label, add = false) {
	const loaded = await Promise.all(srcs.map(async (src) => ({ img: await loadImageData(src), url: typeof src === 'string' ? src : URL.createObjectURL(src) })));
	images = add ? images.concat(loaded) : loaded;
	$('thumb').src = images[0].url;
	$('thumb').parentElement.classList.add('has');
	if (label && !add) { $('name').value = label; jointsFor = null; }
	reanalyse();
}
function reanalyse() {
	if (!images.length) return;
	for (const im of images) im.sh = analyse(im.img, S);
	panels = images.flatMap((im, i) => im.sh.boxes.map((box) => ({ i, box })));
	roles = panels.map((_, k) => (k === 0 ? 'front' : k === 1 && panels.length === 2 ? 'side' : '-'));
	drawPanels();
	rebuildSoon(0);
}
function drawPanels() {
	const wrap = $('panels');
	wrap.replaceChildren();
	if (panels.length < 2) return; // a single figure needs no choice
	const canvases = images.map(({ img }) => { const c0 = new OffscreenCanvas(img.width, img.height); c0.getContext('2d').putImageData(img, 0, 0); return c0; });
	panels.forEach(({ i, box: b }, k) => {
		const div = document.createElement('div');
		div.className = 'panel';
		const c = document.createElement('canvas');
		c.width = b.x1 - b.x0 + 1; c.height = b.y1 - b.y0 + 1;
		c.getContext('2d').drawImage(canvases[i], b.x0, b.y0, c.width, c.height, 0, 0, c.width, c.height);
		const s = document.createElement('div');
		s.className = 'seg';
		for (const [v, t] of [['front', 'Vorne'], ['side', 'Seite'], ['-', '–']]) {
			const btn = Object.assign(document.createElement('button'), { type: 'button', textContent: t });
			btn.classList.toggle('on', roles[k] === v);
			btn.addEventListener('click', () => {
				if (v !== '-') roles = roles.map((r) => (r === v ? '-' : r)); // one front, one side
				roles[k] = v;
				drawPanels();
				rebuildSoon(0);
			});
			s.append(btn);
		}
		div.append(c, s);
		wrap.append(div);
	});
}
const source = (role) => {
	const k = roles.indexOf(role);
	if (k < 0) return null;
	const { i, box } = panels[k];
	return { img: images[i].img, sheet: images[i].sh, box };
};

// drop / pick: one or more images replace the current ones; "+ Bild" adds (e.g. the side view to a front view)
const files = (list) => [...list].filter((f) => f.type.startsWith('image/'));
const zone = $('drop'), input = $('file');
input.addEventListener('change', () => { const f = files(input.files); if (f.length) setImages(f, cleanName(f[0].name)); input.value = ''; });
$('addFile').addEventListener('change', (e) => { const f = files(e.target.files); if (f.length) setImages(f, null, true); e.target.value = ''; });
zone.addEventListener('dragover', (e) => { e.preventDefault(); zone.classList.add('over'); });
zone.addEventListener('dragleave', () => zone.classList.remove('over'));
zone.addEventListener('drop', (e) => {
	e.preventDefault();
	zone.classList.remove('over');
	const f = files(e.dataTransfer.files);
	if (f.length) setImages(f, cleanName(f[0].name));
});

const chars = await (await fetch('/api/characters')).json();
for (const c of chars) $('character').append(new Option(`${c.name} (${c.file})`, c.name));
$('character').addEventListener('change', () => {
	const c = chars.find((x) => x.name === $('character').value);
	if (c) setImages([c.url], c.name);
});

// ---------------------------------------------------------------- the viewer
const vp = $('viewport');
const renderer = new THREE.WebGLRenderer({ antialias: true });
renderer.setPixelRatio(devicePixelRatio);
renderer.shadowMap.enabled = true;
vp.append(renderer.domElement);
const scene = new THREE.Scene();
const camera = new THREE.PerspectiveCamera(32, 1, 0.05, 100);
camera.position.set(1.2, 1.3, 4.2);
const controls = new OrbitControls(camera, renderer.domElement);
controls.target.set(0, 0.9, 0);
const hemi = new THREE.HemisphereLight(0xffffff, 0x8a8577, 1.6);
const key = new THREE.DirectionalLight(0xffffff, 2.2);
key.position.set(2.5, 4, 3);
key.castShadow = true;
key.shadow.mapSize.set(2048, 2048);
Object.assign(key.shadow.camera, { left: -2, right: 2, top: 3, bottom: -1 });
scene.add(hemi, key);
const floor = new THREE.Mesh(new THREE.CircleGeometry(2.5, 48).rotateX(-Math.PI / 2), new THREE.ShadowMaterial({ opacity: 0.18 }));
floor.receiveShadow = true;
scene.add(floor);
const gridHelper = new THREE.GridHelper(4, 16, 0x999999, 0xcccccc);
gridHelper.material.transparent = true; gridHelper.material.opacity = 0.35;
scene.add(gridHelper);

function resize() {
	const { clientWidth: w, clientHeight: h } = vp;
	renderer.setSize(w, h, false);
	camera.aspect = w / Math.max(1, h);
	camera.updateProjectionMatrix();
}
new ResizeObserver(resize).observe(vp);
const dark = matchMedia('(prefers-color-scheme: dark)');
renderer.setAnimationLoop(() => {
	controls.update();
	scene.background = new THREE.Color(view === 'ink' ? 0xffffff : dark.matches ? 0x1b1c1f : 0xeceae5);
	renderer.render(scene, camera);
});

// view modes: shaded (vertex colours, flat), wire, ink (one hard light, black / white – the storyboard's look)
let view = 'shaded';
const toon = (() => {
	const t = new THREE.DataTexture(new Uint8Array([0, 255]), 2, 1, THREE.RedFormat);
	t.minFilter = t.magFilter = THREE.NearestFilter; t.needsUpdate = true;
	return new THREE.MeshToonMaterial({ color: 0xffffff, gradientMap: t });
})();
let current = null, helper = null, shaded = null;
function applyView() {
	if (!current) return;
	const obj = current.object;
	if (view === 'ink') { obj.material = toon; hemi.intensity = 0; key.intensity = 3; }
	else { obj.material = shaded; hemi.intensity = 1.6; key.intensity = 2.2; }
	shaded.wireframe = view === 'wire';
	floor.visible = gridHelper.visible = view !== 'ink';
}
seg('view', null, (v) => { view = v; applyView(); });
$('bones').addEventListener('change', () => { if (helper) helper.visible = $('bones').checked; });

for (const name of humanPoses) $('pose').append(new Option(name, name));
function applyPose() {
	const f = current?.figure;
	if (!f) return;
	const name = $('pose').value;
	if (!name) { f.mesh.skeleton.pose(); return; }
	pose(f, name);
	// figures.js puts the pelvis at 0.98 m (+ the pose's drop); scale that to this figure's own pelvis height
	const p = f.joints.pelvis;
	p.position.y = f.restPelvisY + (p.position.y - 0.98) * (f.restPelvisY / 0.98);
}
$('pose').addEventListener('change', applyPose);

// ---------------------------------------------------------------- build
// Two stages: the mesh (slow, on image / form changes) and the rig (fast, also when a joint is dragged by hand).
// Joints moved by hand are kept per model name in tools/sketch3d/out/<name>-joints.json, relative to the figure
// (x from the centre line, y from the top, in figure heights) – so they survive a change of the resolution.
let timer = 0, base = null;
function rebuildSoon(ms = 250) { clearTimeout(timer); timer = setTimeout(rebuild, ms); }
const modelName = () => $('name').value.trim() || 'figure';
async function loadJoints() {
	const n = modelName();
	if (jointsFor === n) return;
	jointsFor = n;
	joints = {};
	try { const r = await fetch(`/out/${encodeURIComponent(n)}-joints.json`); if (r.ok) joints = await r.json(); } catch { /* none */ }
}
const saveJoints = () => fetch(`/api/save?name=${encodeURIComponent(modelName() + '-joints')}&kind=json`, { method: 'POST', body: JSON.stringify(joints) });
// relative ↔ grid
const frame = () => { const g = base.grid; return { cx: base.mesh.cx, top: g.top, H: g.sole - g.top + 1 }; };
const toRel = ([gx, gy]) => { const f = frame(); return [(gx - f.cx) / f.H, (gy - f.top) / f.H]; };
const toGridP = ([rx, ry]) => { const f = frame(); return [f.cx + rx * f.H, f.top + ry * f.H]; };
const gridJoints = () => Object.fromEntries(Object.entries(joints).map(([k, p]) => [k, toGridP(p)]));

function rebuild() {
	const front = source('front'), side = source('side');
	if (!front) return;
	status('baue …');
	setTimeout(async () => { // (not requestAnimationFrame: it never fires in a hidden tab)
		try { base = buildBase(front, side, S); } catch (e) { status('Fehler: ' + e.message); console.error(e); return; }
		await loadJoints();
		refinish(true);
		status('');
	}, 30);
}
function refinish(frameIt = false) {
	let r;
	try { r = finish(base, S, gridJoints()); } catch (e) { status('Fehler: ' + e.message); console.error(e); return; }
	if (current) scene.remove(current.object);
	if (helper) { scene.remove(helper); helper = null; }
	current = r;
	shaded = r.object.material;
	r.object.castShadow = true;
	scene.add(r.object);
	if (r.figure) {
		helper = new THREE.SkeletonHelper(r.object);
		helper.visible = $('bones').checked;
		scene.add(helper);
	}
	$('pose').disabled = !r.figure;
	applyView();
	applyPose();
	if (frameIt) {
		const bb = new THREE.Box3().setFromObject(r.object);
		controls.target.set(0, (bb.max.y + bb.min.y) / 2, 0);
	}
	const i = r.info;
	$('info').textContent = `${i.tris} Dreiecke (roh ${i.rawTris}) · Gitter ${i.cells} · ${i.input} · ${i.ms.mask + i.ms.mesh + i.ms.rig} ms`;
	$('warn').replaceChildren(...i.warn.map((w) => Object.assign(document.createElement('li'), { textContent: w })));
	$('jointsReset').disabled = !Object.keys(joints).length;
	drawDebug(r);
}

// the silhouette on the grid, the regions, the joints (draggable)
const K = 4; // canvas px per grid cell
let drag = null; // { key, mirror, p }
function drawDebug(r) {
	const { grid } = r;
	const c = $('maskCanvas');
	c.width = grid.w * K; c.height = grid.h * K;
	const g = c.getContext('2d');
	const lab = r.rig?.lm.label;
	const tint = [null, null, [255, 120, 80], [80, 160, 255], [255, 200, 60], [80, 220, 140], [190, 140, 255]];
	for (let y = 0; y < grid.h; y++) for (let x = 0; x < grid.w; x++) {
		const i = y * grid.w + x;
		if (!grid.mask[i]) continue;
		let col = [grid.color[i * 3] * 255, grid.color[i * 3 + 1] * 255, grid.color[i * 3 + 2] * 255];
		const t = lab && tint[lab[i]];
		if (t) col = col.map((v, j) => v * 0.55 + t[j] * 0.45);
		g.fillStyle = `rgb(${col.map(Math.round).join(',')})`;
		g.fillRect(x * K, y * K, K, K);
	}
	if (!r.rig) return;
	// the joints as drawn: the rig's, with the one being dragged (and its mirror) at the pointer
	const P = Object.fromEntries(jointList(r.rig.lm).map(({ key, p }) => [key, p]));
	if (drag) { P[drag.key] = drag.p; if (drag.mirror) P[drag.mirror] = [2 * base.mesh.cx - drag.p[0], drag.p[1]]; }
	const line = (a, b) => { g.beginPath(); g.moveTo(P[a][0] * K, P[a][1] * K); g.lineTo(P[b][0] * K, P[b][1] * K); g.stroke(); };
	g.strokeStyle = '#111'; g.lineWidth = 2;
	line('T.pelvis', 'T.spine'); line('T.spine', 'T.chest'); line('T.chest', 'T.neck'); line('T.neck', 'T.head'); line('T.head', 'T.top');
	for (const s of ['L', 'R']) {
		line('T.chest', `arm.${s}.sh`); line(`arm.${s}.sh`, `arm.${s}.el`); line(`arm.${s}.el`, `arm.${s}.wr`); line(`arm.${s}.wr`, `arm.${s}.tip`);
		line('T.pelvis', `leg.${s}.hip`); line(`leg.${s}.hip`, `leg.${s}.kn`); line(`leg.${s}.kn`, `leg.${s}.an`); line(`leg.${s}.an`, `leg.${s}.toe`);
	}
	for (const [key, p] of Object.entries(P)) {
		if (key === 'T.robe') continue;
		g.beginPath();
		g.arc(p[0] * K, p[1] * K, key.endsWith('tip') || key.endsWith('toe') || key === 'T.top' ? 3 : 5, 0, Math.PI * 2);
		g.fillStyle = joints[key] ? '#2b6cb0' : '#e53e3e'; // blue = set by hand
		g.fill();
	}
}
const mirrorOf = (key) => (/\.(L|R)\./.test(key) ? key.replace(/\.(L|R)\./, (_, s) => (s === 'L' ? '.R.' : '.L.')) : null);
function canvasPoint(e) {
	const c = $('maskCanvas'), b = c.getBoundingClientRect();
	return [((e.clientX - b.left) * (c.width / b.width)) / K, ((e.clientY - b.top) * (c.height / b.height)) / K];
}
$('maskCanvas').addEventListener('pointerdown', (e) => {
	if (!current?.rig) return;
	const p = canvasPoint(e);
	const c = $('maskCanvas'), scale = c.width / c.getBoundingClientRect().width;
	let best = null, bd = (10 * scale) / K; // within 10 css px
	for (const { key, p: q } of jointList(current.rig.lm)) {
		if (key === 'T.robe') continue;
		const d = Math.hypot(q[0] - p[0], q[1] - p[1]);
		if (d < bd) { bd = d; best = key; }
	}
	if (!best) return;
	c.setPointerCapture(e.pointerId);
	drag = { key: best, mirror: $('symmetric').checked ? mirrorOf(best) : null, p };
	drawDebug(current);
});
$('maskCanvas').addEventListener('pointermove', (e) => { if (!drag) return; drag.p = canvasPoint(e); drawDebug(current); });
$('maskCanvas').addEventListener('pointerup', () => {
	if (!drag) return;
	joints[drag.key] = toRel(drag.p);
	if (drag.mirror) joints[drag.mirror] = toRel([2 * base.mesh.cx - drag.p[0], drag.p[1]]);
	drag = null;
	refinish();
	saveJoints();
});
$('jointsReset').addEventListener('click', () => { joints = {}; refinish(); saveJoints(); });
$('name').addEventListener('change', () => { jointsFor = null; rebuildSoon(0); });

// ---------------------------------------------------------------- save
async function listModels() {
	const list = await (await fetch('/api/models')).json();
	$('models').replaceChildren(...list.map((m) => {
		const li = document.createElement('li');
		li.append(Object.assign(document.createElement('a'), { href: m.url, textContent: `${m.name}.glb`, download: `${m.name}.glb` }), ` ${(m.bytes / 1024).toFixed(0)} KB`);
		return li;
	}));
}
$('save').addEventListener('click', async () => {
	if (!current) return;
	const name = modelName();
	// save the rest (bind) pose, with the colours, whatever the view shows
	const obj = current.object;
	const mat = obj.material;
	obj.material = shaded; shaded.wireframe = false;
	current.figure?.mesh.skeleton.pose();
	const buf = await toGLB(obj, name);
	scene.add(obj); // toGLB took it out of the scene
	obj.material = mat; applyView(); applyPose();
	const res = await (await fetch(`/api/save?name=${encodeURIComponent(name)}`, { method: 'POST', body: buf })).json();
	status(res.error ? 'Fehler: ' + res.error : `gespeichert: ${res.file} (${(buf.byteLength / 1024).toFixed(0)} KB)`);
	listModels();
});
listModels();

// start with the first character's A-pose
if (chars.length && !images.length) { $('character').value = chars[0].name; setImages([chars[0].url], chars[0].name); }
