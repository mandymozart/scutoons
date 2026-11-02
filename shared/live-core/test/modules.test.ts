// The browser's module host (src/modules.ts) with real wasm built against deck/module (`npm run build:modules -w
// shared/live-core`; skipped, saying so, when it is not built): the walk module runs in V8 and plays what the same module
// plays natively (deck/modules/walk/fixtures/walk.json, made by the module's Rust test), a panic silences that instance,
// the descriptor is read without running anything.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { existsSync, readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { LogLimiter, ModuleInstance, parseDescriptor, readDescriptor } from '../src/modules.ts';
import { OFFSETS, ABI_VERSION } from '../src/module-abi.ts';

const dir = fileURLToPath(new URL('../../../deck/target/wasm32-unknown-unknown/release/', import.meta.url));
const walkPath = `${dir}walk_module.wasm`;
const fixturePath = `${dir}fixture_module.wasm`;
const skip = existsSync(walkPath) && existsSync(fixturePath) ? false : 'the module wasm is not built – npm run build:modules -w shared/live-core';
const RATE = 48_000;
const native = JSON.parse(readFileSync(fileURLToPath(new URL('../../../deck/modules/walk/fixtures/walk.json', import.meta.url)), 'utf8'));

const load = async (path: string) => WebAssembly.compile(readFileSync(path));

test('the descriptor is read from the custom section without running the module', { skip }, async () => {
	const d = readDescriptor(await load(walkPath));
	assert.deepEqual(d, JSON.parse(native.descriptor));
	assert.equal(d.name, 'walk');
	assert.equal(d.abi, ABI_VERSION);
	assert.deepEqual(d.params.map((p) => p.name), ['spread', 'rate', 'glide', 'base', 'octaves', 'length']);
});

test('the walk module in V8 plays what it plays natively', { skip }, async () => {
	const m = await ModuleInstance.create(await load(walkPath), RATE, { seed: native.seed });
	const p = (name: string) => m.desc.params.findIndex((x) => x.name === name);
	for (const [k, v] of Object.entries(native.params)) m.setParam(p(k), v as number);
	m.setTempo({ frame: 0, beat: 0, bpm: native.bpm, rate: RATE, running: true, perBar: 4 });
	const got: { block: number; e: ReturnType<ModuleInstance['events']>[number] }[] = [];
	const cv: number[][] = [];
	const l = new Float32Array(native.block);
	const r = new Float32Array(native.block);
	for (let b = 0; b < native.blocks; b++) {
		assert.ok(m.process(b * native.block, native.block), m.silenced ?? '');
		for (const e of m.events()) got.push({ block: b, e });
		m.readAudioOut(0, l, r);
		cv.push([l[0], l[l.length - 1], r.reduce((s, x) => s + x, 0)]);
	}
	assert.equal(got.length, native.events.length, 'the same notes');
	got.forEach(({ block, e }, i) => {
		const want = native.events[i];
		assert.equal(block, want.block);
		assert.equal(e.name, want.name);
		assert.equal(e.start, want.start, 'the beat of a step');
		assert.equal(e.len, want.len);
		assert.ok(Math.abs(e.props.pitch - want.pitch) <= 1e-4 * want.pitch, `pitch ${e.props.pitch} vs ${want.pitch}`);
		assert.equal(e.props.glide, want.glide);
	});
	cv.forEach((c, i) => {
		const want = native.cv[i];
		assert.ok(Math.abs(c[0] - want[0]) <= 1e-3 * Math.max(1, want[0]), `block ${i}: left first ${c[0]} vs ${want[0]}`);
		assert.ok(Math.abs(c[1] - want[1]) <= 1e-3 * Math.max(1, want[1]), `block ${i}: left last ${c[1]} vs ${want[1]}`);
		assert.equal(c[2], want[2], `block ${i}: gate`);
	});
});

test('audio, tempo, character and variables cross the boundary; a bad parameter is clamped', { skip }, async () => {
	const m = await ModuleInstance.create(await load(fixturePath), RATE, { seed: 7 });
	const ones = new Float32Array(64).fill(1);
	m.setAudioIn(0, ones, ones);
	assert.ok(m.process(0, 64));
	const l = new Float32Array(64);
	const r = new Float32Array(64);
	m.readAudioOut(0, l, r);
	assert.deepEqual([l[0], l[1], r[0], r[63]], [64.25, 0.25, 64.25, 0.25]);
	assert.equal(m.events()[0].name, 'tick');
	m.setParam(1, 5); // bias, range ±1
	m.process(64, 64);
	m.readAudioOut(0, l, r);
	assert.equal(l[1], 1.25);
	m.setParam(0, 4); // mode 4: bpm / 100 + fx.level, and the beat
	m.setTempo({ frame: 0, beat: 2, bpm: 90, rate: RATE, running: true, perBar: 4 });
	m.setVars({ 'fx.level': 0.5 });
	m.setCharacter('catpope', 'dream', 'calm');
	m.process(24_000, 64);
	m.readAudioOut(0, l, r);
	assert.ok(Math.abs(l[0] - 1.4) < 1e-6, `${l[0]}`);
	assert.ok(Math.abs(r[0] - (2 + 24_000 * (90 / 60 / RATE))) < 1e-4, `${r[0]}`);
});

test('a panic silences that instance and only it', { skip }, async () => {
	const module = await load(fixturePath);
	const a = await ModuleInstance.fromModule(module, RATE, { seed: 7 });
	const b = await ModuleInstance.fromModule(module, RATE, { seed: 7 });
	a.setParam(0, 1);
	assert.equal(a.process(0, 64), false);
	assert.match(a.silenced ?? '', /dm_process/);
	assert.equal(a.process(64, 64), false, 'and it stays silent');
	assert.equal(b.process(0, 64), true);
	assert.equal(b.silenced, null);
});

test('log lines pass the rate limit, a burst and then a steady rate', { skip }, async () => {
	const lines: string[] = [];
	let t = 0;
	const m = await ModuleInstance.create(await load(fixturePath), RATE, { onLog: (_, text) => lines.push(text), now: () => t });
	m.setParam(0, 3); // 50 lines a block
	m.process(0, 64);
	assert.equal(lines.length, 10);
	assert.equal(lines[0], 'line 0');
	t += 500;
	m.process(64, 64);
	assert.equal(lines.length, 15);
});

test('the limiter and the descriptor checks', () => {
	const l = new LogLimiter();
	assert.equal([...Array(50)].filter(() => l.allow(0)).length, 10);
	assert.equal(l.dropped, 40);
	const ok = { abi: 1, name: 'm', version: '1', params: [{ name: 'a', min: 0, max: 1, default: 0.5 }], ports: [{ name: 'o', dir: 'out', kind: 'audio' }] };
	assert.equal(parseDescriptor(JSON.stringify(ok)).name, 'm');
	assert.throws(() => parseDescriptor(JSON.stringify({ ...ok, abi: 2 })), /ABI 2/);
	assert.throws(() => parseDescriptor(JSON.stringify({ ...ok, name: 'Bad' })), /identifier/);
	assert.throws(() => parseDescriptor(JSON.stringify({ ...ok, params: [{ name: 'a', min: 0, max: 1, default: 2 }] })), /default/);
	assert.throws(() => parseDescriptor(JSON.stringify({ ...ok, ports: [{ name: 'a', dir: 'out', kind: 'audio' }] })), /twice/);
	assert.equal(OFFSETS.event.size, 88);
});

test('a wasm with another import, or none of ours, is refused', async () => {
	const leb = (n: number) => (n < 128 ? [n] : [(n & 0x7f) | 0x80, n >> 7]);
	const json = new TextEncoder().encode(JSON.stringify({ abi: 1, name: 'evil', version: '1', params: [], ports: [] }));
	const name = new TextEncoder().encode('deck.module');
	const custom = [0, ...leb(1 + name.length + json.length), name.length, ...name, ...json];
	const head = [0, 0x61, 0x73, 0x6d, 1, 0, 0, 0];
	const types = [1, 4, 1, 0x60, 0, 0];
	const imports = [2, 7, 1, 1, 0x78, 1, 0x79, 0, 0]; // import "x"."y" (func)
	const evil = new Uint8Array([...head, ...types, ...imports, ...custom]);
	await assert.rejects(ModuleInstance.create(evil, RATE), /imports x\.y/);
	await assert.rejects(ModuleInstance.create(new Uint8Array([...head, ...types]), RATE), /deck\.module/);
});
