// #478: the deck's proof ask carries the reader's sign-in to the API's proof route only – the cookie always, the stored
// session token as Bearer when there is one (a browser that refuses cookies).
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { fetchProof, type SignedView } from '../src/deck-key.ts';

const API = 'http://api.test/api';
const URL_OK = `${API}/live/proof?session=CATH`;

function host() {
	const got: [number, string][] = [];
	const view: SignedView = { outlet_signed: () => {}, outlet_proof: (status, body) => void got.push([status, body]) };
	return { host: { view, stopped: false }, got };
}

async function asked(o: { api: string; bearer?: () => string | null }, url = URL_OK) {
	const calls: { url: string; init: RequestInit }[] = [];
	const was = globalThis.fetch;
	globalThis.fetch = (async (u: string, init: RequestInit) => {
		calls.push({ url: u, init });
		return new Response('{"proof":"p.s","expires_in":3600}', { status: 200 });
	}) as typeof fetch;
	try {
		const { host: h, got } = host();
		await fetchProof(h, o, url);
		return { calls, got };
	} finally {
		globalThis.fetch = was;
	}
}

test('#478: the proof ask sends the stored token as Bearer, and the cookie', async () => {
	const { calls, got } = await asked({ api: API, bearer: () => 'tok-1' });
	assert.equal(calls.length, 1);
	assert.equal(calls[0].init.credentials, 'include');
	assert.deepEqual(calls[0].init.headers, { Authorization: 'Bearer tok-1' });
	assert.deepEqual(got, [[200, '{"proof":"p.s","expires_in":3600}']]);
});

test('#478: without a stored token only the cookie goes', async () => {
	for (const bearer of [undefined, () => null]) {
		const { calls } = await asked({ api: API, bearer });
		assert.deepEqual(calls[0].init.headers, {});
		assert.equal(calls[0].init.credentials, 'include');
	}
});

test('#478: a URL that is not the API proof route is refused, the token never leaves', async () => {
	await assert.rejects(asked({ api: API, bearer: () => 'tok-1' }, 'http://elsewhere.test/api/live/proof?session=CATH'), /not the API/);
	await assert.rejects(asked({ api: API, bearer: () => 'tok-1' }, `${API}/decks/media?x=1`), /not the API/);
});
