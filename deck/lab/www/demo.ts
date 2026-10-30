// demo.html (#184): @scutoons/live-core playing a small scene – a wobbling FM bass, a pad, a lead voice whose four
// inputs (freq, ratio, index, gain) are one float vector sent live from the sliders (force / blend / release, #189);
// the FFT bands and the spectrum of what is heard, drawn from the analysis frames. `?transport=transfer` forces the
// fallback without SharedArrayBuffer; served without COOP / COEP (`?coi=0`, ts/serve.ts) the page falls back by itself.
import { LiveCore, type Analysis, type Mode } from '../../../shared/live-core/src/index.ts';

const SCENE = `
[[node]]
id = "wob"
kind = "lfo"
rate = 0.25
depth = 1.0
[[node]]
id = "wob_hz"
kind = "mul"
b = 8.0
[[link]]
from = "wob"
to = "wob_hz.a"
[[node]]
id = "bass"
kind = "fm"
ratio = 1.0
index = 2.5
gain = 0.18
[[link]]
from = "wob_hz"
to = "bass.freq"
offset = 55.0
[[node]]
id = "breath"
kind = "lfo"
rate = 0.1
depth = 0.6
[[node]]
id = "pad_index"
kind = "smooth"
time = 0.5
[[link]]
from = "breath"
to = "pad_index.in"
[[node]]
id = "pad1"
kind = "fm"
freq = 220.0
ratio = 2.0
gain = 0.05
[[link]]
from = "pad_index"
to = "pad1.index"
offset = 1.0
[[node]]
id = "pad2"
kind = "fm"
freq = 277.18
ratio = 2.0
index = 0.8
gain = 0.04
[[node]]
id = "pad3"
kind = "fm"
freq = 329.63
ratio = 2.0
index = 0.8
gain = 0.04
[[node]]
id = "lead"
kind = "fm"
freq = 880.0
ratio = 1.5
index = 1.0
gain = 0.0
`;

const $ = <T extends HTMLElement>(s: string) => document.querySelector(s) as T;
const status = $<HTMLElement>('#status');
const canvas = $<HTMLCanvasElement>('#fft');
const g = canvas.getContext('2d')!;
const sliders = ['freq', 'ratio', 'index', 'gain'].map((k) => $<HTMLInputElement>(`#${k}`));
const amount = $<HTMLInputElement>('#amount');
const modes = [...document.querySelectorAll<HTMLInputElement>('input[name=mode]')];
const params = new URLSearchParams(location.search);
let core: LiveCore | null = null;

/** the slider positions → the lead's vector; freq on a log scale */
const vector = () => {
	const [f, r, i, gn] = sliders.map((s) => Number(s.value));
	return new Float32Array([110 * 2 ** (f * 5), r, i, gn]);
};
const mode = (): Mode => (modes.find((m) => m.checked)?.value as Mode) ?? 'blend';

function send() {
	if (!core) return;
	const v = vector();
	core.set('lead', v, { mode: mode(), amount: Number(amount.value) });
	$<HTMLElement>('#vector').textContent = `[${[...v].map((x) => x.toFixed(x > 10 ? 0 : 2)).join(', ')}] · ${mode()}${mode() === 'blend' ? ` ${Number(amount.value).toFixed(2)}` : ''}`;
}

function draw(a: Analysis) {
	const { width: w, height: h } = canvas;
	g.fillStyle = '#111';
	g.fillRect(0, 0, w, h);
	const names = ['level', 'bass', 'mid', 'high'];
	const values = [a.level, a.bass, a.mid, a.high];
	const bw = w * 0.1;
	g.font = `${Math.round(h * 0.06)}px ui-monospace, monospace`;
	values.forEach((v, i) => {
		const x = i * (bw + w * 0.02) + w * 0.02;
		g.fillStyle = ['#e8e8e8', '#ff7a59', '#ffd166', '#5ec8ff'][i];
		g.fillRect(x, h * 0.9 - v * h * 0.8, bw, v * h * 0.8);
		g.fillStyle = '#aaa';
		g.fillText(`${names[i]} ${v.toFixed(2)}`, x, h * 0.97);
	});
	// the spectrum, 0 – 8 kHz (the first 171 bins), on the right
	const x0 = w * 0.5;
	const bins = 171;
	g.strokeStyle = '#7ee787';
	g.lineWidth = 2;
	g.beginPath();
	for (let k = 0; k < bins; k++) {
		const x = x0 + (k / bins) * (w - x0 - w * 0.02);
		const y = h * 0.9 - a.spectrum[k] * h * 0.8;
		if (k) g.lineTo(x, y);
		else g.moveTo(x, y);
	}
	g.stroke();
	g.fillStyle = '#aaa';
	g.fillText('spectrum 0 – 8 kHz', x0, h * 0.97);
}

async function start() {
	$<HTMLButtonElement>('#start').disabled = true;
	try {
		const transport = (params.get('transport') as 'shared' | 'transfer' | null) ?? 'auto';
		core = await LiveCore.start({
			worker: new Worker(new URL('./live-worker.js', import.meta.url), { type: 'module' }),
			wasm: new URL('./live_core.wasm', import.meta.url),
			worklet: new URL('./live-worklet.js', import.meta.url),
			transport,
			lead: Number(params.get('lead') ?? 1024)
		});
		const table = await core.load(SCENE);
		core.onAnalysis(draw);
		send();
		const show = () => {
			if (!core) return;
			const s = core.stats();
			status.textContent =
				`transport ${s.transport}${s.transport === 'transfer' ? ' (no SharedArrayBuffer – postMessage transfer)' : ' (SharedArrayBuffer rings)'} · ` +
				`crossOriginIsolated ${self.crossOriginIsolated} · ${core.context.sampleRate} Hz · ${table.nodes.length} nodes · ` +
				`render ${s.renderUs.toFixed(1)} µs / block · ${s.blocks} blocks/s · ${s.records} sends/s · underruns ${s.underruns} · dropped ${s.dropped}`;
		};
		setInterval(show, 500);
		show();
		(window as unknown as { __core: LiveCore }).__core = core;
		document.title = 'playing';
	} catch (e) {
		status.textContent = `failed: ${(e as Error).message}`;
		document.title = 'failed';
	}
}

$<HTMLButtonElement>('#start').addEventListener('click', start);
for (const el of [...sliders, amount, ...modes]) el.addEventListener('input', send);
