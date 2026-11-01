// #231: the vote screens' link to the relay (src/lib/live-vote-link.ts) against a fake socket.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import type { Envelope } from '@scutoons/live';
import { codeOfRelay, RelayVote, type LinkStatus, type SocketLike } from '../src/lib/live-vote-link.ts';

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

const OPEN = { vote: 'v1', options: ['Yes', 'No'], rule: 'majority', seconds: 20 };

function join(role: 'audience' | 'conductor', extra: Record<string, unknown> = {}) {
	FakeSocket.all = [];
	const status: LinkStatus[] = [];
	const link = new RelayVote({ relay: 'ws://relay:7070/live/CATH', role, socket: FakeSocket, onStatus: (s) => status.push(s), ...extra });
	const ws = FakeSocket.all[0];
	ws.open();
	return { link, ws, status };
}

test('the relay address: ws://host:port/live/CODE', () => {
	assert.deepEqual(codeOfRelay('wss://live.scutoons.com/live/A7K2'), { base: 'wss://live.scutoons.com', code: 'A7K2' });
	assert.throws(() => codeOfRelay('http://x/live/A'));
});

test('joins with its role and group in the URL and says hello first, the conductor with its token', () => {
	const a = join('audience', { group: 'balkon', nick: 'Mia' });
	assert.equal(a.ws.url, 'ws://relay:7070/live/CATH?role=audience&group=balkon');
	assert.deepEqual(a.ws.sent[0], { v: 1, seq: 0, from: '', kind: 'hello', p: { code: 'CATH', role: 'audience', group: 'balkon', nick: 'Mia' } });
	const c = join('conductor', { token: 'tok' });
	assert.equal((c.ws.sent[0].p as { token: string }).token, 'tok');
	a.link.dispose();
	c.link.dispose();
});

test('the welcome snapshot hands an open vote and the group instruction to the screen; a late screen gets the vote at once', () => {
	const { link, ws, status } = join('audience', { group: 'balkon' });
	const seen: string[] = [];
	link.on('audience', (e) => seen.push(e.kind));
	ws.push('welcome', { client: 'c7', snapshot: { vote: OPEN, say: { balkon: { text: 'Stand up' } }, actions: [] }, now: 0 });
	assert.deepEqual(seen, ['vote.open', 'say']);
	assert.deepEqual(status.at(-1), { state: 'connected', client: 'c7' });
	const late: string[] = [];
	link.on('audience', (e) => late.push(e.kind));
	assert.deepEqual(late, ['vote.open']);
	ws.push('vote.result', { vote: 'v1', option: 0, counts: [1, 0], how: 'rule' });
	const later: string[] = [];
	link.on('audience', (e) => later.push(e.kind));
	assert.deepEqual(later, [], 'a closed vote is not handed out again');
	link.dispose();
});

test('cast, hold and close address the open vote; open sends vote.open', () => {
	const { link, ws } = join('conductor', { token: 't' });
	ws.push('welcome', { client: 'c1', snapshot: { actions: [] }, now: 0 });
	link.close();
	assert.equal(ws.sent.length, 1, 'nothing is open: nothing is sent');
	link.open(OPEN as never);
	ws.push('vote.open', OPEN);
	link.hold(true);
	link.close(1);
	link.cast('ignored', 1);
	assert.deepEqual(
		ws.sent.slice(1).map((e) => [e.kind, e.p]),
		[
			['vote.open', OPEN],
			['vote.hold', { vote: 'v1', hold: true }],
			['vote.close', { vote: 'v1', override: 1 }],
			['vote.cast', { vote: 'v1', option: 1 }]
		]
	);
	link.dispose();
});

test('a refused token shows; a closed socket joins again', async () => {
	const { link, ws, status } = join('conductor', { token: 'bad' });
	ws.push('error', { code: 'token', message: 'bad signature' });
	assert.deepEqual(status.at(-1), { state: 'refused', why: 'bad signature' });
	ws.onclose?.({});
	assert.equal(status.at(-1)?.state, 'offline');
	link.dispose();
	assert.equal(FakeSocket.all.length, 1, 'disposed: no new socket');
});
