// #518 (plan E2b): a reader in a session sends the story events it fires – the runner's `fired` (script-runner.ts), the
// ear (reader-story.ts) and the link (reader-session.ts: deck-space's relay client, relay-web's wasm, as role `reader`);
// normal reading opens nothing and sends nothing. #528: it joins without claiming – only the person's yes (`drive`) does;
// the core's place (`{ driving }`) is what the question and the linked sign show. #552: a story question's vote result
// is the driving reader's choice where it stands (the core's `{ choice }`; the pick as a tap). The wasm tests skip, saying so, when relay-web isn't built
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
import { createBox, createBoxData } from '../src/lib/visual-novel/box-state.ts';
import { listenPick, listenSession, tellChoicesClosed, tellSession } from '../src/lib/reader-story.ts';
import { ReaderLink, joinSession, leaveSession, sessionOf, type Driving, type VoteChoice } from '../src/lib/reader-session.ts';
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
/** the core's `{ driving }` words (#528) into a list */
const hear = (into: Driving[]) => (o: Record<string, never>) => void ('driving' in o && into.push(o.driving));

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

test('a reader in a session joins as reader and claims nothing before the yes; yes claims, it sends only while it drives, take back claims again (#527, #528)', { skip: NO_CORE }, async () => {
	FakeSocket.all = [];
	const heard: Driving[] = [];
	const link = joinSession('ws://relay:7070/live/CATH', { core: await core(), socket: FakeSocket, onOther: hear(heard) });
	tellSession(JOIN);
	const ws = FakeSocket.all[0];
	assert.equal(ws.url, 'ws://relay:7070/live/CATH?role=reader');
	ws.open();
	assert.deepEqual(ws.sent[0], { v: 1, seq: 0, from: '', kind: 'hello', p: { code: 'CATH', role: 'reader' } });
	ws.push('welcome', { client: 'c4', snapshot: { driver: 'c9' }, now: 0 });
	assert.deepEqual(heard, [{ claim: 'none', other: true }], "the welcome: the core's place, another reader drives (the newcomer's warning)");
	tellSession(SAY);
	assert.deepEqual(
		ws.sent.map((e) => e.kind),
		['hello'],
		'asked first: no reader.drive and no story before the yes'
	);
	assert.ok(link.drive(), 'yes');
	assert.deepEqual(
		ws.sent.map((e) => e.kind),
		['hello', 'reader.drive'],
		'the claim on the yes'
	);
	assert.deepEqual(heard.at(-1), { claim: 'asked', other: true });
	tellSession(SAY);
	assert.deepEqual(told(ws), [], "nothing before the relay's answer");
	ws.push('reader.drive', { driver: 'c4' });
	assert.deepEqual(heard.at(-1), { claim: 'drives', other: false }, 'the linked sign');
	tellSession(SAY);
	assert.deepEqual(told(ws), [SAY], 'what fired before the answer is dropped, not sent later');
	ws.push('reader.drive', { driver: 'c9' });
	assert.deepEqual(heard.at(-1), { claim: 'taken_over', other: true }, 'the sign says taken over');
	tellSession(JOIN);
	assert.deepEqual(told(ws), [SAY], 'taken over: nothing more');
	link.drive();
	assert.equal(ws.sent.filter((e) => e.kind === 'reader.drive').length, 2, 'take back: a new claim');
	assert.equal(joinSession('ws://relay:7070/live/OTHER'), link, 'one session for the page');
	leaveSession();
	tellSession(JOIN);
	assert.equal(told(ws).length, 1, 'left: nothing more');
	listenSession(null);
});

test('no: the reader leaves the session before it ever claimed – the socket closes, nothing was sent but the hello', { skip: NO_CORE }, async () => {
	FakeSocket.all = [];
	const heard: Driving[] = [];
	joinSession('ws://relay:7070/live/CATH', { core: await core(), socket: FakeSocket, onOther: hear(heard) });
	const ws = FakeSocket.all[0];
	ws.open();
	ws.push('welcome', { client: 'c4', snapshot: {}, now: 0 });
	assert.deepEqual(heard, [{ claim: 'none', other: false }], 'the question, nobody to take over from');
	leaveSession();
	assert.equal(ws.readyState, 3, 'closed');
	tellSession(SAY);
	assert.deepEqual(
		ws.sent.map((e) => e.kind),
		['hello']
	);
});

test('what fires while relay-web still loads reaches the core, which drops it: the reader has not been asked yet', { skip: NO_CORE }, async () => {
	FakeSocket.all = [];
	const loading = core();
	const link = new ReaderLink({ relay: 'ws://relay:7070/live/CATH', core: loading, socket: FakeSocket });
	link.fired(JOIN);
	link.fired(SAY);
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

/** a relay of its own on `port` (the dev token admits a conductor) and its conductor, joined to `url` */
async function relayWithConductor(port: number, code: string) {
	const env = { ...process.env };
	delete env.LIVE_TOKEN_SECRET;
	const relay = spawn(relayExe, [`127.0.0.1:${port}`], { env, stdio: 'ignore' });
	const url = `ws://127.0.0.1:${port}/live/${code}`;
	const end = Date.now() + PATIENCE;
	for (;;) {
		try {
			return { relay, url, c: await raw(url, { role: 'conductor', group: 'decks', token: 'dev' }) };
		} catch (e) {
			if (Date.now() > end) {
				relay.kill();
				throw e;
			}
			await new Promise((r) => setTimeout(r, 100));
		}
	}
}

test('real relay (#528): asked first – nothing before the yes; the yes drives and the story reaches the conductor and the deck that asked; a newcomer sees another drives before it answers, its yes takes over; no leaves', { skip: NO_RELAY }, async () => {
	const { relay, url, c } = await relayWithConductor(Number(process.env.LAB_RELAY_PORT ?? 7072) + 4, 'READ');
	try {
		const deck = await raw(url, { role: 'musician', group: 'decks' });
		deck.ws.send(JSON.stringify({ v: 1, seq: 0, from: '', kind: 'deck.story', p: {}, to: { group: 'decks' } }));
		const presence = () => (c.msgs.filter((m) => m.kind === 'presence').at(-1)?.p ?? {}) as { readers?: Record<string, unknown>; driver?: string };
		const readers = () => Object.keys(presence().readers ?? {}).length;
		const stories = (to: { msgs: Envelope[] }) => to.msgs.filter((m) => m.kind === 'story');

		// no: the reader leaves the session
		const no: Driving[] = [];
		joinSession(url, { core: core(), socket: WebSocket as never, onOther: hear(no) });
		await until(() => no.length > 0 && readers() === 1);
		assert.deepEqual(no, [{ claim: 'none', other: false }], 'the question');
		leaveSession();
		await until(() => readers() === 0);

		// the first reader: asked first, nothing before the yes
		const a: Driving[] = [];
		const first = joinSession(url, { core: core(), socket: WebSocket as never, onOther: hear(a) });
		await until(() => a.length > 0 && readers() === 1);
		tellSession(JOIN);
		await new Promise((r) => setTimeout(r, 300));
		assert.equal(presence().driver, undefined, 'no reader.drive before the yes');
		assert.deepEqual([stories(c), stories(deck)], [[], []], 'no story before the yes');
		assert.deepEqual(a.at(-1), { claim: 'none', other: false });
		assert.ok(first.drive(), 'yes');
		await until(() => presence().driver !== undefined && a.at(-1)?.claim === 'drives');
		tellSession(JOIN);
		await until(() => stories(deck).length > 0 && stories(c).length > 0);
		for (const to of [c, deck]) assert.deepEqual(stories(to)[0].p, JOIN, 'as fired');

		// a newcomer: it sees another drives before it answers; its yes takes over, the first shows taken over
		const b: Driving[] = [];
		const second = new ReaderLink({ relay: url, core: core(), socket: WebSocket as never, onOther: hear(b) });
		await until(() => b.length > 0);
		assert.deepEqual(b, [{ claim: 'none', other: true }], "the newcomer's warning before it answers");
		const before = presence().driver;
		assert.ok(second.drive());
		await until(() => b.at(-1)?.claim === 'drives' && a.at(-1)?.claim === 'taken_over');
		assert.deepEqual(a.at(-1), { claim: 'taken_over', other: true });
		await until(() => presence().driver !== before);
		second.dispose();
		leaveSession();
		c.ws.close();
		deck.ws.close();
	} finally {
		relay.kill();
	}
});

/** #552: a reader page with the pure box (box-state.ts): the runner with the reader's ears (`fired`, `choicesClosed`), and the
 *  choice buttons' pick as choices.svelte does it for a divert (the divert, then the choices go) – `picked` lists each */
function reading() {
	const data = createBoxData<unknown, any>();
	const box = createBox(data);
	const vn = { ...box, findInScene: (id: string) => ({ name: id }), joinScene: () => {}, leaveScene: () => {}, showTextBox: () => true, setTextBoxResolver: () => {} };
	const env: RunnerEnv = { readerKnown: () => Promise.resolve(), setReaderName: () => Promise.resolve(), readerNameMax: 32, setVariable: () => {}, cleanText: (t) => t, fired: tellSession, choicesClosed: tellChoicesClosed };
	const game = { getVariable: () => undefined, setVariable: () => {}, state: { toonId: 't' } };
	const picked: number[] = [];
	const tap = (i: number) => {
		const choice = data.choicesVisible ? data.pendingChoices[i] : undefined;
		if (!choice) return false;
		picked.push(i);
		if (choice.onSelect?.divert) box.setPendingDivert(choice.onSelect.divert.to);
		box.clearChoices();
		return true;
	};
	const release = listenPick(tap);
	return { data, picked, tap, release, run: (events: VNEvent[]) => runEvents(events, 0, { vn, game, env, navigate: () => {} } as never) };
}

/** the door of ch 3: stay, an option its "Only if" hides from this reader, go – each diverts; then what each leads to */
const DOOR = [
	{ type: 'label', id: 'ch3-e1', params: { name: 'nave' } },
	{ type: 'addChoice', id: 'ch3-stay', params: { label: 'Stay', onSelect: { divert: { to: 'stay' } } } },
	{ type: 'addChoice', id: 'ch3-key', if: { variable: 'key', op: '==', value: 1 }, params: { label: 'Unlock', onSelect: { divert: { to: 'stay' } } } },
	{ type: 'addChoice', id: 'ch3-go', params: { label: 'Go', onSelect: { divert: { to: 'go' } } } },
	{ type: 'showChoices', id: 'ch3-door', params: {} },
	{ type: 'label', id: 'ch3-e10', params: { name: 'stay' } },
	{ type: 'joinScene', id: 'ch3-e11', params: { characters: [{ character: 'ghost' }] } },
	{ type: 'divert', id: 'ch3-e12', params: { to: 'end' } },
	{ type: 'label', id: 'ch3-e20', params: { name: 'go' } },
	{ type: 'joinScene', id: 'ch3-e21', params: { characters: [{ character: 'catpope' }] } },
	{ type: 'label', id: 'ch3-e99', params: { name: 'end' } }
] as unknown as VNEvent[];
/** the vote's story as the conductor opens it (deck-show `vote::open`: the showChoices' id, every option's id) */
const ASKS_DOOR = { event: 'ch3-door', options: ['ch3-stay', 'ch3-key', 'ch3-go'] };

test('real relay (#552): a story question\'s result is the driving reader\'s choice – at the choice it picks the voted option and its events go out; the option hidden here, or a reader elsewhere: ignored, said, nothing picked', { skip: NO_RELAY }, async () => {
	const { relay, url, c } = await relayWithConductor(Number(process.env.LAB_RELAY_PORT ?? 7072) + 5, 'VOTE');
	const page = reading();
	try {
		const ids = () => c.msgs.filter((m) => m.kind === 'story').map((m) => (m.p as { id?: string }).id);
		const said: Driving[] = [];
		const choices: VoteChoice[] = [];
		const link = joinSession(url, { core: core(), socket: WebSocket as never, onOther: (o: Record<string, never>) => void ('driving' in o ? said.push(o.driving) : 'choice' in o && choices.push(o.choice)) });
		await until(() => said.length > 0);
		link.drive();
		await until(() => said.at(-1)?.claim === 'drives');
		/** the relay takes 5 messages a second from a reader (session.rs `handle`): each burst of the story waits for it */
		const breathe = () => new Promise((r) => setTimeout(r, 1100));
		let n = 0;
		const vote = async (option: number) => {
			await breathe();
			const v = `v${++n}`;
			for (const [kind, p] of [
				['vote.open', { vote: v, options: ['Stay', 'Unlock', 'Go'], rule: 'majority', seconds: 30, story: ASKS_DOOR }],
				['vote.close', { vote: v, override: option }]
			] as const)
				c.ws.send(JSON.stringify({ v: 1, seq: 0, from: '', kind, p }));
			return until(() => choices.length === n);
		};

		// at the choice: the audience's Go is picked as a tap – its divert, the events after it go out as story
		await breathe();
		const first = page.run(DOOR);
		await until(() => page.data.choicesVisible);
		await vote(2);
		assert.deepEqual(choices[0], { event: 'ch3-door', option: 'ch3-go', pick: 1 }, 'go is the second option this reader shows (the key is hidden)');
		assert.deepEqual(page.picked, [1]);
		assert.equal(await first, 'done');
		await until(() => ids().includes('ch3-e99'));
		assert.deepEqual(ids(), ['ch3-e1', 'ch3-stay', 'ch3-go', 'ch3-door', 'ch3-e20', 'ch3-e21', 'ch3-e99'], 'what fired, in order: go, not stay');

		// the option hidden here (its "Only if"): ignored, the choice stays open; the reader taps itself
		await breathe();
		const second = page.run(DOOR);
		await until(() => page.data.choicesVisible);
		await vote(1);
		assert.deepEqual(choices[1], { event: 'ch3-door', option: 'ch3-key', ignored: 'not_offered' });
		assert.ok(page.data.choicesVisible, 'still open');
		await breathe();
		assert.ok(page.tap(0), 'the reader stays');
		assert.equal(await second, 'done');

		// a reader elsewhere – past the choice, standing at none: ignored, nothing picked
		await vote(2);
		assert.deepEqual(choices[2], { event: 'ch3-door', option: 'ch3-go', ignored: 'elsewhere' });
		assert.deepEqual(page.picked, [1, 0], 'only its own tap since');
		await until(() => ids().filter((id) => id === 'ch3-e11').length === 1);
		assert.ok(!ids().slice(7).includes('ch3-e21'), 'its own way: stay');
	} finally {
		page.release();
		leaveSession();
		c.ws.close();
		relay.kill();
	}
});

test("#535: a link with the relay's QUIC door (the core's reader_link text, deck-space linked.rs) – the reader's session names the door and its hash", () => {
	const link = 'https://scutoons.com/catpope/chapter/3?live=A7K2&relay=ws%3A%2F%2F192.168.1.116%3A7070&quic=192.168.1.116%3A7071&quichash=0a1B';
	assert.equal(sessionOf(new URL(link).searchParams, 'wss://live.scutoons.com'), 'ws://192.168.1.116:7070/live/A7K2?quic=192.168.1.116%3A7071&quichash=0a1B');
	const own = 'https://scutoons.com/catpope/chapter/3?live=A7K2&relay=ws%3A%2F%2F192.168.1.116%3A7070&quic=';
	assert.equal(sessionOf(new URL(own).searchParams, 'wss://live.scutoons.com'), 'ws://192.168.1.116:7070/live/A7K2?quic=', "an empty door: the relay's own");
});
