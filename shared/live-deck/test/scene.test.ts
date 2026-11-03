// `npm test -w shared/live-deck` (#320): the deck's scenes in the real engine – the fixed scene and the core rigs load in
// live_core.wasm (deck-engine), sound, and every knob and hose the page uses is a path the engine answers (skipped,
// saying so, when live_core.wasm is not built: `npm run build:wasm -w shared/live-core`).
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { existsSync, readFileSync } from 'node:fs';
import { Engine, WasmCore } from '@scutoons/live-core/engine';
import { wasmPath } from '@scutoons/live-core/wasm-path';
import { analysisOf, valueIndex, writeSend } from '@scutoons/live-core';
import { FIXED_SCENE, MELODY, OUTPUTS, PARAMS } from '../src/scene.ts';
import { rigParams, rigStreamed } from '../src/rig.ts';
import type { RigPlayed } from '../src/space.ts';

const path = wasmPath();
const skip = existsSync(path) ? false : 'live_core.wasm is not built – npm run build:wasm -w shared/live-core';
const RIGS = JSON.parse(readFileSync(new URL('./fixtures/rigs.json', import.meta.url), 'utf8')) as Record<string, RigPlayed>;

/** `scene` loaded at 84 bpm, its knobs set as the deck does, 2 s rendered: the table, the last analysis, the loudest sample */
async function play(scene: string, knobs: Record<string, number>) {
	const c = await WasmCore.instantiate(readFileSync(path), 48_000, 2);
	const table = c.load(scene);
	c.tempo(84);
	let last = new Float32Array(0);
	const e = new Engine(c, { lead: 1024, onAnalysis: (f) => (last = f.slice()) });
	const b = new Uint8Array(64);
	for (const [p, v] of Object.entries(knobs)) {
		const [id, port] = [p.slice(0, p.lastIndexOf('.')), p.slice(p.lastIndexOf('.') + 1)];
		const node = table.nodes.findIndex((n) => n.id === id);
		assert.ok(node >= 0, `${p}: a node of the scene`);
		const at = table.nodes[node].ports.indexOf(port);
		assert.ok(at >= 0, `${p}: an input of ${id}`);
		e.applyBytes(b.subarray(0, writeSend(b, 0, { mode: 'base', node, port: at, amount: 0, v: [v] })));
	}
	let loud = 0;
	for (let i = 0; i < 750; i++) e.block(0, (a) => (loud = Math.max(loud, ...a.map(Math.abs))));
	return { table, a: analysisOf(last), loud };
}

test('the fixed scene plays in the engine; its knobs are inputs, its hoses outputs the engine reports', { skip }, async () => {
	const knobs = Object.fromEntries(Object.entries({ ...PARAMS, ...MELODY }).map(([p, s]) => [p, s.value]));
	const { table, a, loud } = await play(FIXED_SCENE, { ...knobs, 'melody.spread': 0.8 });
	assert.ok(loud > 0.05 && loud < 1, `the voice and the drone: ${loud}`);
	for (const p of [...Object.keys(knobs), ...Object.keys(OUTPUTS)]) assert.ok(valueIndex(table, p) >= 0, p);
	assert.ok(a.values[valueIndex(table, 'voice.out')] > 0.01, 'the being’s meters: the voice is heard');
	assert.ok(a.values[valueIndex(table, 'melody.cv')] > 30, 'the melody’s cv: a pitch in Hz');
	const sway = a.values[valueIndex(table, 'sway.out')];
	assert.ok(sway >= 0 && sway <= PARAMS['sway.depth'].value, `the LFO is unipolar: ${sway}`);
	assert.ok(Math.abs(a.values[valueIndex(table, 'incense.cutoff')] - (1800 + 2500 * sway)) < 1, 'the cutoff: its base, the sway on top');
});

test('every core rig plays in the engine with its knobs and streams', { skip }, async () => {
	for (const [name, rig] of Object.entries(RIGS)) {
		const knobs = Object.fromEntries(Object.entries(rigParams(rig)).map(([p, s]) => [p, s.value]));
		const { table, loud } = await play(rig.engine.scene, name.startsWith('incense') ? { ...knobs, 'air_hiss.gain': 0.1 } : knobs);
		assert.ok(loud > 0.01, `${name} sounds`);
		for (const p of Object.keys(rigStreamed(rig))) assert.ok(valueIndex(table, p) >= 0, `${name}: ${p}`);
	}
});
