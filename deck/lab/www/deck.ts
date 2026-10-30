// deck.html (#197): a phone joins the Cathedral of decks as a being. It joins the relay at once (`?relay=`, default
// ws://<this host>:7070/live/CATH) and streams its being; Start (the AudioContext needs a gesture) starts live-core
// with the trimmed cathedral deck (deck-scene.ts), the page's sequencer, LFO and noise. The pads and sliders are the
// being's exposed parameters; the head wears the character's mood sprite (the API's scene-content, #195).
import { LiveCore, type Analysis } from '../../../shared/live-core/src/index.ts';
import { DeckLink, ValueStream } from './deck-client.ts';
import { deckGraph, ENGINE_TOML, HELD_NOTE, ladderFor, midiHz, Moods, PARAMS, SEQ, STREAMED, toPos, toValue } from './deck-scene.ts';

const API = 'https://api.scutoons.com/api';
const CDN = 'https://cdn.scutoons.com/';
const q = new URLSearchParams(location.search);
const relay = q.get('relay') ?? `ws://${location.hostname || 'localhost'}:7070/live/CATH`;
const $ = <T extends HTMLElement>(s: string) => document.querySelector(s) as T;
const store = {
	get: (k: string) => { try { return localStorage.getItem(k); } catch { return null; } },
	set: (k: string, v: string) => { try { localStorage.setItem(k, v); } catch { /* private mode */ } }
};

// ── state: the parameter values, the sequencer, the LFO, what the analysis said ──
const value: Record<string, number> = Object.fromEntries(Object.entries(PARAMS).map(([k, s]) => [k, s.value]));
let character = q.get('character') ?? store.get('deck.character') ?? 'yellowmouse';
let portraits: Record<string, { path: string }> = {};
const moods = new Moods(['neutral']);
let mood = 'neutral';
let bands: Analysis = { level: 0, bass: 0, mid: 0, high: 0, spectrum: new Float32Array(0) };
let core: LiveCore | null = null;
let noise: { filter: BiquadFilterNode; gain: GainNode } | null = null;
let gate = 0;
let note = 0;
let swayPhase = 0;
let sway = 0;
let touch = 0;
const status = (s: string) => ($('#status').textContent = `${s} · ${relay}`);

const link = new DeckLink({ relay, graph: deckGraph(character, ['neutral']), nick: `browser ${character}`.slice(0, 24), onStatus: status });
const stream = new ValueStream(STREAMED);
(window as unknown as { __deck: unknown }).__deck = { link, value, stream };

// ── the head: characters from the API, the mood sprite ──
async function pickCharacter(id: string) {
	character = id;
	store.set('deck.character', id);
	try {
		const r = await fetch(`${API}/library/scene-content?characters=${encodeURIComponent(id)}`);
		portraits = (await r.json()).characters?.[id]?.portraits ?? {};
	} catch {
		portraits = {};
	}
	const have = Object.keys(portraits);
	moods.moods = ladderFor(have);
	link.setGraph(deckGraph(id, [...moods.moods, ...have.filter((m) => !moods.moods.includes(m))]));
	showFace(true);
}

function showFace(force = false) {
	$('#mood').textContent = mood;
	const p = portraits[mood] ?? portraits.neutral ?? Object.values(portraits)[0];
	const img = $<HTMLImageElement>('#face');
	const src = p ? CDN + p.path : '';
	if (force || img.dataset.src !== src) {
		img.dataset.src = src;
		if (src) img.src = src;
		else img.removeAttribute('src');
	}
}

async function loadCharacters() {
	const sel = $<HTMLSelectElement>('#character');
	let list: { id: string; name?: Record<string, string> }[] = [];
	try {
		list = (await (await fetch(`${API}/library/characters`)).json()).characters ?? [];
	} catch {
		/* offline: the current one only */
	}
	if (!list.some((c) => c.id === character)) list.unshift({ id: character });
	for (const c of list) sel.add(new Option(c.name?.en ?? c.id, c.id, false, c.id === character));
	sel.addEventListener('change', () => pickCharacter(sel.value));
	await pickCharacter(character);
}

// ── the controls ──
function setParam(path: string, v: number) {
	value[path] = v;
	if (!core) return;
	if (path === 'voice.ratio' || path === 'voice.index') core.set(path, [v]);
	else if (path === 'voice.gain') core.set('vgain.b', [v]);
	else if (path === 'drone.freq' || path === 'drone.gain') {
		core.set('drone', [value['drone.freq'], value['drone.gain']]);
		core.set('fifth', [value['drone.freq'] * 1.5, value['drone.gain'] * 0.3]);
	} else if (path === 'incense.gain' && noise) noise.gain.gain.setTargetAtTime(v, noise.gain.context.currentTime, 0.05);
}

function buildSliders() {
	const box = $('#sliders');
	const onPads = new Set([...document.querySelectorAll<HTMLElement>('.pad')].flatMap((p) => [p.dataset.x!, p.dataset.y!]));
	for (const [path, s] of Object.entries(PARAMS)) {
		if (onPads.has(path)) continue;
		const label = Object.assign(document.createElement('label'), { textContent: s.label, htmlFor: path });
		const input = Object.assign(document.createElement('input'), { type: 'range', id: path, min: '0', max: '1', step: '0.001' });
		input.value = String(toPos(s, s.value));
		const out = document.createElement('output');
		const show = () => (out.textContent = value[path].toFixed(value[path] >= 10 ? 0 : 2));
		input.addEventListener('input', () => { setParam(path, toValue(s, Number(input.value))); show(); });
		show();
		box.append(label, input, out);
	}
}

function bindPad(pad: HTMLElement) {
	const [px, py] = [pad.dataset.x!, pad.dataset.y!];
	const dot = pad.querySelector('i')!;
	const place = () => {
		dot.style.left = `${toPos(PARAMS[px], value[px]) * 100}%`;
		dot.style.top = `${(1 - toPos(PARAMS[py], value[py])) * 100}%`;
	};
	const move = (e: PointerEvent) => {
		const r = pad.getBoundingClientRect();
		const clamp = (t: number) => Math.min(1, Math.max(0, t));
		setParam(px, toValue(PARAMS[px], clamp((e.clientX - r.left) / r.width)));
		setParam(py, toValue(PARAMS[py], clamp(1 - (e.clientY - r.top) / r.height)));
		place();
	};
	pad.addEventListener('pointerdown', (e) => { pad.setPointerCapture(e.pointerId); pad.classList.add('on'); touch = 1; move(e); });
	pad.addEventListener('pointermove', (e) => { if (pad.hasPointerCapture(e.pointerId)) move(e); });
	const up = () => { pad.classList.remove('on'); touch = 0; };
	pad.addEventListener('pointerup', up);
	pad.addEventListener('pointercancel', up);
	place();
}

// ── the bands meter ──
function drawBands() {
	const c = $<HTMLCanvasElement>('#bands');
	const w = (c.width = c.clientWidth * devicePixelRatio);
	const h = (c.height = c.clientHeight * devicePixelRatio);
	const g = c.getContext('2d')!;
	const vals = [bands.level, bands.bass, bands.mid, bands.high, moods.energy];
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

// ── the frame: sequencer, LFO, mood, stream ──
const audioPeak = (x: number) => Math.min(1, Math.max(0, x));
const read = (p: string): number => {
	switch (p) {
		case 'seq.cv': return gate ? note / 127 : 0;
		case 'voice.out': return audioPeak(gate * value['voice.gain'] * 4);
		case 'drone.out': return audioPeak(value['drone.gain'] * 3);
		case 'incense.out': return audioPeak(value['incense.gain'] * 5 * (0.4 + 0.6 * toPos(PARAMS['incense.cutoff'], cutoff())));
		case 'mix.out': return audioPeak(bands.level);
		case 'sway.out': return sway;
		case 'fft.level': return bands.level;
		case 'fft.bass': return bands.bass;
		case 'incense.cutoff': return cutoff();
		default: return value[p] ?? NaN;
	}
};
const cutoff = () => Math.min(12000, value['incense.cutoff'] * 2 ** (sway * 1.5));

const SEQ_ON = new URLSearchParams(location.search).get('seq') === '1';
let lastStep = -1;
let last = performance.now() / 1000;
function frame() {
	try {
		tick();
	} finally {
		requestAnimationFrame(frame);
	}
}

function tick() {
	const now = performance.now() / 1000;
	const dt = Math.min(0.1, now - last);
	last = now;
	swayPhase = (swayPhase + value['sway.rate'] * dt) % 1;
	sway = Math.sin(swayPhase * 2 * Math.PI) * value['sway.depth'];
	if (core && !SEQ_ON) {
		if (!gate) {
			note = HELD_NOTE;
			core.set('voice.freq', [midiHz(note)]);
			gate = 1;
			core.set('env', [1]);
		}
		noise?.filter.frequency.setTargetAtTime(cutoff(), core.context.currentTime, 0.03);
	} else if (core) {
		const stepDur = 60 / SEQ.bpm / SEQ.division;
		const t = core.context.currentTime;
		const step = Math.floor(t / stepDur);
		if (step !== lastStep) {
			lastStep = step;
			note = SEQ.steps[step % SEQ.steps.length];
			if (note > 0) core.set('voice.freq', [midiHz(note)]);
			gate = note > 0 ? 1 : 0;
			core.set('env', [gate]);
		} else if (gate && t - step * stepDur > stepDur * SEQ.gate) {
			gate = 0;
			core.set('env', [0]);
		}
		noise?.filter.frequency.setTargetAtTime(cutoff(), core.context.currentTime, 0.03);
	}
	const m = moods.update(now, dt, bands.level, touch);
	if (m !== mood) { mood = m; showFace(); }
	const changes = stream.sample(now, read, mood);
	if (changes) link.send(changes);
	drawBands();
	$('#stats').textContent = `${link.client || '–'} · sent ${link.count.graph} graph / ${link.count.values} values` +
		(core ? ` · ${core.stats().transport} · isolated ${self.crossOriginIsolated} · underruns ${core.stats().underruns}` : '');
}

// ── Start: live-core + the noise ──
async function start() {
	const btn = $<HTMLButtonElement>('#start');
	btn.disabled = true;
	btn.textContent = 'starting…';
	try {
		const c = await LiveCore.start({
			worker: new Worker(new URL('./live-worker.js', import.meta.url), { type: 'module' }),
			wasm: new URL('./live_core.wasm', import.meta.url),
			worklet: new URL('./live-worklet.js', import.meta.url),
			transport: (q.get('transport') as 'shared' | 'transfer' | null) ?? 'auto',
			lead: Number(q.get('lead') ?? 2048)
		});
		await c.load(ENGINE_TOML);
		c.onAnalysis((a) => (bands = a));
		const ctx = c.context;
		const buf = ctx.createBuffer(1, ctx.sampleRate * 2, ctx.sampleRate);
		const d = buf.getChannelData(0);
		let b0 = 0, b1 = 0, b2 = 0;
		for (let i = 0; i < d.length; i++) {
			// pink-ish: three leaky integrators of white noise (cathedral.toml's incense is pink)
			const w = Math.random() * 2 - 1;
			b0 = 0.99765 * b0 + w * 0.099; b1 = 0.963 * b1 + w * 0.2965; b2 = 0.57 * b2 + w * 1.0527;
			d[i] = (b0 + b1 + b2 + w * 0.1848) * 0.2;
		}
		const src = Object.assign(ctx.createBufferSource(), { buffer: buf, loop: true });
		const filter = Object.assign(ctx.createBiquadFilter(), { type: 'lowpass' as BiquadFilterType });
		const gain = ctx.createGain();
		gain.gain.value = value['incense.gain'];
		src.connect(filter).connect(gain).connect(ctx.destination);
		src.start();
		noise = { filter, gain };
		core = c;
		for (const p of Object.keys(PARAMS)) setParam(p, value[p]);
		btn.hidden = true;
	} catch (e) {
		btn.disabled = false;
		btn.textContent = 'Start';
		status(`failed: ${(e as Error).message}`);
	}
}

$('#start').addEventListener('click', start);
buildSliders();
document.querySelectorAll<HTMLElement>('.pad').forEach(bindPad);
loadCharacters();
requestAnimationFrame(frame);
