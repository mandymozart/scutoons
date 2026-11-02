import { test } from 'node:test';
import assert from 'node:assert/strict';
import { EngineLog, formatLine, messageOf } from '../src/log.ts';

test('lines are kept in order with a number each; an empty line is nothing', () => {
	const log = new EngineLog(10, () => 1000);
	log.push('host', 'mounting');
	log.push('3d', '3D webgpu');
	log.push('3d', '   ');
	assert.deepEqual(log.lines.map((l) => [l.id, l.source, l.text]), [[1, 'host', 'mounting'], [2, '3d', '3D webgpu']]);
});

test('a line repeated straight away is counted, not listed again – and the buffer is a new array for a keyed list', () => {
	let t = 0;
	const log = new EngineLog(10, () => ++t);
	log.push('sound', 'dropout', 'warn');
	const before = log.lines;
	log.push('sound', 'dropout', 'warn');
	log.push('sound', 'dropout', 'warn');
	assert.equal(log.lines.length, 1);
	assert.equal(log.lines[0].count, 3);
	assert.equal(log.lines[0].time, 3);
	assert.notEqual(log.lines, before);
	log.push('sound', 'dropout', 'error');
	assert.equal(log.lines.length, 2, 'another level is another line');
});

test('the buffer is bounded: the oldest go', () => {
	const log = new EngineLog(3);
	for (let i = 0; i < 6; i++) log.push('host', `line ${i}`);
	assert.deepEqual(log.lines.map((l) => l.text), ['line 3', 'line 4', 'line 5']);
	assert.equal(log.lines[2].id, 6);
});

test('subscribers get the buffer now and on every change, until they leave', () => {
	const log = new EngineLog();
	const seen: number[] = [];
	const off = log.subscribe((l) => seen.push(l.length));
	log.push('host', 'a');
	log.push('host', 'b');
	off();
	log.push('host', 'c');
	log.clear();
	assert.deepEqual(seen, [0, 1, 2]);
	assert.equal(log.lines.length, 0);
});

test('a line as text; an error as a message', () => {
	const log = new EngineLog(5, () => new Date(2026, 10, 2, 12, 3, 41, 250).getTime());
	log.push('3d', '3D webgpu');
	log.push('3d', '3D webgpu');
	assert.equal(formatLine(log.lines[0]), '12:03:41.250 [3d] 3D webgpu ×2');
	assert.equal(messageOf(new Error('boom')), 'boom');
	assert.equal(messageOf('plain'), 'plain');
});
