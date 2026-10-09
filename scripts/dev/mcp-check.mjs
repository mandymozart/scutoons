#!/usr/bin/env node
// Check of the SCUTOONS MCP server (tools/mcp, CUTOVER 10.28): starts it over stdio like Claude Code does, against the
// LOCAL dev stack (API :9000) as the local test user (`npm run dev:test-user`, or SCUTOONS_TOKEN), and drives the
// tools: rules resources → list_toons → upload_asset (a 1×1 PNG into the sandbox toon, 409 the second time, deleted
// again with the admin API) → list_chapters → read_entity (a scene = its one language-neutral document, agents/NEUTRAL.md)
// → validate (a broken divert is an error) → save_draft (a per-locale scene key is refused; refused with errors; saved
// with a new script point; 409 on an outdated base) → list_versions →
// restore_version (the original content is back as a new draft). No publish tool (agents/MCP.md: the member publishes in
// the creator); the tools follow the member's roles (the test user: creator + admin → content, Embassy, admin). Never runs against anything but localhost; whatever it
// changed is restored, also on failure.
//
//   npm run check:mcp
import { existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
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
/** the scene's one language-neutral document (agents/NEUTRAL.md) */
const KEY = SCENE;
const POINT = 'mcp-check';

const results = [];
const check = (ok, what, detail = '') => (results.push(!!ok), console.log(`${ok ? 'ok  ' : 'FAIL'} ${what}${!ok && detail ? `  — ${detail}` : ''}`));

async function connect(env = {}) {
	const transport = new StdioClientTransport({
		command: process.execPath,
		args: [resolve(ROOT, 'tools/mcp/bin/scutoons-mcp.mjs')],
		env: { ...process.env, SCUTOONS_API: API, SCUTOONS_TOKEN: TOKEN, ...env },
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
	const expected = ['list_toons', 'list_chapters', 'read_entity', 'read_characters', 'read_map_places', 'search_assets', 'upload_asset', 'list_entities', 'search_content', 'submit_for_review', 'list_versions', 'validate', 'save_draft', 'restore_version'];
	check(expected.every((t) => tools.includes(t)), 'every tool is there', `missing: ${expected.filter((t) => !tools.includes(t))}`);
	check(!tools.includes('publish_entity'), 'no publish tool');
	check(['whoami', 'read_flow', 'story_variables', 'compare_versions', 'today', 'propose_changes'].every((t) => tools.includes(t)), 'tools by role: content + story + Embassy + admin');

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
	const listedToon = toons.json?.find((t) => t.id === TOON);
	check(!!listedToon?.names?.de && listedToon.names.de === listedToon.title?.de, "list_toons: the names are the entry's title (decision 4: one name)", JSON.stringify(listedToon ?? null).slice(0, 300));

	const chapters = await call(client, 'list_chapters', { toon: TOON, chapter: 2 });
	const groups = chapters.json?.chapters?.[0]?.groups ?? [];
	check(groups.some((g) => g.scene === SCENE), 'list_chapters: chapter 2 references scene-14', chapters.raw.slice(0, 300));
	check(groups.some((g) => !g.scene && typeof g.label === 'string' && g.label), 'list_chapters: inline groups with their labels', JSON.stringify(groups).slice(0, 300));
	check(chapters.json?.chapters?.[0]?.key === `chapter:${TOON}:2` && typeof chapters.json?.chapters?.[0]?.title === 'string', 'list_chapters: the language-neutral chapter, its title in the source language', chapters.raw.slice(0, 300));

	// finding things: the document list, a full-text search in the screenplay (source language), a part of a long text
	const listed = await call(client, 'list_entities', { toon: TOON });
	check(listed.json?.some((e) => e.key === `script:${TOON}:de`), 'list_entities has the screenplay', listed.raw.slice(0, 200));
	const found1 = await call(client, 'search_content', { query: 'not my city', toon: TOON });
	const hit = found1.json?.hits?.find((h) => h.key === `script:${TOON}:de` && / line \d+$/.test(h.where));
	check(hit, 'search_content finds the song in the German screenplay', found1.raw.slice(0, 300));
	if (hit) {
		const line = Number(/ line (\d+)$/.exec(hit.where)[1]);
		const part = await call(client, 'read_entity', { key: `script:${TOON}:de`, path: ['text'], lines: [line, line + 2] });
		check(typeof part.json?.content === 'string' && part.json.content.startsWith(`${line}: `) && part.json.content.split('\n').length === 3, 'read_entity lines: three numbered lines', part.raw.slice(0, 200));
	}

	const chars = await call(client, 'read_characters', { toon: TOON });
	check(!chars.error && chars.json?.characters?.length > 0 && chars.json?.moods?.length > 0, 'read_characters: cast + moods', chars.raw.slice(0, 200));
	const places = await call(client, 'read_map_places', { toon: TOON });
	check(!places.error && Array.isArray(places.json), 'read_map_places', places.raw.slice(0, 200));
	const found = await call(client, 'search_assets', { q: 'catpope', page_size: 3 });
	check(!found.error && typeof found.json?.total === 'number', 'search_assets', found.raw.slice(0, 200));

	// upload: a new file, never an overwrite; removed again afterwards (the test user is admin)
	const dir = mkdtempSync(join(tmpdir(), 'mcp-check-'));
	const png = join(dir, 'dot.png');
	writeFileSync(png, Buffer.from('iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mP8z8BQDwAEhQGAhKmMIQAAAABJRU5ErkJggg==', 'base64'));
	const target = `sandbox/mcp-check/dot-${Date.now()}.png`;
	const up = await call(client, 'upload_asset', { file: png, path: target, title: 'mcp-check', tags: ['mcp-check'] });
	check(!up.error && up.json?.uploaded === target && up.json?.contentPath === target.replace('sandbox/', ''), 'upload_asset: a new file, the content path relative to the toon', up.raw.slice(0, 300));
	const twice = await call(client, 'upload_asset', { file: png, path: target });
	check(twice.error && /409/.test(twice.raw), 'upload_asset never overwrites (409)', twice.raw.slice(0, 200));
	if (up.json?.asset) {
		const del = await api('DELETE', `/admin/assets/${encodeURIComponent(up.json.asset)}/?force=1`);
		check(del?.status === 'success', 'test upload deleted again', JSON.stringify(del).slice(0, 200));
	}
	rmSync(dir, { recursive: true, force: true });

	const read = await call(client, 'read_entity', { key: KEY });
	check(!read.error && !Array.isArray(read.json) && read.json?.key === KEY && read.json?.version?.id > 0, 'read_entity: the scene is one language-neutral document (draft, version id)', read.raw.slice(0, 200));
	original = { id: read.json.version.id, content: read.json.content };
	const events = original.content.events ?? [];
	check(!events.some((e) => e?.params?.name === POINT), `no "${POINT}" point yet (an earlier run was restored)`);
	const perLocale = await call(client, 'save_draft', { key: `${SCENE}:de`, base: original.id, ops: [{ op: 'set', path: ['label'], value: 'x' }] });
	check(perLocale.error && /language-neutral/.test(perLocale.raw), 'save_draft refuses a per-locale scene key (one neutral document)', perLocale.raw.slice(0, 200));

	// the toon settings: the toon entry (agents/NEUTRAL.md step 4) – texts per language, the old per-language key refused
	const toonEntry = await call(client, 'read_entity', { key: `toon:${TOON}` });
	check(!toonEntry.error && typeof toonEntry.json?.content?.title === 'object', 'read_entity: the toon entry holds the toon settings (title per language)', toonEntry.raw.slice(0, 200));
	const settingsKey = await call(client, 'save_draft', { key: `toon-settings:${TOON}:de`, base: null, content: { id: TOON } });
	check(settingsKey.error && /toon:/.test(settingsKey.raw), 'save_draft refuses the old per-language toon settings key', settingsKey.raw.slice(0, 200));
	const entryCheck = await call(client, 'validate', { key: `toon:${TOON}`, ops: [{ op: 'set', path: ['languageStatus', 'en'], value: 'later' }] });
	check(entryCheck.json?.problems?.some((p) => p.level === 'error' && p.locale === 'en' && p.path.includes('languageStatus')), "validate: the toon entry's status per language (validateNeutral)", entryCheck.raw.slice(0, 300));
	check(!('i18n' in (toonEntry.json?.content ?? {})), 'the toon entry stores no i18n (the name is title)', toonEntry.raw.slice(0, 200));
	const oldName = await call(client, 'validate', { key: `toon:${TOON}`, ops: [{ op: 'set', path: ['i18n'], value: { de: { name: 'Alt' } } }] });
	check(oldName.json?.problems?.some((p) => p.level === 'warning' && p.path[0] === 'i18n' && /title/.test(p.message)), 'validate: an i18n in the toon entry is a warning (drop it, the name is title)', oldName.raw.slice(0, 300));

	// the scene's group script (the newest version may have none yet)
	const start = Array.isArray(original.content.events) ? [] : [{ op: 'set', path: ['events'], value: [] }];
	const end = events.length;
	const broken = [...start, { op: 'insert', path: ['events', end], value: { type: 'divert', params: { to: 'nowhere-mcp' } } }];
	const v = await call(client, 'validate', { key: KEY, ops: broken });
	check(v.json?.errors > 0 && v.raw.includes('nowhere-mcp'), 'validate: a divert to a missing point is an error', v.raw.slice(0, 300));
	const untranslated = await call(client, 'validate', { key: KEY, ops: [...start, { op: 'insert', path: ['events', '-'], value: { type: 'showMessage', params: { text: { de: 'Nur auf Deutsch – mcp-check' } } } }] });
	check(untranslated.json?.problems?.some((p) => p.level === 'warning' && p.locale === 'en' && p.path.includes('text')), 'validate: a text without its en translation is a warning for en (validateNeutral)', untranslated.raw.slice(0, 300));
	const refused = await call(client, 'save_draft', { key: KEY, base: original.id, ops: broken });
	check(refused.error && refused.json?.saved === false, 'save_draft refuses errors', refused.raw.slice(0, 200));

	const ops = [
		...start,
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

const bad = results.filter((r) => !r).length;
console.log(bad ? `mcp-check: ${bad} failed` : `mcp-check: all ${results.length} passed`);
process.exit(bad ? 1 : 0);
