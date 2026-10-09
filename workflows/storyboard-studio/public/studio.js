// Storyboard Studio – the page. The layers of one chapter side by side (script → scene → shots → render → variants);
// a shot opened renders live with the frames' own renderer; every change goes back into the storyboard files through
// the local server (server.mjs). Vocabularies = workflows/scenes/storyboard.md.
import * as THREE from 'three';
import { renderShot } from '/render/core.js';
import { shotSpec } from '/render/spec.js';

const V = {
	format: ['viewport', 'vertical', 'loop360', 'strip'],
	aspect: ['9:16', '1:2', '1:3', '1:4', '1:5', '1:6', '1:7', '2:1', '3:1', '4:1', '6:1'],
	projection: ['perspective', 'top-down', 'isometric', 'polar'],
	move: ['hold', 'tilt-down', 'tilt-up', 'fall', 'climb', 'travel', 'descend', 'reveal', 'look-around', 'pass'],
	size: ['ecu', 'cu', 'mcu', 'medium', 'full', 'wide', 'ews'],
	angle: ['eye', 'high', 'low', 'bird', 'worm', 'dutch'],
	view: ['real', 'belief', 'narration', 'memory'],
	want: ['photo', 'direct', 'unreal', 'generated', 'art', 'schematic'],
	key: ['', 'left', 'right', 'top', 'bottom', 'front', 'back', 'top-left', 'top-right', 'bottom-left', 'bottom-right'],
	height: ['', 'below', 'low', 'eye', 'high', 'overhead'],
	source: ['', 'sun', 'moon', 'snow', 'window', 'door', 'candle', 'screen', 'neon', 'fire', 'lightning', 'flash', 'energy', 'none'],
	shadow: ['', 'long', 'short', 'none'],
	accent: ['', 'red', 'blue', 'green', 'amber', 'yellow'],
	pose: ['stand', 'walk', 'run', 'jump', 'fall', 'slip', 'crouch', 'kneel', 'crawl', 'sit', 'lie', 'reach', 'point', 'throw', 'cast', 'arms-up', 'hug-self', 'stamp', 'push', 'look-back', 'pray', 'roar'],
	facing: ['front', 'away', 'left', 'right', 'three-quarter-left', 'three-quarter-right'],
	framing: ['full', 'mcu', 'cu', 'ecu'],
	persona: ['', 'regular', 'human', 'cat', 'mountaintiger', 'biker'],
	mood: ['', 'acceptance', 'angry', 'bargaining', 'confused', 'crying', 'denial', 'disgusted', 'dizzy', 'embarrassed', 'furious', 'happy', 'laughing', 'neutral', 'sad', 'scared', 'smug', 'surprised', 'thinking']
};

const $ = (s) => document.querySelector(s);
const el = (tag, attrs = {}, ...kids) => {
	const n = document.createElement(tag);
	for (const [k, v] of Object.entries(attrs)) {
		if (k === 'on') for (const [ev, fn] of Object.entries(v)) n.addEventListener(ev, fn);
		else if (k === 'class') n.className = v;
		else if (v != null && v !== false) n.setAttribute(k, v === true ? '' : v);
	}
	for (const k of kids.flat()) if (k != null) n.append(k.nodeType ? k : String(k));
	return n;
};
/** replaceChildren without the nulls of conditional parts */
const fill = (node, ...kids) => node.replaceChildren(...kids.flat().filter((k) => k != null && k !== false));
const api = async (method, path, body) => {
	const r = await fetch(path, { method, headers: { 'content-type': 'application/json' }, body: body ? JSON.stringify(body) : undefined });
	const j = await r.json();
	if (!r.ok) throw new Error(j.error ?? r.statusText);
	return j;
};
const clone = (o) => JSON.parse(JSON.stringify(o));
const bust = () => `?t=${Date.now()}`;

const state = { toon: null, ch: null, data: null, sel: null, dirty: false };
const renderer = new THREE.WebGLRenderer({ antialias: false, preserveDrawingBuffer: true });

// --- chapter ------------------------------------------------------------------------------------------------------
async function loadChapters() {
	const list = await api('GET', '/api/chapters');
	const sel = $('#chapter');
	sel.replaceChildren(...list.map((c) => el('option', { value: `${c.toon}/${c.ch}` }, `${c.toon} · ${c.ch}`)));
	const want = localStorageGet('chapter') ?? `${list[0]?.toon}/${list[0]?.ch}`;
	sel.value = list.some((c) => `${c.toon}/${c.ch}` === want) ? want : sel.options[0]?.value;
	sel.onchange = () => { localStorageSet('chapter', sel.value); loadChapter(); };
	await loadChapter();
}
function localStorageGet(k) { try { return localStorage.getItem(`studio.${k}`); } catch { return null; } }
function localStorageSet(k, v) { try { localStorage.setItem(`studio.${k}`, v); } catch {} }

async function loadChapter(keepSel = false) {
	[state.toon, state.ch] = $('#chapter').value.split('/');
	state.data = await api('GET', `/api/chapter?toon=${state.toon}&ch=${state.ch}`);
	const t = state.data.summary.total;
	$('#summary').textContent = `${t.shots} Shots · ${t.screens.toFixed(1)} Screens · ${(t.seconds / 60).toFixed(1)} min · ${state.data.problems.length} Hinweise`;
	drawStrip();
	if (keepSel && state.sel) selectShot(state.sel.id, true);
	else if (!state.sel) drawScript(state.data.scenes[0]);
}

function drawStrip() {
	const strip = $('#strip');
	strip.replaceChildren();
	for (const [si, sc] of state.data.scenes.entries()) {
		strip.append(el('div', { class: 'scene-head', on: { click: () => drawScript(sc) } }, `${sc.sb.source?.scene} ${sc.sb.source?.heading ?? ''}`));
		for (const g of sc.sb.groups ?? []) for (const shot of g.shots ?? []) {
			const nv = state.data.variants[shot.id] ?? 0;
			strip.append(el('div', { class: `card${state.sel?.id === shot.id ? ' sel' : ''}`, 'data-id': shot.id, on: { click: () => selectShot(shot.id) } },
				el('img', { src: `/out/${state.ch}/frames/${shot.id}.png${bust()}`, alt: '', loading: 'lazy' }),
				el('div', {},
					el('div', { class: 'id' }, el('span', { class: `dot ${shot.review?.status ?? ''}` }), shot.id),
					el('div', { class: 'meta' }, `${shot.format} ${shot.aspect} · ${shot.view ?? '–'}${nv ? ` · ${nv} Var.` : ''}`),
					el('div', { class: 'meta' }, (shot.action?.de ?? '').slice(0, 70) + ((shot.action?.de ?? '').length > 70 ? '…' : '')))));
		}
	}
}

function drawScript(sc) {
	const box = $('#script');
	if (!sc) return box.replaceChildren();
	const s = sc.script;
	fill(box,
		el('p', { class: 'heading' }, `${sc.sb.source?.scene} ${sc.sb.source?.heading ?? ''}`),
		state.data.scriptError ? el('p', { class: 'problem' }, state.data.scriptError) : null,
		s ? el('pre', {}, s.lines.join('\n')) : el('p', { class: 'muted' }, 'Kein Skripttext für diese Szene.'),
		...(sc.sb.notes ?? []).map((n) => el('p', { class: 'problem' }, n))
	);
}

// --- a shot ---------------------------------------------------------------------------------------------------------
function findShot(id) {
	for (const sc of state.data.scenes) for (const g of sc.sb.groups ?? []) for (const shot of g.shots ?? []) if (shot.id === id) return { sc, shot };
	return null;
}

async function selectShot(id, keep = false) {
	if (state.dirty && !keep && !confirm('Ungespeicherte Änderungen verwerfen?')) return;
	const hit = findShot(id);
	if (!hit) return;
	state.sel = { id, sc: hit.sc, shot: clone(hit.shot) };
	state.dirty = false;
	document.body.classList.remove('dirty');
	$('#shot-title').textContent = id;
	history.replaceState(null, '', `?shot=${id}`);
	for (const c of document.querySelectorAll('.card')) c.classList.toggle('sel', c.dataset.id === id);
	drawScript(hit.sc);
	drawEditor();
	await preview();
	drawVariants();
}

let timer = null;
function changed() {
	state.dirty = true;
	document.body.classList.add('dirty');
	clearTimeout(timer);
	timer = setTimeout(preview, 220);
}

// the live preview: render at 1×, overlay the figures' boxes for dragging
let rendering = false, again = false;
async function preview() {
	if (!state.sel) return;
	if (rendering) { again = true; return; }
	rendering = true;
	const stage = $('#stage');
	let frame = stage.querySelector('.frame');
	if (!frame) { frame = el('div', { class: 'frame' }); stage.replaceChildren(frame); }
	frame.classList.add('busy');
	try {
		const spec = shotSpec(state.sel.sc.sb, state.sel.shot, 1);
		const { canvas, figures } = await renderShot(spec, { renderer, shadowSize: 2048 });
		const scale = Math.min(1, (stage.clientWidth - 24) / spec.w);
		canvas.style.width = `${(spec.w * scale) / 16}rem`;
		frame.replaceChildren(canvas, ...figures.map((f) => figHandle(f, spec, scale)));
	} catch (e) {
		frame.replaceChildren(el('p', { class: 'problem error' }, `Render: ${e.message}`));
	}
	frame.classList.remove('busy');
	rendering = false;
	if (again) { again = false; preview(); }
}

function figHandle(f, spec, scale) {
	const fig = state.sel.shot.figures[f.index];
	const b = f.box;
	const h = el('div', { class: 'fig', title: 'ziehen = Position · Mausrad = Größe · Alt + Mausrad = drehen' }, el('span', {}, `${fig.cast} · ${fig.pose ?? 'stand'}`));
	Object.assign(h.style, { left: `${(b.x0 * scale) / 16}rem`, top: `${(b.y0 * scale) / 16}rem`, width: `${((b.x1 - b.x0) * scale) / 16}rem`, height: `${((b.y1 - b.y0) * scale) / 16}rem` });
	h.addEventListener('pointerdown', (e) => {
		e.preventDefault();
		h.setPointerCapture(e.pointerId);
		h.classList.add('drag');
		const sx = e.clientX, sy = e.clientY, x0 = fig.x ?? 0.5, y0 = fig.y ?? 0.5;
		const move = (ev) => {
			fig.x = +(x0 + (ev.clientX - sx) / (spec.outW * scale)).toFixed(3);
			fig.y = +(y0 + (ev.clientY - sy) / (spec.outH * scale)).toFixed(3);
			h.style.transform = `translate(${(ev.clientX - sx) / 16}rem, ${(ev.clientY - sy) / 16}rem)`;
		};
		const up = () => { h.removeEventListener('pointermove', move); h.classList.remove('drag'); drawEditor(); changed(); };
		h.addEventListener('pointermove', move);
		h.addEventListener('pointerup', up, { once: true });
	});
	h.addEventListener('wheel', (e) => {
		e.preventDefault();
		if (e.altKey) fig.turn = (((fig.turn ?? { front: 0, away: 180, left: -90, right: 90, 'three-quarter-left': -40, 'three-quarter-right': 40 }[fig.facing] ?? 0) + Math.sign(e.deltaY) * 15) + 540) % 360 - 180;
		else fig.h = +Math.max(0.03, (fig.h ?? 0.3) * (e.deltaY < 0 ? 1.08 : 1 / 1.08)).toFixed(3);
		drawEditor();
		changed();
	}, { passive: false });
	return h;
}

// --- the description (form) ------------------------------------------------------------------------------------------
function field(label, value, onInput, kind = 'text', opts = {}) {
	let input;
	if (Array.isArray(kind)) {
		input = el('select', { on: { change: (e) => onInput(e.target.value) } }, ...kind.map((o) => el('option', { value: o, selected: o === (value ?? '') }, o || '–')));
	} else if (kind === 'range') {
		const out = el('span', { class: 'muted' }, value == null ? 'auto' : String(value));
		input = el('div', {}, el('input', { type: 'range', min: opts.min, max: opts.max, step: opts.step ?? 1, value: value ?? opts.dflt ?? 0, on: { input: (e) => { out.textContent = e.target.value; onInput(Number(e.target.value)); } } }), out);
	} else if (kind === 'area') {
		input = el('textarea', { on: { input: (e) => onInput(e.target.value) } }, value ?? '');
	} else if (kind === 'check') {
		input = el('input', { type: 'checkbox', checked: !!value, on: { change: (e) => onInput(e.target.checked) } });
	} else {
		input = el('input', { type: kind, value: value ?? '', step: opts.step, on: { input: (e) => onInput(kind === 'number' ? Number(e.target.value) : e.target.value) } });
	}
	return el('div', { class: 'row' }, el('label', {}, label), input);
}
const setOrDel = (o, k, v) => { if (v === '' || v == null) delete o[k]; else o[k] = v; };

function drawEditor() {
	const s = state.sel?.shot;
	const box = $('#editor');
	if (!s) return box.replaceChildren();
	s.shot ??= {};
	s.light ??= {};
	s.source ??= { type: 'schematic' };
	const sceneLight = state.sel.sc.sb.light ?? {};
	const castRefs = (state.sel.sc.sb.cast ?? []).map((c) => c.ref);
	const upd = (fn) => (v) => { fn(v); changed(); };
	box.replaceChildren(
		el('fieldset', {}, el('legend', {}, 'Was das Bild zeigt'),
			field('Aktion', s.action?.de, upd((v) => { s.action = { ...(s.action ?? {}), de: v }; }), 'area'),
			field('Prompt (en)', s.prompt?.en, upd((v) => setOrDel(s, 'prompt', v ? { en: v } : null)), 'area'),
			field('Perspektive', s.view, upd((v) => setOrDel(s, 'view', v)), V.view),
			field('Quelle', s.source.want, upd((v) => { s.source.want = v; }), V.want),
			field('Notiz', s.note, upd((v) => setOrDel(s, 'note', v)), 'area')),
		el('fieldset', {}, el('legend', {}, 'Format + Kamera'),
			field('Format', s.format, upd((v) => { s.format = v; }), V.format),
			field('Seitenverh.', s.aspect, upd((v) => { s.aspect = v; }), V.aspect),
			field('Projektion', s.projection, upd((v) => { s.projection = v; }), V.projection),
			field('Bewegung', s.move, upd((v) => { s.move = v; }), V.move),
			field('Größe', s.shot.size, upd((v) => { s.shot.size = v; }), V.size),
			field('Winkel', s.shot.angle, upd((v) => { s.shot.angle = v; }), V.angle),
			el('p', { class: 'muted' }, 'Kamera von Hand (leer = aus Größe / Winkel):'),
			...[['dist', 'Abstand m', 0.5, 40, 0.1], ['height', 'Höhe m', 0, 30, 0.1], ['pitch', 'Neigung °', -90, 60, 1], ['yaw', 'Schwenk °', -180, 180, 1], ['roll', 'Rollen °', -45, 45, 1], ['fov', 'Brennw. °', 15, 150, 1]].map(([k, l, min, max, step]) =>
				field(l, s.camera?.[k], upd((v) => { s.camera = { ...(s.camera ?? {}), [k]: v }; }), 'range', { min, max, step, dflt: { dist: 6, height: 1.55, pitch: 0, yaw: 0, roll: 0, fov: 50 }[k] })),
			el('button', { on: { click: () => { delete s.camera; drawEditor(); changed(); } } }, 'Kamera zurücksetzen')),
		el('fieldset', {}, el('legend', {}, `Licht (Szene: ${sceneLight.key ?? '–'} / ${sceneLight.height ?? '–'} / ${sceneLight.source ?? '–'})`),
			field('Richtung', s.light.key, upd((v) => setOrDel(s.light, 'key', v)), V.key),
			field('Höhe', s.light.height, upd((v) => setOrDel(s.light, 'height', v)), V.height),
			field('Quelle', s.light.source, upd((v) => setOrDel(s.light, 'source', v)), V.source),
			field('Schatten', s.light.shadow, upd((v) => setOrDel(s.light, 'shadow', v)), V.shadow),
			field('Randlicht', s.light.rim, upd((v) => setOrDel(s.light, 'rim', v || null)), 'check'),
			field('Azimut °', s.light.azimuth, upd((v) => { s.light.azimuth = v; }), 'range', { min: -180, max: 180, dflt: -45 }),
			field('Elevation °', s.light.elevation, upd((v) => { s.light.elevation = v; }), 'range', { min: -40, max: 90, dflt: 30 }),
			el('button', { on: { click: () => { delete s.light.azimuth; delete s.light.elevation; drawEditor(); changed(); } } }, 'Freie Richtung aus'),
			field('Akzent', s.light.accent?.color, upd((v) => setOrDel(s.light, 'accent', v ? { ...(s.light.accent ?? {}), color: v } : null)), V.accent),
			field('Licht-Notiz', s.light.note?.de, upd((v) => setOrDel(s.light, 'note', v ? { de: v } : null)), 'text')),
		el('fieldset', {}, el('legend', {}, 'Figuren'),
			...(s.figures ?? []).map((f, i) => el('div', { class: 'fig-box' },
				field('Figur', f.cast, upd((v) => { f.cast = v; }), castRefs),
				field('Persona', f.persona, upd((v) => setOrDel(f, 'persona', v)), V.persona),
				field('Pose', f.pose, upd((v) => { f.pose = v; }), V.pose),
				field('Blick', f.facing, upd((v) => { f.facing = v; delete f.turn; }), V.facing),
				field('Ausschnitt', f.framing, upd((v) => { f.framing = v; }), V.framing),
				field('Stimmung', f.mood, upd((v) => setOrDel(f, 'mood', v)), V.mood),
				field('x', f.x, upd((v) => { f.x = v; }), 'number', { step: 0.01 }),
				field('y', f.y, upd((v) => { f.y = v; }), 'number', { step: 0.01 }),
				field('Höhe (Screens)', f.h, upd((v) => { f.h = v; }), 'number', { step: 0.01 }),
				el('button', { on: { click: () => { s.figures.splice(i, 1); drawEditor(); changed(); } } }, 'Figur entfernen'))),
			el('button', { on: { click: () => { (s.figures ??= []).push({ cast: castRefs[0], pose: 'stand', facing: 'front', framing: 'full', x: 0.5, y: 0.6, h: 0.3 }); drawEditor(); changed(); } } }, '+ Figur')),
		el('fieldset', {}, el('legend', {}, 'Urteil'),
			el('p', { class: 'muted' }, s.review ? `${s.review.status} ${s.review.date ?? ''} ${s.review.note ?? ''}` : 'noch kein Urteil'),
			field('Notiz', '', (v) => { state.reviewNote = v; }, 'text'),
			field('Regel', '', (v) => { state.tasteRule = v; }, 'text'),
			el('div', { class: 'actions' },
				el('button', { on: { click: () => review('ok') } }, '✓ ok (→ Datensatz)'),
				el('button', { on: { click: () => review('change') } }, '✎ ändern'),
				el('button', { on: { click: addTaste } }, '+ Regel für taste.md'))),
		field('Varianten-Notiz', '', (v) => { state.variantNote = v; }, 'text'),
		el('div', { class: 'actions' },
			el('button', { class: 'primary', on: { click: saveShot } }, 'Speichern + Frame'),
			el('button', { on: { click: saveVariant } }, 'Als Variante'),
			el('button', { on: { click: () => { state.dirty = false; selectShot(state.sel.id, true); } } }, 'Verwerfen'),
			el('button', { on: { click: () => runCmd('refpacks', state.sel.id) } }, 'Referenzpaket'))
	);
}

// --- saving, variants, reviews ----------------------------------------------------------------------------------------
async function renderPng(shot) {
	const spec = shotSpec(state.sel.sc.sb, shot, 2);
	const { canvas } = await renderShot(spec, { renderer });
	const small = document.createElement('canvas');
	small.width = spec.outW; small.height = spec.outH;
	const g = small.getContext('2d');
	g.imageSmoothingQuality = 'high';
	g.drawImage(canvas, 0, 0, spec.outW, spec.outH);
	return small.toDataURL('image/png');
}
async function saveShot() {
	const r = await api('PUT', '/api/shot', { toon: state.toon, ch: state.ch, shot: state.sel.shot });
	await api('POST', '/api/frame', { toon: state.toon, ch: state.ch, id: state.sel.id, png: await renderPng(state.sel.shot) });
	state.dirty = false;
	document.body.classList.remove('dirty');
	await loadChapter(true);
	if (r.problems?.length) log(`Gespeichert. Hinweise:\n${r.problems.join('\n')}`);
}
async function saveVariant() {
	const note = state.variantNote ?? '';
	const { id: vid } = await api('POST', '/api/variants', { toon: state.toon, ch: state.ch, id: state.sel.id, shot: state.sel.shot, note });
	await api('POST', '/api/frame', { toon: state.toon, ch: state.ch, id: state.sel.id, variant: vid, png: await renderPng(state.sel.shot) });
	state.dirty = false;
	document.body.classList.remove('dirty');
	await loadChapter(true);
}
async function drawVariants() {
	const box = $('#variants');
	if (!state.sel) return box.replaceChildren();
	const list = await api('GET', `/api/variants?toon=${state.toon}&ch=${state.ch}&id=${state.sel.id}`);
	fill(box,
		el('div', { class: 'variant' }, el('img', { src: `/out/${state.ch}/frames/${state.sel.id}.png${bust()}`, alt: '' }), el('div', { class: 'note' }, el('strong', {}, 'aktuell'))),
		...list.map((v) => el('div', { class: 'variant' },
			el('img', { src: `/out/${state.ch}/variants/${state.sel.id}--${v.id}.png${bust()}`, alt: '' }),
			el('div', { class: 'note' }, el('strong', {}, v.id), ' ', v.note),
			el('div', { class: 'actions' },
				el('button', { on: { click: async () => { await api('POST', '/api/variants/pick', { toon: state.toon, ch: state.ch, id: state.sel.id, vid: v.id }); await rerenderFrame(); } } }, 'Diese nehmen'),
				el('button', { on: { click: () => { state.sel.shot = { ...clone(v.shot), id: state.sel.id }; drawEditor(); changed(); } } }, 'Laden')))),
		list.length ? null : el('p', { class: 'muted' }, 'Noch keine Varianten. Ändern → „Als Variante“.')
	);
}
async function rerenderFrame() {
	await loadChapter(true);
	const hit = findShot(state.sel.id);
	await api('POST', '/api/frame', { toon: state.toon, ch: state.ch, id: state.sel.id, png: await renderPng(hit.shot) });
	await selectShot(state.sel.id, true);
	drawStrip();
}
async function review(status) {
	if (state.dirty) await saveShot();
	const r = await api('POST', '/api/review', { toon: state.toon, ch: state.ch, id: state.sel.id, status, note: state.reviewNote ?? '' });
	log(r.out);
	await loadChapter(true);
}
async function addTaste() {
	const rule = (state.tasteRule ?? '').trim();
	if (!rule) return log('Erst die Regel ins Feld „Regel“ schreiben.');
	const quote = state.reviewNote ?? '';
	await api('POST', '/api/taste', { rule, quote });
	log('In workflows/scenes/storyboard-taste.md eingetragen.');
}

// --- scripts + problems ----------------------------------------------------------------------------------------------
function log(text) { $('#log-text').textContent = text; $('#log').showModal(); }
async function runCmd(cmd, only) {
	log(`${cmd} läuft …`);
	const r = await api('POST', '/api/run', { toon: state.toon, ch: state.ch, cmd, only });
	$('#log-text').textContent = r.out || (r.ok ? 'fertig' : 'Fehler');
	if (cmd === 'frames') { await loadChapter(true); }
}
for (const b of document.querySelectorAll('[data-run]')) b.addEventListener('click', () => runCmd(b.dataset.run));
$('#problems-btn').addEventListener('click', () => log(state.data.problems.map((p) => `${p.level === 'error' ? 'FEHLER ' : ''}${p.m}`).join('\n') || 'Keine Hinweise.'));
window.addEventListener('beforeunload', (e) => { if (state.dirty) e.preventDefault(); });

loadChapters()
	.then(() => { const id = new URLSearchParams(location.search).get('shot'); if (id) return selectShot(id); })
	.catch((e) => log(e.message));
// for tests / automation: the state and a way to know a render is done
window.studio = { state, preview: () => preview(), idle: () => !rendering };
