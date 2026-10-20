// The handheld camera's link (src/lib/editor/storyboard-pairing.ts, #126): the phone's URL, the messages checked field by
// field (they come from another device), the signaling calls (the PC's session client, the phone's plain fetch without a
// cookie), the PC waiting for the answer, the ICE gathering.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { decodePc, decodePhone, gathered, phoneUrl, publicCalls, PairingError, sessionCalls, waitAnswer, type Gathering, type PairingView } from '../src/lib/editor/storyboard-pairing.ts';

const CODE = 'AbCdEfGhIjKlMnOpQrSt_-';

test('the phone\'s page: /h/<code> on the creator', () => {
	assert.equal(phoneUrl('https://creator.scutoons.com/', CODE), `https://creator.scutoons.com/h/${CODE}`);
});

test('the phone\'s messages: only the known ones, every field checked', () => {
	assert.deepEqual(decodePhone(JSON.stringify({ type: 'pose', t: 12.5, p: [1, 2, 3], q: [0, 0, 0, 1], extra: 1 })), { type: 'pose', t: 12.5, p: [1, 2, 3], q: [0, 0, 0, 1] });
	assert.equal(decodePhone(JSON.stringify({ type: 'pose', t: 1, p: [1, 2], q: [0, 0, 0, 1] })), null);
	assert.equal(decodePhone(JSON.stringify({ type: 'pose', t: 1, p: [1, 2, 'x'], q: [0, 0, 0, 1] })), null);
	assert.equal(decodePhone(JSON.stringify({ type: 'pose', t: Infinity, p: [1, 2, 3], q: [0, 0, 0, 1] })), null);
	assert.deepEqual(decodePhone('{"type":"state","mode":"xr","tracking":false}'), { type: 'state', mode: 'xr', tracking: false });
	assert.deepEqual(decodePhone('{"type":"state","mode":"evil"}'), { type: 'state', mode: '', tracking: true });
	assert.deepEqual(decodePhone('{"type":"rec","on":"yes"}'), { type: 'rec', on: false });
	assert.deepEqual(decodePhone('{"type":"recenter"}'), { type: 'recenter' });
	assert.deepEqual(decodePhone(JSON.stringify({ type: 'hello', ua: 'x'.repeat(500) })), { type: 'hello', ua: 'x'.repeat(300) });
	for (const bad of ['', 'nope', '[]', 'null', '{"type":"drop"}', 42, 'x'.repeat(5000)]) assert.equal(decodePhone(bad), null, String(bad));
	assert.deepEqual(decodePc('{"type":"state","shot":"sb-1","on":true,"recording":true,"seconds":2.5}'), { type: 'state', shot: 'sb-1', on: true, recording: true, seconds: 2.5 });
	assert.equal(decodePc('{"type":"pose"}'), null);
});

test('the PC\'s calls go through its session client', async () => {
	const calls: string[] = [];
	const api = {
		get: async <T>(p: string) => (calls.push(`GET ${p}`), { pairing: { code: CODE, answer: 'v=0 a' } } as T),
		post: async <T>(p: string, b?: unknown) => (calls.push(`POST ${p} ${JSON.stringify(b)}`), { pairing: { code: CODE } } as T),
		put: async <T>(p: string, b?: unknown) => (calls.push(`PUT ${p} ${JSON.stringify(b)}`), {} as T),
		delete: async <T>(p: string) => (calls.push(`DELETE ${p}`), {} as T)
	};
	const c = sessionCalls(api);
	assert.equal((await c.create({ toon: 'catpope', chapter: 3, shot: 'sb-1' })).code, CODE);
	await c.offer(CODE, 'v=0 o');
	assert.equal((await c.read(CODE)).answer, 'v=0 a');
	await c.remove(CODE);
	assert.deepEqual(calls, [
		'POST /storyboard/pairings {"toon":"catpope","chapter":3,"shot":"sb-1"}',
		`PUT /storyboard/pairings/${CODE}/offer {"sdp":"v=0 o"}`,
		`GET /storyboard/pairings/${CODE}`,
		`DELETE /storyboard/pairings/${CODE}`
	]);
});

test('the phone\'s calls: plain fetch, no cookie, the API\'s error as a PairingError', async () => {
	const seen: { url: string; init: RequestInit }[] = [];
	const fetchFn = (async (url: string, init: RequestInit) => {
		seen.push({ url, init });
		if (init.method === 'PUT') return new Response(JSON.stringify({ status: 'error', error: 'answer_exists', message: 'used' }), { status: 409 });
		return new Response(JSON.stringify({ pairing: { code: CODE, offer: 'v=0 o', answered: false } }), { status: 200 });
	}) as typeof fetch;
	const c = publicCalls('http://api.test/api/', fetchFn);
	assert.equal((await c.read(CODE)).offer, 'v=0 o');
	await assert.rejects(c.answer(CODE, 'v=0 a'), (e: unknown) => e instanceof PairingError && e.status === 409 && e.code === 'answer_exists');
	assert.deepEqual(seen.map((s) => [s.init.method, s.url, s.init.credentials]), [
		['GET', `http://api.test/api/storyboard/pairings/${CODE}`, 'omit'],
		['PUT', `http://api.test/api/storyboard/pairings/${CODE}/answer`, 'omit']
	]);
	assert.equal(seen[1].init.body, '{"sdp":"v=0 a"}');
	await assert.rejects(c.create({ toon: 'catpope', chapter: 3 }));
});

test('the PC polls until the answer is there; an abort ends it', async () => {
	const views: Partial<PairingView>[] = [{ answered: false }, { answered: false }, { answered: true, answer: 'v=0 a' }];
	let reads = 0;
	const sleeps: number[] = [];
	const answer = await waitAnswer({ read: async () => views[reads++] as PairingView }, CODE, { every: 7, sleep: async (ms) => void sleeps.push(ms) });
	assert.deepEqual([answer, reads, sleeps], ['v=0 a', 3, [7, 7]]);
	const stop = new AbortController();
	stop.abort();
	assert.equal(await waitAnswer({ read: async () => ({}) as PairingView }, CODE, { signal: stop.signal }), null);
});

test('ICE gathering: the description once complete – or after the wait with what was found', async () => {
	const fake = (state: RTCIceGatheringState): Gathering & { fire(): void } => {
		const fns = new Set<() => void>();
		return {
			iceGatheringState: state,
			localDescription: { sdp: 'v=0 x' },
			addEventListener: (_t, fn) => void fns.add(fn),
			removeEventListener: (_t, fn) => void fns.delete(fn),
			fire() {
				this.iceGatheringState = 'complete';
				fns.forEach((f) => f());
			}
		};
	};
	assert.equal(await gathered(fake('complete')), 'v=0 x');
	const later = fake('gathering');
	const p = gathered(later, 10000);
	later.fire();
	assert.equal(await p, 'v=0 x');
	assert.equal(await gathered(fake('gathering'), 5), 'v=0 x', 'the wait ran out');
});
