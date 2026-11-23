import { test } from 'node:test';
import assert from 'node:assert/strict';
import { ApiError, createApiClient } from '../src/client.ts';
import { itemPath, newestVersion } from '../src/newest-version.ts';

/** a client whose server answers `answer` and records what it was asked */
function client(answer: (url: string, init: RequestInit) => Response) {
	const asked: { url: string; headers: Record<string, string> }[] = [];
	const fetch = (async (url: string, init: RequestInit) => {
		asked.push({ url, headers: init.headers as Record<string, string> });
		return answer(url, init);
	}) as typeof globalThis.fetch;
	return { api: createApiClient({ baseUrl: 'http://api.test/api', fetch }), asked };
}

test('the item route of a key', () => {
	assert.equal(itemPath('chapter:catpope:3'), '/entities/chapter:catpope:3');
	assert.equal(itemPath('scene:nave'), '/scenes/nave');
	assert.equal(itemPath('show:gala'), '/shows/gala');
});

test('304 is the same version; the held id goes as If-None-Match', async () => {
	const { api, asked } = client(() => new Response(null, { status: 304, headers: { ETag: '"41"' } }));
	assert.deepEqual(await newestVersion(api, 'events', 41), { state: 'same' });
	assert.equal(asked[0].url, 'http://api.test/api/entities/events?only=version');
	assert.equal(asked[0].headers['If-None-Match'], '"41"');
});

test('a 200 with another id is newer; with the held id the same', async () => {
	const newer = client(() => Response.json({ status: 'success', key: 'show:gala', version: 57 }, { headers: { ETag: '"57"' } }));
	assert.deepEqual(await newestVersion(newer.api, 'show:gala', 41), { state: 'newer', version: 57 });
	assert.equal(newer.asked[0].url, 'http://api.test/api/shows/gala?only=version');
	const same = client(() => Response.json({ status: 'success', key: 'events', version: 41 }));
	assert.deepEqual(await newestVersion(same.api, 'events', 41), { state: 'same' });
});

test('errors are thrown as the API answered them', async () => {
	const { api } = client(() => Response.json({ status: 'error', error: 'forbidden', message: 'no' }, { status: 403 }));
	await assert.rejects(newestVersion(api, 'events', 1), (e: unknown) => e instanceof ApiError && e.status === 403 && e.code === 'forbidden');
});
