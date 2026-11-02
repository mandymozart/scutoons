// The worker's engine with the real live_core.wasm (deck/live-core, `npm run build:wasm -w shared/live-core` – the
// tests are skipped, saying so, when it is not built): the load edge, records through the control ring into the inbox,
// the audio ring kept at its lead, analysis frames of what is heard.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { existsSync, readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { BLOCK, Engine, WasmCore } from '../src/engine.ts';
import { Ring, ringBuffers } from '../src/ring.ts';
import { FRAME, SPECTRUM, writeSend } from '../src/wire.ts';

const wasmPath = fileURLToPath(new URL('../../../deck/target/wasm32-unknown-unknown/release/live_core.wasm', import.meta.url));
const skip = existsSync(wasmPath) ? false : 'live_core.wasm is not built – npm run build:wasm -w shared/live-core';
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
`;

const core = () => WasmCore.instantiate(readFileSync(wasmPath), RATE);

test('load: the table comes out, a broken scene is an error with the reason', { skip }, async () => {
	const c = await core();
	const t = c.load(SCENE);
	assert.deepEqual(t.nodes[1], { id: 'lead', kind: 'fm', ports: ['freq', 'ratio', 'index', 'gain'] });
	assert.throws(() => c.load('[[node]]\nid = "x"\nkind = "fft"\n'), /does not batch/);
	assert.equal(c.load(SCENE).nodes.length, 2);
	assert.throws(() => c.applyBytes(new Uint8Array([1, 0, 0])), /truncated header/);
});

test('a vector through the control ring reaches the sound and the analysis', { skip }, async () => {
	const c = await core();
	c.load(SCENE);
	const frames: Float32Array[] = [];
	const e = new Engine(c, { lead: 1024, onAnalysis: (f) => frames.push(f.slice()) });
	const control = Ring.bytes(ringBuffers(1 << 12, 1, false));
	const audio = Ring.floats(ringBuffers(1 << 12, 4, false));
	const scratch = new Uint8Array(64);
	e.fill(audio);
	assert.equal(audio.available(), 1024, 'filled to the lead');
	const block = new Float32Array(BLOCK);
	audio.read(block);
	assert.ok(block.every((x) => x === 0), 'gain 0: silence');
	// 1500 Hz, ratio 1, no FM, gain 0.5 – written into the ring as the page does, drained into the inbox
	control.writeAll(scratch.subarray(0, writeSend(scratch, 0, { mode: 'force', node: 1, port: 0, amount: 1, v: [1500, 1, 0, 0.5] })));
	assert.equal(e.drain(control), 1);
	assert.equal(control.available(), 0);
	for (let i = 0; i < 40; i++) {
		while (audio.available() > 0) audio.read(block);
		e.fill(audio);
	}
	assert.ok(block.some((x) => Math.abs(x) > 0.3), 'sound');
	const last = frames.at(-1)!;
	assert.equal(last.length, FRAME);
	assert.ok(last[2] > 0.8 && last[2] > last[1] + 0.3, `mid: ${[...last.subarray(0, 4)]}`);
	const peak = last.subarray(4, 4 + SPECTRUM).reduce((best, v, i, s) => (v > s[best] ? i : best), 0);
	assert.equal(peak, 32, '1500 Hz / 46.875 Hz per bin');
	const s = e.stats();
	assert.ok(s.blocks > 300 && s.records === 1 && s.renderUs > 0, JSON.stringify(s));
});

test('a blend mixes, a release returns the input as built', { skip }, async () => {
	const c = await core();
	c.load(SCENE);
	const e = new Engine(c, { lead: 512, onAnalysis: () => {} });
	// the inputs glide (#188): settle 80 blocks (8.5 time constants), then the peak of 8
	const peak = () => {
		let m = 0;
		for (let i = 0; i < 80; i++) e.block(0, () => {});
		for (let i = 0; i < 8; i++) e.block(0, (a) => (m = Math.max(m, ...a.map(Math.abs))));
		return m;
	};
	const send = (mode: 'force' | 'blend' | 'release', amount: number, v: number[]) => {
		const b = new Uint8Array(64);
		return e.applyBytes(b.subarray(0, writeSend(b, 0, { mode, node: 1, port: 3, amount, v })));
	};
	send('force', 1, [0.4]);
	assert.ok(Math.abs(peak() - 0.4) < 0.01);
	send('blend', 0.5, [0.4]);
	assert.ok(Math.abs(peak() - 0.2) < 0.01, 'half of 0.4 over the built 0');
	send('release', 0, [0]);
	assert.ok(peak() < 1e-3);
});
