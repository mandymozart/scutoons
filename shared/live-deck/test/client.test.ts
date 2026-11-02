// `npm test -w shared/live-deck` (#197): the browser deck's
// client logic – the graph message, the values stream (only what moved, ≤ 30 / s, the mood), and in process against a
// real relay (deck/target/debug/relay.exe on LAB_RELAY_PORT, default 7072; skipped, saying so, when it isn't built):
// a conductor sees the deck's `deck.graph` first, then its `deck.values`.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { existsSync, readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { DeckLink, envelope, epsilon, parseRelay, RATE, ValueStream } from '../src/client.ts';
import { deckGraph, ladderFor, LADDER, Moods, PARAMS, STREAMED, toPos, toValue } from '../src/scene.ts';
import type { Envelope } from '@scutoons/live';

test('the relay URL and the envelope', () => {
	assert.deepEqual(parseRelay('ws://localhost:7070/live/CATH'), { base: 'ws://localhost:7070', code: 'CATH' });
	// a venue's relay under our name – the deck's LAN IP with dashes (bunny DNS answers it) –, wss with the wildcard certificate (#201)
	assert.deepEqual(parseRelay('wss://192-168-1-116.lan.scutoons.com:7700/live/CATH'), { base: 'wss://192-168-1-116.lan.scutoons.com:7700', code: 'CATH' });
	assert.deepEqual(parseRelay('wss://live.scutoons.com/live/A7K2/'), { base: 'wss://live.scutoons.com', code: 'A7K2' });
	assert.throws(() => parseRelay('http://x/live/CATH'));
	assert.equal(envelope({ kind: 'ping', p: { t0: 0 } }), '{"v":1,"seq":0,"from":"","kind":"ping","p":{"t0":0}}');
});

test('deck.graph: deck-stage kinds, links between known ports, the character, its moods, the exposed params', () => {
	const g = deckGraph('yellowmouse', ['neutral', 'happy']);
	const ids = new Set(g.nodes.map((n) => n.id));
	for (const l of g.links) for (const end of [l.from, l.to]) assert.ok(ids.has(end.split('.')[0]), end);
	assert.deepEqual(g.nodes.map((n) => n.kind).sort(), ['audio_out', 'fft', 'fm', 'lfo', 'mixer', 'noise', 'osc', 'seq', 'space']);
	assert.equal(g.character, 'yellowmouse');
	assert.deepEqual(g.moods, ['neutral', 'happy']);
	assert.deepEqual(g.expose, Object.keys(PARAMS));
	for (const e of g.expose!) assert.ok(ids.has(e.split('.')[0]), e);
	// deck-stage's link.rs test builds this fixture as the conductor would: keep it equal
	const fixture = JSON.parse(readFileSync(new URL('../../../deck/lab/fixtures/deck-graph.json', import.meta.url), 'utf8'));
	assert.deepEqual(fixture, { kind: 'deck.graph', p: deckGraph('yellowmouse', LADDER) });
});

test('values: the first sample sends all, then only what moved past its epsilon, ≤ 30 / s, the mood on change', () => {
	const s = new ValueStream({ 'voice.index': { input: [0, 8] }, 'voice.out': 'audio' });
	const v: Record<string, number> = { 'voice.index': 2, 'voice.out': 0.5 };
	const first = s.sample(0, (p) => v[p], 'neutral');
	assert.deepEqual(first, { values: { 'voice.index': [2], 'voice.out': [0.5] }, mood: 'neutral' });
	assert.equal(s.sample(0.01, (p) => v[p], 'neutral'), null, 'between two slots');
	v['voice.index'] = 2 + epsilon({ input: [0, 8] }) / 2;
	assert.equal(s.sample(1 / RATE, (p) => v[p], 'neutral'), null, 'under its epsilon');
	v['voice.out'] = 0.6;
	assert.deepEqual(s.sample(2 / RATE, (p) => v[p], 'happy'), { values: { 'voice.out': [0.6] }, mood: 'happy' });
	// 120 frames a second for 2 s, always moving: ≈ 60 samples
	let n = 0;
	for (let f = 0; f < 240; f++) {
		v['voice.out'] = f / 240;
		if (s.sample(10 + f / 120, (p) => v[p], 'happy')) n++;
	}
	assert.ok(n >= 59 && n <= 61, `30 a second: ${n}`);
});

test('the streamed paths: every exposed input + the hoses', () => {
	for (const p of Object.keys(PARAMS)) assert.ok(p in STREAMED, p);
	for (const p of ['voice.out', 'mix.out', 'fft.level']) assert.ok(p in STREAMED, p);
	for (const s of Object.values(PARAMS)) for (const t of [0, 0.3, 1]) assert.ok(Math.abs(toPos(s, toValue(s, t)) - t) < 1e-9);
});

test('moods: the ladder from what the character has; energy climbs it, a rung holds', () => {
	assert.deepEqual(ladderFor(['neutral']), ['neutral']);
	assert.deepEqual(ladderFor(['sad', 'happy', 'neutral', 'surprised']), ['neutral', 'happy', 'surprised']);
	assert.deepEqual(ladderFor(LADDER), LADDER);
	const m = new Moods(LADDER, 1);
	let t = 0;
	let mood = '';
	for (; t < 1; t += 0.05) mood = m.update(t, 0.05, 0, 0);
	assert.equal(mood, 'neutral');
	for (; t < 10; t += 0.05) mood = m.update(t, 0.05, 1, 0);
	assert.equal(mood, 'laughing', 'loud at 84 bpm (being.rs: 0.8 level + 0.2 tempo)');
	for (; t < 20; t += 0.05) mood = m.update(t, 0.05, 1, 1);
	assert.equal(mood, 'surprised', 'a finger on a pad lifts it to the top');
});

test('moods: a pinned mood (the designer chips) holds while the energy keeps climbing; unpinning follows the ladder', () => {
	const m = new Moods(LADDER, 1);
	m.pinned = 'surprised';
	let t = 0;
	let mood = '';
	for (; t < 2; t += 0.05) mood = m.update(t, 0.05, 0, 0);
	assert.equal(mood, 'surprised', 'silence, yet the pinned one');
	m.pinned = 'not-on-the-ladder';
	mood = m.update(t, 0.05, 0, 0);
	assert.equal(mood, 'neutral', 'a pin the character does not have is ignored');
	m.pinned = null;
	for (; t < 12; t += 0.05) mood = m.update(t, 0.05, 1, 0);
	assert.equal(mood, 'laughing');
});

/** a raw conductor on the relay: its messages */
function conductor(url: string): Promise<{ msgs: Envelope[]; ws: WebSocket }> {
	const { code } = parseRelay(url);
	const ws = new WebSocket(`${url}?role=conductor&group=decks`);
	const msgs: Envelope[] = [];
	return new Promise((ok, fail) => {
		ws.onopen = () => ws.send(envelope({ kind: 'hello', p: { code, role: 'conductor', group: 'decks', token: 'dev' } }));
		ws.onmessage = (e) => {
			const m = JSON.parse(String(e.data)) as Envelope;
			msgs.push(m);
			if (m.kind === 'welcome') ok({ msgs, ws });
		};
		ws.onerror = () => fail(new Error('conductor: no relay'));
	});
}

const until = async (what: () => boolean, ms = 5000) => {
	const end = Date.now() + ms;
	while (!what()) {
		if (Date.now() > end) throw new Error('timed out');
		await new Promise((r) => setTimeout(r, 20));
	}
};

const relayExe = fileURLToPath(new URL('../../../deck/target/debug/relay.exe', import.meta.url));
test('in process: a relay, a conductor, the deck – graph first, values rate-limited, again after a reconnect', { skip: !existsSync(relayExe) && 'deck/target/debug/relay.exe not built (cargo build -p relay)' }, async () => {
	const port = Number(process.env.LAB_RELAY_PORT ?? 7072);
	const env = { ...process.env };
	delete env.LIVE_TOKEN_SECRET;
	const relay = spawn(relayExe, [`127.0.0.1:${port}`], { env, stdio: 'ignore' });
	try {
		const url = `ws://127.0.0.1:${port}/live/LAB`;
		let c: Awaited<ReturnType<typeof conductor>> | null = null;
		const end = Date.now() + 5000;
		while (!c) {
			try {
				c = await conductor(url);
			} catch (e) {
				if (Date.now() > end) throw e;
				await new Promise((r) => setTimeout(r, 100));
			}
		}
		const deck = new DeckLink({ relay: url, graph: deckGraph('yellowmouse', ['neutral', 'happy']), socket: WebSocket as never });
		await until(() => deck.joined);
		// 1 s of values at 120 frames a second, through the stream
		const s = new ValueStream(STREAMED);
		const t0 = performance.now();
		let f = 0;
		while (performance.now() - t0 < 1000) {
			const ch = s.sample((performance.now() - t0) / 1000, (p) => (p === 'voice.out' ? Math.abs(Math.sin(f)) : (PARAMS[p]?.value ?? 0)), f > 60 ? 'happy' : 'neutral');
			if (ch) deck.send(ch);
			f++;
			await new Promise((r) => setTimeout(r, 8));
		}
		await until(() => c!.msgs.some((m) => m.kind === 'deck.values' && m.p.mood === 'happy'));
		const mine = c.msgs.filter((m) => m.from === deck.client);
		assert.equal(mine[0].kind, 'deck.graph', `the graph first: ${mine.map((m) => m.kind).join(' ')}`);
		assert.equal((mine[0].p as { character?: string }).character, 'yellowmouse');
		const values = mine.filter((m) => m.kind === 'deck.values');
		assert.ok(values.length >= 20 && values.length <= 33, `≤ 30 / s: ${values.length}`);
		// the relay drops the being when the deck leaves; the deck joins again with graph + keyframe
		const before = c.msgs.length;
		(deck as unknown as { ws: WebSocket }).ws.close();
		await until(() => c!.msgs.slice(before).some((m) => m.kind === 'deck.values' && Object.keys(m.p.values).length >= Object.keys(STREAMED).length - 2));
		const again = c.msgs.slice(before).filter((m) => m.kind === 'deck.graph' || m.kind === 'deck.values');
		assert.equal(again[0].kind, 'deck.graph');
		deck.close();
		c.ws.close();
	} finally {
		relay.kill();
	}
});

test('#220: the render quality goes out once joined and again after a rejoin', () => {
	const sockets: FakeSocket[] = [];
	class FakeSocket {
		readyState = 1;
		sent: { kind: string; p: unknown }[] = [];
		onopen: ((ev: unknown) => void) | null = null;
		onmessage: ((ev: { data: unknown }) => void) | null = null;
		onclose: ((ev: unknown) => void) | null = null;
		onerror: ((ev: unknown) => void) | null = null;
		constructor() {
			sockets.push(this);
		}
		send(data: string) {
			this.sent.push(JSON.parse(data));
		}
		close() {}
	}
	const link = new DeckLink({ relay: 'ws://localhost:1/live/CATH', graph: deckGraph('', LADDER), socket: FakeSocket });
	const ws = sockets[0];
	const q = { level: 'medium' as const, scale: 0.8, auto: true, frame: 17.1, ceiling: 'medium' as const };
	link.quality(q);
	assert.equal(link.count.quality, 0, 'not joined: kept');
	const welcome = { kind: 'welcome', p: { client: 'c5', snapshot: { seq: 0 }, now: 0 } };
	ws.onmessage!({ data: JSON.stringify(welcome) });
	assert.deepEqual(ws.sent.map((m) => m.kind), ['deck.graph', 'deck.quality']);
	link.quality({ ...q, level: 'low', scale: 0.6 });
	assert.deepEqual(ws.sent.at(-1), { v: 1, seq: 0, from: '', kind: 'deck.quality', p: { ...q, level: 'low', scale: 0.6 } });
	assert.equal(link.count.quality, 2);
	link.close();
});
