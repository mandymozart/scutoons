// #518 (plan E2b): a reader in a session sends the story events it fires – the runner's `fired` (script-runner.ts), the
// ear (reader-story.ts) and the link (reader-session.ts: deck-space's relay client, relay-web's wasm, as role `reader`);
// normal reading opens nothing and sends nothing. The wasm tests skip, saying so, when relay-web isn't built
// (`npm run build:wasm -w shared/live-deck -- --relay`); the relay test when deck/target/debug/relay.exe isn't (`cargo
// build -p relay`).
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { existsSync, readFileSync } from 'node:fs';
import type { Envelope } from '@scutoons/live';
import type { VNEvent } from '@scutoons/content';
import { loadRelay } from '@scutoons/live-deck';
import { runEvents, type RunnerEnv } from '../src/lib/visual-novel/script-runner.ts';
import { listenSession, tellSession } from '../src/lib/reader-story.ts';
import { ReaderLink, joinSession, leaveSession, sessionOf } from '../src/lib/reader-session.ts';
import type { SocketLike } from '../src/lib/live-vote-link.ts';
import { deckTarget } from '../../shared/live-core/scripts/wasm.ts';

const wasmDir = new URL('../../shared/live-deck/wasm/', import.meta.url);
const NO_CORE = existsSync(new URL('relay_web_bg.wasm', wasmDir)) ? false : 'relay-web is not built (npm run build:wasm -w shared/live-deck -- --relay)';
const core = () => loadRelay(wasmDir.href, readFileSync(new URL('relay_web_bg.wasm', wasmDir)));
const relayExe = `${deckTarget()}/debug/relay.exe`;
const NO_RELAY = existsSync(relayExe) ? NO_CORE : `${relayExe} not built (cargo build -p relay)`;

const JOIN = { type: 'joinScene', id: 'ch3-e12', params: { characters: [{ character: 'catpope' }] } } as unknown as VNEvent;
const SAY = { type: 'showMessage', id: 'ch3-e13', params: { character: 'catpope', text: { de: 'Hallo' } } } as unknown as VNEvent;

class FakeSocket implements SocketLike {
	static all: FakeSocket[] = [];
	readyState = 0;
	sent: Envelope[] = [];
	onopen: SocketLike['onopen'] = null;
	onmessage: SocketLike['onmessage'] = null;
	onclose: SocketLike['onclose'] = null;
	onerror: SocketLike['onerror'] = null;
	constructor(public url: string) {
		FakeSocket.all.push(this);
	}
	send(data: string) {
		this.sent.push(JSON.parse(data));
	}
	close() {
		this.readyState = 3;
	}
	open() {
		this.readyState = 1;
		this.onopen?.({});
	}
	push(kind: string, p: unknown) {
		this.onmessage?.({ data: JSON.stringify({ v: 1, seq: 1, from: 'relay', kind, p }) });
	}
}

const told = (ws: FakeSocket) => ws.sent.filter((e) => e.kind === 'story').map((e) => e.p);

test('the runner says each event that passes its condition, in order, every type alike – not one whose condition fails', async () => {
	const fired: VNEvent[] = [];
	const vars: Record<string, string | number> = {};
	const env: RunnerEnv = {
		readerKnown: () => Promise.resolve(),
		setReaderName: () => Promise.resolve(),
		readerNameMax: 32,
		setVariable: (a) => void ('set' in a && (vars[a.set.variable] = a.set.value)),
		cleanText: (t) => t,
		fired: (e) => void fired.push(e)
	};
	const vn = { joinScene: () => {}, leaveScene: () => {} };
	const events = [
		{ type: 'label', id: 'e1', params: { name: 'start' } },
		{ type: 'setVariable', id: 'e2', params: { variable: 'met', value: 1 } },
		{ type: 'joinScene', id: 'e3', if: { variable: 'met', op: '==', value: 2 }, params: { characters: [{ character: 'ghost' }] } },
		JOIN,
		{ type: 'leaveScene', id: 'e5', params: { characters: [{ character: 'catpope' }] } }
	] as unknown as VNEvent[];
	await runEvents(events, 0, { vn, game: { getVariable: (n: string) => vars[n], setVariable: () => {}, state: { toonId: 't' } }, env, navigate: () => {} } as never);
	assert.deepEqual(
		fired.map((e) => e.id),
		['e1', 'e2', 'ch3-e12', 'e5']
	);
	assert.equal(fired[2], events[3], 'the event as it is');
});

test('normal reading: no ?live= names no session, and nobody listens – nothing is held or sent', () => {
	assert.equal(sessionOf(new URLSearchParams(''), 'wss://live.example'), null);
	assert.equal(sessionOf(new URLSearchParams('code=ABC'), 'wss://live.example'), null, 'a printed ?code= is no session');
	assert.equal(sessionOf(new URLSearchParams('live=CATH'), 'wss://live.example'), 'wss://live.example/live/CATH');
	assert.equal(sessionOf(new URLSearchParams('live=LAB&relay=ws://127.0.0.1:7070'), 'wss://live.example'), 'ws://127.0.0.1:7070/live/LAB');
	FakeSocket.all = [];
	tellSession(JOIN);
	assert.equal(FakeSocket.all.length, 0, 'no link opened');
});

test('a reader in a session joins as reader, claims the story at the welcome and sends what it fires only while it drives (#527)', { skip: NO_CORE }, async () => {
	FakeSocket.all = [];
	const link = joinSession('ws://relay:7070/live/CATH', { core: await core(), socket: FakeSocket });
	tellSession(JOIN);
	const ws = FakeSocket.all[0];
	assert.equal(ws.url, 'ws://relay:7070/live/CATH?role=reader');
	ws.open();
	assert.deepEqual(ws.sent[0], { v: 1, seq: 0, from: '', kind: 'hello', p: { code: 'CATH', role: 'reader' } });
	ws.push('welcome', { client: 'c4', snapshot: {}, now: 0 });
	assert.ok(link.joined);
	assert.deepEqual(
		ws.sent.map((e) => e.kind),
		['hello', 'reader.drive'],
		'the claim at the welcome'
	);
	tellSession(SAY);
	assert.deepEqual(told(ws), [], "nothing before the relay's answer");
	ws.push('reader.drive', { driver: 'c4' });
	tellSession(SAY);
	assert.deepEqual(told(ws), [SAY], 'what fired before the answer is dropped, not sent later');
	ws.push('reader.drive', { driver: 'c9' });
	tellSession(JOIN);
	assert.deepEqual(told(ws), [SAY], 'taken over: nothing more');
	assert.equal(joinSession('ws://relay:7070/live/OTHER'), link, 'one session for the page');
	leaveSession();
	tellSession(JOIN);
	assert.equal(told(ws).length, 1, 'left: nothing more');
	listenSession(null);
});

test('what fires while relay-web still loads reaches the core, which drops it: the reader does not drive yet', { skip: NO_CORE }, async () => {
	FakeSocket.all = [];
	const loading = core();
	const link = new ReaderLink({ relay: 'ws://relay:7070/live/CATH', core: loading, socket: FakeSocket });
	link.tell(JOIN);
	link.tell(SAY);
	await loading;
	await new Promise((r) => setImmediate(r));
	const ws = FakeSocket.all[0];
	ws.open();
	ws.push('welcome', { client: 'c4', snapshot: {}, now: 0 });
	ws.push('reader.drive', { driver: 'c4' });
	assert.deepEqual(told(ws), []);
	link.dispose();
});

/** a raw client on the relay: its messages */
function raw(url: string, hello: Record<string, unknown>): Promise<{ msgs: Envelope[]; ws: WebSocket }> {
	const code = url.split('/live/')[1];
	const ws = new WebSocket(`${url}?role=${hello.role}${hello.group ? `&group=${hello.group}` : ''}`);
	const msgs: Envelope[] = [];
	return new Promise((ok, fail) => {
		ws.onopen = () => ws.send(JSON.stringify({ v: 1, seq: 0, from: '', kind: 'hello', p: { code, ...hello } }));
		ws.onmessage = (e) => {
			const m = JSON.parse(String(e.data)) as Envelope;
			msgs.push(m);
			if (m.kind === 'welcome') ok({ msgs, ws });
		};
		ws.onerror = () => fail(new Error('no relay'));
	});
}

const PATIENCE = 60_000;
const until = async (what: () => boolean) => {
	const end = Date.now() + PATIENCE;
	while (!what()) {
		if (Date.now() > end) throw new Error('timed out');
		await new Promise((r) => setTimeout(r, 20));
	}
};

test("in process: the driving reader's joinScene reaches the conductor and the deck that asked, as fired", { skip: NO_RELAY }, async () => {
	const port = Number(process.env.LAB_RELAY_PORT ?? 7072) + 4;
	const env = { ...process.env };
	delete env.LIVE_TOKEN_SECRET;
	const relay = spawn(relayExe, [`127.0.0.1:${port}`], { env, stdio: 'ignore' });
	try {
		const url = `ws://127.0.0.1:${port}/live/READ`;
		let conductor: Awaited<ReturnType<typeof raw>> | null = null;
		const end = Date.now() + PATIENCE;
		while (!conductor) {
			try {
				conductor = await raw(url, { role: 'conductor', group: 'decks', token: 'dev' });
			} catch (e) {
				if (Date.now() > end) throw e;
				await new Promise((r) => setTimeout(r, 100));
			}
		}
		const deck = await raw(url, { role: 'musician', group: 'decks' });
		deck.ws.send(JSON.stringify({ v: 1, seq: 0, from: '', kind: 'deck.story', p: {}, to: { group: 'decks' } }));
		const link = joinSession(url, { core: core(), socket: WebSocket as never });
		// #527: it claims at the welcome; what it fires counts once the relay made it the driver (the conductor's presence says so)
		await until(() => conductor!.msgs.some((m) => m.kind === 'presence' && (m.p as { driver?: string }).driver !== undefined));
		tellSession(JOIN);
		await until(() => deck.msgs.some((m) => m.kind === 'story') && conductor!.msgs.some((m) => m.kind === 'story'));
		for (const c of [conductor, deck]) assert.deepEqual(c.msgs.find((m) => m.kind === 'story')?.p, JOIN);
		assert.ok(link.joined);
		leaveSession();
		conductor.ws.close();
		deck.ws.close();
	} finally {
		relay.kill();
	}
});
