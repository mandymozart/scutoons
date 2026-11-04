// The worker's engine with the real live_core.wasm (deck/live-core, `npm run build:wasm -w shared/live-core` – the
// tests are skipped, saying so, when it is not built): the load edge, records through the control ring into the inbox,
// the audio ring kept at its lead (the device channels interleaved), analysis frames of what is heard with the heard
// block's values.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { existsSync, readFileSync } from 'node:fs';
import { BLOCK, Engine, WasmCore } from '../src/engine.ts';
import { clockOf } from '../src/clock.ts';
import { Ring, ringBuffers } from '../src/ring.ts';
import { valueIndex } from '../src/index.ts';
import { analysisOf, FRAME, SPECTRUM, VALUE_COUNT, writeSend } from '../src/wire.ts';
import { wasmPath } from '../scripts/wasm.ts';

const path = wasmPath();
const skip = existsSync(path) ? false : 'live_core.wasm is not built – npm run build:wasm -w shared/live-core';
const RATE = 48_000;
const SCENE = `
[[node]]
id = "wob"
kind = "lfo"
rate = 0.0
depth = 1.0
phase = 0.25
[[node]]
id = "lead"
kind = "fm"
freq = 220.0
ratio = 2.0
index = 0.0
gain = 0.0
[[node]]
id = "out"
kind = "audio_out"
[[link]]
from = "lead"
to = "out.in"
`;
/** the fm's inputs: cv, freq, ratio, index, gain */
const FREQ = 1;
const GAIN = 4;

const core = (channels = 2) => WasmCore.instantiate(readFileSync(path), RATE, channels);

test('load: the table comes out, a broken scene or a kind the browser lacks is an error with the reason', { skip }, async () => {
	const c = await core();
	const t = c.load(SCENE);
	assert.equal(t.channels, 2);
	assert.deepEqual(t.nodes[1], { id: 'lead', kind: 'fm', ports: ['cv', 'freq', 'ratio', 'index', 'gain'], outs: ['out'], at: 3 });
	assert.throws(() => c.load('[[node]]\nid = "x"\nkind = "chapter_music"\n'), /does not carry 'chapter_music'/);
	assert.equal(c.load(SCENE).nodes.length, 3);
	assert.throws(() => c.applyBytes(new Uint8Array([1, 0, 0])), /truncated header/);
});

test('#321: two browser decks on one anchor play the same beat (the show clock in the real wasm)', { skip }, async () => {
	const t0 = 1_762_000_000_000; // the page's ms
	const beatAtMs = async (relayAhead: number, skew: number, audioAt: number) => {
		const c = await core();
		c.load(SCENE);
		// the relay's clock `relayAhead` ms ahead of this page's; exact round trips
		for (let k = 0; k < 5; k++) {
			const t = t0 + k * 250;
			c.clock({ op: 'pong', t0: t * 1000, t1: Math.round(t + relayAhead), t2: t }, 0);
		}
		c.clock({ op: 'tempo', bpm: 128, perBar: 4, beat: 64, at: t0 + 2000 + relayAhead, running: true, arrived: t0 + 1500 }, 0);
		// the context's frame 0 is heard at the page's `audioAt`; the worklet played `skew` frames of silence first
		c.clock({ op: 'stamp', context: 1, heard: audioAt + 1000 }, skew);
		const s = clockOf(c.clockFrame(t0 + 3000));
		assert.equal(s.source, 'relay', 'a scene without a tempo node follows the show clock');
		assert.ok(s.locked && s.anchor, JSON.stringify(s));
		// the line is deck-core's, the native HUD's words (owner 2026-11-03: one overlay, not two)
		assert.match(c.clockLine(t0 + 3000), /^BPM 128\.0 {2}BAR \d+ BEAT \d( CLK)? {2}SHOW [+-]\d+\.\dMS RTT \d+\.\dMS$/);
		// #387: the report's line too – the frame's snap onto the line is no error, so no window yet
		assert.match(c.clockReport(t0 + 3000), /^clock relay · 128\.0 BPM · SHOW [+-]\d+\.\dMS RTT \d+\.\dMS · no minute of following yet$/);
		// the frame's beat, heard at `heardMs`, a line on to t0 + 3000
		return s.beat + ((t0 + 3000 - s.heardMs) / 60_000) * s.bpm;
	};
	const a = await beatAtMs(777_000, 0, t0 + 900);
	const b = await beatAtMs(-12_345, 4096, t0 + 2500.5);
	// relay = t0 + 3000 is a second after the anchor: 64 + 128 / 60
	assert.ok(Math.abs(a - b) < 1e-6, `${a} vs ${b}`);
	assert.ok(Math.abs(a - (64 + 128 / 60)) < 1e-6, String(a));
});

test('#321: MIDI clock out in the real wasm – the native schedule, handed out ahead with the page\'s ms', { skip }, async () => {
	const c = await core();
	c.load(SCENE);
	const t0 = 1_762_000_000_000;
	c.clock({ op: 'mode', mode: 1 }, 0); // the deck's own clock
	c.clock({ op: 'out', on: true }, 0);
	const msgs: number[][] = [];
	for (let k = 0; k < 40; k++) {
		// the worker's 25 ms: the audio renders along (the clock's frames are the audio's)
		for (let q = 0; q < 1200 / 128; q++) c.render(128);
		c.clockFrame(t0 + k * 25);
		const out = c.clockOut();
		for (let i = 0; out && i < out.length; i += 4) msgs.push([out[i], out[i + 1]]);
	}
	assert.ok([0xfa, 0xf2].includes(msgs[0][1]), JSON.stringify(msgs[0]));
	const ticks = msgs.filter((m) => m[1] === 0xf8).map((m) => m[0]);
	assert.ok(ticks.length >= 40, String(ticks.length));
	assert.ok(ticks[0] >= t0 && ticks.every((t, i) => i === 0 || t > ticks[i - 1]), 'in time order, on the page\'s clock');
	// #370: the state block says the choice, the clock out and the flash; the tempo line is deck-core's
	const s = clockOf(c.clockFrame(t0 + 1000));
	assert.deepEqual([s.mode, s.midiOut, s.flash?.ms], [1, true, 60]);
	assert.match(c.clockLine(t0 + 1000), /^BPM 120\.0 {2}BAR \d+ BEAT \d( CLK)?$/, "the tempo line, deck-core's");
	c.clock({ op: 'out', on: false }, 0);
	c.clockFrame(t0 + 1100);
	assert.equal(c.clockOut(), null);
});

test('a vector through the control ring reaches the sound, the analysis and the heard values', { skip }, async () => {
	const c = await core();
	const table = c.load(SCENE);
	const frames: Float32Array[] = [];
	const e = new Engine(c, { lead: 1024, onAnalysis: (f) => frames.push(f.slice()) });
	const control = Ring.bytes(ringBuffers(1 << 12, 1, false));
	const audio = Ring.floats(ringBuffers(1 << 13, 4, false));
	const scratch = new Uint8Array(64);
	e.fill(audio);
	assert.equal(audio.available(), 2 * 1024, 'filled to the lead: two channels interleaved');
	const block = new Float32Array(2 * BLOCK);
	audio.read(block);
	assert.ok(block.every((x) => x === 0), 'gain 0: silence');
	// 1500 Hz, ratio 1, no FM, gain 0.5 – written into the ring as the page does, drained into the inbox
	control.writeAll(scratch.subarray(0, writeSend(scratch, 0, { mode: 'force', node: 1, port: FREQ, amount: 1, v: [1500, 1, 0, 0.5] })));
	assert.equal(e.drain(control), 1);
	assert.equal(control.available(), 0);
	for (let i = 0; i < 40; i++) {
		while (audio.available() > 0) audio.read(block);
		e.fill(audio);
	}
	assert.ok(block.some((x) => Math.abs(x) > 0.3), 'sound');
	for (let f = 0; f < block.length; f += 2) assert.equal(block[f], block[f + 1], 'the stereo main: both channels');
	const last = frames.at(-1)!;
	assert.equal(last.length, FRAME + last[VALUE_COUNT]);
	assert.ok(last[2] > 0.8 && last[2] > last[1] + 0.3, `mid: ${[...last.subarray(0, 4)]}`);
	const peak = last.subarray(4, 4 + SPECTRUM).reduce((best, v, i, s) => (v > s[best] ? i : best), 0);
	assert.ok(Math.abs(peak - 32) <= 1, `1500 Hz / 46.875 Hz per bin: ${peak}`);
	const a = analysisOf(last);
	assert.equal(a.outputs.length, 2);
	assert.equal(a.values[valueIndex(table, 'lead.freq')], 1500, 'the heard block’s input');
	assert.ok(Math.abs(a.values[valueIndex(table, 'wob')] - 1) < 1e-5, 'the LFO’s output: unipolar, at its top');
	assert.equal(a.values[valueIndex(table, 'lead.out')], 0, 'an audio output: no meter without a being (`space` node)');
	const w = window(c);
	assert.ok(w.blocks > 300 && w.max > 0, JSON.stringify(w));
});

/** #364: the core's audio window, read from its words (deck-core `AudioWindow::to_words`: the longest load, the sum,
 *  analysis µs, gap ms, the limiter's deepest dB, its sum, frames, frames limiting, lead, rate, warm-up ms, the bins) */
function window(c: WasmCore) {
	const w = c.monitorTake();
	return { max: w[0], limitDb: w[4], limitMean: w[6] ? w[5] / w[6] : 0, limitingShare: w[6] ? w[7] / w[6] : 0, lead: w[8], blocks: w.slice(11).reduce((a, b) => a + b, 0) };
}

test('#365: the core’s lead – a phone starts generous, a dropout grows it a step', { skip }, async () => {
	const c = await core();
	assert.equal(c.x.lc_lead_class(0), 4096, 'deck-core: a phone starts two steps over its floor of 3072');
	assert.equal(c.x.lc_lead(0, 0), 4096);
	assert.equal(c.x.lc_lead(1000, 1), 4608);
	assert.equal(c.x.lc_lead_class(1), 3072, 'a desktop');
	assert.equal(window(c).lead, 3072, 'the window carries the lead');
});

test('a blend mixes, a release returns the input as built', { skip }, async () => {
	const c = await core();
	c.load(SCENE);
	const e = new Engine(c, { lead: 512, onAnalysis: () => {} });
	const peak = () => {
		let m = 0;
		for (let i = 0; i < 80; i++) e.block(0, () => {});
		for (let i = 0; i < 8; i++) e.block(0, (a) => (m = Math.max(m, ...a.map(Math.abs))));
		return m;
	};
	const send = (mode: 'force' | 'blend' | 'release', amount: number, v: number[]) => {
		const b = new Uint8Array(64);
		return e.applyBytes(b.subarray(0, writeSend(b, 0, { mode, node: 1, port: GAIN, amount, v })));
	};
	send('force', 1, [0.4]);
	const full = peak();
	assert.ok(full > 0.2, `${full}`);
	send('blend', 0.5, [0.4]);
	assert.ok(Math.abs(peak() - full / 2) < 0.005, 'half of 0.4 over the built 0');
	send('release', 0, [0]);
	assert.ok(peak() < 1e-3);
});

test('a six-channel device: the stereo main on the first two, the rest silent', { skip }, async () => {
	const c = await core(6);
	c.load(SCENE);
	const e = new Engine(c, { lead: 512, onAnalysis: () => {} });
	const b = new Uint8Array(64);
	e.applyBytes(b.subarray(0, writeSend(b, 0, { mode: 'force', node: 1, port: GAIN, amount: 1, v: [0.4] })));
	const peaks = new Array(6).fill(0);
	for (let i = 0; i < 40; i++) e.block(0, (a) => a.forEach((x, k) => (peaks[k % 6] = Math.max(peaks[k % 6], Math.abs(x)))));
	assert.ok(peaks[0] > 0.3 && peaks[1] > 0.3, `${peaks}`);
	assert.deepEqual(peaks.slice(2), [0, 0, 0, 0]);
});

test('#356: the warm-up plays a twin – the loaded scene sounds the same sample for sample; the core counted every block', { skip }, async () => {
	const heard = async (warm: boolean) => {
		const c = await core();
		c.load(SCENE.replace('gain = 0.0', 'gain = 0.4'));
		const e = new Engine(c, { lead: 512, onAnalysis: () => {} });
		const out: number[] = [];
		e.block(0, (a) => out.push(...a));
		if (warm) c.warm(SCENE, 64);
		for (let i = 0; i < 20; i++) e.block(0, (a) => out.push(...a));
		return { out, stats: window(c) };
	};
	const cold = await heard(false);
	const warmed = await heard(true);
	assert.ok(cold.out.some((x) => Math.abs(x) > 0.1));
	assert.deepEqual(warmed.out, cold.out);
	const s = warmed.stats;
	assert.equal(s.blocks, 21);
	assert.ok(s.max > 0, JSON.stringify(s));
	const c = await core();
	assert.throws(() => c.warm('[[no'), /./);
});

test('#356: the limiter in the stats – a loud voice is held down (peak, mean, share of frames), a quiet one is not', { skip }, async () => {
	const limiting = async (gain: number) => {
		const c = await core();
		c.load(SCENE.replace('gain = 0.0', `gain = ${gain.toFixed(1)}`));
		const e = new Engine(c, { lead: 512, onAnalysis: () => {} });
		for (let i = 0; i < 375; i++) e.block(0, () => {});
		return window(c);
	};
	const loud = await limiting(3);
	assert.ok(loud.limitDb > 3 && loud.limitMean > 1 && loud.limitingShare > 0.5, JSON.stringify(loud));
	assert.ok(loud.limitMean <= loud.limitDb);
	const quiet = await limiting(0.1);
	assert.deepEqual([quiet.limitDb, quiet.limitMean, quiet.limitingShare], [0, 0, 0]);
});
