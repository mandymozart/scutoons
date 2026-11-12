// The scene tools (#490) against a mocked API: what they ask `/api/scenes`, what the agent reads back, and that saving
// is a draft with the creator's link – never a publish.
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { ApiError, sceneFacts, sceneState, type SceneListEntry } from '@scutoons/api';
import { RULES } from '../src/resources.ts';
import { filterScenes, holderWords, registerSceneTools, saveRefused, sceneId, sceneRow, whereWords } from '../src/scenes.ts';
import { creatorHref } from '../src/services.ts';
import { sceneElsewhere } from '../src/server.ts';

const links = { creator: 'http://c', admin: 'http://a', embassy: 'http://e', viewer: 'http://v' };
const nave = {
	format: 2 as const,
	id: 'nave',
	version: 3,
	toons: ['catpope'],
	holder: [{ id: 'catpope', ref: 'character:catpope', rig: { ref: 'rig:character.catpope' } }],
	stage: { name: 'Nave' },
	node: [{ id: 'space', kind: 'space', set: 'cathedral' }]
};
const stage = {
	format: 2 as const,
	id: 'stage',
	toons: [],
	stage: { name: 'Stage' },
	location: { id: 'where', fill: { kind: ['location' as const] } },
	holder: [
		{ id: 'left', spot: 'spot_left', fill: { kind: ['character' as const] }, enter: 'join' as const },
		{ id: 'crowd', fill: { kind: ['item' as const], ref: ['item:incense'] }, take: 'many' as const, arrange: { kind: 'row' as const }, max: 8 }
	]
};
const entry = (doc: typeof nave | typeof stage, latest: number, published: number | null, same = false): SceneListEntry => ({ key: `scene:${doc.id}`, content: doc as never, latest, published, same_as_published: same });

test('a scene id is the key\'s id, and only what the server accepts', () => {
	assert.equal(sceneId('scene:nave'), 'nave');
	assert.equal(sceneId('avatar-home'), 'avatar-home');
	assert.equal(sceneId('Nave'), null);
	assert.equal(sceneId('scene:a/b'), null);
});

test('the state: published, a newer draft waits, never published', () => {
	assert.equal(sceneState(entry(nave, 5, 5)), 'published');
	assert.equal(sceneState(entry(nave, 6, 5)), 'draft');
	assert.equal(sceneState(entry(nave, 6, 5, true)), 'published', 'a restore of the published document is nothing new');
	assert.equal(sceneState(entry(nave, 6, null)), 'unpublished');
	assert.equal(sceneState({ key: 'scene:nave', content: nave as never }), 'published', 'a reader sees only published ones');
});

test('where it plays and who stands there, in words', () => {
	const n = sceneFacts('scene:nave', nave as never);
	const st = sceneFacts('scene:stage', stage as never);
	assert.equal(whereWords(n), 'set cathedral');
	assert.equal(whereWords(st), 'where (placeholder: location)');
	assert.equal(whereWords(sceneFacts('scene:x', { ...nave, location: 'location:dom' } as never)), 'location:dom');
	assert.equal(holderWords(n.holders[0]), 'catpope = character:catpope');
	assert.equal(holderWords(st.holders[0]), 'left ← character at spot_left (join)');
	assert.equal(holderWords(st.holders[1]), 'crowd ← item of item:incense × many (row, max 8)');
});

test('the list filters by toon (no toon = the templates), a word and the state', () => {
	const rows = [entry(nave, 6, 5), entry(stage, 2, null), { key: 'scene:catpope:2:scene-14:de', content: null, latest: 1, published: 1 }];
	assert.equal(sceneRow(rows[0], links).open, 'http://c/scenes/nave');
	const ids = (list: SceneListEntry[]) => list.map((e) => e.key.slice(6));
	assert.deepEqual(ids(filterScenes(rows, { toon: 'catpope' })), ['nave']);
	assert.deepEqual(ids(filterScenes(rows, { toon: '' })), ['stage']);
	assert.deepEqual(ids(filterScenes(rows, { q: 'INCENSE' })), ['stage']);
	assert.deepEqual(ids(filterScenes(rows, { state: 'published' })), []);
	assert.deepEqual(ids(filterScenes(rows, { state: 'draft' })), ['nave', 'stage']);
});

test('a scene key links to its page and leaves the generic content tools', () => {
	assert.equal(creatorHref(links, 'scene:avatar-home'), 'http://c/scenes/avatar-home');
	assert.match(sceneElsewhere('scene:nave') ?? '', /save_scene_draft/);
	assert.equal(sceneElsewhere('chapter:catpope:2'), null);
});

test('the schema\'s problems reach the agent; other errors stay the usual ones', () => {
	assert.deepEqual(saveRefused(new ApiError(422, 'invalid_content', 'Not a scene.', { problems: ['/holder/0: ref or fill'] })), { saved: false, reason: 'Not a scene.', problems: ['/holder/0: ref or fill'] });
	assert.equal(saveRefused(new ApiError(409, 'conflict', 'newer')), null);
});

test('the rules know format 2', () => {
	const rule = RULES.find((r) => r.uri === 'scutoons://rules/scenes');
	assert.ok(rule);
	const text = rule.text();
	for (const word of ['export type SceneDocument', 'export type Holder', 'take?: Take', 'fill?: Fill', 'save_scene_draft']) assert.ok(text.includes(word), word);
});

// ── the tools, the API mocked ─────────────────────────────────────────────────────────────────────────────────────

type Call = { method: string; url: string; body: any; auth: string | null };
type Tool = { handler: (a: any) => Promise<{ content: { type: string; text: string }[]; isError?: boolean }> };

function harness(answer: (call: Call) => { status?: number; body: unknown }, opts: { write?: boolean; token?: string } = {}) {
	const calls: Call[] = [];
	const fake = (async (url: string, init: RequestInit) => {
		const call = { method: String(init.method), url: String(url), body: init.body ? JSON.parse(String(init.body)) : null, auth: (init.headers as Record<string, string>).Authorization ?? null };
		calls.push(call);
		const { status = 200, body } = answer(call);
		return new Response(JSON.stringify(body), { status, headers: { 'content-type': 'application/json' } });
	}) as unknown as typeof fetch;
	const tools: Record<string, Tool> = {};
	const server = { registerTool: (name: string, _def: unknown, handler: Tool['handler']) => (tools[name] = { handler }) };
	registerSceneTools(server as never, { api: 'http://api.test/api', token: opts.token ?? 'sct_abc', tokenSource: 'env', local: false }, { write: opts.write ?? true, links, fetch: fake });
	const run = async (name: string, args: unknown) => {
		const r = await tools[name].handler(args);
		return { error: !!r.isError, text: r.content[0].text, json: (() => { try { return JSON.parse(r.content[0].text); } catch { return null; } })() };
	};
	return { calls, tools, run };
}

const wrapped = (doc: unknown, id: number) => ({ status: 'success', version: { id, status: 'draft', created_at: '2026-11-12T10:00:00Z', author: 'u1', author_email: 'a@b', message: 'm', size: 1 }, published: { id: 5, created_at: '2026-11-11T10:00:00Z' }, content: doc });

test('no publish tool; a reading member gets only the reading tools', () => {
	assert.deepEqual(Object.keys(harness(() => ({ body: {} })).tools).sort(), ['list_scene_versions', 'list_scenes', 'read_scene', 'save_scene_draft']);
	assert.deepEqual(Object.keys(harness(() => ({ body: {} }), { write: false }).tools).sort(), ['list_scene_versions', 'list_scenes', 'read_scene']);
});

test('list_scenes reads /api/scenes with the member\'s token and filters', async () => {
	const h = harness(() => ({ body: { status: 'success', scenes: [entry(nave, 6, 5), entry(stage, 2, null)] } }));
	const r = await h.run('list_scenes', { toon: '' });
	assert.equal(h.calls[0].url, 'http://api.test/api/scenes');
	assert.equal(h.calls[0].auth, 'Bearer sct_abc');
	assert.equal(r.json.of, 2);
	assert.deepEqual(r.json.scenes.map((s: { id: string; state: string }) => [s.id, s.state]), [['stage', 'unpublished']]);
});

test('read_scene: the newest draft with its version id, the published one, a stamp', async () => {
	const h = harness((c) => ({ body: c.url.includes('draft=1') ? wrapped(nave, 7) : { status: 'success', scene: nave } }));
	const draft = await h.run('read_scene', { id: 'scene:nave' });
	assert.equal(h.calls[0].url, 'http://api.test/api/scenes/nave?draft=1');
	assert.equal(draft.json.version.id, 7);
	assert.equal(draft.json.open, 'http://c/scenes/nave');
	await h.run('read_scene', { id: 'nave', source: 'published' });
	assert.equal(h.calls[1].url, 'http://api.test/api/scenes/nave');
	const stamped = await h.run('read_scene', { id: 'nave', stamp: 3 });
	assert.equal(h.calls[2].url, 'http://api.test/api/scenes/nave?v=3');
	assert.equal(stamped.json.content.version, 3);
	assert.equal((await h.run('read_scene', { id: 'Not An Id' })).error, true);
});

test('list_scene_versions asks for the history', async () => {
	const h = harness(() => ({ body: { status: 'success', versions: [{ id: 7, status: 'draft', created_at: 'x', author: 'u', author_email: 'a@b', message: 'm (AI: Claude)', size: 9 }] } }));
	const r = await h.run('list_scene_versions', { id: 'nave', limit: 5 });
	assert.equal(h.calls[0].url, 'http://api.test/api/scenes/nave/versions?limit=5');
	assert.deepEqual(r.json, [{ id: 7, status: 'draft', created_at: 'x', author: 'a@b', message: 'm (AI: Claude)', size: 9 }]);
});

test('save_scene_draft PUTs a draft with its base and hands back the scene page', async () => {
	const h = harness((c) => ({ body: c.method === 'PUT' ? { status: 'success', unchanged: false, version: { id: 8, status: 'draft' } } : wrapped(nave, 7) }));
	const r = await h.run('save_scene_draft', { id: 'nave', base: 7, ops: [{ op: 'set', path: ['stage', 'name'], value: 'Nave at night' }], message: 'night' });
	const put = h.calls.find((c) => c.method === 'PUT')!;
	assert.equal(put.url, 'http://api.test/api/scenes/nave');
	assert.equal(put.body.base, 7);
	assert.equal(put.body.content.stage.name, 'Nave at night');
	assert.equal(put.body.message, 'night', 'the API signs a token\'s save "(AI: …)" itself');
	assert.deepEqual(r.json, { saved: true, unchanged: false, version: 8, status: 'draft', open: 'http://c/scenes/nave', publish: 'the member publishes on the scene page (open)' });
	assert.ok(!h.calls.some((c) => c.url.includes('/publish')), 'never a publish');
});

test('save_scene_draft: a new scene, a stale base, a wrong id, the schema\'s refusal', async () => {
	const fresh = harness(() => ({ body: { status: 'success', unchanged: false, version: { id: 1, status: 'draft' } } }), { token: 'session' });
	await fresh.run('save_scene_draft', { id: 'stage', base: null, content: stage });
	assert.equal(fresh.calls[0].body.base, null);
	assert.equal(fresh.calls[0].body.message, 'AI edit (agent)', 'a copied session is signed by the tool');

	const stale = harness(() => ({ body: wrapped(nave, 9) }));
	const r = await stale.run('save_scene_draft', { id: 'nave', base: 7, ops: [{ op: 'remove', path: ['link'] }] });
	assert.equal(r.error, true);
	assert.match(r.text, /409 conflict: the newest draft is version 9/);
	assert.equal(stale.calls.length, 1, 'nothing saved');

	const wrong = harness(() => ({ body: {} }));
	assert.match((await wrong.run('save_scene_draft', { id: 'nave', base: 7, content: stage })).text, /must be "nave"/);
	assert.equal(wrong.calls.length, 0);

	const refused = harness(() => ({ status: 422, body: { status: 'error', error: 'invalid_content', message: 'Not a scene document.', problems: ['/format: must be 2'] } }));
	const no = await refused.run('save_scene_draft', { id: 'nave', base: 7, content: { ...nave, format: 1 } });
	assert.equal(no.error, true);
	assert.deepEqual(no.json, { saved: false, reason: 'Not a scene document.', problems: ['/format: must be 2'] });
});

test('save_scene_draft: an AI connection that may not edit content reads the API\'s 403', async () => {
	const h = harness(() => ({ status: 403, body: { status: 'error', error: 'forbidden', message: 'content editors only' } }));
	const r = await h.run('save_scene_draft', { id: 'nave', base: null, content: nave });
	assert.equal(r.error, true);
	assert.match(r.text, /^403/);
});
