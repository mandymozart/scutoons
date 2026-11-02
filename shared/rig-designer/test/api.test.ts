// `npm test -w @scutoons/rig-designer`: the /api/modules client on a fake fetch – the answers of server/docs/API.md
// "Modules" become values, never exceptions.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createDesignerApi, failureOf } from '../src/api.ts';

interface Call {
	url: string;
	method: string;
	headers: Record<string, string>;
	body: unknown;
}

function fake(status: number, body: unknown, calls: Call[] = []): typeof fetch {
	return (async (url: string, init: RequestInit) => {
		calls.push({ url, method: String(init.method), headers: init.headers as Record<string, string>, body: init.body ? JSON.parse(String(init.body)) : undefined });
		return { status, json: async () => body } as Response;
	}) as unknown as typeof fetch;
}

const files = { 'lib.rs': 'fn main() {}' };
const descriptor = { abi: 1, name: 'walk', version: '0.1.0', params: [], ports: [] };
const wasm = { hash: 'a'.repeat(64), bytes: 1234, toolchain: 'rustc 1.90', url: 'https://cdn/modules/a.wasm' };

test('compile: 200 gives the hash, descriptor, wasm and warnings; the request carries the files (+ the member)', async () => {
	const calls: Call[] = [];
	const api = createDesignerApi({
		fetch: fake(200, { status: 'success', hash: wasm.hash, descriptor, wasm, warnings: [{ file: 'lib.rs', line: 2, column: 1, level: 'warning', message: 'unused' }], module: 'm:X' }, calls),
		baseUrl: 'http://api.test/api/',
		getToken: () => 'tok'
	});
	const r = await api.compile({ files, member: 'm:X' });
	assert.ok(r.ok);
	if (r.ok) {
		assert.equal(r.hash, wasm.hash);
		assert.equal(r.wasm.url, wasm.url);
		assert.equal(r.module, 'm:X');
		assert.equal(r.warnings[0].message, 'unused');
	}
	assert.equal(calls[0].url, 'http://api.test/api/modules/compile');
	assert.equal(calls[0].method, 'POST');
	assert.deepEqual(calls[0].body, { files, member: 'm:X' });
	assert.equal(calls[0].headers.Authorization, 'Bearer tok');
	assert.equal((await createDesignerApi({ fetch: fake(200, {}, calls), baseUrl: 'x' }).compile({ files })).ok, false, 'an answer without a module is no success');
});

test('compile: 422 compile_failed gives the errors', async () => {
	const body = { status: 'error', error: 'compile_failed', message: 'expected `;`', errors: [{ file: 'lib.rs', line: 3, column: 9, level: 'error', message: 'expected `;`' }] };
	const r = await createDesignerApi({ fetch: fake(422, body), baseUrl: 'x' }).compile({ files });
	assert.ok(!r.ok && r.reason === 'errors');
	if (!r.ok && r.reason === 'errors') assert.deepEqual(r.errors, [{ file: 'lib.rs', line: 3, column: 9, level: 'error', message: 'expected `;`' }]);
});

test('compile: 403 creator_only is a reason, not an exception', async () => {
	const r = await createDesignerApi({ fetch: fake(403, { status: 'error', error: 'creator_only', message: 'Only creators compile modules.' }), baseUrl: 'x' }).compile({ files });
	assert.deepEqual(r, { ok: false, reason: 'creator_only', code: 'creator_only', message: 'Only creators compile modules.' });
});

test('the other answers are grouped: unavailable (503s), invalid (422 invalid_module), signed out, error', () => {
	for (const code of ['live_show', 'build_busy', 'no_build_service', 'build_unavailable']) assert.equal(failureOf(503, { error: code }).reason, 'unavailable', code);
	const invalid = failureOf(422, { error: 'invalid_module', message: 'files needs a lib.rs', problems: ['files needs a lib.rs', 7] });
	assert.deepEqual([invalid.reason, invalid.problems], ['invalid', ['files needs a lib.rs']]);
	assert.equal(failureOf(401, { error: 'unauthorized' }).reason, 'signed_out');
	assert.equal(failureOf(502, { error: 'build_failed' }).reason, 'error');
	assert.equal(failureOf(500, {}).code, 'http_500');
});

test('a network failure is a value too', async () => {
	const down = (async () => {
		throw new Error('offline');
	}) as unknown as typeof fetch;
	const r = await createDesignerApi({ fetch: down, baseUrl: 'x' }).compile({ files });
	assert.deepEqual(r, { ok: false, reason: 'error', code: 'network', message: 'offline' });
});

test('save / load of the member module: POST creates, PUT replaces, GET reads the files', async () => {
	const calls: Call[] = [];
	const row = { id: 'm:01ABC', name: 'walk', files, descriptor, wasm };
	const created = await createDesignerApi({ fetch: fake(201, { status: 'success', module: row }, calls), baseUrl: 'http://a/api' }).create('walk', files);
	assert.ok(created.ok && created.module.id === 'm:01ABC' && created.module.wasm?.hash === wasm.hash);
	assert.deepEqual([calls[0].method, calls[0].url, calls[0].body], ['POST', 'http://a/api/modules', { name: 'walk', files }]);

	const saved = await createDesignerApi({ fetch: fake(200, { status: 'success', module: row }, calls), baseUrl: 'http://a/api' }).save('m:01ABC', 'walk', files);
	assert.ok(saved.ok);
	assert.deepEqual([calls[1].method, calls[1].url], ['PUT', 'http://a/api/modules/m%3A01ABC']);

	const loaded = await createDesignerApi({ fetch: fake(200, { status: 'success', module: row }, calls), baseUrl: 'http://a/api' }).load('m:01ABC');
	assert.ok(loaded.ok && loaded.module.files['lib.rs'] === files['lib.rs']);

	const missing = await createDesignerApi({ fetch: fake(404, { error: 'not_found', message: 'No module.' }), baseUrl: 'x' }).load('m:NOPE');
	assert.ok(!missing.ok && missing.code === 'not_found');

	const bad = await createDesignerApi({ fetch: fake(422, { error: 'invalid_module', message: 'x', problems: ['files needs a lib.rs'] }), baseUrl: 'x' }).save('m:1', 'n', {});
	assert.ok(!bad.ok && bad.reason === 'invalid' && bad.problems?.[0] === 'files needs a lib.rs');
});

test('list gives the own modules without sources; remove answers ok', async () => {
	const list = await createDesignerApi({ fetch: fake(200, { status: 'success', items: [{ id: 'm:1', name: 'a', descriptor }, { id: 'm:2', name: 'b' }] }), baseUrl: 'x' }).list();
	assert.ok(list.ok);
	if (list.ok) assert.deepEqual(list.items.map((i) => i.id), ['m:1', 'm:2']);
	assert.deepEqual(await createDesignerApi({ fetch: fake(200, { status: 'success' }), baseUrl: 'x' }).remove('m:1'), { ok: true });
});
