// The handheld camera on the PC (README "Handheld camera", research/HANDHELD-CAMERA.md): the pairing with the phone,
// the live pose into the 3D view (smoothed), recording takes, the selected shot's takes file (on, active take, trims).
// The phone does the origin maths and sends the camera pose in the scene's world; the PC smooths, shows and records.
import { api, ch, specOf, studio, toon } from '../studio.svelte.js';
import { connect, newCode } from './link.js';
import { handheldView } from './view.js';
import { packSample, sampleAt, smooth, smoother } from './track.js';

export const hh = $state({
	code: '',
	/** the phone page joined (its user agent) */
	phone: null,
	/** what the phone reports: { mode: 'xr' | 'orientation' | '', tracking, origin } */
	phoneState: {},
	/** live = the phone's pose drives the view */
	live: false,
	smooth: 0,
	scale: 1,
	recording: false,
	recSeconds: 0,
	/** the selected shot's takes file { on, active, takes } (null while it loads) */
	doc: null,
	docShot: null,
	/** the trim preview: the playhead (s, of the whole take) */
	playhead: 0,
	playing: false
});

let link = null;
let filter = null;
let rec = null; // { t0, samples }
let lastSpec = '';

// the code survives a reload of the PC page (sessionStorage): the phone keeps its link
const KEY = 'storyboard-studio:handheld-code';
const stored = () => { try { return sessionStorage.getItem(KEY); } catch { return null; } };
export function pair(code = stored() ?? newCode()) {
	if (link) return;
	hh.code = code;
	try { sessionStorage.setItem(KEY, code); } catch {}
	link = connect(hh.code, 'pc', onMessage);
	sendSpec(true);
}
/** after a reload: pair again with the same code */
export function resume() {
	if (!link && stored()) pair();
}
export function unpair() {
	try { sessionStorage.removeItem(KEY); } catch {}
	link?.close();
	link = null;
	hh.code = '';
	hh.phone = null;
	stopLive();
}

function onMessage(m) {
	if (m.type === 'peer') { hh.phone = { ua: m.ua ?? '' }; sendSpec(true); sendState(); }
	else if (m.type === 'left') { hh.phone = null; hh.phoneState = {}; stopLive(); }
	else if (m.type === 'state') hh.phoneState = { mode: m.mode, tracking: m.tracking, origin: m.origin };
	else if (m.type === 'rec') (m.on ? startRec() : stopRec());
	else if (m.type === 'pose') onPose(m);
}

/** the shot the phone shows: the edited copy's render spec (unsaved edits included) */
export function sendSpec(force = false) {
	if (!link || !studio.sel || !hh.doc?.on) return;
	const spec = specOf(1);
	const key = JSON.stringify(spec);
	if (!force && key === lastSpec) return;
	lastSpec = key;
	link.send('spec', { shot: studio.sel.id, spec });
}
function sendState() {
	link?.send('state', { recording: hh.recording, shot: studio.sel?.id ?? null, scale: hh.scale, on: !!hh.doc?.on });
}
export const setScale = (v) => { hh.scale = v; sendState(); };
export const setOrigin = () => link?.send('origin');

function onPose(m) {
	if (!studio.sel || m.shot !== studio.sel.id || hh.playing) return;
	const s = { t: m.t / 1000, position: m.position, quaternion: m.quaternion, fov: m.fov };
	if (!hh.live) { hh.live = true; filter = smoother(hh.smooth); }
	handheldView.api?.pose(filter.step(s));
	if (rec) {
		rec.t0 ??= s.t;
		rec.samples.push(packSample({ ...s, t: s.t - rec.t0 }));
		hh.recSeconds = s.t - rec.t0;
	}
}
export function setSmooth(v) {
	hh.smooth = v;
	filter = smoother(v);
}
export function stopLive() {
	if (rec) stopRec();
	hh.live = false;
	handheldView.api?.release();
}

export function startRec() {
	if (rec || !hh.doc?.on) return;
	rec = { t0: null, samples: [] };
	hh.recording = true;
	hh.recSeconds = 0;
	sendState();
}
export async function stopRec() {
	if (!rec) return;
	const { samples } = rec;
	rec = null;
	hh.recording = false;
	sendState();
	if (samples.length < 2) { studio.message = 'Nothing recorded – is the phone connected and tracking?'; return; }
	const n = hh.doc.takes.reduce((m, t) => Math.max(m, Number(t.id.slice(1)) || 0), 0) + 1;
	const take = { id: `t${n}`, created: new Date().toISOString(), smooth: hh.smooth, scale: hh.scale, in: 0, out: samples[samples.length - 1].t, samples };
	hh.doc.takes.push(take);
	hh.doc.active = take.id;
	await saveDoc();
	mark();
	studio.message = `Take ${take.id}: ${take.out.toFixed(1)} s, ${samples.length} samples.`;
}

// --- the selected shot's takes file ---------------------------------------------------------------------------------
export async function loadDoc(id) {
	hh.doc = null;
	hh.docShot = id;
	const doc = await api('GET', `/api/takes?toon=${toon()}&ch=${ch()}&id=${id}`);
	if (hh.docShot !== id) return;
	hh.doc = doc;
	hh.playhead = 0;
	sendSpec(true);
	sendState();
}
let saveTimer;
export function saveDoc(later = false) {
	clearTimeout(saveTimer);
	const id = hh.docShot, doc = $state.snapshot(hh.doc);
	const go = () => api('PUT', '/api/takes', { toon: toon(), ch: ch(), id, doc }).catch((e) => (studio.message = e.message));
	if (later) { saveTimer = setTimeout(go, 400); return Promise.resolve(); }
	return go();
}
export async function setOn(on) {
	hh.doc.on = on;
	if (!on) stopLive();
	await saveDoc();
	mark();
	sendSpec(true);
	sendState();
}
/** the shot list's marker (studio.data.handheld: { id: takes }) */
function mark() {
	if (!studio.data) return;
	const m = { ...(studio.data.handheld ?? {}) };
	if (hh.doc.on) m[hh.docShot] = hh.doc.takes.length;
	else delete m[hh.docShot];
	studio.data.handheld = m;
}
export const activeTake = () => hh.doc?.takes.find((t) => t.id === hh.doc.active) ?? null;

// --- trim preview ---------------------------------------------------------------------------------------------------
let cache = { key: '', samples: [] };
/** the take smoothed with its own smoothing value (cached per take + value) */
export function smoothed(take) {
	const key = `${take.id}/${take.smooth}/${take.samples.length}`;
	if (cache.key !== key) cache = { key, samples: smooth(take.samples, take.smooth ?? 0) };
	return cache.samples;
}
export function preview(t) {
	const take = activeTake();
	if (!take) return;
	hh.playhead = t;
	const s = sampleAt(smoothed(take), t);
	if (s) handheldView.api?.pose(s);
}
let raf = 0;
export function play() {
	const take = activeTake();
	if (!take || hh.playing) return;
	hh.playing = true;
	const from = hh.playhead >= take.out - 0.01 ? take.in : Math.max(take.in, hh.playhead);
	const start = performance.now();
	const tick = () => {
		const t = from + (performance.now() - start) / 1000;
		if (!hh.playing || t >= take.out) { hh.playing = false; preview(Math.min(t, take.out)); return; }
		preview(t);
		raf = requestAnimationFrame(tick);
	};
	raf = requestAnimationFrame(tick);
}
export function pause() {
	hh.playing = false;
	cancelAnimationFrame(raf);
}
export function endPreview() {
	pause();
	handheldView.api?.release();
}
