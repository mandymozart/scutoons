// A shot's generations (src/lib/editor/storyboard-generations.ts, #123): the server's paths and bodies (#120,
// StoryboardController), where a file can be seen from here, Share = the local file into the work storage.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { apiPath, listGenerations, placeOf, shareGeneration, shotGenerations, updateGeneration, workFile, type Generation, type StoryboardApi } from '../src/lib/editor/storyboard-generations.ts';

const gen = (over: Partial<Generation>): Generation => ({
	id: 1, toon: 'catpope', chapter: 3, shot: 'sb-25-1', kind: 'image', take: null, parents: [], generator: 'comfy-local', model: null, status: 'succeeded',
	prompt: null, seed: null, ratio: null, duration: null, estimate: null, cost: null, currency: null, file: 'sb-25-1--g1.png', machine: 'tilman-pc',
	work_path: null, work_url: null, mark: null, approved: false, dataset: false, note: null, error: null, author: null, via: null, created_at: null, ...over
});

function fakeApi() {
	const calls: { method: string; path: string; body?: unknown }[] = [];
	const api: StoryboardApi = {
		get: async <T>(path: string) => (calls.push({ method: 'GET', path }), { items: [gen({})] } as T),
		put: async <T>(path: string, body?: unknown) => (calls.push({ method: 'PUT', path, body }), { item: gen({ work_path: 'sb-25-1--g1.png' }), file: {} } as T),
		fetchRaw: async (path: string) => (calls.push({ method: 'RAW', path }), new Response('bytes'))
	};
	return { api, calls };
}

test('the list and an update go to /storyboard/generations', async () => {
	const { api, calls } = fakeApi();
	assert.equal((await listGenerations(api, 'catpope', 3)).length, 1);
	await listGenerations(api, 'catpope', 3, 'sb-25-1');
	await updateGeneration(api, 7, { mark: 'review' });
	assert.deepEqual(calls, [
		{ method: 'GET', path: '/storyboard/generations?toon=catpope&chapter=3' },
		{ method: 'GET', path: '/storyboard/generations?toon=catpope&chapter=3&shot=sb-25-1' },
		{ method: 'PUT', path: '/storyboard/generations/7', body: { mark: 'review' } }
	]);
});

test('the work storage URL is fetched through the API client (members only)', async () => {
	const { api, calls } = fakeApi();
	assert.equal(apiPath('/api/storyboard/work/catpope/3/a.png'), '/storyboard/work/catpope/3/a.png');
	assert.equal(await (await workFile(api, '/api/storyboard/work/catpope/3/a%20b.png')).text(), 'bytes');
	assert.deepEqual(calls[0], { method: 'RAW', path: '/storyboard/work/catpope/3/a%20b.png' });
});

test('a file shows from the workspace folder, else the work storage, else names its machine', () => {
	const local = new Set(['sb-25-1--g1.png']);
	assert.deepEqual(placeOf(gen({}), local), { at: 'local' });
	assert.deepEqual(placeOf(gen({ work_url: '/api/x' }), new Set()), { at: 'work', url: '/api/x' });
	assert.deepEqual(placeOf(gen({}), new Set()), { at: 'elsewhere', machine: 'tilman-pc' });
	assert.deepEqual(placeOf(gen({ status: 'failed', file: null }), new Set()), { at: 'none' });
});

test('a shot\'s generations: the approved first, then the newest', () => {
	const all = [gen({ id: 1 }), gen({ id: 2, approved: true }), gen({ id: 3 }), gen({ id: 4, shot: 'sb-25-2' })];
	assert.deepEqual(shotGenerations(all, 'sb-25-1').map((g) => g.id), [2, 3, 1]);
});

test('Share uploads the local file under its name, then the record names it', async () => {
	const { api, calls } = fakeApi();
	const blob = new Blob(['png']);
	const workspace = { read: async (toon: string, chapter: number, name: string) => (toon === 'catpope' && chapter === 3 && name === 'sb-25-1--g1.png' ? blob : null) };
	const shared = await shareGeneration(api, workspace, gen({ id: 5 }));
	assert.equal(shared.work_path, 'sb-25-1--g1.png');
	assert.equal(calls[0].path, '/storyboard/work?toon=catpope&chapter=3&name=sb-25-1--g1.png');
	assert.equal(calls[0].body, blob);
	assert.deepEqual(calls[1], { method: 'PUT', path: '/storyboard/generations/5', body: { work_path: 'sb-25-1--g1.png' } });
	// not on this machine: refused before anything is sent
	await assert.rejects(shareGeneration(api, { read: async () => null }, gen({ id: 6 })), /not in your workspace folder/);
	await assert.rejects(shareGeneration(api, workspace, gen({ file: null })), /no file/);
	assert.equal(calls.length, 2);
});
