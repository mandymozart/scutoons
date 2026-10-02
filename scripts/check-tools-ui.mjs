#!/usr/bin/env node
// The brutalist tools' regression check (agents/DESIGN.md "Brutalist tools", owner 2026-10-01).
//
//   node scripts/check-tools-ui.mjs --static      no px font size (and no font size under caption in em) in the tools'
//                                                 CSS: creator, admin, embassy, @scutoons/ui (part of `npm run check`);
//                                                 no bold (owner 2026-10-02 "only use bold very rarely"): no bold
//                                                 font-weight in their CSS, no <b>, <strong> only with class sc-emphasis
//                                                 (a line carrying `bold-ok` + the reason is exempt: the page title h1)
//   node scripts/check-tools-ui.mjs               + the pages in headless Chrome (dev servers from `npm run dev`, the
//                                                 test user's session from `npm run dev:test-user`):
//                                                 - 1440 px, fine pointer: no visible text under the caption size
//                                                 - 390 px, touch (coarse pointer): the same + every button / field /
//                                                   nav entry at least 44 px high (the touch minimum)
//   --only=<name>  --list  --verbose   SCUTOONS_CREATOR / SCUTOONS_ADMIN / SCUTOONS_EMBASSY for other origins
//
// Exempt: comic lettering (Bangers, or text inside a slice: container-type inline-size, sized in cqw) and a CSS line
// carrying the comment `px-ok` (with the reason).
import { spawn } from 'node:child_process';
import { existsSync, mkdtempSync, readFileSync, readdirSync, rmSync, statSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { local } from './dev/ports.mjs';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const args = process.argv.slice(2);
const flag = (n) => args.includes(n);
const opt = (n) => args.find((a) => a.startsWith(`--${n}=`))?.split('=')[1];
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

// ── static: the CSS of the tools ────────────────────────────────────────────────────────────────────────────────
const DIRS = ['creator/src', 'admin/src', 'embassy/src', 'shared/ui/src', 'shared/auth/src', 'shared/tokens/tools.css'];
function walk(p) {
	const abs = join(ROOT, p);
	if (!existsSync(abs)) return [];
	if (statSync(abs).isFile()) return [abs];
	return readdirSync(abs, { withFileTypes: true }).flatMap((e) =>
		e.isDirectory() ? (e.name === 'paraglide' ? [] : walk(join(p, e.name))) : /\.(svelte|css)$/.test(e.name) ? [join(abs, e.name)] : []
	);
}
const problems = [];
const bolds = [];
const BOLD_CSS = /font-weight:\s*(?:[6-9]00|bold|bolder|var\(--font-weight-bold\))/;
const BOLD_TAG = /<b[\s>]|<strong(?![^>]*\bsc-emphasis\b)[\s>]/;
for (const file of DIRS.flatMap(walk)) {
	const text = readFileSync(file, 'utf8');
	const name = relative(ROOT, file).replace(/\\/g, '/');
	const style = file.endsWith('.css') ? text : (text.match(/<style[^>]*>[\s\S]*?<\/style>/g) ?? []).join('\n');
	const offset = file.endsWith('.css') ? 0 : text.indexOf(style.slice(0, 40));
	style.split('\n').forEach((line, i) => {
		const lineNo = offset >= 0 ? text.slice(0, offset).split('\n').length + i : i + 1;
		if (BOLD_CSS.test(line) && !/bold-ok/.test(line)) bolds.push(`${name}:${lineNo}: ${line.trim()}`);
		if (/px-ok/.test(line)) return;
		const px = line.match(/font-size:\s*([\d.]+)px/);
		const em = line.match(/font-size:\s*(0?\.\d+|0)(em|rem)\b/);
		if (px || (em && Number(em[1]) < 0.75)) problems.push(`${name}:${lineNo}: ${line.trim()}`);
	});
	if (file.endsWith('.svelte')) {
		// markup only (the style block blanked, line numbers kept)
		text.replace(/<style[^>]*>[\s\S]*?<\/style>/g, (m) => m.replace(/[^\n]/g, ' ')).split('\n').forEach((line, i) => {
			if (BOLD_TAG.test(line) && !/bold-ok/.test(line)) bolds.push(`${name}:${i + 1}: ${line.trim().slice(0, 120)}`);
		});
	}
}
if (problems.length) console.error(`tools CSS: ${problems.length} font sizes outside the type scale (use --type-* tokens; DESIGN.md "Brutalist tools"):\n  ${problems.join('\n  ')}`);
if (bolds.length) console.error(`tools: ${bolds.length} bold (owner 2026-10-02 "only use bold very rarely": regular weight; only the page title h1 and <strong class="sc-emphasis"> are bold; DESIGN.md "Brutalist tools"):\n  ${bolds.join('\n  ')}`);
if (problems.length || bolds.length) process.exit(1);
console.log('tools: no px font sizes, nothing under caption, no bold beyond the page title / sc-emphasis – ok');
if (flag('--static')) process.exit(0);

// ── browser ─────────────────────────────────────────────────────────────────────────────────────────────────────
const CREATOR = (process.env.SCUTOONS_CREATOR ?? local('creator')).replace(/\/$/, '');
const ADMIN = (process.env.SCUTOONS_ADMIN ?? local('admin')).replace(/\/$/, '');
const EMBASSY = (process.env.SCUTOONS_EMBASSY ?? local('embassy')).replace(/\/$/, '');
const PAGES = [
	['creator-home', `${CREATOR}/`],
	['creator-toon', `${CREATOR}/catpope`],
	['creator-season', `${CREATOR}/catpope/season/1`],
	['creator-chapter-meta', `${CREATOR}/catpope/chapter/1`],
	['creator-chapter-audio', `${CREATOR}/catpope/chapter/1?section=audio`],
	['creator-chapter-slice', `${CREATOR}/catpope/chapter/1?group=1&slice=3`],
	['creator-cast', `${CREATOR}/catpope/cast`],
	['creator-credits', `${CREATOR}/catpope/credits`],
	['creator-script', `${CREATOR}/catpope/script`],
	['creator-toon-settings', `${CREATOR}/catpope/toon`],
	['creator-characters', `${CREATOR}/characters`],
	['creator-moods', `${CREATOR}/characters?tab=moods`],
	['creator-wanted', `${CREATOR}/characters?tab=wanted`],
	['creator-guide', `${CREATOR}/guide`],
	['admin-dashboard', `${ADMIN}/`],
	['admin-codes', `${ADMIN}/codes`],
	['admin-code-new', `${ADMIN}/codes/new`],
	['admin-batches', `${ADMIN}/batches`],
	['admin-users', `${ADMIN}/users`],
	['admin-events', `${ADMIN}/events`],
	['admin-content', `${ADMIN}/content`],
	['admin-catalog', `${ADMIN}/catalog`],
	['admin-homepage', `${ADMIN}/homepage`],
	['admin-settings', `${ADMIN}/settings`],
	['embassy-today', `${EMBASSY}/`],
	['embassy-leads', `${EMBASSY}/leads`],
	['embassy-org-new', `${EMBASSY}/orgs/new`]
].filter(([name]) => !opt('only') || name.includes(opt('only')));
if (flag('--list')) {
	for (const [name, url] of PAGES) console.log(name.padEnd(24), url);
	process.exit(0);
}
const sessionFile = join(ROOT, 'logs/dev/test-user.json');
if (!existsSync(sessionFile)) {
	console.error('no test-user session: run `npm run dev:test-user` first');
	process.exit(2);
}
const token = JSON.parse(readFileSync(sessionFile, 'utf8')).token;

const CHROME = process.env.CHROME || ['C:/Program Files/Google/Chrome/Application/chrome.exe', 'C:/Program Files (x86)/Google/Chrome/Application/chrome.exe', 'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe', '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome', '/usr/bin/google-chrome'].find((p) => existsSync(p));
if (!CHROME) { console.error('Chrome not found (set CHROME)'); process.exit(2); }
const port = 9800 + Math.floor(Math.random() * 150);
const profile = mkdtempSync(join(tmpdir(), 'check-tools-ui-'));
const chrome = spawn(CHROME, ['--headless=new', `--remote-debugging-port=${port}`, `--user-data-dir=${profile}`, '--no-first-run', '--no-default-browser-check', '--disable-extensions', '--mute-audio', 'about:blank'], { stdio: 'ignore' });
async function done(code) {
	const exited = chrome.exitCode !== null ? Promise.resolve() : new Promise((r) => chrome.once('exit', r));
	chrome.kill();
	await Promise.race([exited, sleep(5000)]);
	try { rmSync(profile, { recursive: true, force: true, maxRetries: 10, retryDelay: 200 }); } catch {}
	process.exit(code);
}
let ws;
for (let i = 0; i < 50 && !ws; i++) {
	try { const u = (await (await fetch(`http://127.0.0.1:${port}/json/list`)).json()).find((t) => t.type === 'page')?.webSocketDebuggerUrl; if (u) ws = new WebSocket(u); } catch {}
	if (!ws) await sleep(200);
}
await new Promise((r) => ws.addEventListener('open', r));
let seq = 0; const pending = new Map(); let lastNet = Date.now();
ws.addEventListener('message', (e) => {
	const m = JSON.parse(e.data);
	if (m.id && pending.has(m.id)) { pending.get(m.id)(m); pending.delete(m.id); return; }
	if (m.method?.startsWith('Network.')) lastNet = Date.now();
});
const send = (method, params = {}) => new Promise((r) => { const i = ++seq; pending.set(i, r); ws.send(JSON.stringify({ id: i, method, params })); });
const evaluate = async (expression) => { const r = await send('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true }); return r.result?.result?.value; };
await send('Page.enable'); await send('Runtime.enable'); await send('Network.enable');
await send('Page.addScriptToEvaluateOnNewDocument', { source: `try { localStorage.setItem('scutoons_session', ${JSON.stringify(token)}); } catch {}` });
async function idle(quietMs = 700, maxMs = 8000) { const t0 = Date.now(); while (Date.now() - t0 < maxMs) { if (Date.now() - lastNet > quietMs) return; await sleep(100); } }

const INSPECT = `(async () => {
	await document.fonts.ready;
	const root = parseFloat(getComputedStyle(document.documentElement).fontSize);
	const caption = root * 0.75 - 0.05;
	const coarse = matchMedia('(pointer: coarse)').matches;
	const lettering = (el) => { for (let n = el; n && n !== document.body; n = n.parentElement) { const cs = getComputedStyle(n); if (/^\\s*["']?Bangers/.test(cs.fontFamily) || /inline-size|size/.test(cs.containerType)) return true; } return false; };
	const describe = (el) => el.tagName.toLowerCase() + ([...el.classList].filter((c) => !c.startsWith('svelte-')).map((c) => '.' + c).join('')) + ' "' + (el.value || el.getAttribute('aria-label') || el.textContent || '').trim().replace(/\\s+/g, ' ').slice(0, 30) + '"';
	const small = [], tiny = [];
	for (const el of document.body.querySelectorAll('*')) {
		if (/^(SCRIPT|STYLE|svg|path|g|title)$/.test(el.tagName)) continue;
		if (!el.getClientRects().length) continue;
		const cs = getComputedStyle(el);
		if (cs.visibility === 'hidden' || cs.display === 'none' || Number(cs.opacity) === 0) continue;
		const control = /^(INPUT|SELECT|TEXTAREA|BUTTON)$/.test(el.tagName) || el.classList.contains('sc-nav-item');
		const own = [...el.childNodes].some((n) => n.nodeType === 3 && n.textContent.trim());
		if ((own || control) && parseFloat(cs.fontSize) < caption && !lettering(el)) small.push(cs.fontSize + ' ' + describe(el));
		if (coarse && control && !['checkbox', 'radio', 'range', 'color', 'file', 'hidden'].includes(el.type)) {
			const r = el.getBoundingClientRect();
			if (r.height < 43.5 && r.width > 0 && !lettering(el)) tiny.push(Math.round(r.height) + 'px ' + describe(el));
		}
	}
	return { root, coarse, small, tiny };
})()`;

let failed = 0;
// desktop first: once touch emulation was on, Chrome keeps reporting a coarse pointer for the tab
for (const width of [1440, 390]) {
	for (const [name, url] of PAGES) {
		await send('Emulation.setDeviceMetricsOverride', { width, height: width < 768 ? 844 : 900, deviceScaleFactor: 1, mobile: width < 768 });
		await send('Emulation.setTouchEmulationEnabled', { enabled: width < 768, maxTouchPoints: width < 768 ? 5 : 0 });
		await send('Page.navigate', { url });
		await sleep(600); await idle(); await sleep(1200);
		const r = await evaluate(INSPECT);
		if (!r) { console.log(`✗ ${name} @${width}: page did not answer`); failed++; continue; }
		const bad = r.small.length + r.tiny.length;
		if (bad) failed++;
		console.log(`${bad ? '✗' : '✓'} ${name.padEnd(24)} @${width} root ${r.root}px${r.coarse ? ' coarse' : ''}: ${r.small.length} texts < caption, ${r.tiny.length} controls < 44px`);
		if (bad && (flag('--verbose') || bad <= 6)) for (const line of [...r.small, ...r.tiny].slice(0, 12)) console.log('    ' + line);
	}
}
console.log(failed ? `tools UI: ${failed} page views with problems` : 'tools UI: every page – ok');
await done(failed ? 1 : 0);
