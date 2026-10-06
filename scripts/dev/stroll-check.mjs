#!/usr/bin/env node
// The stroll (owner 2026-10-03, CUTOVER 10.27: random `anyOf` jumps between groups) played in headless Chrome (phone
// size, its own profile) on the local dev stack as the local test user. The chapter 2 stroll it was written for is gone
// from the content (production is the only source since CUTOVER 10.30), so the run brings its own: a stroll is saved as
// a DRAFT of sandbox chapter 9, read with the draft preview, and afterwards the draft rows and the test user's save
// game are put back as they were (the dev DB is as before).
//   - the stroll: five stroll groups; each one's last slice starts its "stroll-end" (a slice event diverting into the
//     group's script) → "Es ist H:MM Uhr." ({clock(time)}) + "Spaziergang beenden" / "Weiter spazieren"; "Weiter" adds
//     15 minutes and jumps to a random OTHER stroll group (`anyOf`, never the one the reader is on); at 7:00 the stroll
//     ends by itself ("Schon 7:00 …" → the home group); a stroll starts at 5:30, arriving home sets 7:00 / not strolling
//   - the first stroll group's own story (the old scene-14, owner 2026-10-03: the intro's choices leaked into the stroll
//     end): slice 1 diverts to the point "record-player" and parks the intro until it is answered (the page frozen), the
//     intro comes back, the mouse asks (two choices), "Ja" continues at "friends" (Cat Pope joins), then the stroll end
//     shows exactly its two choices
//
//   node scripts/dev/stroll-check.mjs        (viewer :8000, API :9000)
import { execFileSync, spawn } from 'node:child_process';
import { existsSync, mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { local } from './ports.mjs';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const VIEWER = (process.env.SCUTOONS_VIEWER ?? local('viewer')).replace(/\/$/, '');
const API = (process.env.SCUTOONS_API ?? local('api', '/api')).replace(/\/$/, '');
const DB = process.env.SCUTOONS_DB_NAME ?? 'scutoons';
const CONTAINER = process.env.SCUTOONS_DB_CONTAINER ?? 'scutoons-mariadb-1';
const { token, email } = JSON.parse(readFileSync(resolve(ROOT, 'logs/dev/test-user.json'), 'utf8'));
const KEY = 'chapter:sandbox:9';
const STROLLS = ['stroll-a', 'stroll-b', 'stroll-c', 'stroll-d', 'stroll-e'];
const HOME = 'stroll-home';
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

// ── the stroll ──────────────────────────────────────────────────────────────────────────────────────────────────
const START = 330; // 5:30
const LIMIT = 420; // 7:00
const STEP = 15;
const t = (de, en = de) => ({ de, en });
const IMG = (n) => `../catpope/s1/c2/s1c2-slice-${n}.webp`;
const slice = (n) => ({ image: IMG(n), width: 390, height: 625 });
const jump = (label) => ({ type: 'chapter', value: '9', label });
const msg = (text, wait = true) => ({ type: 'showMessage', params: { text: t(text), wait } });
const set = (variable, value, extra = {}) => ({ type: 'setVariable', params: { variable, value, ...extra } });
const notStrolling = { variable: 'strolling', op: '!=', value: 1 };
// a stroll starts once: when the reader is not strolling yet (never, or after the home group)
const init = [set('time', START, {}), set('strolling', 1)].map((e) => ({ ...e, if: notStrolling }));
const strollEnd = [
	{ type: 'divert', params: { to: 'end' } },
	{ type: 'label', params: { name: 'stroll-end' } },
	set('time', STEP, { op: 'add' }),
	{ type: 'showMessage', params: { text: t('Schon {clock(time)} – Zeit für die Arbeit!', "It's {clock(time)} already – time for work!"), wait: true }, if: { variable: 'time', op: '>=', value: LIMIT } },
	{ type: 'jumpTo', params: { target: jump(HOME) }, if: { variable: 'time', op: '>=', value: LIMIT } },
	{ type: 'showMessage', params: { text: t('Es ist {clock(time)} Uhr.', "It's {clock(time)}."), wait: false } },
	{ type: 'addChoice', params: { label: t('Spaziergang beenden', 'End the stroll'), onSelect: { jumpTo: { target: jump(HOME) } } } },
	{ type: 'addChoice', params: { label: t('Weiter spazieren', 'Keep walking'), onSelect: { jumpTo: { target: { type: 'chapter', value: '9', anyOf: STROLLS } } } } },
	{ type: 'showChoices' },
	{ type: 'label', params: { name: 'end' } }
];
/** the last slice starts the stroll end when it scrolls into view */
const strollSlices = (images) => images.map((n, i) => (i === images.length - 1 ? { ...slice(n), events: [{ type: 'divert', params: { to: 'stroll-end' }, trigger: { type: 'intersect' } }] } : slice(n)));
const group = (label, images, events) => ({ label, renderer: { type: 'vertical' }, slices: strollSlices(images), events });
const doc = (base) => ({
	...base,
	groups: [
		// the old scene-14: its own story first, then the stroll end
		{
			...group('stroll-a', [9, 10, 11], [
				...init,
				msg('INTRO Ein Laden voller Platten.'),
				msg('MAUS Kommst du mit?'),
				{ type: 'addChoice', params: { label: t('Ja'), onSelect: { divert: { to: 'friends' } } } },
				{ type: 'addChoice', params: { label: t('Nein'), onSelect: { divert: { to: 'end' } } } },
				{ type: 'showChoices' },
				{ type: 'label', params: { name: 'friends' } },
				{ type: 'joinScene', params: { characters: [{ character: 'catpope', persona: 'regular', look: 'default', mood: 'neutral', variant: 'avatar' }] } },
				msg('FREUNDE Los geht’s.', false),
				{ type: 'divert', params: { to: 'end' } },
				{ type: 'label', params: { name: 'record-player' } },
				msg('PLATTE Ein Plattenspieler!', false),
				{ type: 'addChoice', params: { label: t('Anhören', 'Listen') } },
				{ type: 'showChoices' },
				...strollEnd
			]),
			// slice 1 is in view at once: its trigger diverts to the record player and parks the intro until answered
			slices: [{ ...slice(9), events: [{ type: 'divert', params: { to: 'record-player' }, trigger: { type: 'intersect' } }] }, ...strollSlices([10, 11])]
		},
		group('stroll-b', [12, 13], [...init, ...strollEnd]),
		group('stroll-c', [14, 15], [...init, ...strollEnd]),
		group('stroll-d', [16, 17], [...init, ...strollEnd]),
		group('stroll-e', [18, 19], [...init, ...strollEnd]),
		// arriving home: 7:00, the stroll is over (the next one starts at 5:30 again)
		{ label: HOME, renderer: { type: 'vertical' }, slices: [slice(20)], events: [set('time', LIMIT), set('strolling', 0), msg('HOME', false)] }
	]
});

// ── API + DB ────────────────────────────────────────────────────────────────────────────────────────────────────
async function api(method, path, body) {
	const res = await fetch(`${API}${path}`, { method, headers: { Authorization: `Bearer ${token}`, 'Content-Type': 'application/json' }, body: body ? JSON.stringify(body) : undefined });
	const json = await res.json().catch(() => null);
	if (!res.ok) throw new Error(`${method} ${path}: ${res.status} ${JSON.stringify(json)}`);
	return json;
}
const sql = (query) => execFileSync('docker', ['exec', '-i', CONTAINER, 'sh', '-c', `mariadb -uroot -p"$MARIADB_ROOT_PASSWORD" -N -r ${DB}`], { input: query }).toString().trim();
const quote = (s) => (s === null ? 'NULL' : `'${String(s).replace(/\\/g, '\\\\').replace(/'/g, "\\'")}'`);
const userId = sql(`SELECT id FROM users WHERE email = ${quote(email)}`);
const progressBefore = sql(`SELECT CONCAT(updated_at, CHAR(9), state) FROM progress WHERE user_id = ${quote(userId)} AND toon_id = 'scutoons'`);
const before = await api('GET', `/entities/${KEY}?draft=1&wrap=1`);
const created = [];
async function restore() {
	if (created.length) {
		sql(`DELETE FROM entity_versions WHERE entity_key = ${quote(KEY)} AND id IN (${created.join(',')})`);
		const after = await api('GET', `/entities/${KEY}?draft=1&wrap=1`);
		const same = after.version.id === before.version.id && JSON.stringify(after.content) === JSON.stringify(before.content);
		console.log(`${same ? 'ok  ' : 'FAIL'} restored ${KEY} (newest version ${after.version.id}, was ${before.version.id})`);
		if (!same) process.exitCode = 1;
		created.length = 0;
	}
	if (progressBefore) {
		const tab = progressBefore.indexOf('\t');
		sql(`UPDATE progress SET state = ${quote(progressBefore.slice(tab + 1))}, updated_at = ${Number(progressBefore.slice(0, tab))} WHERE user_id = ${quote(userId)} AND toon_id = 'scutoons'`);
	} else sql(`DELETE FROM progress WHERE user_id = ${quote(userId)} AND toon_id = 'scutoons'`);
	const now = sql(`SELECT CONCAT(updated_at, CHAR(9), state) FROM progress WHERE user_id = ${quote(userId)} AND toon_id = 'scutoons'`);
	console.log(`${now === progressBefore ? 'ok  ' : 'FAIL'} restored the test user's save game (progress row scutoons)`);
	if (now !== progressBefore) process.exitCode = 1;
}
const saved = await api('PUT', `/entities/${KEY}`, { content: doc(before.content), base: before.version.id, message: 'stroll-check (removed again)' });
created.push(saved.version.id);

// ── headless Chrome ─────────────────────────────────────────────────────────────────────────────────────────────
const CHROME = process.env.CHROME || ['C:/Program Files/Google/Chrome/Application/chrome.exe', 'C:/Program Files (x86)/Google/Chrome/Application/chrome.exe', 'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe', '/usr/bin/google-chrome'].find((p) => existsSync(p));
if (!CHROME) throw new Error('no Chrome/Edge found (set CHROME)');
const port = 9800 + Math.floor(Math.random() * 150);
const profile = mkdtempSync(join(tmpdir(), 'stroll-'));
const chrome = spawn(CHROME, ['--headless=new', `--remote-debugging-port=${port}`, `--user-data-dir=${profile}`, '--no-first-run', '--disable-extensions', '--window-size=390,844', 'about:blank'], { stdio: 'ignore' });
const cleanup = () => {
	try { chrome.kill(); } catch {}
	try { rmSync(profile, { recursive: true, force: true, maxRetries: 10, retryDelay: 200 }); } catch {}
};
let wsUrl;
for (let i = 0; i < 50 && !wsUrl; i++) {
	try { wsUrl = (await (await fetch(`http://127.0.0.1:${port}/json/list`)).json()).find((x) => x.type === 'page')?.webSocketDebuggerUrl; } catch {}
	if (!wsUrl) await sleep(200);
}
const ws = new WebSocket(wsUrl);
await new Promise((r) => ws.addEventListener('open', r));
let seq = 0;
const pending = new Map();
const errors = [];
ws.addEventListener('message', (e) => {
	const m = JSON.parse(e.data);
	if (m.id && pending.has(m.id)) return pending.get(m.id)(m), pending.delete(m.id);
	if (m.method === 'Runtime.exceptionThrown') errors.push(m.params.exceptionDetails.exception?.description ?? m.params.exceptionDetails.text);
});
const send = (method, params = {}) => new Promise((r) => { const i = ++seq; pending.set(i, r); ws.send(JSON.stringify({ id: i, method, params })); });
async function js(expression) {
	const r = await send('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true });
	if (r.result?.exceptionDetails) throw new Error(r.result.exceptionDetails.exception?.description);
	return r.result?.result?.value;
}
async function until(expression, ms = 15000) {
	const end = Date.now() + ms;
	while (Date.now() < end) {
		const v = await js(expression).catch(() => null);
		if (v) return v;
		await sleep(200);
	}
	return null;
}
const results = [];
const check = (ok, what, detail = '') => (results.push(!!ok), console.log(`${ok ? 'ok  ' : 'FAIL'} ${what}${!ok && detail ? `  — ${detail}` : ''}`));

const PAGE = `${VIEWER}/sandbox/chapter/9`;
const label = () => js(`new URLSearchParams(location.search).get('label')`);
/** scrolls the group's slices one by one (the last one starts the stroll end), returns the message + choices */
async function reachEnd() {
	await until(`document.querySelectorAll('.slice').length > 0`);
	await sleep(800);
	const n = await js(`document.querySelectorAll('.slice').length`);
	for (let i = 0; i < n; i++) {
		// the reader answers what the story asks on the way (the page doesn't scroll meanwhile): a choice → the first, a
		// waited message → its click
		for (let k = 0; k < 6; k++) {
			const asks = await js(`(() => { const t = document.querySelector('.vn-message .text')?.innerText ?? ''; const c = document.querySelectorAll('.vn-choice').length; return /\\d:\\d\\d/.test(t) ? null : c ? 'choice' : document.querySelector('.vn-message.visible .continue-overlay') ? 'click' : null; })()`);
			if (!asks) break;
			await js(asks === 'choice' ? `document.querySelectorAll('.vn-choice')[0].click(), true` : `document.querySelector('.continue-overlay').click(), true`);
			await sleep(400);
		}
		await js(`document.querySelectorAll('.slice')[${i}].scrollIntoView({ block: 'center' }), true`);
		await sleep(500);
	}
	return until(`(() => { const t = document.querySelector('.vn-message.visible .text')?.innerText ?? ''; const c = [...document.querySelectorAll('.vn-choice')].map((b) => b.innerText.trim()); return /\\d:\\d\\d/.test(t) ? { text: t.trim(), choices: c } : null; })()`, 10000);
}
/** a known start: arriving home ends any stroll (7:00, not strolling) – the next stroll starts at 5:30 */
async function home() {
	await send('Page.navigate', { url: `${PAGE}?label=${HOME}` });
	await until(`document.querySelector('.vn-message.visible .text')?.innerText.trim() === 'HOME'`, 15000);
	await sleep(500);
}

try {
	await send('Runtime.enable');
	await send('Page.enable');
	await send('Emulation.setDeviceMetricsOverride', { width: 390, height: 844, deviceScaleFactor: 2, mobile: true });
	await send('Page.navigate', { url: `${VIEWER}/` });
	await until('document.readyState === "complete"');
	await js(`localStorage.setItem('scutoons_session', ${JSON.stringify(token)})`);
	await js(`localStorage.setItem('scutoons.draft-preview', '1')`);
	// a fresh reader: no save game yet (one for every toon – other toons' variables would be there too; put back afterwards)
	sql(`DELETE FROM progress WHERE user_id = ${quote(userId)} AND toon_id = 'scutoons'`);

	await home();
	// the first stroll group's own story: slice 1 diverts to "record-player" and parks the intro until it is answered
	await send('Page.navigate', { url: `${PAGE}?label=stroll-a` });
	const record = await until(`(() => { const c = [...document.querySelectorAll('.vn-choice')].map((b) => b.innerText.trim()); return c.length ? c : null; })()`, 15000);
	check(record?.length === 1, `slice 1 → the record player (point "record-player"): ${JSON.stringify(record)}`);
	const frozen = await js(`(() => { const y = scrollY; scrollBy(0, 400); return { overflow: document.documentElement.style.overflow, moved: scrollY !== y }; })()`);
	check(frozen.overflow === 'hidden', `the page is frozen while the question is open (overflow ${frozen.overflow})`);
	await js(`document.querySelectorAll('.vn-choice')[0].click(), true`);
	const intro = await until(`document.querySelector('.vn-message.visible .continue-overlay') ? document.querySelector('.vn-message .text')?.innerText.trim() : null`, 8000);
	check(/^INTRO/.test(intro ?? ''), `the intro comes back after the record player: ${intro}`);
	await js(`document.querySelector('.continue-overlay')?.click(), true`);
	await sleep(400);
	// the mouse asks (a waited message, then its two choices)
	await js(`document.querySelector('.vn-message.visible .continue-overlay')?.click(), true`);
	const asked = await until(`(() => { const c = document.querySelectorAll('.vn-choice'); return c.length ? { text: document.querySelector('.vn-message .text')?.innerText.trim(), n: c.length } : null; })()`, 8000);
	check(asked?.n === 2, `the mouse asks, 2 choices (${asked?.text ?? '–'})`);
	const onStage = await js(`document.querySelectorAll('.character-portrait, .character-full').length`);
	await js(`document.querySelectorAll('.vn-choice')[0].click(), true`);
	const joined = await until(`(() => { const n = document.querySelectorAll('.character-portrait, .character-full').length; return n > ${onStage} ? n : null; })()`, 5000);
	check(!!joined, `Ja → continues at "friends": Cat Pope joins (${onStage} → ${joined ?? onStage} on stage)`);
	const endA = await reachEnd();
	check(endA?.choices.length === 2, `the first group's stroll end has only its 2 choices: ${JSON.stringify(endA?.choices)}`);

	// the walk: from home to stroll-b, keep walking until the stroll ends by itself
	await home();
	await send('Page.navigate', { url: `${PAGE}?label=stroll-b` });
	await until(`new URLSearchParams(location.search).get('label') === 'stroll-b'`);
	const visited = ['stroll-b'];
	let last = null;
	const times = [];
	for (let step = 0; step < 10; step++) {
		const here = await label();
		const end = await reachEnd();
		if (!end) {
			check(false, `${here}: the stroll end shows at the last slice`, 'no message with a time');
			break;
		}
		times.push(end.text.match(/\d+:\d\d/)?.[0]);
		if (/Zeit für die Arbeit|time for work/.test(end.text)) {
			check(true, `${here}: ${end.text} (forced end)`);
			await js(`document.querySelector('.continue-overlay')?.click(), true`);
			const arrived = await until(`new URLSearchParams(location.search).get('label') === '${HOME}' ? '${HOME}' : null`, 8000);
			check(arrived === HOME, 'the forced end arrives home');
			last = end.text;
			break;
		}
		const hasBoth = end.choices.some((c) => /beenden|End/.test(c)) && end.choices.some((c) => /Weiter|Keep/.test(c));
		check(hasBoth, `${here}: "${end.text}" + ${end.choices.join(' / ')}`);
		const from = await label();
		await js(`[...document.querySelectorAll('.vn-choice')].find((b) => /Weiter|Keep/.test(b.innerText))?.click(), true`);
		const next = await until(`(() => { const l = new URLSearchParams(location.search).get('label'); return l && l !== ${JSON.stringify(from)} ? l : null; })()`, 8000);
		check(!!next && STROLLS.includes(next) && next !== from, `keep walking → a random other stroll group (${from} → ${next})`);
		visited.push(next);
		last = end.text;
	}
	check(/7:00/.test(last ?? ''), `the stroll ends at 7:00 (last: ${last})`);
	check(times[0] === '5:45', `the stroll starts at 5:30 (the first stroll end shows ${times[0]})`);
	check(errors.length === 0, 'no page errors', errors.slice(0, 3).join(' | '));
	console.log(`visited: ${visited.join(' → ')}`);
} catch (error) {
	check(false, 'the run', String(error?.stack ?? error));
} finally {
	await sleep(1000);
	await restore().catch((error) => check(false, 'restore', String(error)));
	cleanup();
}
const bad = results.filter((r) => !r).length;
console.log(bad ? `stroll-check: ${bad} failed` : `stroll-check: all ${results.length} passed`);
process.exit(bad || process.exitCode ? 1 : 0);
