// `npm test -w @scutoons/rig-designer`: the console's line buffer – capped and rate-limited (RIG-EDITING.md §5).
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { ConsoleBuffer } from '../src/console.ts';

test('lines keep their order and get ids', () => {
	let t = 0;
	const c = new ConsoleBuffer({ now: () => t });
	c.push('info', 'a');
	t = 5;
	c.push('ok', 'b');
	assert.deepEqual(c.lines.map((l) => [l.id, l.level, l.text, l.at]), [[1, 'info', 'a', 0], [2, 'ok', 'b', 5]]);
});

test('the oldest lines go when the cap is reached', () => {
	let t = 0;
	const c = new ConsoleBuffer({ cap: 3, perSecond: 100, now: () => t });
	for (let i = 1; i <= 5; i++) c.push('log', `l${i}`);
	assert.deepEqual(c.lines.map((l) => l.text), ['l3', 'l4', 'l5']);
});

test('more than perSecond lines in a second: the rest is counted and reported once, in the next second', () => {
	let t = 1000;
	const c = new ConsoleBuffer({ perSecond: 3, now: () => t });
	const kept = [1, 2, 3, 4, 5, 6].map((i) => c.push('log', `l${i}`));
	assert.deepEqual(kept, [true, true, true, false, false, false]);
	assert.deepEqual(c.lines.map((l) => l.text), ['l1', 'l2', 'l3']);
	t = 2100;
	assert.equal(c.push('log', 'next'), true);
	assert.deepEqual(c.lines.map((l) => l.text), ['l1', 'l2', 'l3', '… 3 lines suppressed', 'next']);
	assert.equal(c.lines[3].level, 'warn');
});

test('a compile result is never swallowed by the rate limit', () => {
	let t = 0;
	const c = new ConsoleBuffer({ perSecond: 1, now: () => t });
	c.push('log', 'noise');
	assert.equal(c.push('log', 'dropped'), false);
	c.pushAlways('ok', 'compiled');
	assert.deepEqual(c.lines.map((l) => l.text), ['noise', '… 1 lines suppressed', 'compiled']);
});

test('very long lines are cut, clear empties', () => {
	const c = new ConsoleBuffer({ now: () => 0 });
	c.push('error', 'x'.repeat(5000));
	assert.equal(c.lines[0].text.length, 2001);
	c.clear();
	assert.equal(c.lines.length, 0);
	assert.equal(c.push('info', 'again'), true);
});
