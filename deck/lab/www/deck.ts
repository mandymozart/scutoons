// deck.html (#197, the dev harness since #198 – the reader's /live/deck is the real page): a phone joins the Cathedral
// of decks as a being. A thin page over @scutoons/live-deck (the same Deck and SpaceStage the viewer uses): it joins the
// relay at once (`?relay=`, default ws://<this host>:7070/live/CATH); Start (the AudioContext needs a gesture) starts
// the sound; the pads and sliders are the being's exposed parameters; `?space=1` draws the being in 3D above them
// (deck/space-web's wasm, copied into dist/ by ts/build.ts).
import { charactersOf, Deck, loadDeckScene, SpaceStage, toPos, toValue } from '../../../shared/live-deck/src/index.ts';

const API = 'https://api.scutoons.com/api';
const CDN = 'https://cdn.scutoons.com/';
const q = new URLSearchParams(location.search);
const relay = q.get('relay') ?? `ws://${location.hostname || 'localhost'}:7070/live/CATH`;
const $ = <T extends HTMLElement>(s: string) => document.querySelector(s) as T;
const store = {
	get: (k: string) => {
		try {
			return localStorage.getItem(k);
		} catch {
			return null;
		}
	},
	set: (k: string, v: string) => {
		try {
			localStorage.setItem(k, v);
		} catch {
			/* private mode */
		}
	}
};

const character = q.get('character') ?? store.get('deck.character') ?? 'yellowmouse';
const deck = new Deck({ relay, character, nick: `browser ${character}`, api: API, onStatus: (s) => ($('#status').textContent = `${s} · ${relay}`) });
(window as unknown as { __deck: unknown }).__deck = deck;
let space: SpaceStage | null = null;
const SPACE_WEB = new URL('./space-web/', import.meta.url).href;

function showFace() {
	$('#mood').textContent = deck.mood;
	const p = deck.portraits[deck.mood] ?? deck.portraits.neutral ?? Object.values(deck.portraits)[0];
	const img = $<HTMLImageElement>('#face');
	const src = p ? CDN + p.path : '';
	if (img.dataset.src !== src) {
		img.dataset.src = src;
		if (src) img.src = src;
		else img.removeAttribute('src');
	}
}
deck.onMood = showFace;

async function pick(id: string) {
	store.set('deck.character', id);
	await deck.pickCharacter(id);
	space?.setBeing();
	showFace();
}

async function loadCharacters() {
	const sel = $<HTMLSelectElement>('#character');
	for (const c of await charactersOf(API, character)) sel.add(new Option(c.name?.en ?? c.id, c.id, false, c.id === character));
	sel.addEventListener('change', () => pick(sel.value));
	await pick(character);
	if (q.get('space') === '1') {
		const host = $('#space');
		host.hidden = false;
		space = await SpaceStage.start({ host, base: SPACE_WEB, deck, api: API, cdn: CDN, prefer: q.get('gl') === '1' ? 'webgl2' : 'auto', onStatus: (s) => ($('#stats').textContent = s) }).catch((e) => {
			$('#stats').textContent = `3D: ${(e as Error).message}`;
			return null;
		});
	}
}

// ── the controls: they and the being's buttons turn the deck's one state ──
const follow = new Map<string, () => void>();
deck.onValue = (path) => follow.get(path)?.();

function buildSliders() {
	const box = $('#sliders');
	const onPads = new Set([...document.querySelectorAll<HTMLElement>('.pad')].flatMap((p) => [p.dataset.x!, p.dataset.y!]));
	for (const [path, s] of Object.entries(deck.params)) {
		if (onPads.has(path)) continue;
		const label = Object.assign(document.createElement('label'), { textContent: s.label, htmlFor: path });
		const input = Object.assign(document.createElement('input'), { type: 'range', id: path, min: '0', max: '1', step: '0.001' });
		const out = document.createElement('output');
		const show = () => {
			input.value = String(toPos(s, deck.value[path]));
			out.textContent = deck.value[path].toFixed(deck.value[path] >= 10 ? 0 : 2);
		};
		input.addEventListener('input', () => deck.setParam(path, toValue(s, Number(input.value))));
		follow.set(path, show);
		show();
		box.append(label, input, out);
	}
}

function bindPad(pad: HTMLElement) {
	const [px, py] = [pad.dataset.x!, pad.dataset.y!];
	const PARAMS = deck.params;
	if (!(px in PARAMS && py in PARAMS)) return void (pad.hidden = true);
	const dot = pad.querySelector('i')!;
	const place = () => {
		dot.style.left = `${toPos(PARAMS[px], deck.value[px]) * 100}%`;
		dot.style.top = `${(1 - toPos(PARAMS[py], deck.value[py])) * 100}%`;
	};
	const move = (e: PointerEvent) => {
		const r = pad.getBoundingClientRect();
		const clamp = (t: number) => Math.min(1, Math.max(0, t));
		deck.setParam(px, toValue(PARAMS[px], clamp((e.clientX - r.left) / r.width)));
		deck.setParam(py, toValue(PARAMS[py], clamp(1 - (e.clientY - r.top) / r.height)));
	};
	pad.addEventListener('pointerdown', (e) => {
		pad.setPointerCapture(e.pointerId);
		pad.classList.add('on');
		deck.touch = 1;
		move(e);
	});
	pad.addEventListener('pointermove', (e) => {
		if (pad.hasPointerCapture(e.pointerId)) move(e);
	});
	const up = () => {
		pad.classList.remove('on');
		deck.touch = 0;
	};
	pad.addEventListener('pointerup', up);
	pad.addEventListener('pointercancel', up);
	follow.set(px, place);
	follow.set(py, place);
	place();
}

// ── the bands meter ──
function drawBands() {
	const c = $<HTMLCanvasElement>('#bands');
	const w = (c.width = c.clientWidth * devicePixelRatio);
	const h = (c.height = c.clientHeight * devicePixelRatio);
	const g = c.getContext('2d')!;
	const b = deck.bands;
	const vals = [b.level, b.bass, b.mid, b.high, deck.moods.energy];
	const colors = ['#e8e8e8', '#ff7a59', '#ffd166', '#5ec8ff', '#00d1ff'];
	const names = ['level', 'bass', 'mid', 'high', 'energy'];
	const bw = w / vals.length;
	g.font = `${Math.round(h * 0.16)}px system-ui, sans-serif`;
	vals.forEach((v, i) => {
		const bh = Math.min(1, v) * h * 0.78;
		g.fillStyle = colors[i];
		g.fillRect(i * bw + bw * 0.1, h * 0.8 - bh, bw * 0.8, bh);
		g.fillStyle = '#999';
		g.fillText(names[i], i * bw + bw * 0.1, h * 0.97);
	});
}

function frame() {
	try {
		const now = performance.now() / 1000;
		deck.tick(now);
		space?.frame(now);
		drawBands();
		const core = deck.core;
		if (!space)
			$('#stats').textContent =
				`${deck.link?.client || '–'} · sent ${deck.link?.count.graph ?? 0} graph / ${deck.link?.count.values ?? 0} values` +
				(core ? ` · ${core.stats().transport} · isolated ${self.crossOriginIsolated} · underruns ${core.stats().underruns}` : '');
	} finally {
		requestAnimationFrame(frame);
	}
}

$('#start').addEventListener('click', async () => {
	const btn = $<HTMLButtonElement>('#start');
	btn.disabled = true;
	btn.textContent = 'starting…';
	try {
		await deck.start({
			worker: new Worker(new URL('./live-worker.js', import.meta.url), { type: 'module' }),
			wasm: new URL('./live_core.wasm', import.meta.url),
			worklet: new URL('./live-worklet.js', import.meta.url),
			transport: (q.get('transport') as 'shared' | 'transfer' | null) ?? 'auto',
			lead: Number(q.get('lead') ?? 2048)
		});
		btn.hidden = true;
	} catch (e) {
		btn.disabled = false;
		btn.textContent = 'Start';
		$('#status').textContent = `failed: ${(e as Error).message}`;
	}
});
// #360: the deck's own scene from the core (space-web's `deck_scene`, the X1's cathedral) – its knobs are the sliders
loadDeckScene(SPACE_WEB)
	.then((own) => {
		deck.useOwn(own);
		buildSliders();
		document.querySelectorAll<HTMLElement>('.pad').forEach(bindPad);
	})
	.catch((e) => ($('#status').textContent = `no scene: ${(e as Error).message}`));
loadCharacters();
requestAnimationFrame(frame);
