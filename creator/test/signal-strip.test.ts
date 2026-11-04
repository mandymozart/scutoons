// `npm test -w scutoons-creator`: the deck half's signal strip model (lib/editor/signal-strip.ts, #282).
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { inputMeters, knobColumns, knobName, knobPosition, outputMeters, sameColumns, stripStatus } from '../src/lib/editor/signal-strip.ts';

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

test('output meters: one per reported reading as the core gave it, padded with dim slots', () => {
	const stereo = outputMeters([{ level: 0.5, position: 0.82, zone: 'hot' }]);
	assert.deepEqual(stereo.map((m) => [m.channel, m.level, m.active, m.zone, m.position]), [[1, 0.5, true, 'hot', 0.82], [2, 0, false, 'ok', 0]]);
	const eight = outputMeters(Array.from({ length: 8 }, () => ({ level: 0.1, position: 0.4, zone: 'ok' as const })));
	assert.equal(eight.length, 8);
	assert.ok(eight.every((m) => m.active));
});

test('input meters carry their source; no inputs, no meters', () => {
	assert.deepEqual(inputMeters([]), []);
	const r = { level: 0.25, position: 0.5, zone: 'ok' as const };
	const [a] = inputMeters([{ id: 'in1', source: 'line', label: 'Mic', ...r }, { id: 'in2', source: 'usb', ...r }]);
	assert.deepEqual([a.id, a.source, a.label, a.channel, a.zone, a.position], ['in1', 'line', 'Mic', 1, 'ok', 0.5]);
	assert.equal(inputMeters([{ id: 'in2', source: 'usb', ...r }])[0].label, 'in2');
});

test('the stats line: the state words, then the monitor’s items as the core worded them', () => {
	const bar = [{ label: 'FPS', value: '60' }, { label: 'CPU', value: '4.2/6.0MS' }, { label: 'AUDIO', value: '0/10M 2/5/9% +43MS' }];
	const s = { phase: 'running', transport: 'shared', backend: 'webgl2', stats: { bar } };
	assert.deepEqual(stripStatus(s), { state: 'running · shared · webgl2', perf: 'FPS 60 · CPU 4.2/6.0MS · AUDIO 0/10M 2/5/9% +43MS' });
	assert.equal(stripStatus({ ...s, stats: null }).perf, '', 'no report yet');
	assert.equal(stripStatus({ ...s, phase: 'error', error: 'no audio' }).state, 'error: no audio · shared · webgl2');
});
