// `npm test -w shared/live-deck` (#195, #371): the browser deck's answer to the conductor's scene sync. The decisions are
// deck-space's push machine (space-web `DeckPush`); test/fixtures/sync.json holds scripted pushes – what the API and
// the CDN reply – and every call the adapter must make on the core with the core's answers (written and kept current by
// `UPDATE_FIXTURES=1 cargo test -p deck-space push_fixture`). sync.ts runs each script against a stub fetch and a
// stand-in core that replays the calls: the URLs it fetched, the statuses, lengths and first bytes it handed in, the
// answers and lines it reported must be the core's. And the link's message handling with a stub socket: the welcome's
// `sync` and a `scene.sync` reach the sync, its answers go out as `deck.synced`.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import type { DeckSynced, Envelope, SceneSync } from '@scutoons/live';
import { DeckLink, type WebSocketLike } from '../src/client.ts';
import { BrowserSync, type DeckPushView } from '../src/sync.ts';
import { ownGraph } from './own.ts';

interface Reply {
	status?: number | null;
	bytes?: number;
	head?: string;
}
interface Call {
	name: keyof DeckPushView;
	args: unknown[];
	answer: unknown;
}
interface Script {
	name: string;
	push: SceneSync;
	quality: string;
	api: { status?: number | null; body?: unknown };
	files: Record<string, Reply>;
	calls: Call[];
}
const FIXTURE = JSON.parse(readFileSync(new URL('./fixtures/sync.json', import.meta.url), 'utf8')) as { api: string; cdn: string; scripts: Script[] };
const { api: API, cdn: CDN } = FIXTURE;
const script = (name: string) => FIXTURE.scripts.find((s) => s.name === name)!;

/** a stub fetch from a script's replies (status 0: the network is gone; a CDN path not in it: 404); every URL asked is kept */
function stub(s: Script) {
	const asked: string[] = [];
	const f = (async (url: string) => {
		asked.push(url);
		const r: Reply = url.startsWith(`${API}/`) ? { status: s.api.status } : (s.files[url.slice(CDN.length)] ?? { status: 404 });
		if (r.status === 0) throw new TypeError('fetch failed');
		if (r.status) return new Response('nope', { status: r.status });
		if (url.startsWith(`${API}/`)) return new Response(JSON.stringify(s.api.body));
		const bytes = new Uint8Array(r.bytes ?? 0);
		bytes.set(new TextEncoder().encode(r.head ?? '').subarray(0, bytes.length));
		return new Response(bytes);
	}) as typeof fetch;
	return { f, asked };
}

/** A stand-in for space-web's `DeckPush` replaying a script's calls: each call must be the next one, with its
 *  arguments; it answers what the core answered. */
function core(s: Script): DeckPushView & { left: () => number } {
	const calls = [...s.calls];
	const next = (name: keyof DeckPushView, args: unknown[]): unknown => {
		const c = calls.shift();
		assert.ok(c, `${s.name}: ${name} after the core's last call`);
		assert.deepEqual([name, args], [c.name, c.args], `${s.name}: the adapter's call`);
		return c.answer;
	};
	return {
		start: (sync, quality) => next('start', [JSON.parse(sync), quality]) as string,
		asked: (status, body) => void next('asked', [status, body]),
		next_fetch: () => {
			const a = next('next_fetch', []);
			return a === null ? undefined : JSON.stringify(a);
		},
		received: (n, status, len, head) => void next('received', [n, status, len, [...head]]),
		answer: () => JSON.stringify(next('answer', [])),
		line: () => next('line', []) as string,
		left: () => calls.length
	};
}

/** a script through sync.ts: the final answer, every report, the URLs fetched */
async function run(name: string) {
	const s = script(name);
	const { f, asked } = stub(s);
	const reports: [DeckSynced, string][] = [];
	const sync = new BrowserSync({ quality: s.quality, fetch: f, report: (r, line) => reports.push([r, line]) });
	const c = core(s);
	sync.useCore(c);
	const final = await sync.push(s.push);
	assert.equal(c.left(), 0, `${name}: every call the core expects was made`);
	const answers = s.calls.filter((k) => k.name === 'answer').map((k) => k.answer);
	assert.deepEqual(
		reports.map(([r]) => r),
		answers,
		`${name}: every answer reported`
	);
	assert.deepEqual(
		reports.map(([, l]) => l),
		s.calls.filter((k) => k.name === 'line').map((k) => k.answer),
		`${name}: every line reported`
	);
	assert.deepEqual(final, answers.at(-1));
	assert.deepEqual(sync.last, final);
	assert.equal(sync.line, reports.at(-1)![1]);
	return { final: final!, reports, asked };
}

test('every scripted push: the adapter makes the core’s calls and reports its answers', async () => {
	for (const s of FIXTURE.scripts) await run(s.name);
});

test('a push: one request, every file fetched whole, progress after each, then ready', async () => {
	const { final, reports, asked } = await run('ready');
	assert.equal(asked[0], `${API}/library/scene-content?sets=cathedral&characters=catpope`);
	assert.equal(asked.filter((u) => u.startsWith(CDN)).length, 3, 'the low 3D and both portraits');
	assert.deepEqual(final, { scene: 'cathedral', state: 'ready', done: 3, total: 3, bytes: 46, ready: ['set:cathedral', 'character:catpope'], missing: [] });
	assert.deepEqual(
		reports.map(([r, line]) => `${r.state} ${r.done}/${r.total} ${line}`),
		['syncing 0/3 PUSH 0/3 CATHEDRAL', 'syncing 1/3 PUSH 1/3 CATHEDRAL', 'syncing 2/3 PUSH 2/3 CATHEDRAL', 'ready 3/3 PUSH READY CATHEDRAL']
	);
});

test('missing: unknown to the API, a short file; the deck’s quality; offline: the network is gone', async () => {
	const { final: r, reports } = await run('missing');
	assert.equal(r.state, 'missing');
	assert.deepEqual(r.ready, ['character:catpope']);
	assert.deepEqual(r.missing, ['set:cathedral', 'character:ghost']);
	assert.match(r.why!['set:cathedral'], /3 bytes, the API said 4/);
	assert.match(r.why!['character:ghost'], /unknown to the API/);
	assert.equal(reports.at(-1)![1], 'PUSH MISSING 2 CATHEDRAL');
	const high = await run('high');
	assert.match(high.final.why!['set:cathedral'], /HTTP 404/, 'the deck’s own quality');
	assert.ok(high.asked.some((u) => u.endsWith('cathedral.high.v1.glb')));
	const off = await run('offline');
	assert.equal(off.final.state, 'offline');
	assert.deepEqual(off.final.missing, ['set:cathedral', 'character:catpope']);
	assert.equal(off.asked.length, 1, 'no file asked without the API');
	assert.equal((await run('cdn-gone')).final.state, 'offline');
	assert.equal((await run('api-error')).final.state, 'missing');
});

test('a push before the core waits for it', async () => {
	const s = script('ready');
	const reports: DeckSynced[] = [];
	const sync = new BrowserSync({ fetch: stub(s).f, report: (r) => reports.push(r) });
	assert.equal(await sync.push(s.push), null);
	assert.equal(reports.length, 0);
	sync.useCore(core(s));
	await new Promise((r) => setTimeout(r, 10));
	assert.equal(reports.at(-1)?.state, 'ready');
});

/** a socket the test drives: what the deck sent, and the relay's side */
class Sock implements WebSocketLike {
	static last: Sock;
	readyState = 0;
	sent: Envelope[] = [];
	onopen: ((ev: unknown) => void) | null = null;
	onmessage: ((ev: { data: unknown }) => void) | null = null;
	onclose: ((ev: unknown) => void) | null = null;
	onerror: ((ev: unknown) => void) | null = null;
	constructor(readonly url: string) {
		Sock.last = this;
	}
	send(data: string) {
		this.sent.push(JSON.parse(data) as Envelope);
	}
	close() {}
	open() {
		this.readyState = 1;
		this.onopen?.({});
	}
	relay(e: object) {
		this.onmessage?.({ data: JSON.stringify({ v: 1, seq: 1, from: 'relay', ...e }) });
	}
}

const push = (characters: string[]): SceneSync => ({ scene: 'cathedral', sets: ['cathedral'], characters });

test('the link: the welcome’s sync and a scene.sync reach onSync; synced() sends deck.synced', () => {
	const got: SceneSync[] = [];
	const link = new DeckLink({ relay: 'ws://relay.test/live/SYNC', graph: ownGraph(), socket: Sock, onSync: (s) => got.push(s) });
	const ws = Sock.last;
	link.synced({ scene: 'x', state: 'ready', ready: [], missing: [] });
	assert.equal(ws.sent.length, 0, 'not before the welcome');
	ws.open();
	ws.relay({ kind: 'welcome', p: { client: 'c7', group: 'decks', snapshot: { sync: push(['catpope']) }, now: 1 } });
	assert.deepEqual(got, [push(['catpope'])], 'a deck joining later syncs too');
	ws.relay({ from: 'c1', kind: 'scene.sync', p: push([]) });
	assert.equal(got.length, 2);
	link.synced({ scene: 'cathedral', state: 'syncing', done: 1, total: 3, bytes: 4, ready: [], missing: [] });
	const out = ws.sent.filter((e) => e.kind === 'deck.synced');
	assert.equal(out.length, 1);
	assert.deepEqual(out[0].p, { scene: 'cathedral', state: 'syncing', done: 1, total: 3, bytes: 4, ready: [], missing: [] });
	assert.equal(link.count.synced, 1);
	link.close();
});
