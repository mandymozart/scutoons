// The studio's state and its calls to the local API (api.mjs). One chapter, one selected shot (edited as a copy until
// it is saved), the selected figure in the 3D view.
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
const clone = (o) => JSON.parse(JSON.stringify(o));
const remember = (k, v) => { try { localStorage.setItem(`storyboard-studio:${k}`, v); } catch {} };
const recall = (k) => { try { return localStorage.getItem(`storyboard-studio:${k}`); } catch { return null; } };

export const studio = $state({
	chapters: [],
	/** "catpope/s1c3" */
	chapter: '',
	data: null,
	/** the selected shot: { id, scene (the scene's storyboard), shot (the edited copy) } */
	sel: null,
	dirty: false,
	/** the figure picked in the 3D view (index) */
	figure: null,
	/** a counter the panels watch to reload (variants, generated) */
	version: 0,
	/** the look of the renders (core.js sinPass): solid black and white by default; hatch / tone, colour, inner lines, wobble */
	look: { shade: 'solid', color: false, inner: false, wobble: 0 },
	/** the last message (a status line) */
	message: ''
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

export function select(id) {
	const hit = find(id);
	if (!hit) return;
	studio.sel = { id, scene: hit.scene, shot: clone(hit.shot) };
	studio.dirty = false;
	studio.figure = null;
	history.replaceState(null, '', `?shot=${id}`);
}

/** the edited shot changed (a field, a drag in the 3D view) */
export function touched() {
	studio.dirty = true;
}

/** the render spec of the edited shot (ss = supersampling) */
export const specOf = (ss = 1) => shotSpec(studio.sel.scene, $state.snapshot(studio.sel.shot), ss, $state.snapshot(studio.look));

export async function save(png) {
	const shot = $state.snapshot(studio.sel.shot);
	const r = await api('PUT', '/api/shot', { toon: toon(), ch: ch(), shot });
	if (png) await api('POST', '/api/frame', { toon: toon(), ch: ch(), id: shot.id, png });
	studio.dirty = false;
	await loadChapter(true);
	studio.message = r.problems?.length ? `Saved – ${r.problems.length} note(s): ${r.problems[0]}` : 'Saved.';
}

export async function saveVariant(note, png) {
	const shot = $state.snapshot(studio.sel.shot);
	const { id: vid } = await api('POST', '/api/variants', { toon: toon(), ch: ch(), id: shot.id, shot, note });
	if (png) await api('POST', '/api/frame', { toon: toon(), ch: ch(), id: shot.id, variant: vid, png });
	studio.dirty = false;
	await loadChapter(true);
	studio.message = `Kept as variant ${vid}.`;
}

export function revert() {
	const hit = find(studio.sel.id);
	studio.sel = { ...studio.sel, shot: clone(hit.shot) };
	studio.dirty = false;
}

export async function run(cmd, only) {
	studio.message = `${cmd} …`;
	const r = await api('POST', '/api/run', { toon: toon(), ch: ch(), cmd, only });
	studio.message = (r.out || (r.ok ? 'done' : 'failed')).split('\n').filter(Boolean).slice(-1)[0];
	if (cmd === 'frames') await loadChapter(true);
	return r;
}
