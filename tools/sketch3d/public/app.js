// The page: inputs + parameters on the left, the model in the middle (orbit, wireframe, ink, bones, storyboard poses),
// the silhouette with the found joints on the right. ?batch=all|a,b → builds every character's A-pose and saves the
// GLBs (server.mjs --batch).
import * as THREE from 'three';
import { OrbitControls } from 'three/addons/controls/OrbitControls.js';
import { DEFAULTS, analyse, build, loadImageData, toGLB } from '/core/pipeline.js';
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
			const r = build({ img: im, sheet: sh, box: sh.boxes[0] }, null, DEFAULTS);
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

// ---------------------------------------------------------------- inputs: one image or a character sheet
// The image is split into its figures (panels); each gets a role: Vorne (the silhouette that is inflated + rigged),
// Seite (a true side profile, optional: cuts the depth) or – (ignored, e.g. the ¾ and back views of a sheet).
let img = null, sh = null, roles = [];
const status = (t) => { $('status').textContent = t; };
const cleanName = (n) => n.replace(/\.[^.]+$/, '').replace(/_apose.*$/, '').replace(/[^a-z0-9_-]+/gi, '-').toLowerCase();
async function setImage(src, label) {
	img = await loadImageData(src);
	$('thumb').src = typeof src === 'string' ? src : URL.createObjectURL(src);
	$('thumb').parentElement.classList.add('has');
	if (label) $('name').value = label;
	reanalyse();
}
function reanalyse() {
	if (!img) return;
	sh = analyse(img, S);
	roles = sh.boxes.map((_, i) => (i === 0 ? 'front' : '-'));
	drawPanels();
	rebuildSoon(0);
}
function drawPanels() {
	const wrap = $('panels');
	wrap.replaceChildren();
	if (sh.boxes.length < 2) return; // a single figure needs no choice
	const c0 = new OffscreenCanvas(img.width, img.height);
	c0.getContext('2d').putImageData(img, 0, 0);
	sh.boxes.forEach((b, i) => {
		const div = document.createElement('div');
		div.className = 'panel';
		const c = document.createElement('canvas');
		c.width = b.x1 - b.x0 + 1; c.height = b.y1 - b.y0 + 1;
		c.getContext('2d').drawImage(c0, b.x0, b.y0, c.width, c.height, 0, 0, c.width, c.height);
		const s = document.createElement('div');
		s.className = 'seg';
		for (const [v, t] of [['front', 'Vorne'], ['side', 'Seite'], ['-', '–']]) {
			const btn = Object.assign(document.createElement('button'), { type: 'button', textContent: t });
			btn.classList.toggle('on', roles[i] === v);
			btn.addEventListener('click', () => {
				// one front, one side
				if (v !== '-') roles = roles.map((r) => (r === v ? '-' : r));
				roles[i] = v;
				drawPanels();
				rebuildSoon(0);
			});
			s.append(btn);
		}
		div.append(c, s);
		wrap.append(div);
	});
}
const source = (role) => { const i = roles.indexOf(role); return i < 0 ? null : { img, sheet: sh, box: sh.boxes[i] }; };

const zone = $('drop'), input = $('file');
input.addEventListener('change', () => input.files[0] && setImage(input.files[0], cleanName(input.files[0].name)));
zone.addEventListener('dragover', (e) => { e.preventDefault(); zone.classList.add('over'); });
zone.addEventListener('dragleave', () => zone.classList.remove('over'));
zone.addEventListener('drop', (e) => {
	e.preventDefault();
	zone.classList.remove('over');
	const f = e.dataTransfer.files[0];
	if (f) setImage(f, cleanName(f.name));
});

const chars = await (await fetch('/api/characters')).json();
for (const c of chars) $('character').append(new Option(`${c.name} (${c.file})`, c.name));
$('character').addEventListener('change', () => {
	const c = chars.find((x) => x.name === $('character').value);
	if (c) setImage(c.url, c.name);
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
let timer = 0;
function rebuildSoon(ms = 250) { clearTimeout(timer); timer = setTimeout(rebuild, ms); }
function rebuild() {
	const front = source('front'), side = source('side');
	if (!front) return;
	status('baue …');
	setTimeout(async () => { // (not requestAnimationFrame: it never fires in a hidden tab)
		let r;
		try { r = build(front, side, S); } catch (e) { status('Fehler: ' + e.message); console.error(e); return; }
		if (current) { scene.remove(current.object); current.object.geometry.dispose(); }
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
		const bb = new THREE.Box3().setFromObject(r.object);
		controls.target.set(0, (bb.max.y + bb.min.y) / 2, 0);
		const i = r.info;
		$('info').textContent = `${i.tris} Dreiecke (roh ${i.rawTris}) · Gitter ${i.cells} · ${i.input} · ${i.ms.mask + i.ms.mesh + i.ms.rig} ms`;
		$('warn').replaceChildren(...i.warn.map((w) => Object.assign(document.createElement('li'), { textContent: w })));
		drawDebug(r);
		status('');
	}, 30);
}

// the silhouette on the grid, the regions, the joints
function drawDebug(r) {
	const { grid } = r;
	const c = $('maskCanvas');
	const k = 4;
	c.width = grid.w * k; c.height = grid.h * k;
	const g = c.getContext('2d');
	const lab = r.rig?.lm.label;
	const tint = [null, null, [255, 120, 80], [80, 160, 255], [255, 200, 60], [80, 220, 140]];
	for (let y = 0; y < grid.h; y++) for (let x = 0; x < grid.w; x++) {
		const i = y * grid.w + x;
		if (!grid.mask[i]) continue;
		let col = [grid.color[i * 3] * 255, grid.color[i * 3 + 1] * 255, grid.color[i * 3 + 2] * 255];
		const t = lab && tint[lab[i]];
		if (t) col = col.map((v, j) => v * 0.55 + t[j] * 0.45);
		g.fillStyle = `rgb(${col.map(Math.round).join(',')})`;
		g.fillRect(x * k, y * k, k, k);
	}
	if (!r.rig) return;
	const { T, arm, leg } = r.rig.lm;
	const line = (a, b) => { g.beginPath(); g.moveTo(a[0] * k, a[1] * k); g.lineTo(b[0] * k, b[1] * k); g.stroke(); };
	const dot = (p) => { g.beginPath(); g.arc(p[0] * k, p[1] * k, 4, 0, Math.PI * 2); g.fill(); };
	g.strokeStyle = '#111'; g.fillStyle = '#e53e3e'; g.lineWidth = 2;
	line(T.pelvis, T.spine); line(T.spine, T.chest); line(T.chest, T.neck); line(T.neck, T.head); line(T.head, T.top);
	for (const s of ['L', 'R']) {
		const a = arm[s], l = leg[s];
		line(T.chest, a.sh); line(a.sh, a.el); line(a.el, a.wr); line(a.wr, a.tip);
		line(T.pelvis, l.hip); line(l.hip, l.kn); line(l.kn, l.an); line(l.an, l.toe);
		[a.sh, a.el, a.wr, l.hip, l.kn, l.an].forEach(dot);
	}
	[T.pelvis, T.spine, T.chest, T.neck, T.head].forEach(dot);
}

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
	const name = $('name').value.trim() || 'figure';
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
if (chars.length && !img) { $('character').value = chars[0].name; setImage(chars[0].url, chars[0].name); }
