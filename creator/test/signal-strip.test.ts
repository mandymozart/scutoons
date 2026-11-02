// `npm test -w scutoons-creator`: the deck half's signal strip model (lib/editor/signal-strip.ts, #282).
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { METER_MARKS, dbPosition, inputMeters, knobColumns, knobName, knobPosition, levelDb, meterZone, outputMeters, reductionBar, sameColumns, signalRows, stripStatus } from '../src/lib/editor/signal-strip.ts';

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

test('the meter is a dB scale with the 0.0 dB mark and green / amber / red zones', () => {
	assert.equal(levelDb(1), 0);
	assert.ok(Math.abs(levelDb(0.5) + 6.02) < 0.01);
	assert.equal(levelDb(0), -48, 'silence sits on the floor');
	assert.equal(levelDb(1e-9), -48);
	assert.equal(dbPosition(-48), 0);
	assert.equal(dbPosition(3), 1);
	assert.ok(METER_MARKS.zero > 0.9 && METER_MARKS.zero < 1, '0 dB is near the top, with headroom above');
	assert.ok(METER_MARKS.ok < METER_MARKS.hot && METER_MARKS.hot < METER_MARKS.zero);
	assert.deepEqual([meterZone(-30), meterZone(-12), meterZone(-6), meterZone(-3), meterZone(0)], ['ok', 'hot', 'hot', 'clip', 'clip']);
});

test('output meters: one per reported peak, padded with dim slots, levels clamped', () => {
	const stereo = outputMeters([0.5]);
	assert.deepEqual(stereo.map((m) => [m.channel, m.level, m.active, m.zone]), [[1, 0.5, true, 'hot'], [2, 0, false, 'ok']]);
	assert.ok(stereo[0].position > stereo[1].position);
	const eight = outputMeters([0.1, 1.5, -1, 0.2, 0, 0, 0, 0.9]);
	assert.equal(eight.length, 8);
	assert.deepEqual([eight[1].level, eight[2].level], [1, 0]);
	assert.ok(eight.every((m) => m.active));
	assert.equal(reductionBar(0), 0);
	assert.equal(reductionBar(-6), 0.5);
	assert.equal(reductionBar(-40), 1);
});

test('input meters carry their source; no inputs, no meters', () => {
	assert.deepEqual(inputMeters([]), []);
	const [a] = inputMeters([{ id: 'in1', source: 'line', label: 'Mic', level: 0.25 }, { id: 'in2', source: 'usb', level: 0 }]);
	assert.deepEqual([a.id, a.source, a.label, a.channel, a.zone], ['in1', 'line', 'Mic', 1, 'ok']);
	assert.equal(inputMeters([{ id: 'in2', source: 'usb', level: 0 }])[0].label, 'in2');
});

test('the stats line lists the frame rate once; drops only after the sound has run', () => {
	const s = { phase: 'running', transport: 'shared', backend: 'webgl2', fps: 59.6, cpu: 4.24, dropouts: 0 };
	assert.deepEqual(stripStatus(s), { state: 'running · shared · webgl2', perf: '60 fps · 4.2 ms · 0 drops' });
	assert.equal(stripStatus({ ...s, phase: 'ready', transport: null, dropouts: null }).perf, '60 fps · 4.2 ms');
	assert.equal(stripStatus({ ...s, phase: 'error', error: 'no audio' }).state, 'error: no audio · shared · webgl2');
});

test('signal rows: only protocols that carried a message; the lamp is lit while the activity is up', () => {
	const none = { count: 0, activity: 0, text: '' };
	assert.deepEqual(signalRows(null), []);
	assert.deepEqual(signalRows({ midi: none, osc: none, dmx: none }), [], 'nothing wired, nothing shown');
	const rows = signalRows({ midi: { count: 3, activity: 0.9, text: 'CC 74 = 127 · ch 1' }, osc: none, dmx: { count: 1, activity: 0.05, text: 'DMX 1 = 255' } });
	assert.deepEqual(rows.map((r) => [r.kind, r.text, r.active]), [['midi', 'CC 74 = 127 · ch 1', true], ['dmx', 'DMX 1 = 255', false]]);
});
