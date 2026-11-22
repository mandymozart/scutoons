// autosaver.ts: the creator's autosave timing – a pause after the last edit, one save at a time, one follow-up.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { Autosaver } from '../src/lib/editor/autosaver.ts';

const tick = (ms: number) => new Promise((resolve) => setTimeout(resolve, ms));

test('saves once, a pause after the last of several edits', async () => {
	let saves = 0;
	const a = new Autosaver(async () => (saves++, true), 20);
	a.schedule();
	await tick(10);
	a.schedule();
	await tick(10);
	a.schedule();
	assert.equal(a.pending, true);
	await tick(40);
	assert.equal(saves, 1);
	assert.equal(a.pending, false);
});

test('a flush during a save waits for it and saves once more after it, never two at once', async () => {
	let running = 0;
	let most = 0;
	let saves = 0;
	const a = new Autosaver(async () => {
		running++;
		most = Math.max(most, running);
		await tick(15);
		running--;
		saves++;
		return true;
	});
	const first = a.flush();
	assert.equal(a.busy, true);
	const second = a.flush();
	const third = a.flush();
	assert.equal(second, third);
	assert.equal(await first, true);
	assert.equal(await second, true);
	assert.equal(saves, 2);
	assert.equal(most, 1);
	assert.equal(a.busy, false);
});

test('a flush cancels the waiting save; a refused or throwing save answers false', async () => {
	let saves = 0;
	const a = new Autosaver(async () => (saves++, false), 10);
	a.schedule();
	assert.equal(await a.flush(), false);
	await tick(25);
	assert.equal(saves, 1);
	const b = new Autosaver(async () => {
		throw new Error('offline');
	});
	assert.equal(await b.flush(), false);
	assert.equal(b.busy, false);
});
