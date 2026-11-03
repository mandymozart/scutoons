// The signal records against the fixture both sides read: deck/live-core/fixtures/signals.json (the Rust side encodes
// every row to its bytes, deck/live-core/src/signals.rs) – TS decodes the bytes to the same message. Plus the real wasm:
// a scene with [outputs] sends records, and the analysis frame carries the output peaks (skipped, saying so, when
// live_core.wasm is not built).
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { existsSync, readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { readSignals, type DeviceSignal } from '../src/signals.ts';
import { BLOCK, Engine, WasmCore } from '../src/engine.ts';
import { analysisOf, FRAME, MAX_OUTPUTS, OUTPUT_COUNT, PEAKS, VALUE_COUNT, WireError, writeSend } from '../src/wire.ts';
import { wasmPath as builtWasm } from '../scripts/wasm.ts';

type Row = { name: string; at: number; msg: Record<string, unknown>; hex: string };
const fixture = fileURLToPath(new URL('../../../deck/live-core/fixtures/signals.json', import.meta.url));
const rows = JSON.parse(readFileSync(fixture, 'utf8')) as Row[];
const bytes = (h: string) => new Uint8Array(Buffer.from(h, 'hex'));

/** the fixture's message as the decoder gives it (the JSON is the protocol's shape; `at` joins it) */
const expected = (r: Row): DeviceSignal => {
	const m = r.msg;
	if (m.kind === 'midi') return { kind: 'midi', port: m.port as 'out' | 'conductor', bytes: m.bytes as number[], at: r.at };
	if (m.kind === 'dmx') return { kind: 'dmx', ch: m.ch as number, values: m.values as number[], flash: m.flash === true, at: r.at };
	return { kind: 'osc', address: m.address as string, args: m.args as never, at: r.at };
};

test('fixture: every record decodes to its message', () => {
	assert.ok(rows.length >= 5);
	for (const r of rows) assert.deepEqual([...readSignals(bytes(r.hex))], [expected(r)], r.name);
});

test('records back to back, at an unaligned offset of a bigger buffer', () => {
	const all = rows.map((r) => r.hex).join('');
	assert.deepEqual([...readSignals(bytes(all))], rows.map(expected));
	const big = new Uint8Array(3 + all.length / 2);
	big.set(bytes(all), 3);
	assert.deepEqual([...readSignals(big.subarray(3))], rows.map(expected));
});

test('malformed bytes stop the walk with the offset', () => {
	const one = bytes(rows[0].hex);
	assert.throws(() => [...readSignals(one.subarray(0, 7))], (e: unknown) => e instanceof WireError && e.what === 'a truncated signal header');
	assert.throws(() => [...readSignals(one.subarray(0, 14))], (e: unknown) => e instanceof WireError && e.what === 'truncated signal payload');
	const bad = one.slice();
	bad[0] = 9;
	assert.throws(() => [...readSignals(bad)], (e: unknown) => e instanceof WireError && e.at === 0 && e.what === 'an unknown signal kind');
	const osc = bytes(rows.find((r) => r.msg.kind === 'osc' && (r.msg.args as unknown[]).length > 0)!.hex);
	osc[12 + 2 + 14 + 1] = 77; // the first argument's tag
	assert.throws(() => [...readSignals(osc)], /malformed osc/);
	const two = new Uint8Array([...one, 1, 0, 0]);
	const it = readSignals(two);
	assert.equal(it.next().done, false);
	assert.throws(() => it.next(), (e: unknown) => e instanceof WireError && e.at === one.length);
});

const wasmPath = builtWasm();
const skip = existsSync(wasmPath) ? false : 'live_core.wasm is not built – npm run build:wasm -w shared/live-core';

const NODES = `
[[node]]
id = "lead"
kind = "fm"
freq = 220.0
ratio = 2.0
index = 0.0
gain = 0.0
[[node]]
id = "knob"
kind = "add"
a = 0.0
b = 0.0
[[node]]
id = "out"
kind = "audio_out"
[[link]]
from = "lead"
to = "out.in"
`;
const OUTPUTS = `
[outputs]
midi = "x"
[[outputs.map]]
from = "knob.a"
to = [{ cc = 74, channel = 1 }, { cc = 20, port = "conductor" }]
`;

test('the engine: the frame carries the output peaks, [outputs] arrive as signal records', { skip }, async () => {
	const c = await WasmCore.instantiate(readFileSync(wasmPath), 48_000);
	const table = c.load(NODES + OUTPUTS);
	const frames: Float32Array[] = [];
	const got: DeviceSignal[] = [];
	const e = new Engine(c, { lead: 1024, onAnalysis: (f) => frames.push(f.slice()), onSignals: (r) => got.push(...readSignals(r)) });
	const rec = new Uint8Array(64);
	const send = (node: number, port: number, v: number[]) => e.applyBytes(rec.subarray(0, writeSend(rec, 0, { mode: 'force', node, port, amount: 1, v })));
	send(table.nodes.findIndex((n) => n.id === 'knob'), 0, [1]);
	send(0, 1, [1500, 1, 0, 0.5]);
	for (let i = 0; i < 200; i++) e.block(0, () => {});
	const last = frames.at(-1)!;
	assert.equal(last.length, FRAME + last[VALUE_COUNT]);
	assert.equal(last[OUTPUT_COUNT], 2);
	assert.ok(last[PEAKS] > 0.3 && last[PEAKS] <= 1, `peak ${last[PEAKS]}`);
	assert.equal(last[PEAKS], last[PEAKS + 1]);
	assert.deepEqual([...last.subarray(PEAKS + 2, PEAKS + MAX_OUTPUTS)], new Array(MAX_OUTPUTS - 2).fill(0));
	const a = analysisOf(last);
	assert.equal(a.outputs.length, 2);
	assert.ok(a.outputs[0] > 0.3);
	assert.deepEqual(got.map((s) => s.kind === 'midi' && [s.port, s.bytes]), [['out', [0xb0, 74, 127]], ['conductor', [0xb0, 20, 127]]]);
	assert.ok(got.every((s) => s.at <= (200 * BLOCK * 1000) / 48_000), 'on the audio clock');
	// a scene without [outputs] sends none; a wrong path fails the load with the list
	const quiet = await WasmCore.instantiate(readFileSync(wasmPath), 48_000);
	quiet.load(NODES);
	let n = 0;
	const q = new Engine(quiet, { lead: 1024, onAnalysis: () => {}, onSignals: () => n++ });
	for (let i = 0; i < 50; i++) q.block(0, () => {});
	assert.equal(n, 0);
	assert.throws(() => quiet.load(NODES + OUTPUTS.replace('knob.a', 'knob.nope')), /\[outputs\][\s\S]*knob\.nope/);
});
