#!/usr/bin/env node
// Check of the SCUTOONS MCP server (tools/mcp, CUTOVER 10.28): starts it over stdio like Claude Code does, against the
// LOCAL dev stack (API :9000) as the local test user (`npm run dev:test-user`, or SCUTOONS_TOKEN), and drives the
// tools: rules resources → list_toons → list_chapters → read_entity → validate (a broken divert is an error) →
// save_draft (refused with errors; saved with a new script point; 409 on an outdated base) → list_versions →
// restore_version (the original content is back as a new draft). Publish is off (no tool); with SCUTOONS_MCP_PUBLISH=1
// the tool exists and refuses a call without confirm: true. Never runs against anything but localhost; whatever it
// changed is restored, also on failure.
//
//   npm run check:mcp
import { existsSync, readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { Client } from '@modelcontextprotocol/client';
import { StdioClientTransport } from '@modelcontextprotocol/client/stdio';
import { local } from './ports.mjs';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const API = (process.env.SCUTOONS_API ?? local('api', '/api')).replace(/\/$/, '');
if (!['localhost', '127.0.0.1', '[::1]'].includes(new URL(API).hostname)) {
	console.error(`mcp-check: refuses to run against ${API} – local dev API only`);
	process.exit(1);
}
const SESSION = resolve(ROOT, 'logs/dev/test-user.json');
const TOKEN = process.env.SCUTOONS_TOKEN || (existsSync(SESSION) ? JSON.parse(readFileSync(SESSION, 'utf8')).token : '');
if (!TOKEN) {
	console.error('mcp-check: no test user session – run `npm run dev:test-user`');
	process.exit(1);
}
const TOON = 'catpope';
const SCENE = `scene:${TOON}:2:scene-14`;
const KEY = `${SCENE}:de`;
const POINT = 'mcp-check';

const results = [];
const check = (ok, what, detail = '') => (results.push(!!ok), console.log(`${ok ? 'ok  ' : 'FAIL'} ${what}${!ok && detail ? `  — ${detail}` : ''}`));

async function connect(env = {}) {
	const transport = new StdioClientTransport({
		command: process.execPath,
		args: [resolve(ROOT, 'tools/mcp/bin/scutoons-mcp.mjs')],
		env: { ...process.env, SCUTOONS_API: API, SCUTOONS_TOKEN: TOKEN, SCUTOONS_MCP_PUBLISH: '', ...env },
		stderr: 'ignore'
	});
	const client = new Client({ name: 'mcp-check', version: '1.0.0' });
	await client.connect(transport);
	return client;
}
const textOf = (r) => (r.content ?? []).map((c) => c.text ?? '').join('');
async function call(client, name, args = {}) {
	const r = await client.callTool({ name, arguments: args });
	const raw = textOf(r);
	let json = null;
	try {
		json = JSON.parse(raw);
	} catch {}
	return { error: !!r.isError, raw, json };
}

// direct API access for the safety net only
async function api(method, path, body) {
	const res = await fetch(`${API}${path}`, { method, headers: { Authorization: `Bearer ${TOKEN}`, 'Content-Type': 'application/json' }, body: body ? JSON.stringify(body) : undefined });
	return res.json();
}

const client = await connect();
let original = null; // { id, content }
let restored = false;
try {
	const tools = (await client.listTools()).tools.map((t) => t.name);
	const expected = ['list_toons', 'list_chapters', 'read_entity', 'read_characters', 'read_map_places', 'search_assets', 'list_versions', 'validate', 'save_draft', 'restore_version'];
	check(expected.every((t) => tools.includes(t)), 'every tool is there', `missing: ${expected.filter((t) => !tools.includes(t))}`);
	check(!tools.includes('publish_entity'), 'no publish tool without SCUTOONS_MCP_PUBLISH=1');

	const resources = (await client.listResources()).resources.map((r) => r.uri);
	check(resources.length >= 6, `${resources.length} rule resources`, resources.join(', '));
	const vn = textOf({ content: (await client.readResource({ uri: 'scutoons://rules/vn-events' })).contents });
	check(/divert/.test(vn) && /anyOf/.test(vn) && /interface VNEvent|VNEvent/.test(vn), 'vn-events rules come from the source (divert, anyOf, types)');
	const flow = textOf({ content: (await client.readResource({ uri: 'scutoons://rules/flow' })).contents });
	check(/divert/.test(flow) && /joinScene|joined/.test(flow), 'flow rules from validate.ts');
	const presets = textOf({ content: (await client.readResource({ uri: 'scutoons://rules/text-presets' })).contents });
	check(/onomatopoeia/.test(presets) && /"variant"/.test(presets), 'text presets');

	const toons = await call(client, 'list_toons');
	check(!toons.error && toons.json?.some((t) => t.id === TOON), 'list_toons has catpope', toons.raw.slice(0, 200));

	const chapters = await call(client, 'list_chapters', { toon: TOON, chapter: 2 });
	const groups = chapters.json?.chapters?.[0]?.groups ?? [];
	check(groups.some((g) => g.scene === SCENE), 'list_chapters: chapter 2 references scene-14', chapters.raw.slice(0, 300));
	check(groups.some((g) => g.label === 'scene-20'), 'list_chapters: group labels (scene-20)');

	const chars = await call(client, 'read_characters', { toon: TOON });
	check(!chars.error && chars.json?.characters?.length > 0 && chars.json?.moods?.length > 0, 'read_characters: cast + moods', chars.raw.slice(0, 200));
	const places = await call(client, 'read_map_places', { toon: TOON });
	check(!places.error && Array.isArray(places.json), 'read_map_places', places.raw.slice(0, 200));
	const found = await call(client, 'search_assets', { q: 'catpope', page_size: 3 });
	check(!found.error && typeof found.json?.total === 'number', 'search_assets', found.raw.slice(0, 200));

	const both = await call(client, 'read_entity', { key: SCENE });
	check(Array.isArray(both.json) && both.json.length >= 2, 'read_entity without locale: every language', both.raw.slice(0, 200));
	const read = await call(client, 'read_entity', { key: KEY });
	check(!read.error && read.json?.version?.id > 0, 'read_entity: draft with version id', read.raw.slice(0, 200));
	original = { id: read.json.version.id, content: read.json.content };
	const events = original.content.events ?? [];
	check(!events.some((e) => e?.params?.name === POINT), `no "${POINT}" point yet (an earlier run was restored)`);

	const end = events.length;
	const broken = [{ op: 'insert', path: ['events', end], value: { type: 'divert', params: { to: 'nowhere-mcp' } } }];
	const v = await call(client, 'validate', { key: KEY, ops: broken });
	check(v.json?.errors > 0 && v.raw.includes('nowhere-mcp'), 'validate: a divert to a missing point is an error', v.raw.slice(0, 300));
	const refused = await call(client, 'save_draft', { key: KEY, base: original.id, ops: broken });
	check(refused.error && refused.json?.saved === false, 'save_draft refuses errors', refused.raw.slice(0, 200));

	const ops = [
		{ op: 'insert', path: ['events', '-'], value: { type: 'divert', params: { to: POINT }, if: { variable: 'mcp-check', op: '==', value: 1 } } },
		{ op: 'insert', path: ['events', '-'], value: { type: 'label', params: { name: POINT } } }
	];
	const saved = await call(client, 'save_draft', { key: KEY, base: original.id, ops, message: 'mcp-check' });
	check(!saved.error && saved.json?.saved && saved.json.version > original.id, 'save_draft: new draft version', saved.raw.slice(0, 300));
	const stale = await call(client, 'save_draft', { key: KEY, base: original.id, ops });
	check(stale.error && /409 conflict/.test(stale.raw), 'save_draft on an outdated base: 409 conflict, clearly', stale.raw.slice(0, 200));

	const after = await call(client, 'list_chapters', { toon: TOON, chapter: 2 });
	check(after.json?.chapters?.[0]?.groups?.find((g) => g.scene === SCENE)?.points?.includes(POINT), 'list_chapters shows the new script point');
	const versions = await call(client, 'list_versions', { key: KEY, limit: 3 });
	check(versions.json?.[0]?.id === saved.json?.version && /agent/.test(versions.json?.[0]?.message ?? ''), 'list_versions: newest = the agent save', versions.raw.slice(0, 200));

	const back = await call(client, 'restore_version', { key: KEY, version: original.id, message: 'mcp-check restore' });
	restored = !back.error;
	check(!back.error && back.json?.version > saved.json.version, 'restore_version: a new draft version', back.raw.slice(0, 200));
	const again = await call(client, 'read_entity', { key: KEY });
	check(JSON.stringify(again.json?.content) === JSON.stringify(original.content), 'the original content is back');
} catch (error) {
	check(false, 'run', error?.stack ?? String(error));
} finally {
	await client.close();
	// safety net: put the original back when the run stopped between save and restore
	if (original && !restored) {
		const now = await api('GET', `/entities/${KEY}?draft=1&wrap=1`);
		if (now?.version?.id !== original.id) await api('POST', `/entities/${KEY}/restore`, { id: original.id, message: 'mcp-check restore (after failure)' });
		console.log(`restored ${KEY} to version ${original.id}`);
	}
}

// publish switch: the tool exists only with SCUTOONS_MCP_PUBLISH=1 and refuses a call without confirm: true
const publisher = await connect({ SCUTOONS_MCP_PUBLISH: '1' });
try {
	const tools = (await publisher.listTools()).tools.map((t) => t.name);
	check(tools.includes('publish_entity'), 'SCUTOONS_MCP_PUBLISH=1: publish_entity is there');
	let refusedPublish = false;
	try {
		const r = await publisher.callTool({ name: 'publish_entity', arguments: { key: `${SCENE.replace('scene-14', 'mcp-check-none')}:de`, confirm: false } });
		refusedPublish = !!r.isError;
	} catch {
		refusedPublish = true;
	}
	check(refusedPublish, 'publish_entity without confirm: true is refused');
} finally {
	await publisher.close();
}

const bad = results.filter((r) => !r).length;
console.log(bad ? `mcp-check: ${bad} failed` : `mcp-check: all ${results.length} passed`);
process.exit(bad ? 1 : 0);
