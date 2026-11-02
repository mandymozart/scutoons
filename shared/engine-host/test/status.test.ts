// `npm test -w @scutoons/engine-host` (#298): the phases and how a page words them
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { canStart, canStop, emptySnapshot, leadMs, loadPercent, meterPercent, statusLine, transportNote } from '../src/status.ts';

test('Start is live once the being is there and no sound runs; after an error it tries again', () => {
	assert.deepEqual((['idle', 'mounting', 'ready', 'starting', 'running', 'stopped', 'error'] as const).filter(canStart), ['ready', 'stopped', 'error']);
	assert.deepEqual((['idle', 'mounting', 'ready', 'starting', 'running', 'stopped', 'error'] as const).filter(canStop), ['running']);
});

test('the transport in a sentence says why it is the slow one', () => {
	assert.match(transportNote('shared', true), /SharedArrayBuffer/);
	assert.match(transportNote('transfer', false), /not cross-origin isolated/);
	assert.match(transportNote(null, false), /not started.*transfer fallback/);
	assert.match(transportNote(null, true), /not started.*shared rings/);
});

test('the status line: phase, transport, 3D backend, fps; an error shows its message', () => {
	const s = emptySnapshot();
	assert.equal(statusLine(s), 'idle');
	assert.equal(statusLine({ ...s, phase: 'running', transport: 'shared', backend: 'webgpu', fps: 59.6 }), 'running · shared · 3D webgpu · 60 fps');
	assert.equal(statusLine({ ...s, phase: 'error', error: 'no worklet' }), 'error: no worklet');
});

test('meters and loads: clamped, rounded, never NaN', () => {
	assert.equal(meterPercent(0.256), 26);
	assert.equal(meterPercent(2), 100);
	assert.equal(meterPercent(-1), 0);
	assert.equal(meterPercent(NaN), 0);
	assert.equal(loadPercent(500, 2000), 25);
	assert.equal(loadPercent(5, 0), 500, 'a zero budget counts as 1 µs, not a division by zero');
	assert.equal(leadMs(2048, 48000), 43);
	assert.equal(leadMs(2048, 0), 2048000);
});
