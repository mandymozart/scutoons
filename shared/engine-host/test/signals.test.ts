// `npm test -w @scutoons/engine-host` (#282): the output meters and the signal monitor
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { FALL_SECONDS, OutputMeters } from '../src/meters.ts';
import { ACTIVITY_SECONDS, SignalMonitor, describeSignal, emptySignals } from '../src/monitor.ts';
import { emptySnapshot } from '../src/status.ts';
import type { DeviceSignal } from '@scutoons/live-core';

test('meters: a peak is held, falls, and a higher one takes over; the count follows the engine', () => {
	const m = new OutputMeters();
	assert.deepEqual(m.read(), []);
	m.push([0.8, 0.4], 1 / 60);
	assert.deepEqual(m.read().map((x) => Math.fround(x)), [Math.fround(0.8), Math.fround(0.4)]);
	m.push([0, 0], FALL_SECONDS);
	const [a, b] = m.read();
	assert.ok(Math.abs(a - 0.8 / Math.E) < 1e-6 && Math.abs(b - 0.4 / Math.E) < 1e-6, 'one time constant: ×1/e');
	m.push([0.9, 0], 1 / 60);
	assert.ok(Math.abs(m.read()[0] - 0.9) < 1e-6);
	m.push([0.1, 0.1, 0.5], 0);
	assert.equal(m.read().length, 3, 'a multi-channel engine grows the meter');
	m.push([0.2], 0);
	assert.equal(m.read().length, 1, 'and shrinks it');
	m.reset();
	assert.deepEqual(m.read(), []);
});

test('meters: peaks is one Float32Array updated in place, a new view only when the count changes', () => {
	const m = new OutputMeters();
	assert.equal(m.peaks.length, 0);
	m.push([0.5, 0.25], 0);
	const view = m.peaks;
	assert.ok(view instanceof Float32Array);
	assert.deepEqual([...view], [0.5, 0.25]);
	m.push([0.75, 0.5], 1 / 60);
	assert.equal(m.peaks, view, 'the same object');
	assert.deepEqual([...view], [0.75, 0.5], 'updated in place');
	m.push([0.1, 0.1, 0.1], 10);
	assert.equal(m.peaks.length, 3);
	m.push([0.1], 10);
	assert.deepEqual([...m.peaks], [Math.fround(0.1)]);
	m.push([0.1, 0.1], 10);
	assert.ok(m.peaks[1] === Math.fround(0.1), 'a slot that came back starts from the new peak, not an old one');
	m.reset();
	assert.equal(m.peaks.length, 0);
});

test('meters: junk is silence, the reading stays within 0..1', () => {
	const m = new OutputMeters();
	m.push([NaN, 7, -1], 1 / 60);
	assert.deepEqual(m.read(), [0, 1, 0]);
	m.push([0, 0, 0], -5);
	assert.deepEqual(m.read(), [0, 1, 0], 'a negative dt is no time');
});

const cc: DeviceSignal = { kind: 'midi', port: 'out', bytes: [0xb0, 74, 127], at: 10 };

test('the monitor keeps the last message per kind and counts', () => {
	const mon = new SignalMonitor();
	assert.deepEqual(mon.view(0), emptySignals());
	mon.push(cc, 1000);
	mon.push({ kind: 'midi', port: 'conductor', bytes: [0x90, 60, 100], at: 20 }, 1010);
	mon.push({ kind: 'dmx', ch: 20, values: [255, 255], flash: false, at: 30 }, 1020);
	const v = mon.view(1020);
	assert.equal(v.midi.count, 2);
	assert.equal(mon.text('midi'), v.midi.text);
	assert.equal(v.midi.text, 'note on 60 · vel 100 · ch 1 → conductor');
	assert.equal(v.midi.at, 1010);
	assert.equal(v.dmx.last?.kind, 'dmx');
	assert.equal(v.osc.count, 0);
	assert.equal(v.osc.last, null);
	mon.reset();
	assert.deepEqual(mon.view(5000), emptySignals());
});

test('the monitor makes a message wording only when asked', () => {
	const mon = new SignalMonitor();
	mon.push(cc, 1);
	assert.equal(mon.count('midi'), 1);
	assert.equal(mon.last('midi'), cc);
	assert.equal(mon.last('osc'), null);
	assert.equal(mon.text('osc'), '');
	assert.equal(mon.text('midi'), 'CC 74 = 127 · ch 1');
	assert.equal(mon.text('midi'), mon.view(1).midi.text);
});

test('activity: 1 at a message, decays with time, never before the first, per kind', () => {
	const mon = new SignalMonitor();
	assert.equal(mon.activity('midi', 0), 0);
	mon.push(cc, 1000);
	assert.equal(mon.activity('midi', 1000), 1);
	assert.ok(Math.abs(mon.activity('midi', 1000 + ACTIVITY_SECONDS * 1000) - 1 / Math.E) < 1e-9);
	assert.ok(mon.activity('midi', 60_000) < 1e-6);
	assert.equal(mon.activity('osc', 1000), 0, 'another kind is untouched');
	assert.equal(mon.activity('midi', 900), 1, 'a clock that stepped back does not go above 1');
	mon.push(cc, 5000);
	assert.equal(mon.view(5000).midi.activity, 1, 'a new message lights it again');
});

test('describeSignal: short lines for what a monitor shows', () => {
	assert.equal(describeSignal(cc), 'CC 74 = 127 · ch 1');
	assert.equal(describeSignal({ kind: 'midi', port: 'out', bytes: [0x93, 36, 0], at: 0 }), 'note off 36 · ch 4', 'note on with velocity 0 is a note off');
	assert.equal(describeSignal({ kind: 'midi', port: 'out', bytes: [0xe1, 127, 127], at: 0 }), 'bend 16383 · ch 2');
	assert.equal(describeSignal({ kind: 'midi', port: 'out', bytes: [0xc0, 5], at: 0 }), 'program 5 · ch 1');
	assert.equal(describeSignal({ kind: 'midi', port: 'out', bytes: [0xf8], at: 0 }), 'MIDI f8');
	assert.equal(describeSignal({ kind: 'dmx', ch: 7, values: [255], flash: true, at: 0 }), 'DMX 7 = 255 · flash');
	assert.equal(describeSignal({ kind: 'dmx', ch: 1, values: [0, 128, 255], flash: false, at: 0 }), 'DMX 1–3 = 0 128 255');
	assert.equal(
		describeSignal({ kind: 'osc', address: '/fx/level', args: [{ kind: 'float', p: 0.5 }, { kind: 'string', p: 'hi' }, { kind: 'bool', p: true }], at: 0 }),
		'/fx/level 0.5 "hi" true'
	);
	assert.equal(describeSignal({ kind: 'osc', address: '/go', args: [], at: 0 }), '/go');
});

test('the empty snapshot carries no outputs and quiet signals', () => {
	const s = emptySnapshot();
	assert.deepEqual(s.outputs, []);
	assert.deepEqual(s.signals, emptySignals());
});
