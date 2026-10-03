#!/usr/bin/env node
// Plays the chapter 2 stroll (scripts/dev/stroll-chapter2.mjs, CUTOVER 10.27) in headless Chrome on the local dev stack as
// the local test user: from scene-15, every stroll group's last slice starts its "stroll-end" (a slice event diverting
// into the group's script) → "Es ist H:MM Uhr." + "Spaziergang beenden" / "Weiter spazieren"; "Weiter" adds 15 minutes
// and jumps to a random OTHER stroll group; at 7:00 the stroll ends by itself ("Schon 7:00 …" → scene-20).
//
//   node scripts/dev/stroll-check.mjs        (viewer :8000, API :9000)
import { spawn } from 'node:child_process';
import { existsSync, mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { local } from './ports.mjs';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const VIEWER = (process.env.SCUTOONS_VIEWER ?? local('viewer')).replace(/\/$/, '');
const { token } = JSON.parse(readFileSync(resolve(ROOT, 'logs/dev/test-user.json'), 'utf8'));
const STROLLS = ['scene-14', 'scene-15', 'scene-16', 'scene-17', 'scene-18'];
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

const CHROME = process.env.CHROME || ['C:/Program Files/Google/Chrome/Application/chrome.exe', 'C:/Program Files (x86)/Google/Chrome/Application/chrome.exe', 'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe', '/usr/bin/google-chrome'].find((p) => existsSync(p));
if (!CHROME) throw new Error('no Chrome/Edge found (set CHROME)');
const port = 9800 + Math.floor(Math.random() * 150);
const profile = mkdtempSync(join(tmpdir(), 'stroll-'));
const chrome = spawn(CHROME, ['--headless=new', `--remote-debugging-port=${port}`, `--user-data-dir=${profile}`, '--no-first-run', '--disable-extensions', '--window-size=390,844', 'about:blank'], { stdio: 'ignore' });
process.on('exit', () => {
	try { chrome.kill(); } catch {}
	try { rmSync(profile, { recursive: true, force: true, maxRetries: 10, retryDelay: 200 }); } catch {}
});
let wsUrl;
for (let i = 0; i < 50 && !wsUrl; i++) {
	try { wsUrl = (await (await fetch(`http://127.0.0.1:${port}/json/list`)).json()).find((t) => t.type === 'page')?.webSocketDebuggerUrl; } catch {}
	if (!wsUrl) await sleep(200);
}
const ws = new WebSocket(wsUrl);
await new Promise((r) => ws.addEventListener('open', r));
let seq = 0;
const pending = new Map();
const errors = [];
ws.addEventListener('message', (e) => {
	const msg = JSON.parse(e.data);
	if (msg.id && pending.has(msg.id)) return pending.get(msg.id)(msg), pending.delete(msg.id);
	if (msg.method === 'Runtime.exceptionThrown') errors.push(msg.params.exceptionDetails.exception?.description ?? msg.params.exceptionDetails.text);
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
const check = (ok, what, detail = '') => (results.push(ok), console.log(`${ok ? 'ok  ' : 'FAIL'} ${what}${!ok && detail ? `  — ${detail}` : ''}`));

await send('Runtime.enable');
await send('Page.enable');
await send('Page.navigate', { url: `${VIEWER}/` });
await until('document.readyState === "complete"');
await js(`localStorage.setItem('scutoons_session', ${JSON.stringify(token)})`);

const label = () => js(`new URLSearchParams(location.search).get('label')`);
/** scrolls the group's slices one by one (the last one starts the stroll end), returns the message + choices */
async function reachEnd() {
	await until(`document.querySelectorAll('.slice').length > 0`);
	await sleep(800);
	const n = await js(`document.querySelectorAll('.slice').length`);
	for (let i = 0; i < n; i++) {
		await js(`document.querySelectorAll('.slice')[${i}].scrollIntoView({ block: 'center' }), true`);
		await sleep(500);
	}
	return until(`(() => { const t = document.querySelector('.vn-message.visible .text')?.innerText ?? ''; const c = [...document.querySelectorAll('.vn-choice')].map((b) => b.innerText.trim()); return /\\d:\\d\\d/.test(t) ? { text: t.trim(), choices: c } : null; })()`, 10000);
}

// a known start: arriving in scene-20 ends any stroll (7:00, not strolling) – the next stroll starts at 5:30
await send('Page.navigate', { url: `${VIEWER}/catpope/chapter/2?label=scene-20` });
await until(`document.querySelectorAll('.slice').length > 0`);
await sleep(1500);
await send('Page.navigate', { url: `${VIEWER}/catpope/chapter/2?label=scene-15` });
await until(`new URLSearchParams(location.search).get('label') === 'scene-15'`);
let visited = ['scene-15'];
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
		const arrived = await until(`new URLSearchParams(location.search).get('label') === 'scene-20' ? 'scene-20' : null`, 8000);
		check(arrived === 'scene-20', 'the forced end arrives in scene-20');
		last = end.text;
		break;
	}
	const hasBoth = end.choices.some((c) => /beenden|End/.test(c)) && end.choices.some((c) => /Weiter|Keep/.test(c));
	check(hasBoth, `${here}: "${end.text}" + ${end.choices.join(' / ')}`);
	const before = await label();
	await js(`[...document.querySelectorAll('.vn-choice')].find((b) => /Weiter|Keep/.test(b.innerText))?.click(), true`);
	const next = await until(`(() => { const l = new URLSearchParams(location.search).get('label'); return l && l !== ${JSON.stringify(before)} ? l : null; })()`, 8000);
	check(!!next && STROLLS.includes(next) && next !== before, `keep walking → a random other stroll group (${before} → ${next})`);
	visited.push(next);
	last = end.text;
}
check(/7:00/.test(last ?? ''), `the stroll ends at 7:00 (last: ${last})`);
check(times[0] === '5:45', `the stroll starts at 5:30 (first stroll end shows ${times[0]})`);
check(errors.length === 0, 'no page errors', errors.slice(0, 3).join(' | '));
console.log(`visited: ${visited.join(' → ')}`);
const bad = results.filter((r) => !r).length;
console.log(bad ? `stroll-check: ${bad} failed` : `stroll-check: all ${results.length} passed`);
process.exit(bad ? 1 : 0);
