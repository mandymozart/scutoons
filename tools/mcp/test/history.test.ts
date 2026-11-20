// revert_change / revert_to (#687) against a mocked API: both only make a NEW DRAFT through the History routes, and every
// rule is the core's – the real deck-core wasm when it is built (build:wasm), else only the wiring is checked.
import assert from 'node:assert/strict';
import { test } from 'node:test';
import type { HistoryView } from '@scutoons/content';
import { coreDir, coreHistory } from '../src/core-history.ts';
import { registerHistoryTools } from '../src/history.ts';

const links = { creator: 'http://c', admin: 'http://a', embassy: 'http://e', viewer: 'http://v' };
type Call = { method: string; url: string; body: any; auth: string | null };
type Tool = { handler: (a: any) => Promise<{ content: { type: string; text: string }[]; isError?: boolean }> };

const at = (n: number) => new Date(Date.UTC(2026, 10, 1, 10, n * 5)).toISOString();
const version = (id: number, author: string, message: string | null = null) => ({ id, status: 'draft', created_at: at(id), author: 'u', author_email: author, message, size: 1 });

/** versions 1 (ann: first save), 2 (ann: titles), 3 (bob: city), as an events-like document */
const docs: Record<number, unknown> = {
	1: { title: 'A', city: 'x', note: 'n' },
	2: { title: 'B', city: 'x', note: 'n' },
	3: { title: 'B', city: 'y', note: 'n' }
};
const versions = [version(3, 'bob@x'), version(2, 'ann@x'), version(1, 'ann@x')];

function harness(core: () => Promise<HistoryView>, opts: { stale?: boolean; write?: boolean } = {}) {
	const calls: Call[] = [];
	const fake = (async (url: string, init: RequestInit) => {
		const call = { method: String(init.method), url: String(url), body: init.body ? JSON.parse(String(init.body)) : null, auth: (init.headers as Record<string, string>).Authorization ?? null };
		calls.push(call);
		const json = (body: unknown, status = 200) => new Response(JSON.stringify(body), { status, headers: { 'content-type': 'application/json' } });
		const u = new URL(call.url);
		if (call.method === 'GET' && u.pathname.endsWith('/versions')) return json({ status: 'success', versions });
		if (call.method === 'GET' && u.searchParams.get('draft')) return json({ status: 'success', version: versions[0], content: docs[3] });
		if (call.method === 'GET' && u.searchParams.get('version')) {
			const id = Number(u.searchParams.get('version'));
			return json({ status: 'success', version: versions.find((v) => v.id === id), content: docs[id] });
		}
		if (call.method === 'POST' && /\/revert(-to)?$/.test(u.pathname)) {
			if (opts.stale) return json({ status: 'error', error: 'conflict', message: 'saved since' }, 409);
			return json({ status: 'success', unchanged: false, version: { ...version(4, 'agent@x', 'revert'), status: 'draft' } });
		}
		return json({ status: 'error' }, 404);
	}) as unknown as typeof fetch;
	const tools: Record<string, Tool> = {};
	const server = { registerTool: (name: string, _def: unknown, handler: Tool['handler']) => (tools[name] = { handler }) };
	registerHistoryTools(server as never, { api: 'http://api.test/api', token: 'sct_abc', tokenSource: 'env', local: false }, { links, core, fetch: fake });
	const run = async (name: string, args: unknown) => {
		const r = await tools[name].handler(args);
		return { error: !!r.isError, text: r.content[0].text, json: (() => { try { return JSON.parse(r.content[0].text); } catch { return null; } })() };
	};
	return { calls, tools, run };
}

const wasm = coreDir() !== null;
const core = coreHistory('http://v');

test('the two tools, drafts only: no publish, no restore of its own', () => {
	assert.deepEqual(Object.keys(harness(core).tools).sort(), ['revert_change', 'revert_to']);
});

test('revert_to posts the version and the newest draft as base: a new draft, never a publish', { skip: !wasm && 'core wasm not built' }, async () => {
	const h = harness(core);
	const r = await h.run('revert_to', { key: 'events', version: 1 });
	const post = h.calls.find((c) => c.method === 'POST')!;
	assert.equal(post.url, 'http://api.test/api/entities/events/revert-to');
	assert.deepEqual(post.body, { version: 1, base: 3 });
	assert.equal(post.auth, 'Bearer sct_abc');
	assert.equal(r.json.status, 'draft');
	assert.equal(r.json.version, 4);
	assert.equal(r.json.later, 2, 'the core counts the saves after the point');
	assert.deepEqual(r.json.others.sort(), ['ann@x', 'bob@x']);
	assert.ok(!h.calls.some((c) => c.url.includes('publish')));
});

test('revert_change: the core undoes exactly that step and keeps the later one', { skip: !wasm && 'core wasm not built' }, async () => {
	const h = harness(core);
	const r = await h.run('revert_change', { key: 'events', version: 2 });
	assert.equal(r.error, false, r.text);
	const post = h.calls.find((c) => c.method === 'POST')!;
	assert.equal(post.url, 'http://api.test/api/entities/events/revert');
	assert.deepEqual(post.body, { version: 2, content: { title: 'A', city: 'y', note: 'n' }, base: 3 });
	assert.equal(r.json.reverted, true);
	assert.equal(r.json.status, 'draft');
	assert.deepEqual(r.json.kept, []);
});

test('revert_change: nothing to revert when the place changed again, the first version has no before, an unknown version is refused', { skip: !wasm && 'core wasm not built' }, async () => {
	const h = harness(core);
	const again = await h.run('revert_change', { key: 'events', version: 3, base: 3 });
	assert.equal(again.json.reverted, true, 'the newest step reverts onto itself');
	docs[3] = { title: 'C', city: 'y', note: 'n' };
	try {
		const kept = await harness(core).run('revert_change', { key: 'events', version: 2 });
		assert.equal(kept.json.reverted, false, 'the title was changed again since: it stays');
		assert.deepEqual(kept.json.kept, ['title']);
	} finally {
		docs[3] = { title: 'B', city: 'y', note: 'n' };
	}
	const first = await h.run('revert_change', { key: 'events', version: 1 });
	assert.equal(first.error, true);
	assert.match(first.text, /nothing before it/);
	assert.equal((await h.run('revert_change', { key: 'events', version: 99 })).error, true);
	assert.equal(h.calls.filter((c) => c.method === 'POST').length, 1, 'only the one real revert was posted');
});

test('a stale base is a 409 conflict the agent can act on', { skip: !wasm && 'core wasm not built' }, async () => {
	const r = await harness(core, { stale: true }).run('revert_to', { key: 'events', version: 1, base: 2 });
	assert.equal(r.error, true);
	assert.match(r.text, /409 conflict/);
});

test('the core files: SCUTOONS_CORE_DIR wins, a bundled copy without it has none', () => {
	assert.equal(coreDir({ SCUTOONS_CORE_DIR: '/nowhere' } as never, true), null);
});
