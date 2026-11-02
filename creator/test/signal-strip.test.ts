// `npm test -w scutoons-creator`: the deck half's signal strip model (lib/editor/signal-strip.ts, #282).
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { knobColumns, knobName, knobPosition, midiText, outputMeters, reductionBar, sameColumns, signalRows, stripStatus } from '../src/lib/editor/signal-strip.ts';

test('a knob sits in its range, a log knob on its log scale, out of range is clamped', () => {
	assert.equal(knobPosition(5, 0, 10), 0.5);
	assert.equal(knobPosition(-3, 0, 10), 0);
	assert.equal(knobPosition(99, 0, 10), 1);
	assert.ok(Math.abs(knobPosition(632.455, 20, 20000, true) - 0.5) < 1e-3);
	assert.equal(knobPosition(1, 5, 5), 0, 'an empty range is 0');
	assert.equal(knobPosition(Number.NaN, 0, 1), 0);
});

test('columns follow the rig order with labels and live values; a missing value reads as the minimum', () => {
	const knobs = [
		{ path: 'a_osc.freq', min: 20, max: 2000, label: { en: 'Pitch', de: 'Ton' } },
		{ path: 'a_osc.gain', min: 0, max: 1 }
	];
	const cols = knobColumns(knobs, { 'a_osc.gain': 0.25 }, 'de');
	assert.deepEqual(cols.map((c) => [c.path, c.label, c.position]), [['a_osc.freq', 'Ton', 0], ['a_osc.gain', 'gain', 0.25]]);
	assert.equal(knobName({ path: 'x.y', min: 0, max: 1, label: { fr: 'Hauteur' } }), 'Hauteur');
	assert.ok(sameColumns(cols, knobColumns(knobs, { 'a_osc.gain': 0.255 }, 'de')));
	assert.ok(!sameColumns(cols, knobColumns(knobs, { 'a_osc.gain': 0.5 }, 'de')));
});

test('output meters: one per reported peak, padded with dim slots, levels clamped', () => {
	assert.deepEqual(outputMeters([0.5]), [{ channel: 1, level: 0.5, active: true }, { channel: 2, level: 0, active: false }]);
	const eight = outputMeters([0.1, 1.5, -1, 0.2, 0, 0, 0, 0.9]);
	assert.equal(eight.length, 8);
	assert.deepEqual([eight[1].level, eight[2].level], [1, 0]);
	assert.ok(eight.every((m) => m.active));
	assert.equal(reductionBar(0), 0);
	assert.equal(reductionBar(-6), 0.5);
	assert.equal(reductionBar(-40), 1);
});

test('the stats line lists the frame rate once; drops only after the sound has run', () => {
	const s = { phase: 'running', transport: 'shared', backend: 'webgl2', fps: 59.6, cpu: 4.24, dropouts: 0 };
	assert.deepEqual(stripStatus(s), { state: 'running · shared · webgl2', perf: '60 fps · 4.2 ms · 0 drops' });
	assert.equal(stripStatus({ ...s, phase: 'ready', transport: null, dropouts: null }).perf, '60 fps · 4.2 ms');
	assert.equal(stripStatus({ ...s, phase: 'error', error: 'no audio' }).state, 'error: no audio · shared · webgl2');
});

test('signal rows: a dot is lit for `hold` ms after traffic, the text stays', () => {
	const rows = signalRows({ midi: { text: 'CC74 · 90', at: 1000 } }, 1100);
	assert.deepEqual(rows.map((r) => [r.kind, r.text, r.active]), [['midi', 'CC74 · 90', true], ['dmx', '', false], ['osc', '', false]]);
	assert.equal(signalRows({ midi: { text: 'x', at: 1000 } }, 1300)[0].active, false);
	assert.equal(signalRows({ midi: { text: 'x', at: 1000 } }, 900)[0].active, false, 'a time before the message is not activity');
});

test('midi bytes read as short text', () => {
	assert.equal(midiText([0xb0, 74, 90]), 'CC74 · 90');
	assert.equal(midiText([0xb2, 1, 5]), 'CC1 · 5 · ch3');
	assert.equal(midiText([0x90, 60, 100]), 'note on 60 · 100');
	assert.equal(midiText([0x90, 60, 0]), 'note off 60 · 0');
	assert.equal(midiText([0xe0, 0, 64]), 'bend 8192');
	assert.equal(midiText([0xf8]), 'f8');
});
