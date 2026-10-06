import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createScriptQueue, type ScriptResult } from '../src/lib/visual-novel/script-queue.ts';

/** a script that runs until `finish` is called */
function script(log: string[], id: string) {
	let finish!: (result?: ScriptResult) => void;
	const run = () => {
		log.push(`start ${id}`);
		return new Promise<ScriptResult>((resolve) => (finish = (result = 'done') => (log.push(`end ${id}`), resolve(result))));
	};
	return { run, finish: (result?: ScriptResult) => finish(result) };
}
const flush = () => new Promise((resolve) => setTimeout(resolve, 0));

/** manual timers: `tick()` runs what is due */
function timers() {
	let due: (() => void)[] = [];
	return {
		setTimer: (fn: () => void) => (due.push(fn), due.length),
		clearTimer: () => (due = []),
		tick: () => {
			const now = due;
			due = [];
			now.forEach((fn) => fn());
		}
	};
}

test('scripts of a page run one after another, in the order they were seen', async () => {
	const log: string[] = [];
	const q = createScriptQueue({ idle: () => true });
	const a = script(log, 'a');
	const b = script(log, 'b');
	q.register('a', true);
	q.register('b', true);
	assert.equal(q.done, false, 'registered scripts not run yet: the page is not done');
	q.request('b', b.run);
	q.request('a', a.run);
	q.request('b', b.run); // seen again: ignored
	assert.deepEqual(log, ['start b']);
	assert.equal(q.busy, 2);
	b.finish();
	await flush();
	assert.deepEqual(log, ['start b', 'end b', 'start a']);
	assert.equal(q.done, false);
	a.finish();
	await flush();
	assert.deepEqual(q.started, ['b', 'a']);
	assert.equal(q.busy, 0);
	assert.equal(q.done, true);
});

test('the next script waits while the reader is awaited (a message waiting for its click, a choice open)', async () => {
	const log: string[] = [];
	let idle = false;
	const t = timers();
	const q = createScriptQueue({ idle: () => idle, setTimer: t.setTimer, clearTimer: t.clearTimer });
	const a = script(log, 'a');
	q.request('a', a.run);
	assert.deepEqual(log, [], 'not idle: queued');
	t.tick();
	assert.deepEqual(log, []);
	idle = true;
	t.tick();
	assert.deepEqual(log, ['start a']);
	a.finish();
	await flush();
	assert.equal(q.done, true);
});

test('not required (above the shown group) does not hold the page; a navigating or failing script still frees the line', async () => {
	const log: string[] = [];
	const q = createScriptQueue({ idle: () => true });
	q.register('above', false);
	q.register('here', true);
	const here = script(log, 'here');
	q.request('here', here.run);
	q.request('next', () => Promise.reject(new Error('boom')));
	here.finish('navigated');
	await flush();
	await flush();
	assert.equal(q.busy, 0);
	assert.equal(q.done, true, '"above" was never seen – the page is done without it');
});

test('cancel (the page went away / "Nochmal"): nothing starts any more, the state reports idle', async () => {
	const log: string[] = [];
	const states: { busy: number; done: boolean }[] = [];
	const q = createScriptQueue({ idle: () => true, onChange: (s) => states.push(s) });
	const a = script(log, 'a');
	const b = script(log, 'b');
	q.request('a', a.run);
	q.request('b', b.run);
	q.cancel();
	assert.equal(q.busy, 0);
	a.finish('cancelled');
	await flush();
	assert.deepEqual(log, ['start a', 'end a'], 'b never starts');
	q.request('c', script(log, 'c').run);
	assert.deepEqual(log, ['start a', 'end a']);
	assert.equal(states.at(-1)?.busy, 0);
});

test('with subscribe (the box tells about changes): a waiting start goes the moment the reader answered – no timer', async () => {
	const log: string[] = [];
	let idle = false;
	const listeners = new Set<() => void>();
	const changed = () => listeners.forEach((fn) => fn());
	const t = timers();
	const q = createScriptQueue({ idle: () => idle, subscribe: (fn) => (listeners.add(fn), () => listeners.delete(fn)), ...t });
	const a = script(log, 'a');
	q.register('a', true);
	q.request('a', a.run);
	assert.deepEqual(log, [], 'the reader is awaited');
	t.tick();
	assert.deepEqual(log, [], 'no timer involved');
	idle = true;
	changed();
	assert.deepEqual(log, ['start a']);
	assert.equal(listeners.size, 0, 'it stopped listening');
	a.finish();
	await flush();
	assert.equal(q.done, true);
});
