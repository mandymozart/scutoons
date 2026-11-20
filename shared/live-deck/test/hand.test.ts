// The handheld camera's link (src/hand.ts, #469): the session's relay URL, and a phone and an editor on the real core
// (relay-web's wasm) over a fake WebSocket – what each page hands in reaches the socket as the core words it, what the
// core hears comes out, the rung is the core's.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import type { Envelope, HandView } from '@scutoons/live';
import { HandLink, handRelay } from '../src/hand.ts';
import { CameraLink, type Holding } from '../src/camera.ts';
import type { WebSocketLike } from '../src/client.ts';
import { NO_CORE, relayCore } from './relay-core.ts';

interface FollowCore {
	pose(t: number, p: Float64Array, q: Float64Array): boolean;
	keep(on: boolean): void;
	secs(): number;
	held(): Float64Array;
	take(): string;
	free(): void;
}

test('the session URL: the relay, the session, the QUIC door only when named', () => {
	assert.equal(handRelay('ws://127.0.0.1:7070/', 'HQ7K2M'), 'ws://127.0.0.1:7070/live/HQ7K2M');
	assert.equal(handRelay('wss://relay.test', 'HQ7K2M', 'udp.test:443'), 'wss://relay.test/live/HQ7K2M?quic=udp.test%3A443');
});

class FakeSocket implements WebSocketLike {
	static last: FakeSocket;
	readyState = 0;
	sent: string[] = [];
	onopen: ((ev: unknown) => void) | null = null;
	onmessage: ((ev: { data: unknown }) => void) | null = null;
	onclose: ((ev: unknown) => void) | null = null;
	onerror: ((ev: unknown) => void) | null = null;
	constructor(readonly url: string) {
		FakeSocket.last = this;
	}
	send(data: string) {
		this.sent.push(data);
	}
	close() {
		this.readyState = 3;
	}
	open() {
		this.readyState = 1;
		this.onopen?.({});
	}
	say(e: object) {
		this.onmessage?.({ data: JSON.stringify({ v: 1, seq: 1, from: 'relay', ...e }) });
	}
}

const welcome = (client: string) => ({ kind: 'welcome', p: { client, snapshot: {}, now: 0, key: `key-${client}` } });
const kinds = (s: FakeSocket) => s.sent.map((t) => (JSON.parse(t) as Envelope).kind);
const platform = { socket: FakeSocket, now: () => 1000, setTimer: () => 0, clearTimer: () => undefined };

test('a phone: its state kept for the join, its pose and controls on the socket, the editor\'s view heard, the rung', { skip: NO_CORE || false }, async () => {
	const rungs: string[] = [];
	const heard: HandView[] = [];
	const phone = new HandLink({ ...platform, relay: 'ws://relay.test:7070/live/HQ7K2M', role: 'handheld', token: 't-hand', core: await relayCore(), onStatus: (s, rung) => rungs.push(`${s.state}:${rung}`), onView: (v) => heard.push(v) });
	await Promise.resolve();
	phone.state({ mode: 'none', tracking: true, ua: 'S22' });
	phone.pose(1, [0, 1.5, 0], [0, 0, 0, 1]); // not joined yet: worth nothing
	const ws = FakeSocket.last;
	assert.equal(ws.url, 'ws://relay.test:7070/live/HQ7K2M?role=handheld');
	ws.open();
	ws.say(welcome('c2'));
	phone.pose(200, [0.1, 1.5, 0], [0, 0, 0, 1]);
	phone.record(true);
	phone.recenter();
	assert.deepEqual(kinds(ws), ['hello', 'hand.state', 'hand.pose', 'hand.record', 'hand.recenter']);
	assert.equal(JSON.parse(ws.sent[0]).p.token, 't-hand');
	assert.equal(rungs.at(-1), 'connected:WebSocket');
	ws.say({ from: 'c1', kind: 'hand.view', p: { shot: 'sb-1', on: true, recording: true, seconds: 1.5 } });
	assert.deepEqual(heard, [{ shot: 'sb-1', on: true, recording: true, seconds: 1.5 }]);
	phone.close();
});

test('the editor: a phone\'s newest pose only, its view sent again to a phone that joins later', { skip: NO_CORE || false }, async () => {
	const heard: string[] = [];
	const editor = new HandLink({
		...platform,
		relay: 'ws://relay.test:7070/live/HQ7K2M',
		role: 'conductor',
		token: 't-editor',
		core: await relayCore(),
		onPose: (c, p) => heard.push(`${c} pose ${p.t}`),
		onRecord: (c, on) => heard.push(`${c} record ${on}`),
		onRecenter: (c) => heard.push(`${c} recenter`),
		onAway: (c) => heard.push(`${c} away`)
	});
	editor.view({ shot: 'sb-1', on: true, recording: false, seconds: 0 });
	await Promise.resolve();
	const ws = FakeSocket.last;
	ws.open();
	ws.say(welcome('c1'));
	const pose = (t: number) => ({ from: 'c2', kind: 'hand.pose', p: { t, p: [0, 1.5, 0], q: [0, 0, 0, 1] } });
	ws.say(pose(300));
	ws.say(pose(250));
	ws.say(pose(316));
	ws.say({ from: 'c2', kind: 'hand.record', p: { on: true } });
	ws.say({ from: 'c2', kind: 'hand.recenter', p: {} });
	ws.say({ kind: 'presence', p: { groups: {}, ready: [], links: {} } });
	assert.deepEqual(heard, ['c2 pose 300', 'c2 pose 316', 'c2 record true', 'c2 recenter', 'c2 away'], 'the late pose is dropped by the core');
	assert.deepEqual(kinds(ws), ['hello', 'hand.view', 'hand.view'], 'at the welcome, and to the phone heard after it');
	assert.deepEqual(JSON.parse(ws.sent[2]).to, { client: 'c2' });
	editor.close();
});

test('a reader phone: its claim kept for the join, its hold from the relay\'s word, again by itself after a drop', { skip: NO_CORE || false }, async () => {
	const holds: Holding[] = [];
	const lists: number[] = [];
	const phone = new CameraLink({ ...platform, relay: 'ws://relay.test:7070/live/HQ7K2M', key: 'k-phone', core: await relayCore(), onHolding: (h) => holds.push(h), onCameras: (c) => lists.push(c.length) });
	phone.claim('scene:duet');
	await Promise.resolve();
	const ws = FakeSocket.last;
	ws.open();
	ws.say(welcome('c4'));
	const hello = JSON.parse(ws.sent[0]);
	assert.equal(hello.p.device, 'phone');
	assert.deepEqual(JSON.parse(ws.sent[1]), { ...JSON.parse(ws.sent[1]), kind: 'camera.claim', p: { camera: 'scene:duet', key: 'k-phone' } });
	ws.say({ kind: 'cameras', p: { cameras: [{ id: 'scene:duet', holder: 'c4' }] } });
	assert.equal(holds.at(-1)?.hold, 'held');
	assert.deepEqual(lists, [1]);
	ws.say({ kind: 'cameras', p: { cameras: [{ id: 'scene:duet', holder: 'c5', nick: 'Ben' }] } });
	assert.deepEqual(holds.at(-1), { hold: 'taken_over', camera: 'scene:duet', other: 'Ben' });
	phone.claim('scene:duet');
	ws.say({ kind: 'cameras', p: { cameras: [{ id: 'scene:duet', holder: 'c4' }] } });
	assert.equal(holds.at(-1)?.hold, 'held', 'taken back');
	phone.leave();
	assert.equal(JSON.parse(ws.sent.at(-1)!).kind, 'camera.leave');
	phone.link.close();
});

test('a reader phone: its proof fetched with the sign-in before the first hello, which carries it', { skip: NO_CORE || false }, async () => {
	const asked: { url: string; auth: string | null }[] = [];
	const real = globalThis.fetch;
	globalThis.fetch = (async (url: string, init?: RequestInit) => {
		asked.push({ url, auth: new Headers(init?.headers).get('Authorization') });
		return new Response(JSON.stringify({ proof: 'p.s', expires_in: 3600 }), { status: 200 });
	}) as typeof fetch;
	try {
		const soon = { ...platform, setTimer: (f: () => void, ms: number) => (ms <= 0 ? (queueMicrotask(f), 0) : 0) };
		const phone = new CameraLink({ ...soon, relay: 'ws://relay.test:7070/live/HQ7K2M', key: 'k-phone', core: await relayCore(), api: 'https://api.test/api', user: { bearer: 'tok' } });
		for (let i = 0; i < 5; i++) await new Promise((r) => setTimeout(r, 0));
		assert.deepEqual(asked, [{ url: 'https://api.test/api/live/proof?session=HQ7K2M', auth: 'Bearer tok' }]);
		const ws = FakeSocket.last;
		ws.open();
		assert.equal(JSON.parse(ws.sent[0]).p.proof, 'p.s', 'the first hello waited for it');
		phone.link.close();
	} finally {
		globalThis.fetch = real;
	}
});


test('the storyboard\'s follow: the core follows the phone from the shot camera, keeps a take on Record and thins it to keys', { skip: NO_CORE || false }, async () => {
	const { TakeFollow } = await relayCore();
	// the module types the follow as an object: the creator's FollowCore (hand-port.ts) says what it has
	const f = new TakeFollow(Float64Array.of(0, 1.6, 4), Float64Array.of(0, 0, 0, 1), 1) as unknown as FollowCore;
	assert.equal(f.held().length, 0, 'no pose yet');
	assert.equal(f.pose(0, Float64Array.of(0, 0, 0), Float64Array.of(0, 0, 0, 1)), true);
	assert.equal(f.pose(0, Float64Array.of(0, 0, 0), Float64Array.of(0, 0, 0, 1)), false, 'a late pose is dropped');
	assert.equal(f.take(), '', 'nothing kept before Record');
	f.keep(true);
	// 2 m to the right over 4 s at 60 Hz, the hand shaking a little
	for (let i = 0; i <= 240; i++) f.pose(1000 + (i * 1000) / 60, Float64Array.of(Math.min(i / 120, 2), 0.002 * ((i * 7919) % 13) / 13, 0), Float64Array.of(0, 0, 0, 1));
	f.keep(false);
	const keys = JSON.parse(f.take()) as { t: number; position: number[]; quaternion: number[] }[];
	assert.ok(keys.length >= 2 && keys.length < 60, `thinned: ${keys.length}`);
	assert.equal(keys[0].t, 0);
	assert.ok(Math.abs(keys.at(-1)!.t - 4) < 1e-3);
	assert.ok(Math.abs(keys.at(-1)!.position[0] - 2) < 0.05 && Math.abs(keys[0].position[0]) < 0.05, 'from the shot camera, 2 m to the right');
	assert.ok(Math.abs(f.secs() - 4) < 1e-3);
	const held = f.held();
	assert.equal(held.length, 7);
	f.free();
});
