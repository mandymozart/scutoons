// `npm test -w @scutoons/engine-host` (#298): the start gate – idle → starting → running | blocked
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { GATE_EVENTS, StartGate, gateNext, gateOpen, listenForGesture, type GateState } from '../src/gate.ts';

test('the state machine: idle → starting → running | blocked; a refused start can be tried again', () => {
	assert.equal(gateNext('idle', 'gesture'), 'starting');
	assert.equal(gateNext('starting', 'gesture'), 'starting', 'a second gesture while it starts is nothing');
	assert.equal(gateNext('starting', 'started'), 'running');
	assert.equal(gateNext('starting', 'refused'), 'blocked');
	assert.equal(gateNext('blocked', 'gesture'), 'starting');
	assert.equal(gateNext('running', 'gesture'), 'running');
	assert.equal(gateNext('running', 'refused'), 'running');
	assert.equal(gateNext('idle', 'started'), 'idle', 'a result without a start is nothing');
	assert.deepEqual((['idle', 'starting', 'running', 'blocked'] as GateState[]).filter(gateOpen), ['idle', 'starting', 'blocked']);
});

test('the first gesture starts once, synchronously (inside the gesture); more gestures do not start again', async () => {
	let calls = 0;
	const seen: GateState[] = [];
	const gate = new StartGate(
		async () => {
			calls++;
			return true;
		},
		(s) => seen.push(s)
	);
	const first = gate.gesture();
	assert.equal(calls, 1, 'start() was called before the first await');
	void gate.gesture();
	await first;
	void gate.gesture();
	assert.equal(calls, 1);
	assert.deepEqual(seen, ['starting', 'running']);
});

test('a refused start leaves the Play button and the next gesture tries again', async () => {
	let ok = false;
	let calls = 0;
	const gate = new StartGate(async () => {
		calls++;
		return ok;
	});
	await gate.gesture();
	assert.equal(gate.state, 'blocked');
	ok = true;
	await gate.gesture();
	assert.equal(gate.state, 'running');
	assert.equal(calls, 2);
	const throws = new StartGate(() => Promise.reject(new Error('no')));
	await throws.gesture();
	assert.equal(throws.state, 'blocked', 'a rejection is a refusal, not a crash');
});

test('listeners: capture, passive, never swallow the gesture; they are gone once the sound runs', async () => {
	const target = new EventTarget();
	const added: { type: string; options: unknown }[] = [];
	const removed: string[] = [];
	const addEventListener = target.addEventListener.bind(target);
	const removeEventListener = target.removeEventListener.bind(target);
	target.addEventListener = (type: string, cb: EventListenerOrEventListenerObject | null, options?: boolean | AddEventListenerOptions) => {
		added.push({ type, options });
		addEventListener(type, cb, options);
	};
	target.removeEventListener = (type: string, cb: EventListenerOrEventListenerObject | null, options?: boolean | EventListenerOptions) => {
		removed.push(type);
		removeEventListener(type, cb, options);
	};
	let calls = 0;
	const gate = new StartGate(async () => {
		calls++;
		return true;
	});
	listenForGesture(target, gate);
	assert.deepEqual(added.map((a) => a.type), [...GATE_EVENTS]);
	for (const a of added) assert.deepEqual(a.options, { capture: true, passive: true });

	const ev = new Event('pointerdown', { cancelable: true });
	target.dispatchEvent(ev);
	assert.equal(ev.defaultPrevented, false, 'the click still gets its default');
	assert.equal(calls, 1);
	await Promise.resolve();
	await Promise.resolve();
	assert.equal(gate.state, 'running');
	assert.deepEqual(removed, [...GATE_EVENTS], 'listeners removed once running');
	target.dispatchEvent(new Event('keydown'));
	assert.equal(calls, 1);
});

test('removal on destroy, before the sound ever ran', () => {
	const target = new EventTarget();
	let calls = 0;
	const gate = new StartGate(async () => {
		calls++;
		return true;
	});
	const off = listenForGesture(target, gate);
	off();
	target.dispatchEvent(new Event('touchend'));
	assert.equal(calls, 0);
});

test('the page started the sound itself: the gate is done', () => {
	const gate = new StartGate(async () => true);
	let done = 0;
	gate.whenRunning(() => done++);
	gate.markRunning();
	assert.equal(gate.state, 'running');
	assert.equal(done, 1);
	gate.whenRunning(() => done++);
	assert.equal(done, 2, 'late subscribers run at once');
});
