// `npm test -w shared/live-deck` (#320): the deck's scenes in the real engine – the Cathedral test scene (#360), the silent scene (#425) and the core rigs load in
// live_core.wasm (deck-engine), sound, and every knob and hose the page uses is a path the engine answers (skipped,
// saying so, when live_core.wasm is not built: `npm run build:wasm -w shared/live-core`).
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { existsSync, readFileSync } from 'node:fs';
import { Engine, WasmCore } from '@scutoons/live-core/engine';
import { wasmPath } from '@scutoons/live-core/wasm-path';
import { analysisOf, valueIndex, writeSend } from '@scutoons/live-core';
import type { RigPlayed } from '../src/rig.ts';
import { DECK } from './own.ts';

const path = wasmPath();
const skip = existsSync(path) ? false : 'live_core.wasm is not built – npm run build:wasm -w shared/live-core';
const RIGS = JSON.parse(readFileSync(new URL('./fixtures/rigs.json', import.meta.url), 'utf8')) as Record<string, RigPlayed>;

/** `scene` loaded on the core's clock (120 BPM: no show clock), its knobs set as the deck does, 2 s rendered: the table, the last analysis, the loudest sample */
async function play(scene: string, knobs: Record<string, number>) {
	const c = await WasmCore.instantiate(readFileSync(path), 48_000, 2);
	const table = c.load(scene);
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

test("#360: the Cathedral, a test scene since #425 – the X1's old boot scene as deck-space plays it – sounds in the engine: the seq's arp, its knobs inputs, its hoses outputs", { skip }, async () => {
	const own = DECK.own.now.played;
	// every knob the core sends to the engine (its target), at its value – the core's sliders
	const knobs = Object.fromEntries(DECK.own.sliders.filter((s) => DECK.own.targets[s.path]).map((s) => [s.path, s.value]));
	const { table, a, loud } = await play(own.engine.scene, knobs);
	assert.ok(loud > 0.05 && loud < 1, `the cathedral sounds: ${loud}`);
	for (const [p] of own.streamed) assert.ok(valueIndex(table, p) >= 0, p);
	assert.ok(table.nodes.some((n) => n.id === 'seq') && !table.nodes.some((n) => n.id === 'melody'), "the cathedral's arp, no page melody");
	const sway = a.values[valueIndex(table, 'sway.out')];
	assert.ok(sway >= 0 && sway <= 1, `the LFO is unipolar: ${sway}`);
	assert.ok(Math.abs(a.values[valueIndex(table, 'incense.cutoff')] - (knobs['incense.cutoff'] + 3000 * sway)) < 1, 'the cutoff: its base, the sway on top');
});

test('#425: no rig, no sound – the silent scene loads in the engine and renders nothing', { skip }, async () => {
	const silent = DECK.silent.now.played;
	const { table, loud } = await play(silent.engine.scene, {});
	assert.equal(loud, 0, `the silent scene is silent: ${loud}`);
	assert.ok(!table.nodes.some((n) => ['seq', 'voice', 'mix', 'out'].includes(n.id)), 'no audio graph');
	for (const [p] of silent.streamed) assert.ok(valueIndex(table, p) >= 0 || p.startsWith('space.'), p);
});

test('every core rig plays in the engine with its knobs and streams', { skip }, async () => {
	// the deck's own scene is the test above; a core rig plays whole in the browser – every knob goes to the engine
	for (const [name, rig] of Object.entries(RIGS).filter(([n]) => n !== 'deck')) {
		assert.deepEqual(rig.engine.page, [], `${name} plays whole`);
		const knobs = Object.fromEntries(rig.knobs.map((k) => [k.path, k.value]));
		const { table, loud } = await play(rig.engine.scene, name.startsWith('incense') ? { ...knobs, 'air_hiss.gain': 0.1 } : knobs);
		assert.ok(loud > 0.01, `${name} sounds`);
		for (const [p] of rig.streamed) assert.ok(valueIndex(table, p) >= 0, `${name}: ${p}`);
	}
});
