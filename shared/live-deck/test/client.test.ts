// `npm test -w shared/live-deck` (#197, #366): the browser's relay link is deck-space's relay client (relay.rs, relay-web's
// wasm) under client.ts's socket. The core's scripted links (test/fixtures/relay.json, written and kept current by
// `UPDATE_FIXTURES=1 cargo test -p deck-space relay_fixture`) run through `RelaySocket` on a fake socket, clock and timer:
// every turn must ask and say what the core said natively, and the socket must do it. Then, in process against a real
// relay (deck/target/debug/relay.exe on LAB_RELAY_PORT, default 7072): a conductor sees the deck's `deck.graph` first,
// then its `deck.values`, both again after a reconnect; the show clock's pong and anchors. The ones that need the wasm or
// the relay skip, saying so, when it isn't built.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { existsSync, readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { DeckLink, RelaySocket, type RelayCoreView, type RelayModule, type WebSocketLike } from '../src/client.ts';
import { MOODS, OWN, OWN_PARAMS, OWN_STREAMED, ownGraph } from './own.ts';
import { NO_CORE, relayCore } from './relay-core.ts';
import type { Envelope, Tempo } from '@scutoons/live';

interface Step {
	at: number;
	do: string;
	with?: unknown;
}
interface Script {
	name: string;
	join: Record<string, unknown>;
	graph: unknown;
	stream: unknown[];
	steps: Step[];
	turns: { at: number; do: string; outs: Record<string, unknown>[] }[];
}
const FIXTURE = JSON.parse(readFileSync(new URL('./fixtures/relay.json', import.meta.url), 'utf8')) as { scripts: Script[] };
/** the native roles' scripts (the conductor's push, the outlet's resume) – the browser has neither */
const NATIVE_ONLY = new Set(['conductor-push', 'outlet']);

test('deck.graph – its own scene as the core played it (#360): deck-native kinds, links between known ports, the character, its moods, the exposed params', () => {
	const g = ownGraph();
	const ids = new Set(g.nodes.map((n) => n.id));
	for (const l of g.links) for (const end of [l.from, l.to]) assert.ok(ids.has(end.split('.')[0]), end);
	assert.deepEqual(g.nodes.map((n) => n.kind).sort(), ['audio_out', 'chapter_music', 'click', 'fft', 'fm', 'lfo', 'mixer', 'noise', 'osc', 'screen', 'seq', 'space', 'tempo'], "the X1's cathedral (#360)");
	assert.equal(g.character, 'yellowmouse');
	assert.deepEqual(g.moods, MOODS, "the core's ladder");
	assert.deepEqual(g.expose, Object.keys(OWN_PARAMS));
	for (const e of g.expose!) assert.ok(ids.has(e.split('.')[0]), e);
	// deck-native's link.rs test builds this fixture as the conductor would: keep it equal
	const fixture = JSON.parse(readFileSync(new URL('../../../deck/lab/fixtures/deck-graph.json', import.meta.url), 'utf8'));
	assert.deepEqual(fixture, { kind: 'deck.graph', p: ownGraph() });
});

test('the streamed paths: every exposed input + the hoses', () => {
	for (const p of Object.keys(OWN_PARAMS)) assert.ok(p in OWN_STREAMED, p);
	for (const p of ['fft.level', 'fft.bass']) assert.ok(p in OWN_STREAMED, p);
});

/** a socket the script drives */
class FakeSocket implements WebSocketLike {
	static all: FakeSocket[] = [];
	readyState = 0;
	sent: string[] = [];
	closed = false;
	onopen: ((ev: unknown) => void) | null = null;
	onmessage: ((ev: { data: unknown }) => void) | null = null;
	onclose: ((ev: unknown) => void) | null = null;
	onerror: ((ev: unknown) => void) | null = null;
	constructor(readonly url: string) {
		FakeSocket.all.push(this);
	}
	send(data: string) {
		assert.equal(this.readyState, 1, 'sent on an open socket');
		this.sent.push(data);
	}
	close() {
		this.closed = true;
	}
}

/** One script through `RelaySocket` on the real core: the outs of each step (with the timers it fired before it). */
function replay(m: RelayModule, s: Script): { groups: Record<string, unknown>[][]; sockets: FakeSocket[] } {
	FakeSocket.all = [];
	let now = 0;
	const timers: { at: number; fn: () => void; id: number }[] = [];
	let ids = 0;
	// a vote screen's moves too (the viewer's live-vote-link.ts types them)
	const core = new m.RelayClient(JSON.stringify(s.join)) as RelayCoreView & { vote_open(j: string): boolean; vote_cast(o: number): boolean; vote_close(o?: number): boolean; vote_hold(h: boolean): boolean };
	let group: Record<string, unknown>[] = [];
	const groups = [group];
	// every word of the core, as the socket took it
	const spied = new Proxy(core, {
		get(c, k) {
			if (k === 'take')
				return () => {
					const t = c.take();
					group.push(...(JSON.parse(t) as Record<string, unknown>[]));
					return t;
				};
			const v = Reflect.get(c, k) as unknown;
			return typeof v === 'function' ? (v as (...a: unknown[]) => unknown).bind(c) : v;
		}
	}) as RelayCoreView;
	if (s.graph) core.set_graph(JSON.stringify(s.graph));
	core.set_stream(JSON.stringify(s.stream));
	const sock = new RelaySocket(spied, {}, {
		socket: FakeSocket,
		now: () => now,
		setTimer: (fn, ms) => (timers.push({ at: now + ms, fn, id: ++ids }), ids),
		clearTimer: (id) => timers.splice(timers.findIndex((t) => t.id === id) >>> 0, timers.some((t) => t.id === id) ? 1 : 0)
	});
	sock.start();
	const ws = () => FakeSocket.all.at(-1)!;
	for (const st of s.steps) {
		group = [];
		groups.push(group);
		const t = st.at * 1000;
		for (;;) {
			timers.sort((a, b) => a.at - b.at);
			const due = timers[0];
			if (!due || due.at > t) break;
			timers.shift();
			now = Math.max(due.at, core.next_wake() ?? due.at);
			due.fn();
		}
		now = t;
		const w = st.with as never;
		const call = (fn: () => unknown) => (fn(), sock.run());
		switch (st.do) {
			case 'open':
				ws().readyState = 1;
				ws().onopen?.({});
				break;
			case 'close':
			case 'fail':
				ws().readyState = 3;
				ws().onclose?.({ reason: w ?? '' });
				break;
			case 'message':
				ws().onmessage?.({ data: JSON.stringify({ v: 1, seq: 1, from: 'relay', ...(w as object) }) });
				break;
			case 'sample': {
				const v = w as { values: Record<string, number>; mood?: string };
				call(() => core.sample(t / 1000, Float64Array.from(core.paths(), (p) => v.values[p] ?? NaN), v.mood ?? ''));
				break;
			}
			case 'quality':
				call(() => core.quality(JSON.stringify(w)));
				break;
			case 'synced':
				call(() => core.synced(now, JSON.stringify(w)));
				break;
			case 'tempo':
				call(() => core.tempo(JSON.stringify(w)));
				break;
			case 'ping':
				call(() => core.ping(now));
				break;
			case 'vote_open':
				call(() => core.vote_open(JSON.stringify(w)));
				break;
			case 'vote_cast':
				call(() => core.vote_cast(w));
				break;
			case 'vote_close':
				call(() => core.vote_close(w ?? undefined));
				break;
			case 'vote_hold':
				call(() => core.vote_hold(w));
				break;
			case 'stop':
				sock.close();
				break;
			default:
				throw new Error(`${s.name}: the browser has no step '${st.do}'`);
		}
	}
	return { groups, sockets: FakeSocket.all };
}

for (const s of FIXTURE.scripts.filter((x) => !NATIVE_ONLY.has(x.name))) {
	test(`#366 the core's script '${s.name}' through the socket: the same turns as natively, and the socket does them`, { skip: NO_CORE || false }, async () => {
		const { groups, sockets } = replay(await relayCore(), s);
		// the native turns per step: a step's = the timers' turns before it + its own
		const steps: Record<string, unknown>[][] = [[]];
		let pending: Record<string, unknown>[] = [];
		for (const t of s.turns) {
			if (t.do === 'start') steps[0].push(...t.outs);
			else if (t.do === 'wake') pending.push(...t.outs);
			else {
				steps.push([...pending, ...t.outs]);
				pending = [];
			}
		}
		assert.equal(groups.length, steps.length);
		groups.forEach((g, i) => assert.deepEqual(g, steps[i], `${s.name}: step ${i} (${i ? s.steps[i - 1].do : 'start'} at ${i ? s.steps[i - 1].at : 0} s)`));
		// the socket did what the core asked: one socket per dial, at its URL, each sent text on it
		const flat = steps.flat();
		assert.deepEqual(sockets.map((w) => w.url), flat.filter((o) => 'dial' in o).map((o) => o.dial));
		assert.deepEqual(sockets.flatMap((w) => w.sent), flat.filter((o) => 'send' in o).map((o) => o.send));
	});
}

/** a raw conductor on the relay: its messages */
function conductor(url: string): Promise<{ msgs: Envelope[]; ws: WebSocket }> {
	const code = url.split('/live/')[1];
	const ws = new WebSocket(`${url}?role=conductor&group=decks`);
	const msgs: Envelope[] = [];
	return new Promise((ok, fail) => {
		ws.onopen = () => ws.send(JSON.stringify({ v: 1, seq: 0, from: '', kind: 'hello', p: { code, role: 'conductor', group: 'decks', token: 'dev' } }));
		ws.onmessage = (e) => {
			const m = JSON.parse(String(e.data)) as Envelope;
			msgs.push(m);
			if (m.kind === 'welcome') ok({ msgs, ws });
		};
		ws.onerror = () => fail(new Error('conductor: no relay'));
	});
}

/** How long a test waits for something to HAPPEN before it gives up (#382): a deadline, not a measurement - paid only when
 *  the test fails, so wide enough for a machine running three builds. No test here asserts how long something took. */
const PATIENCE = 60_000;

const until = async (what: () => boolean, ms = PATIENCE) => {
	const end = Date.now() + ms;
	while (!what()) {
		if (Date.now() > end) throw new Error('timed out');
		await new Promise((r) => setTimeout(r, 20));
	}
};

const relayExe = fileURLToPath(new URL('../../../deck/target/debug/relay.exe', import.meta.url));
const NO_RELAY = existsSync(relayExe) ? '' : 'deck/target/debug/relay.exe not built (cargo build -p relay)';

test('in process: a relay, a conductor, the deck – graph first, values rate-limited, again after a reconnect', { skip: NO_RELAY || NO_CORE || false }, async () => {
	const port = Number(process.env.LAB_RELAY_PORT ?? 7072);
	const env = { ...process.env };
	delete env.LIVE_TOKEN_SECRET;
	const relay = spawn(relayExe, [`127.0.0.1:${port}`], { env, stdio: 'ignore' });
	try {
		const url = `ws://127.0.0.1:${port}/live/LAB`;
		let c: Awaited<ReturnType<typeof conductor>> | null = null;
		const end = Date.now() + PATIENCE;
		while (!c) {
			try {
				c = await conductor(url);
			} catch (e) {
				if (Date.now() > end) throw e;
				await new Promise((r) => setTimeout(r, 100));
			}
		}
		const deck = new DeckLink({ relay: url, graph: ownGraph(), socket: WebSocket as never, core: relayCore() });
		deck.setStream(OWN.streamed);
		await until(() => deck.joined);
		// 1 s of values at 120 frames a second, through the core's stream - counted frames on the core's own time (the `time`
		// argument), not the wall's: the rate limit is the core's, a loaded machine cannot stretch the second (#382)
		for (let f = 0; f < 120; f++) {
			deck.sample(f / 120, (p) => (p === 'fft.level' ? Math.abs(Math.sin(f)) : (OWN_PARAMS[p]?.value ?? 0)), f > 60 ? 'happy' : 'neutral');
			await new Promise((r) => setTimeout(r, 0));
		}
		await until(() => c!.msgs.some((m) => m.kind === 'deck.values' && m.p.mood === 'happy'));
		const mine = c.msgs.filter((m) => m.from === deck.client);
		assert.equal(mine[0].kind, 'deck.graph', `the graph first: ${mine.map((m) => m.kind).join(' ')}`);
		assert.equal((mine[0].p as { character?: string }).character, 'yellowmouse');
		const values = mine.filter((m) => m.kind === 'deck.values');
		assert.ok(values.length >= 20 && values.length <= 33, `≤ 30 / s: ${values.length}`);
		// the relay drops the being when the deck leaves; the deck joins again with graph + keyframe
		const before = c.msgs.length;
		(deck.socket as unknown as { ws: WebSocket }).ws.close();
		await until(() => c!.msgs.slice(before).some((m) => m.kind === 'deck.values' && Object.keys(m.p.values).length >= Object.keys(OWN_STREAMED).length - 2));
		const again = c.msgs.slice(before).filter((m) => m.kind === 'deck.graph' || m.kind === 'deck.values');
		assert.equal(again[0].kind, 'deck.graph');
		deck.close();
		c.ws.close();
	} finally {
		relay.kill();
	}
});

test("#321 in process: the show clock – a pong with the real t0, a leading deck's tempo to the others and into a late joiner's snapshot", { skip: NO_RELAY || NO_CORE || false }, async () => {
	const port = Number(process.env.LAB_RELAY_PORT ?? 7072) + 1;
	const env = { ...process.env };
	delete env.LIVE_TOKEN_SECRET;
	const relay = spawn(relayExe, [`127.0.0.1:${port}`], { env, stdio: 'ignore' });
	try {
		const url = `ws://127.0.0.1:${port}/live/CLOCK`;
		const pongs: [number, number, number][] = [];
		const tempos: Tempo[] = [];
		const core = relayCore();
		const link = (o: Partial<ConstructorParameters<typeof DeckLink>[0]>) => new DeckLink({ relay: url, graph: ownGraph(), socket: WebSocket as never, core, ...o });
		// the relay is up once a conductor gets in
		const end = Date.now() + PATIENCE;
		for (;;) {
			try {
				(await conductor(url)).ws.close();
				break;
			} catch (e) {
				if (Date.now() > end) throw e;
				await new Promise((r) => setTimeout(r, 100));
			}
		}
		const leader = link({ onPong: (t0, t1, t2) => pongs.push([t0, t1, t2]) });
		await until(() => leader.joined);
		const follower = link({ onTempo: (t) => tempos.push(t) });
		await until(() => follower.joined);
		assert.ok(leader.sendPing());
		await until(() => pongs.length > 0);
		const [t0, t1, t2] = pongs[0];
		const sent = (performance.timeOrigin + performance.now()) * 1000;
		assert.ok(Math.abs(t0 - sent) < 5e6 && t2 * 1000 >= t0, "the real t0 (µs of the page's clock) comes back");
		assert.ok(Math.abs(t1 - Date.now()) < 5000, "the relay's ms");
		assert.ok(leader.tempo({ bpm: 126, per_bar: 4, beat: 32, running: true }));
		await until(() => tempos.length > 0);
		assert.equal(tempos[0].bpm, 126);
		assert.ok(tempos[0].at !== undefined && tempos[0].at > t1, 'the relay scheduled it with its lead');
		// a deck joining later finds the anchor in its welcome
		const late: Tempo[] = [];
		const third = link({ onTempo: (t) => late.push(t) });
		await until(() => late.length > 0);
		assert.equal(late[0].beat, 32);
		for (const l of [leader, follower, third]) l.close();
	} finally {
		relay.kill();
	}
});
