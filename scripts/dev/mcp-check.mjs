#!/usr/bin/env node
// Check of the SCUTOONS MCP server (tools/mcp, CUTOVER 10.28): starts it over stdio like Claude Code does, against the
// LOCAL dev stack (API :9000) as the local test user (`npm run dev:test-user`, or SCUTOONS_TOKEN), and drives the
// tools: rules resources → list_toons → upload_asset (a 1×1 PNG into the sandbox toon, 409 the second time, deleted
// again with the admin API) → list_chapters → read_entity (a scene = its one language-neutral document, agents/NEUTRAL.md)
// → validate (a broken divert is an error) → save_draft (a per-locale scene key is refused; refused with errors; saved
// with a new script point; 409 on an outdated base) → list_versions →
// restore_version (the original content is back as a new draft); the custom collage presets (research/COLLAGE.md §12.6):
// validate refuses a built-in look's id in `colorations`, a draft with a preset and a deleted one is saved, list_entities
// with toon "" has it, a chapter's check judges pieces against it – its versions are deleted again by note (SQL, the
// MariaDB container), and the case leaves an unpublished `colorations` draft alone. No publish tool (agents/MCP.md: the member publishes in
// the creator); the tools follow the member's roles (the test user: creator + admin → content, Embassy, admin). Never runs against anything but localhost; whatever it
// changed is restored, also on failure.
//
//   npm run check:mcp
import { execFileSync } from 'node:child_process';
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
/** chapter 2's one language-neutral document (agents/NEUTRAL.md) – its groups (scenes) inline since 2026-10-19 */
const KEY = `chapter:${TOON}:2`;
/** the group whose script the run edits: chapter 2's third group (the old scene-14 strip) */
const GROUP = 2;
const POINT = 'mcp-check';
/** the custom collage presets' platform-wide document; this run's versions carry the note (save_draft adds " (agent)") */
const PRESETS = 'colorations';
const PRESETS_NOTE = 'mcp-check presets (removed again)';
const DB = process.env.SCUTOONS_DB_NAME ?? 'scutoons';
const CONTAINER = process.env.SCUTOONS_DB_CONTAINER ?? 'scutoons-mariadb-1';
const sql = (query) => execFileSync('docker', ['exec', '-i', CONTAINER, 'sh', '-c', `mariadb -uroot -p"$MARIADB_ROOT_PASSWORD" -N -r ${DB}`], { input: query }).toString().trim();
/** deletes this run's `colorations` versions – how many */
function sweepPresets() {
	const ids = sql(`SELECT id FROM entity_versions WHERE entity_key = '${PRESETS}' AND message IN ('${PRESETS_NOTE}', '${PRESETS_NOTE} (agent)')`).split('\n').filter(Boolean).map(Number);
	if (ids.length) sql(`DELETE FROM entity_versions WHERE id IN (${ids.join(',')})`);
	return ids.length;
}

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
let presetsSaved = false;
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
	check(/divert/.test(flow) && /stageProblems/.test(flow), 'flow rules from validate.ts');
	const presets = textOf({ content: (await client.readResource({ uri: 'scutoons://rules/text-presets' })).contents });
	check(/onomatopoeia/.test(presets) && /"variant"/.test(presets), 'text presets');
	const slices = textOf({ content: (await client.readResource({ uri: 'scutoons://rules/slices' })).contents });
	check(/interface CollagePreset extends PieceStyle/.test(slices) && /removed\?: boolean/.test(slices) && /save_draft` of `colorations`/.test(slices), 'slices rules: custom presets (colorations, the CollagePreset type)', slices.slice(-600));
	const keys = textOf({ content: (await client.readResource({ uri: 'scutoons://rules/entity-keys' })).contents });
	check(/colorations/.test(keys), 'entity-keys rules name colorations');
	// a type block the source lost (a renamed declaration) would be an empty code block
	const empty = [];
	for (const uri of resources)
		for (const [, , body] of textOf({ content: (await client.readResource({ uri })).contents }).matchAll(/```(\w*)\n([\s\S]*?)\n```/g)) if (!body.trim()) empty.push(uri);
	check(!empty.length, 'no rule resource has an empty code block', empty.join(', '));

	const toons = await call(client, 'list_toons');
	check(!toons.error && toons.json?.some((t) => t.id === TOON), 'list_toons has catpope', toons.raw.slice(0, 200));
	const listedToon = toons.json?.find((t) => t.id === TOON);
	check(!!listedToon?.names?.de && listedToon.names.de === listedToon.title?.de, "list_toons: the names are the entry's title (decision 4: one name)", JSON.stringify(listedToon ?? null).slice(0, 300));

	const chapters = await call(client, 'list_chapters', { toon: TOON, chapter: 2 });
	const groups = chapters.json?.chapters?.[0]?.groups ?? [];
	check(groups.length > GROUP && groups.every((g) => !g.scene), 'list_chapters: chapter 2 has its groups inline (no scene references)', chapters.raw.slice(0, 300));
	check(groups.some((g) => typeof g.label === 'string' && g.label), 'list_chapters: inline groups with their labels', JSON.stringify(groups).slice(0, 300));
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

	// custom collage presets (research/COLLAGE.md §12.6): `colorations` is validated; a chapter's check judges a piece's
	// preset against the newest draft – a stored custom preset or a built-in look is fine, a deleted or unknown one warns
	const leftovers = sweepPresets();
	if (leftovers) console.log(`note: an earlier run's ${leftovers} ${PRESETS} version(s) removed`);
	const reserved = await call(client, 'validate', { key: PRESETS, content: { presets: [{ id: 'negative', name: 'Negative 2', fill: '#123456' }] } });
	check(reserved.json?.problems?.some((p) => p.level === 'error' && p.path.join('.') === 'presets.0.id'), "validate colorations: a built-in look's id is an error", reserved.raw.slice(0, 300));
	const presetsBefore = await api('GET', `/entities/${PRESETS}?draft=1&wrap=1`);
	const base = presetsBefore?.version?.id ?? null;
	if (base && presetsBefore.published?.id !== base)
		check(false, `${PRESETS} holds an unpublished draft (version ${base}) – the presets case leaves it alone: publish or restore it first`);
	else {
		const stored = Array.isArray(presetsBefore?.content?.presets) ? presetsBefore.content.presets : [];
		const own = [
			{ id: 'mcp-check-night', name: 'mcp-check night', fill: '#123456' },
			{ id: 'mcp-check-gone', name: 'mcp-check gone', fill: '#654321', removed: true }
		];
		const savedPresets = await call(client, 'save_draft', { key: PRESETS, base, content: { ...presetsBefore?.content, presets: [...stored, ...own] }, message: PRESETS_NOTE });
		presetsSaved = !savedPresets.error;
		check(presetsSaved && savedPresets.json?.saved && savedPresets.json.errors === 0, 'save_draft colorations: a preset and a deleted one, as a draft', savedPresets.raw.slice(0, 300));
		const platform = await call(client, 'list_entities', { toon: '' });
		check(platform.json?.some((e) => e.key === PRESETS), 'list_entities with toon "" lists colorations', platform.raw.slice(0, 200));
		// a picture slice of a chapter with four pieces: the custom preset, the deleted one, an unknown id, a built-in look
		const chapterKey = `chapter:${TOON}:3`;
		const chapter = await call(client, 'read_entity', { key: chapterKey });
		let at = null;
		(chapter.json?.content?.groups ?? []).some((g, gi) => (g?.slices ?? []).some((s, si) => s?.image && (at = ['groups', gi, 'slices', si, 'pieces'])));
		const piece = (n, preset) => ({ id: `piece-mcpchk${n}`, src: 'vectors/mcp-check.svg', ratio: 1, rect: { x: 10 * n, y: 10, width: 20 }, preset });
		const pieces = [piece(1, 'mcp-check-night'), piece(2, 'mcp-check-gone'), piece(3, 'nope'), piece(4, 'ink')];
		const judged = at ? await call(client, 'validate', { key: chapterKey, ops: [{ op: 'set', path: at, value: pieces }] }) : null;
		const presetProblems = (judged?.json?.problems ?? []).filter((p) => p.path.at(-1) === 'preset');
		const of = (i) => presetProblems.filter((p) => p.path.at(-2) === i);
		const detail = at ? JSON.stringify(presetProblems).slice(0, 400) : `no picture slice in ${chapterKey}`;
		check(at && !judged.error && !of(0).length && !of(3).length, 'validate chapter: a stored custom preset and a built-in look – no warning', detail);
		check(of(1).some((p) => p.level === 'warning' && /"mcp-check gone" was deleted/.test(p.message)), 'validate chapter: a deleted preset warns', detail);
		check(of(2).some((p) => p.level === 'warning' && /unknown preset "nope"/.test(p.message)), 'validate chapter: an unknown preset id warns', detail);
		const removed = sweepPresets();
		presetsSaved = false;
		const presetsAfter = await api('GET', `/entities/${PRESETS}?draft=1&wrap=1`);
		check((presetsAfter?.version?.id ?? null) === base, `${PRESETS} restored: ${removed} version(s) removed, newest ${presetsAfter?.version?.id ?? 'none'} (was ${base ?? 'none'})`);
	}

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
	check(!read.error && !Array.isArray(read.json) && read.json?.key === KEY && read.json?.version?.id > 0, 'read_entity: the chapter is one language-neutral document (draft, version id)', read.raw.slice(0, 200));
	original = { id: read.json.version.id, content: read.json.content };
	const events = original.content.groups?.[GROUP]?.events ?? [];
	check(!events.some((e) => e?.params?.name === POINT), `no "${POINT}" point yet (an earlier run was restored)`);
	const perLocale = await call(client, 'save_draft', { key: `${KEY}:de`, base: original.id, ops: [{ op: 'set', path: ['title'], value: 'x' }] });
	check(perLocale.error && /language-neutral/.test(perLocale.raw), 'save_draft refuses a per-locale chapter key (one neutral document)', perLocale.raw.slice(0, 200));
	const sceneKey = await call(client, 'read_entity', { key: `scene:${TOON}:2:scene-14` });
	check(!!sceneKey.error, 'read_entity: a scene key is no key any more (scenes are groups of their chapter)', sceneKey.raw.slice(0, 200));

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

	// the group's script (the newest version may have none yet)
	const at = ['groups', GROUP, 'events'];
	const start = Array.isArray(original.content.groups?.[GROUP]?.events) ? [] : [{ op: 'set', path: at, value: [] }];
	const end = events.length;
	const broken = [...start, { op: 'insert', path: [...at, end], value: { type: 'divert', params: { to: 'nowhere-mcp' } } }];
	const v = await call(client, 'validate', { key: KEY, ops: broken });
	check(v.json?.errors > 0 && v.raw.includes('nowhere-mcp'), 'validate: a divert to a missing point is an error', v.raw.slice(0, 300));
	const untranslated = await call(client, 'validate', { key: KEY, ops: [...start, { op: 'insert', path: [...at, '-'], value: { type: 'showMessage', params: { text: { de: 'Nur auf Deutsch – mcp-check' } } } }] });
	check(untranslated.json?.problems?.some((p) => p.level === 'warning' && p.locale === 'en' && p.path.includes('text')), 'validate: a text without its en translation is a warning for en (validateNeutral)', untranslated.raw.slice(0, 300));
	const refused = await call(client, 'save_draft', { key: KEY, base: original.id, ops: broken });
	check(refused.error && refused.json?.saved === false, 'save_draft refuses errors', refused.raw.slice(0, 200));

	const ops = [
		...start,
		{ op: 'insert', path: [...at, '-'], value: { type: 'divert', params: { to: POINT }, if: { variable: 'mcp-check', op: '==', value: 1 } } },
		{ op: 'insert', path: [...at, '-'], value: { type: 'label', params: { name: POINT } } }
	];
	const saved = await call(client, 'save_draft', { key: KEY, base: original.id, ops, message: 'mcp-check' });
	check(!saved.error && saved.json?.saved && saved.json.version > original.id, 'save_draft: new draft version', saved.raw.slice(0, 300));
	const stale = await call(client, 'save_draft', { key: KEY, base: original.id, ops });
	check(stale.error && /409 conflict/.test(stale.raw), 'save_draft on an outdated base: 409 conflict, clearly', stale.raw.slice(0, 200));

	const after = await call(client, 'list_chapters', { toon: TOON, chapter: 2 });
	check(after.json?.chapters?.[0]?.groups?.[GROUP]?.points?.includes(POINT), 'list_chapters shows the new script point');
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
	if (presetsSaved) console.log(`removed ${sweepPresets()} ${PRESETS} version(s) of this run`);
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
