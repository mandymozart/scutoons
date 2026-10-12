#!/usr/bin/env node
// Check of the world map as documents (DRAFT, agents/MAP-DOCUMENTS.md) through the SCUTOONS MCP server, against a LOCAL
// API (default the dev port; SCUTOONS_API=http://localhost:<port>/api for a worktree's own) as the local test user:
// read_map_places (each place with its key) → read_entity place:<id> (the place without its id) → validate a place
// moved off the map (an error on that place) → save_draft place:<id> (a new version of that place only, no other
// document touched) → read_characters (composed from character:<id>) → validate a character:<id> → restore_version
// (the original back as a new draft). Restores what it changed, also on failure.
//
//   node scripts/dev/map-docs-check.mjs
import { existsSync, readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { Client } from '@modelcontextprotocol/client';
import { StdioClientTransport } from '@modelcontextprotocol/client/stdio';
import { local } from './ports.mjs';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const API = (process.env.SCUTOONS_API ?? local('api', '/api')).replace(/\/$/, '');
if (!['localhost', '127.0.0.1', '[::1]'].includes(new URL(API).hostname)) {
	console.error(`map-docs-check: refuses to run against ${API} – local dev API only`);
	process.exit(1);
}
const SESSION = resolve(ROOT, 'logs/dev/test-user.json');
const TOKEN = process.env.SCUTOONS_TOKEN || (existsSync(SESSION) ? JSON.parse(readFileSync(SESSION, 'utf8')).token : '');
if (!TOKEN) {
	console.error('map-docs-check: no test user session – run `npm run dev:test-user`');
	process.exit(1);
}

const results = [];
const check = (ok, what, detail = '') => (results.push(!!ok), console.log(`${ok ? 'ok  ' : 'FAIL'} ${what}${!ok && detail ? `  — ${detail}` : ''}`));
const textOf = (r) => (r.content ?? []).map((c) => c.text ?? '').join('');
const api = async (method, path, body) =>
	(await fetch(`${API}${path}`, { method, headers: { Authorization: `Bearer ${TOKEN}`, 'Content-Type': 'application/json' }, body: body ? JSON.stringify(body) : undefined })).json();

const transport = new StdioClientTransport({
	command: process.execPath,
	args: [resolve(ROOT, 'tools/mcp/bin/scutoons-mcp.mjs')],
	env: { ...process.env, SCUTOONS_API: API, SCUTOONS_TOKEN: TOKEN },
	stderr: 'ignore'
});
const client = new Client({ name: 'map-docs-check', version: '1.0.0' });
await client.connect(transport);
async function call(name, args = {}) {
	const r = await client.callTool({ name, arguments: args });
	const raw = textOf(r);
	let json = null;
	try {
		json = JSON.parse(raw);
	} catch {}
	return { error: !!r.isError, raw, json };
}

let original = null;
try {
	const places = await call('read_map_places', {});
	const list = Array.isArray(places.json) ? places.json : [];
	check(list.length > 0 && list.every((p) => p.key === `place:${p.id}`), `read_map_places: ${list.length} places, each with its key`, places.raw.slice(0, 200));
	const place = list.find((p) => !p.removed);
	const key = place.key;

	const read = await call('read_entity', { key });
	const doc = read.json?.content ?? read.json?.data?.content ?? read.json;
	check(!read.error && doc && doc.id === undefined && typeof doc.x === 'number', `read_entity ${key}: the place without its id`, read.raw.slice(0, 200));
	original = await api('GET', `/entities/${key}?draft=1&wrap=1`);

	const bad = await call('validate', { key, content: { ...original.content, x: -50 } });
	const errors = (bad.json?.problems ?? bad.json?.errors ?? []).filter?.((p) => p.level === 'error') ?? [];
	check(/outside the map/.test(bad.raw) && !bad.error, `validate ${key}: moved off the map → an error on the place`, bad.raw.slice(0, 300));
	void errors;

	const before = (await api('GET', '/entities?kinds=map,place,route&content=draft')).entities;
	const saved = await call('save_draft', { key, base: original.version.id, content: { ...original.content, x: original.content.x + 1 }, message: 'map-docs-check' });
	const after = (await api('GET', '/entities?kinds=map,place,route&content=draft')).entities;
	const changed = after.filter((e) => before.find((b) => b.key === e.key)?.latest !== e.latest).map((e) => e.key);
	check(!saved.error && changed.length === 1 && changed[0] === key, `save_draft ${key}: only that document has a new version`, `${saved.raw.slice(0, 200)} changed=${changed.join(',')}`);

	// the character registry: the same shape (characters + character:<id>)
	const chars = await call('read_characters', { toon: 'catpope' });
	const ids = (chars.json?.characters ?? []).map((c) => c.id);
	const entities = (await api('GET', '/entities?kinds=character&content=draft')).entities.filter((e) => e.content && !e.content.removed).map((e) => e.key.slice('character:'.length));
	check(!chars.error && ids.length > 0 && ids.every((id) => entities.includes(id)), `read_characters: ${ids.length} characters, composed from character:<id>`, chars.raw.slice(0, 200));
	const one = await api('GET', `/entities/character:${ids[0]}?draft=1&wrap=1`);
	const badChar = await call('validate', { key: `character:${ids[0]}`, content: { ...one.content, testo: 500 } });
	check(!badChar.error && /testo/.test(badChar.raw), `validate character:${ids[0]}: a broken testo → its error`, badChar.raw.slice(0, 300));
} catch (e) {
	check(false, 'run', e instanceof Error ? e.stack : String(e));
} finally {
	if (original) {
		const now = await api('GET', `/entities/${original.version.key}?draft=1&wrap=1`);
		if (now.version?.id !== original.version.id) {
			const r = await api('POST', `/entities/${original.version.key}/restore`, { id: original.version.id, message: 'map-docs-check: restored' });
			check(r.status === 'success', 'restored the original as a new draft');
		}
	}
	await client.close();
}
const failed = results.filter((r) => !r).length;
console.log(failed ? `map-docs-check: ${failed} failed` : `map-docs-check: all ${results.length} ok`);
process.exit(failed ? 1 : 0);
