// The handheld camera's pairing (src/lib/editor/storyboard-pairing.ts, #126, #469): the phone's URL, the calls (the PC's
// session client, the phone's plain fetch without a cookie, its one claim). The messages are no longer checked here: the
// core reads them (deck-space relay.rs, its tests `a_handheld_…` and `the_editor_hears_…`; the relay's in session.rs).
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { phoneUrl, publicCalls, PairingError, sessionCalls } from '../src/lib/editor/storyboard-pairing.ts';

const CODE = 'AbCdEfGhIjKlMnOpQrSt_-';

test('the phone\'s page: /h/<code> on the creator', () => {
	assert.equal(phoneUrl('https://creator.scutoons.com/', CODE), `https://creator.scutoons.com/h/${CODE}`);
});

test('the PC\'s calls go through its session client', async () => {
	const calls: string[] = [];
	const api = {
		post: async <T>(p: string, b?: unknown) => (calls.push(`POST ${p} ${JSON.stringify(b)}`), { pairing: { code: CODE, session: 'HABCDEFGHJKM', token: 'dev' } } as T),
		delete: async <T>(p: string) => (calls.push(`DELETE ${p}`), {} as T)
	};
	const c = sessionCalls(api);
	const p = await c.create({ toon: 'catpope', chapter: 3, shot: 'sb-1' });
	assert.deepEqual([p.code, p.session, p.token], [CODE, 'HABCDEFGHJKM', 'dev']);
	await c.remove(CODE);
	await assert.rejects(c.claim(CODE), 'the PC does not claim its own code');
	assert.deepEqual(calls, ['POST /storyboard/pairings {"toon":"catpope","chapter":3,"shot":"sb-1"}', `DELETE /storyboard/pairings/${CODE}`]);
});

test('the phone\'s claim: plain fetch, no cookie, once – the API\'s error as a PairingError', async () => {
	const seen: { url: string; init: RequestInit }[] = [];
	let claimed = false;
	const fetchFn = (async (url: string, init: RequestInit) => {
		seen.push({ url, init });
		if (claimed) return new Response(JSON.stringify({ status: 'error', error: 'claimed', message: 'used' }), { status: 409 });
		claimed = true;
		return new Response(JSON.stringify({ status: 'success', claim: { shot: 'sb-1', session: 'HABCDEFGHJKM', token: 't-hand' } }), { status: 200 });
	}) as typeof fetch;
	const c = publicCalls('http://api.test/api/', fetchFn);
	assert.deepEqual(await c.claim(CODE), { shot: 'sb-1', session: 'HABCDEFGHJKM', token: 't-hand' });
	await assert.rejects(c.claim(CODE), (e: unknown) => e instanceof PairingError && e.status === 409 && e.code === 'claimed');
	assert.deepEqual(seen.map((s) => [s.init.method, s.url, s.init.credentials, s.init.cache]), [
		['POST', `http://api.test/api/storyboard/pairings/${CODE}/claim`, 'omit', 'no-store'],
		['POST', `http://api.test/api/storyboard/pairings/${CODE}/claim`, 'omit', 'no-store']
	]);
	await assert.rejects(c.create({ toon: 'catpope', chapter: 3 }));
	await assert.rejects(c.remove(CODE));
});
