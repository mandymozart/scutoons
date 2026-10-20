// Files onto the variants board (src/lib/editor/storyboard-board-files.ts, #125): a drop into the member's folder (or,
// without one, the work storage) as a source card, a video's frame as an image with the video as its parent, the names
// already taken.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { recordFrame, recordUpload, takenNames } from '../src/lib/editor/storyboard-board-files.ts';
import type { GenerateApi } from '../src/lib/editor/storyboard-generations.ts';

function fakeApi(work: string[] = []) {
	const calls: { method: string; path: string; body?: unknown }[] = [];
	const api: GenerateApi = {
		get: async <T>(path: string) => (calls.push({ method: 'GET', path }), { items: work.map((name) => ({ name })) } as T),
		put: async <T>(path: string, body?: unknown) => (calls.push({ method: 'PUT', path, body }), { file: {} } as T),
		post: async <T>(path: string, body?: unknown) => (calls.push({ method: 'POST', path, body }), { item: { id: 30, ...(body as object) } } as T),
		fetchRaw: async () => new Response('')
	};
	return { api, calls };
}
const folder = (names: string[]) => {
	const files = new Map<string, Blob>(names.map((n) => [n, new Blob(['x'])]));
	return { files, list: async () => [...files.keys()], write: async (_t: string, _c: number, name: string, data: Blob) => void files.set(name, data) };
};
const AT = { toon: 'catpope', chapter: 3, shot: 'sb-25-1' };
const hash = async () => 'b'.repeat(64);

test('a drop into the folder: the next free name, a source record (upload, no parents)', async () => {
	const { api, calls } = fakeApi();
	const ws = folder(['sb-25-1--g1.png', 'sb-25-1--g4.mp4']);
	const file = Object.assign(new Blob(['png'], { type: 'image/png' }), { name: 'mine.png' });
	await recordUpload({ api, workspace: ws, machine: 'studio-pc', hash }, AT, file, '9:16');
	assert.ok(ws.files.has('sb-25-1--g5.png'));
	assert.deepEqual(calls, [{ method: 'POST', path: '/storyboard/generations', body: { ...AT, kind: 'image', generator: 'upload', status: 'succeeded', file: 'sb-25-1--g5.png', machine: 'studio-pc', file_hash: 'b'.repeat(64), ratio: '9:16' } }]);
	await assert.rejects(recordUpload({ api, workspace: ws, machine: 'm', hash }, AT, Object.assign(new Blob(['x'], { type: 'text/plain' }), { name: 'a.txt' })), /png, jpeg/);
});

test('a drop without a folder: straight into the work storage (machine work)', async () => {
	const { api, calls } = fakeApi(['sb-25-1--g2.png']);
	const file = Object.assign(new Blob(['mp4'], { type: 'video/mp4' }), { name: 'clip.mp4' });
	const taken = () => takenNames(api, AT, [{ file: 'sb-25-1--g3.png', work_path: null }]);
	await recordUpload({ api, workspace: null, machine: 'studio-pc', hash, taken }, AT, file);
	assert.deepEqual(calls.map((c) => `${c.method} ${c.path}`), ['GET /storyboard/work?toon=catpope&chapter=3', 'PUT /storyboard/work?toon=catpope&chapter=3&name=sb-25-1--g4.mp4', 'POST /storyboard/generations']);
	assert.deepEqual(calls[2].body, { ...AT, kind: 'video', generator: 'upload', status: 'succeeded', file: 'sb-25-1--g4.mp4', machine: 'work', work_path: 'sb-25-1--g4.mp4', file_hash: 'b'.repeat(64) });
});

test('a frame: an image of the shot, the video its parent', async () => {
	const { api, calls } = fakeApi();
	const ws = folder(['sb-25-1--g2.mp4']);
	await recordFrame({ api, workspace: ws, machine: 'studio-pc', hash }, AT, new Blob(['png'], { type: 'image/png' }), { id: 12 }, 'the last frame of g2');
	assert.ok(ws.files.has('sb-25-1--g3.png'));
	assert.deepEqual(calls[0].body, { ...AT, kind: 'image', generator: 'frame', status: 'succeeded', parents: [12], file: 'sb-25-1--g3.png', machine: 'studio-pc', file_hash: 'b'.repeat(64), note: 'the last frame of g2' });
});
