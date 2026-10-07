#!/usr/bin/env node
// VN events in linked groups (owner 2026-10-06, agents/DATA.md "Linked groups"), played in headless Chrome (phone size)
// on the local dev stack as the local test user: a test chain of three groups with scripts, a scroll trigger and a text
// box revealed by a script is saved as a DRAFT of sandbox chapter 8, read with the draft preview, and the draft rows are
// removed again afterwards (the dev DB is as before – nothing else is touched).
//   - scripts start when their group's first slice is seen, one after another; the page is frozen (pause mark under the
//     logo) while anything waits; the follow-up bar only when everything is through
//   - `?label=` into the middle: the groups above count as read (no script, text boxes shown, scrolling up works)
//   - a choice jumping back into the same chain starts that page again
//
//   node scripts/dev/chain-vn-check.mjs        (viewer :8000, API :9000, MariaDB container)
import { execFileSync, spawn } from 'node:child_process';
import { existsSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { local } from './ports.mjs';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const VIEWER = (process.env.SCUTOONS_VIEWER ?? local('viewer')).replace(/\/$/, '');
const API = (process.env.SCUTOONS_API ?? local('api', '/api')).replace(/\/$/, '');
const DB = process.env.SCUTOONS_DB_NAME ?? 'scutoons';
const CONTAINER = process.env.SCUTOONS_DB_CONTAINER ?? 'scutoons-mariadb-1';
const { token } = JSON.parse(readFileSync(resolve(ROOT, 'logs/dev/test-user.json'), 'utf8'));
const KEY = 'chapter:sandbox:8';
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

// ── the test chain ──────────────────────────────────────────────────────────────────────────────────────────────
const t = (de, en = de) => ({ de, en });
const slice = (n, dialog) => ({ image: `../catpope/s1/c2/s1c2-slice-${n}.webp`, width: 390, height: 625, ...(dialog ? { dialog } : {}) });
const msg = (text, wait = true) => ({ type: 'showMessage', params: { text: t(text), wait } });
const chain = (base) => ({
	...base,
	groups: [
		{
			label: 'vn-chain-a',
			continues: true,
			renderer: { type: 'vertical' },
			slices: [slice(9), slice(10, [{ id: 'vn-chain-a-box', rect: { x: 10, y: 10, width: 60, height: 20 }, text: t('BOX-A'), variant: 'caption', size: 4 }])],
			events: [
				{ type: 'joinScene', params: { characters: [{ character: 'catpope', persona: 'regular', look: 'default', mood: 'neutral', variant: 'avatar' }] } },
				msg('A1'),
				{ type: 'showTextBox', params: { dialog: 'vn-chain-a-box' } },
				msg('A2', false)
			]
		},
		{
			label: 'vn-chain-b',
			continues: true,
			renderer: { type: 'vertical' },
			slices: [slice(11, undefined), slice(12)].map((s, i) => (i === 0 ? { ...s, events: [{ ...msg('B-trigger'), trigger: { type: 'intersect' } }] } : s)),
			events: [msg('B1')]
		},
		{
			label: 'vn-chain-c',
			renderer: { type: 'vertical' },
			slices: [slice(13)],
			events: [
				msg('C1'),
				{ type: 'addChoice', params: { label: t('stay'), onSelect: { set: { variable: 'vn_chain_check', value: 'stayed' } } } },
				{ type: 'addChoice', params: { label: t('jump'), onSelect: { jumpTo: { target: { type: 'chapter', value: '8', label: 'vn-chain-b' } } } } },
				{ type: 'showChoices' },
				msg('C-end')
			]
		}
	]
});

// ── API + DB ────────────────────────────────────────────────────────────────────────────────────────────────────
async function api(method, path, body) {
	const res = await fetch(`${API}${path}`, { method, headers: { Authorization: `Bearer ${token}`, 'Content-Type': 'application/json' }, body: body ? JSON.stringify(body) : undefined });
	const json = await res.json().catch(() => null);
	if (!res.ok) throw new Error(`${method} ${path}: ${res.status} ${JSON.stringify(json)}`);
	return json;
}
function sql(query) {
	return execFileSync('docker', ['exec', CONTAINER, 'sh', '-c', `mariadb -uroot -p"$MARIADB_ROOT_PASSWORD" -N ${DB} -e "${query}"`]).toString().trim();
}
// a run that died before its restore (a hung Docker, Ctrl+C) left its draft: it goes first (else this run's identical
// save is "unchanged", returns that draft and the restore removes it – reported as a failed restore)
sql(`DELETE FROM entity_versions WHERE entity_key = '${KEY}' AND status = 'draft' AND message = 'chain-vn-check (removed again)'`);
const before = await api('GET', `/entities/${KEY}?draft=1&wrap=1`);
const created = [];
async function restore() {
	if (!created.length) return;
	// the test drafts go again: the entity's newest version is the one it had before
	sql(`DELETE FROM entity_versions WHERE entity_key = '${KEY}' AND id IN (${created.join(',')})`);
	const after = await api('GET', `/entities/${KEY}?draft=1&wrap=1`);
	const same = after.version.id === before.version.id && JSON.stringify(after.content) === JSON.stringify(before.content);
	console.log(`${same ? 'ok  ' : 'FAIL'} restored ${KEY} (newest version ${after.version.id}, was ${before.version.id})`);
	if (!same) process.exitCode = 1;
	created.length = 0;
}
const saved = await api('PUT', `/entities/${KEY}`, { content: chain(before.content), base: before.version.id, message: 'chain-vn-check (removed again)' });
created.push(saved.version.id);

// ── headless Chrome ─────────────────────────────────────────────────────────────────────────────────────────────
const CHROME = process.env.CHROME || ['C:/Program Files/Google/Chrome/Application/chrome.exe', 'C:/Program Files (x86)/Google/Chrome/Application/chrome.exe', 'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe', '/usr/bin/google-chrome'].find((p) => existsSync(p));
if (!CHROME) throw new Error('no Chrome/Edge found (set CHROME)');
const port = 9800 + Math.floor(Math.random() * 150);
const profile = mkdtempSync(join(tmpdir(), 'chain-vn-'));
const chrome = spawn(CHROME, ['--headless=new', `--remote-debugging-port=${port}`, `--user-data-dir=${profile}`, '--no-first-run', '--disable-extensions', '--window-size=375,812', 'about:blank'], { stdio: 'ignore' });
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
async function until(expression, ms = 10000) {
	const end = Date.now() + ms;
	while (Date.now() < end) {
		const v = await js(expression).catch(() => null);
		if (v) return v;
		await sleep(150);
	}
	return null;
}
const results = [];
const check = (ok, what, detail = '') => (results.push(!!ok), console.log(`${ok ? 'ok  ' : 'FAIL'} ${what}${!ok && detail ? `  — ${detail}` : ''}`));

/** what the page shows: the box's text, choices, frozen, the pause mark, the follow-up bar */
const STATE = `(() => ({
	text: document.querySelector('.vn-message.visible .text')?.innerText.trim() ?? null,
	waiting: !!document.querySelector('.vn-message.visible .continue-overlay'),
	choices: [...document.querySelectorAll('.vn-choice')].map((b) => b.innerText.trim()),
	frozen: document.documentElement.style.overflow === 'hidden',
	paused: !!document.querySelector('[data-testid="scroll-paused"]'),
	bar: !!document.querySelector('.next-host.shown, [data-testid="next-bar"]'),
	box: [...document.querySelectorAll('.slice')].some((s) => s.innerText.includes('BOX-A')),
	label: new URLSearchParams(location.search).get('label')
}))()`;
const state = () => js(STATE);
const waitText = (text, ms) => until(`(${STATE}).text === ${JSON.stringify(text)} ? (${STATE}) : null`, ms);
/** the speaker's feet against the shown box's top (px; the characters stand on the box – also on the end actions) */
const FEET = `(() => { const box = document.querySelector('.vn-message.visible'); const p = document.querySelector('.character-portrait'); return box && p ? Math.round(p.getBoundingClientRect().bottom - box.getBoundingClientRect().top) : null; })()`;
const click = () => js(`document.querySelector('.vn-message.visible .continue-overlay')?.click(), true`);
const scrollTo = (index) => js(`document.querySelector('[data-slice-index="${index}"]')?.scrollIntoView({ block: 'center' }), true`);

try {
	await send('Runtime.enable');
	await send('Page.enable');
	await send('Emulation.setDeviceMetricsOverride', { width: 375, height: 812, deviceScaleFactor: 2, mobile: true });
	await send('Page.navigate', { url: `${VIEWER}/` });
	await until('document.readyState === "complete"');
	await js(`localStorage.setItem('scutoons_session', ${JSON.stringify(token)})`);
	await js(`localStorage.setItem('scutoons.draft-preview', '1')`);

	// 1 – from the chain's top: A starts when its first slice is seen, the page is frozen with the pause mark
	await send('Page.navigate', { url: `${VIEWER}/sandbox/chapter/8` });
	let s = await waitText('A1', 15000);
	check(s, 'chain top: script A starts (its first slice is seen)', JSON.stringify(await state()));
	check(s?.frozen && s?.paused, 'frozen with the pause mark while A1 waits for its click', JSON.stringify(s));
	check(!s?.box, 'the box A reveals is hidden before its event');
	await sleep(800);
	const onMessage = await js(FEET);
	// CHAIN_VN_SHOT=<file.png>: a phone screenshot of the frozen page (the pause mark under the logo)
	if (process.env.CHAIN_VN_SHOT) writeFileSync(process.env.CHAIN_VN_SHOT, Buffer.from((await send('Page.captureScreenshot', { format: 'png' })).result.data, 'base64'));
	const y0 = await js('scrollY');
	await send('Input.dispatchMouseEvent', { type: 'mouseWheel', x: 180, y: 400, deltaX: 0, deltaY: 600 });
	await sleep(300);
	check((await js('scrollY')) === y0, 'the wheel does not scroll while frozen');
	await click();
	s = await waitText('A2', 5000);
	await sleep(400);
	s = await state();
	check(s?.text === 'A2' && !s.frozen && !s.paused, 'A2 (no wait) ends script A: not frozen, no pause mark', JSON.stringify(s));
	check(s?.box, 'script A revealed its text box');
	check(!s?.bar, 'no follow-up bar yet');

	// 2 – group B: its scroll trigger and its script, one after another (the trigger first – it fires on the slice)
	await scrollTo(2);
	const first = await until(`(() => { const s = ${STATE}; return s.waiting && (s.text === 'B-trigger' || s.text === 'B1') ? s : null; })()`, 8000);
	check(first?.frozen, `group B: ${first?.text} shows, frozen`, JSON.stringify(first));
	await click();
	const second = await until(`(() => { const s = ${STATE}; return s.waiting && (s.text === 'B-trigger' || s.text === 'B1') && s.text !== ${JSON.stringify(first?.text)} ? s : null; })()`, 8000);
	check(second && second.text !== first?.text, `then ${second?.text} (the other one waited in line)`, JSON.stringify(await state()));
	await click();
	await sleep(500);
	s = await state();
	check(!s.frozen && !s.paused, 'B through: scrolling again', JSON.stringify(s));

	// 3 – group C (the chain's last): C1, the question, "stay" → C-end; the follow-up bar only after it
	await scrollTo(4);
	s = await waitText('C1', 8000);
	check(s?.frozen, 'group C: C1, frozen', JSON.stringify(await state()));
	await click();
	s = await until(`(${STATE}).choices.length === 2 ? (${STATE}) : null`, 5000);
	check(s?.frozen && !s.bar, 'the question is open: frozen, no follow-up bar', JSON.stringify(s));
	await js(`[...document.querySelectorAll('.vn-choice')].find((b) => b.innerText.trim() === 'stay')?.click(), true`);
	s = await waitText('C-end', 5000);
	check(s?.frozen, 'stay → C-end', JSON.stringify(await state()));
	check(!(await state()).bar, 'no follow-up bar while C-end waits');
	await click();
	await sleep(300);
	await js('scrollTo(0, document.documentElement.scrollHeight), true');
	s = await until(`(${STATE}).bar ? (${STATE}) : null`, 6000);
	check(s?.bar && !s.frozen, 'everything through: the follow-up bar shows', JSON.stringify(await state()));
	// BACKLOG 10.48: the end actions are choices of the message box, the scene's characters stand on it
	await sleep(800);
	const end = await js(`(() => { const bar = document.querySelector('.vn-message.visible [data-testid="next-bar"]'); return { inBox: !!bar, choices: bar ? bar.querySelectorAll('.vn-end-choice').length : 0, feet: ${FEET}, onMessage: ${JSON.stringify(onMessage)} }; })()`);
	check(end?.inBox && end.choices >= 1 && end.feet !== null && end.onMessage !== null && Math.abs(end.feet - end.onMessage) <= 2, 'the end actions are choices in the message box, the character stands on it as on a message', JSON.stringify(end));

	// "Nochmal": the chain from its top – every script again, the stage cleared, A's box hidden again
	await js(`document.querySelector('[data-testid="next-bar-replay"]')?.click(), true`);
	s = await waitText('A1', 8000);
	check(s && !s.box && !s.bar, '"Nochmal" plays A again from the top (its box hidden again)', JSON.stringify(await state()));
	await click();
	await sleep(300);

	// 4 – `?label=` into the middle: A counts as read (no script, its box shown), B runs; scrolling up works
	await send('Page.navigate', { url: `${VIEWER}/sandbox/chapter/8?label=vn-chain-b` });
	s = await until(`(() => { const s = ${STATE}; return s.text === 'B-trigger' || s.text === 'B1' ? s : null; })()`, 15000);
	check(s, `deep link to B: B's story starts (${s?.text})`, JSON.stringify(await state()));
	// a FULL page load scrolls to B too (the page may mount after the navigation ended: it waits for B's first slice)
	const atB = await until(`(() => { const el = document.querySelector('.chapter-view [data-slice-index="2"]'); if (!el) return null; const top = Math.round(el.getBoundingClientRect().top); return Math.abs(top) <= 1 && scrollY > 0 ? { top, y: Math.round(scrollY) } : null; })()`, 5000);
	check(atB, `deep link (full load): B's first slice at the top (${JSON.stringify(atB)})`);
	const sawA = await js(`[...document.querySelectorAll('.vn-message .text')].some((t) => /^A[12]$/.test(t.innerText.trim()))`);
	check(!sawA, 'script A (above) does not run');
	check((await state()).box, "A's text box is shown as plain content");
	await click();
	await until(`(${STATE}).waiting && (${STATE}).text !== ${JSON.stringify(s?.text)}`, 5000);
	await click();
	await sleep(500);
	await js('scrollTo(0, 0), true');
	await sleep(300);
	check((await js('scrollY')) === 0 && !(await state()).frozen, 'scrolling back up to A works, nothing starts there');

	// 5 – a choice jumping back into the same chain starts that page again (B from the start)
	await scrollTo(4);
	await waitText('C1', 8000);
	await click();
	await until(`(${STATE}).choices.length === 2`, 5000);
	await js(`[...document.querySelectorAll('.vn-choice')].find((b) => b.innerText.trim() === 'jump')?.click(), true`);
	s = await until(`(() => { const s = ${STATE}; return s.waiting && (s.text === 'B-trigger' || s.text === 'B1') ? s : null; })()`, 10000);
	check(s?.label === 'vn-chain-b', `the jump into the same chain plays B again (${s?.text})`, JSON.stringify(await state()));

	check(errors.length === 0, 'no page errors', errors.slice(0, 3).join(' | '));
} catch (error) {
	check(false, 'the run', String(error?.stack ?? error));
} finally {
	await restore().catch((error) => check(false, 'restore', String(error)));
	cleanup();
}
const bad = results.filter((r) => !r).length;
console.log(bad ? `chain-vn-check: ${bad} failed` : `chain-vn-check: all ${results.length} passed`);
process.exit(bad || process.exitCode ? 1 : 0);
