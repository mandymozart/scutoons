// `npm test -w shared/live-deck` (#195): the browser deck's answer to the conductor's scene sync – sync.ts against a
// stub fetch (progress, ready, missing, a short file, offline), and the link's message handling with a stub socket:
// the welcome's `sync` and a `scene.sync` reach the sync, its answers go out as `deck.synced`.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import type { DeckSynced, Envelope, SceneSync } from '@scutoons/live';
import { DeckLink, type WebSocketLike } from '../src/client.ts';
import { BrowserSync, itemsOf, pushLine } from '../src/sync.ts';
import { ownGraph } from './own.ts';

const API = 'https://api.test/api';
const CDN = 'https://cdn.test/';

/** scene-content for the cathedral and catpope; ghost is unknown */
const CONTENT = {
	sets: { cathedral: { model: { low: { path: 'catpope/map/sets/cathedral.low.v1.glb', size: 4 }, high: { path: 'catpope/map/sets/cathedral.high.v1.glb', size: 9 } } } },
	characters: { catpope: { portraits: { neutral: { path: 'characters/catpope/n.webp', size: 2 }, happy: { path: 'characters/catpope/h.webp' } } } },
	missing: { sets: [], characters: ['ghost'] }
};

/** a stub fetch: the API answers CONTENT, the CDN `files` (path → bytes); every URL asked is kept */
function stub(files: Record<string, number>, down = false) {
	const asked: string[] = [];
	const f = (async (url: string) => {
		asked.push(url);
		if (down) throw new TypeError('fetch failed');
		if (url.startsWith(`${API}/library/scene-content?`)) return new Response(JSON.stringify(CONTENT));
		const path = url.slice(CDN.length);
		return path in files ? new Response(new Uint8Array(files[path])) : new Response('nope', { status: 404 });
	}) as typeof fetch;
	return { f, asked };
}

const push = (characters: string[]): SceneSync => ({ scene: 'cathedral', sets: ['cathedral'], characters });
const FILES = { 'catpope/map/sets/cathedral.low.v1.glb': 4, 'characters/catpope/n.webp': 2, 'characters/catpope/h.webp': 7 };

test('items: sets, then characters', () => {
	assert.deepEqual(itemsOf(push(['catpope', 'ghost'])), ['set:cathedral', 'character:catpope', 'character:ghost']);
	assert.deepEqual(itemsOf({ scene: 'x' }), []);
});

test('a push: one request, every file fetched whole, progress after each, then ready', async () => {
	const { f, asked } = stub(FILES);
	const reports: DeckSynced[] = [];
	const sync = new BrowserSync({ api: API, cdn: CDN, fetch: f, report: (r) => reports.push(r) });
	const final = await sync.push(push(['catpope']));
	assert.equal(asked[0], `${API}/library/scene-content?sets=cathedral&characters=catpope`);
	assert.equal(asked.filter((u) => u.startsWith(CDN)).length, 3, 'the low 3D and both portraits');
	assert.deepEqual(final, { scene: 'cathedral', state: 'ready', done: 3, total: 3, bytes: 13, ready: ['set:cathedral', 'character:catpope'], missing: [] });
	assert.deepEqual(
		reports.map((r) => `${r.state} ${r.done}/${r.total}`),
		['syncing 0/3', 'syncing 1/3', 'syncing 2/3', 'syncing 3/3', 'ready 3/3']
	);
	assert.deepEqual(sync.last, final);
	assert.equal(pushLine(final), 'PUSH READY CATHEDRAL');
	assert.equal(pushLine(reports[1]), 'PUSH 1/3 CATHEDRAL');
});

test('missing: unknown to the API, a short file; offline: the network is gone', async () => {
	const short = { ...FILES, 'catpope/map/sets/cathedral.low.v1.glb': 3 };
	const { f } = stub(short);
	const sync = new BrowserSync({ api: API, cdn: CDN, fetch: f, report: () => {} });
	const r = await sync.push(push(['catpope', 'ghost']));
	assert.equal(r.state, 'missing');
	assert.deepEqual(r.ready, ['character:catpope']);
	assert.deepEqual(r.missing, ['set:cathedral', 'character:ghost']);
	assert.match(r.why!['set:cathedral'], /3 bytes, the API said 4/);
	assert.match(r.why!['character:ghost'], /unknown to the API/);
	assert.equal(pushLine(r), 'PUSH MISSING 2 CATHEDRAL');
	const high = await new BrowserSync({ api: API, cdn: CDN, quality: 'high', fetch: f, report: () => {} }).push(push([]));
	assert.match(high.why!['set:cathedral'], /HTTP 404/, 'the deck’s own quality');

	const down = stub(FILES, true);
	const off = await new BrowserSync({ api: API, cdn: CDN, fetch: down.f, report: () => {} }).push(push(['catpope']));
	assert.equal(off.state, 'offline');
	assert.deepEqual(off.missing, ['set:cathedral', 'character:catpope']);
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

test('the link: the welcome’s sync and a scene.sync reach onSync; synced() sends deck.synced', () => {
	const got: SceneSync[] = [];
	const link = new DeckLink({ relay: 'ws://relay.test/live/SYNC', graph: ownGraph('catpope', ['neutral']), socket: Sock, onSync: (s) => got.push(s) });
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
