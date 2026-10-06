#!/usr/bin/env node
// OBSOLETE (2026-10-06): written for the per-locale chapter 2 documents and the old `{x:clock}` syntax – both gone
// (CUTOVER 10.30 / 10.33; the server refuses `{x:clock}`). Kept as the example the MCP README / CUTOVER 10.27 name;
// `npm run check:stroll` brings its own stroll now (a sandbox chapter 9 draft).
// The chapter 2 stroll (owner 2026-10-03, CUTOVER 10.27) on the LOCAL dev stack, as the local test user: each of the five
// stroll groups (scene-14 … scene-18) gets a "stroll-end" section in its script that its last slice starts when it scrolls
// into view – 15 minutes pass, at 7:00 the stroll ends (forced jump to scene-20), else the time shows and the reader picks
// "end the stroll" (scene-20) or "keep walking" (a random other stroll group). A stroll starts at 5:30: the first stroll
// group a reader enters sets it (and `strolling` = 1); arriving in scene-20 the time is 7:00 (owner 2026-10-03) and
// `strolling` = 0, so the next read-through strolls from 5:30 again. Re-runnable: removes what an earlier run added,
// keeps every other event. Saves drafts; `--publish` publishes them too.
//
//   node scripts/dev/stroll-chapter2.mjs [--publish]
import { readFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { local } from './ports.mjs';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const API = (process.env.SCUTOONS_API ?? local('api', '/api')).replace(/\/$/, '');
const { token } = JSON.parse(readFileSync(resolve(ROOT, 'logs/dev/test-user.json'), 'utf8'));
const PUBLISH = process.argv.includes('--publish');
const TOON = 'catpope';
const STROLLS = ['scene-14', 'scene-15', 'scene-16', 'scene-17', 'scene-18'];
const END = 'scene-20';
const LOCALES = ['de', 'en'];
const START = 330; // 5:30
const LIMIT = 420; // 7:00
const STEP = 15;

const TEXT = {
	de: { late: 'Schon {time:clock} – Zeit für die Arbeit!', now: 'Es ist {time:clock} Uhr.', stop: 'Spaziergang beenden', walk: 'Weiter spazieren' },
	en: { late: "It's {time:clock} already – time for work!", now: "It's {time:clock}.", stop: 'End the stroll', walk: 'Keep walking' }
};

async function call(method, path, body) {
	const res = await fetch(`${API}${path}`, {
		method,
		headers: { Authorization: `Bearer ${token}`, ...(body ? { 'Content-Type': 'application/json' } : {}) },
		body: body ? JSON.stringify(body) : undefined
	});
	const json = await res.json().catch(() => ({}));
	if (!res.ok) throw new Error(`${method} ${path} → ${res.status} ${JSON.stringify(json).slice(0, 200)}`);
	return json;
}
const draft = (key) => call('GET', `/entities/${key}?draft=1&wrap=1`);
async function save(key, content, base, message) {
	const r = await call('PUT', `/entities/${key}`, { content, base, message });
	if (PUBLISH) await call('POST', `/entities/${key}/publish`, {});
	return r;
}

const jump = (label) => ({ type: 'chapter', value: '2', label });

function strollSection(locale) {
	const t = TEXT[locale];
	return [
		{ type: 'divert', params: { to: 'end' } },
		{ type: 'label', params: { name: 'stroll-end' } },
		{ type: 'setVariable', params: { variable: 'time', value: STEP, op: 'add' } },
		{ type: 'showMessage', params: { text: t.late, wait: true }, if: { variable: 'time', op: '>=', value: LIMIT } },
		{ type: 'jumpTo', params: { target: jump(END) }, if: { variable: 'time', op: '>=', value: LIMIT } },
		{ type: 'showMessage', params: { text: t.now, wait: false } },
		{ type: 'addChoice', params: { label: t.stop, onSelect: { jumpTo: { target: jump(END) } } } },
		{ type: 'addChoice', params: { label: t.walk, onSelect: { jumpTo: { target: { type: 'chapter', value: '2', anyOf: STROLLS } } } } },
		{ type: 'showChoices' },
		{ type: 'label', params: { name: 'end' } }
	];
}
// a stroll starts once: when the reader is not strolling yet (never, or after scene-20)
const notStrolling = { variable: 'strolling', op: '!=', value: 1 };
const init = [
	{ type: 'setVariable', params: { variable: 'time', value: START }, if: notStrolling },
	{ type: 'setVariable', params: { variable: 'strolling', value: 1 }, if: notStrolling }
];
const arrive = [
	{ type: 'setVariable', params: { variable: 'time', value: LIMIT } },
	{ type: 'setVariable', params: { variable: 'strolling', value: 0 } }
];
// what earlier runs added (removed before adding the current form)
const OLD = [
	{ type: 'setVariable', params: { variable: 'time', value: START }, if: { variable: 'time', op: 'unset' } },
	{ type: 'setVariable', params: { variable: 'time', value: START } }
];
const same = (a, b) => JSON.stringify(a) === JSON.stringify(b);
const generated = (locale) => [...OLD, ...init, ...arrive, ...strollSection(locale)];
const strip = (events, locale) => (Array.isArray(events) ? events : []).filter((e) => !generated(locale).some((g) => same(g, e)));
const isStrollTrigger = (e) => e?.type === 'divert' && e.params?.to === 'stroll-end';

for (const scene of STROLLS) {
	for (const locale of LOCALES) {
		const key = `scene:${TOON}:2:${scene}:${locale}`;
		const { content, version } = await draft(key);
		content.events = [...init, ...strip(content.events, locale), ...strollSection(locale)];
		const slices = Array.isArray(content.slices) ? content.slices : [];
		for (const slice of slices) if (Array.isArray(slice.events)) slice.events = slice.events.filter((e) => !isStrollTrigger(e));
		const last = slices[slices.length - 1];
		if (last) last.events = [...(Array.isArray(last.events) ? last.events : []), { type: 'divert', params: { to: 'stroll-end' }, trigger: { type: 'intersect' } }];
		for (const slice of slices) if (Array.isArray(slice.events) && !slice.events.length) delete slice.events;
		await save(key, content, version?.id ?? null, 'the stroll (scripts/dev/stroll-chapter2.mjs)');
		console.log(`ok   ${key}: ${content.events.length} events, last slice ${slices.length} starts the stroll end`);
	}
}

for (const locale of LOCALES) {
	const key = `chapter:${TOON}:2:${locale}`;
	const { content, version } = await draft(key);
	const group = (content.groups ?? []).find((g) => g?.label === END);
	if (!group) throw new Error(`${key}: no group ${END}`);
	group.events = [...arrive, ...strip(group.events, locale)];
	await save(key, content, version?.id ?? null, `${END}: 7:00, the stroll is over`);
	console.log(`ok   ${key}: ${END} sets 7:00 and ends the stroll`);
}
console.log(PUBLISH ? 'published' : 'saved as drafts (add --publish to publish)');
