// The studio's state and its calls to the local API (api.mjs). One chapter; the CURRENT shot (`sel` – the one the panels
// edit, as a copy until it is saved, shared by every step) and the PICKED shots (`picked` – the shot list's multi-select,
// what the timeline shows; owner 2026-10-19: "the other timeline underneath the clip only shows those shots that are
// selected"); the selected figure in the 3D view; the messages as the shared toasts (DESIGN "Notifications").
import { confirmDialog, createToaster } from '@scutoons/ui';
import { shotSpec } from '../../../../tools/storyboard-render/spec.js';

export const VOCAB = {
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

export async function api(method, path, body) {
	const r = await fetch(path, { method, headers: { 'content-type': 'application/json' }, body: body ? JSON.stringify(body) : undefined });
	const j = await r.json();
	if (!r.ok) throw new Error(j.error ?? r.statusText);
	return j;
}
/** the studio's toasts (one ToastStack in App): transient info fades, an error stays until dismissed */
export const toaster = createToaster();
export const notify = (message, type = 'info') => message && toaster.addToast({ message, type });
export const fail = (e) => notify(e?.message ?? String(e), 'error');

const clone = (o) => JSON.parse(JSON.stringify(o));
const remember = (k, v) => { try { localStorage.setItem(`storyboard-studio:${k}`, v); } catch {} };
const recall = (k) => { try { return localStorage.getItem(`storyboard-studio:${k}`); } catch { return null; } };

export const studio = $state({
	chapters: [],
	/** "catpope/s1c3" */
	chapter: '',
	data: null,
	/** the current shot: { id, scene (the scene's storyboard), shot (the edited copy) } – the last one clicked */
	sel: null,
	/** the picked shots' ids in toon order – always with the current one (pickedShots) */
	picked: [],
	dirty: false,
	/** the figure picked in the 3D view (index) */
	figure: null,
	/** a counter the panels watch to reload (variants, generated) */
	version: 0
});

export const toon = () => studio.chapter.split('/')[0];
export const ch = () => studio.chapter.split('/')[1];

export async function loadChapters() {
	studio.chapters = await api('GET', '/api/chapters');
	const want = recall('chapter');
	const list = studio.chapters.map((c) => `${c.toon}/${c.ch}`);
	studio.chapter = list.includes(want) ? want : list[0] ?? '';
	await loadChapter();
	const id = new URLSearchParams(location.search).get('shot');
	if (id) select(id);
}

export async function loadChapter(keep = false) {
	if (!studio.chapter) return;
	remember('chapter', studio.chapter);
	studio.data = await api('GET', `/api/chapter?toon=${toon()}&ch=${ch()}`);
	if (keep && studio.sel) {
		const hit = find(studio.sel.id);
		if (hit && !studio.dirty) studio.sel = { id: hit.shot.id, scene: hit.scene, shot: clone(hit.shot) };
	}
	studio.version++;
}

export function find(id) {
	for (const sc of studio.data?.scenes ?? []) for (const g of sc.sb.groups ?? []) for (const shot of g.shots ?? []) if (shot.id === id) return { sc, scene: sc.sb, shot };
	return null;
}

/** the chapter's shots in toon order (the shot list's order – the shot stepper, the picked set) */
export function allShots() {
	const list = [];
	for (const sc of studio.data?.scenes ?? []) for (const g of sc.sb.groups ?? []) for (const shot of g.shots ?? []) list.push({ id: shot.id, shot });
	return list;
}

/** the picked shots' ids in toon order, the current one always among them (the timeline's clips) */
export function pickedShots() {
	const set = new Set(studio.picked);
	if (studio.sel) set.add(studio.sel.id);
	return allShots().filter((s) => set.has(s.id)).map((s) => s.id);
}

let anchor = null;
/** a click in the shot list (owner 2026-10-19: "the other timeline underneath the clip only shows those shots that are
 * selected"): plain = only that shot; `toggle` (Ctrl / ⌘) = in or out of the picked set; `range` (Shift) = the shots
 * from the last clicked one – none yet (a ?shot= link, the stepper): the current one – (with `toggle` added to the set).
 * The clicked shot becomes the current one (unsaved changes ask first – pickShot); one taken out of the set hands the
 * current to its neighbour in the set; the last one stays. */
export async function clickShot(id, { toggle = false, range = false } = {}) {
	const order = allShots().map((s) => s.id);
	const now = pickedShots();
	const from = order.includes(anchor) ? anchor : studio.sel?.id;
	let next;
	if (range && order.includes(from)) {
		const [a, b] = [order.indexOf(from), order.indexOf(id)].sort((x, y) => x - y);
		next = [...(toggle ? now : []), ...order.slice(a, b + 1)];
	} else if (toggle && now.includes(id)) {
		if (now.length === 1) return true;
		const rest = now.filter((x) => x !== id);
		if (id === studio.sel?.id && !(await pickShot(rest[Math.min(now.indexOf(id), rest.length - 1)]))) return false;
		studio.picked = rest;
		anchor = id;
		return true;
	} else next = [...(toggle ? now : []), id];
	if (!(await pickShot(id, order.filter((x) => next.includes(x))))) return false;
	// a range keeps where it started (the next Shift + click spans from there again)
	anchor = range && order.includes(from) ? from : id;
	return true;
}

/** pick a shot (the list, the timeline, the stepper): unsaved changes → an in-page confirmation first; true when `id`
 * is the current shot afterwards (false: kept the old one – the timeline puts its playhead back). A shot outside the
 * picked set becomes the only picked one – or `picked` is the new set (the shot list's multi-select). */
export async function pickShot(id, picked) {
	if (!id) return false;
	if (id === studio.sel?.id) {
		if (picked) studio.picked = picked;
		return true;
	}
	if (studio.dirty) {
		const ok = await confirmDialog({ title: 'Discard the changes?', message: `${studio.sel.id} has unsaved changes.`, confirmLabel: 'Discard' });
		if (!ok) return false;
	}
	return select(id, picked);
}

export function select(id, picked) {
	const hit = find(id);
	if (!hit) return false;
	studio.sel = { id, scene: hit.scene, shot: clone(hit.shot) };
	studio.picked = picked ?? (studio.picked.includes(id) ? studio.picked : [id]);
	studio.dirty = false;
	studio.figure = null;
	history.replaceState(null, '', `?shot=${id}`);
	return true;
}

/** the edited shot changed (a field, a drag in the 3D view) */
export function touched() {
	studio.dirty = true;
}

/** the render spec of the edited shot (ss = supersampling) */
export const specOf = (ss = 1) => shotSpec(studio.sel.scene, $state.snapshot(studio.sel.shot), ss);

export async function save(png) {
	const shot = $state.snapshot(studio.sel.shot);
	const r = await api('PUT', '/api/shot', { toon: toon(), ch: ch(), shot });
	if (png) await api('POST', '/api/frame', { toon: toon(), ch: ch(), id: shot.id, png });
	studio.dirty = false;
	await loadChapter(true);
	notify(r.problems?.length ? `Saved – ${r.problems.length} note(s): ${r.problems[0]}` : 'Saved.', r.problems?.length ? 'warning' : 'success');
}

export async function saveVariant(note, png) {
	const shot = $state.snapshot(studio.sel.shot);
	const { id: vid } = await api('POST', '/api/variants', { toon: toon(), ch: ch(), id: shot.id, shot, note });
	if (png) await api('POST', '/api/frame', { toon: toon(), ch: ch(), id: shot.id, variant: vid, png });
	studio.dirty = false;
	await loadChapter(true);
	notify(`Kept as variant ${vid}.`, 'success');
}

export function revert() {
	const hit = find(studio.sel.id);
	studio.sel = { ...studio.sel, shot: clone(hit.shot) };
	studio.dirty = false;
}
// the chapter scripts (/api/run): actions.svelte.js runScript – "Running …" where the doing happens, the result one toast
