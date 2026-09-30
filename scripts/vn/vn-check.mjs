// Plays a toon's VN groups in headless Chrome (own temp profile, CDP, no dependencies) and checks the VN layout
// (agents/DESIGN.md → "VN layout") and the one-click rule:
//   - one reader click per waited message, the last one included (the click on the last message jumps)
//   - `avatar` characters = the left slot of the dialogue box while speaking, `portrait` ones on the right stage
//   - portrait boxes don't overlap (unless the stage is crowded = stacked, stage.ts), a character further back stands
//     further right, the speaker glows, no mood text
// usage: node scripts/vn/vn-check.mjs [--base=http://localhost:8000] [--toon=sandbox] [--locale=de] [--width=375]
//        [--height=812] [--from=1] [--to=<last chapter>] [--shots=<dir>] [--json=<file>]
// Needs the dev viewer (and API / CDN) running. Exit code 1 on any failed check.
import { spawn } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const opt = Object.fromEntries(process.argv.slice(2).filter((a) => a.startsWith('--')).map((a) => { const [k, v] = a.slice(2).split('='); return [k, v ?? true]; }));
const base = String(opt.base || 'http://localhost:8000').replace(/\/$/, '');
const toon = opt.toon || 'sandbox';
const locale = opt.locale || 'de';
const width = Number(opt.width || 375), height = Number(opt.height || 812);
const CHROME = process.env.CHROME || ['C:/Program Files/Google/Chrome/Application/chrome.exe', 'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe', '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome', '/usr/bin/google-chrome'].find((p) => existsSync(p));
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

const port = 9400 + Math.floor(Math.random() * 400);
const chrome = spawn(CHROME, ['--headless=new', `--remote-debugging-port=${port}`, `--user-data-dir=${mkdtempSync(join(tmpdir(), 'vn-check-'))}`, '--no-first-run', '--no-default-browser-check', '--disable-extensions', '--mute-audio', `--window-size=${width},${height}`, 'about:blank'], { stdio: 'ignore' });
let ws;
for (let i = 0; i < 50 && !ws; i++) {
	try { const u = (await (await fetch(`http://127.0.0.1:${port}/json/list`)).json()).find((t) => t.type === 'page')?.webSocketDebuggerUrl; if (u) ws = new WebSocket(u); } catch {}
	if (!ws) await sleep(200);
}
await new Promise((r) => ws.addEventListener('open', r));
let seq = 0; const pending = new Map(); const consoleErrors = [];
ws.addEventListener('message', (e) => {
	const m = JSON.parse(e.data);
	if (m.id && pending.has(m.id)) { pending.get(m.id)(m); pending.delete(m.id); }
	else if (m.method === 'Runtime.consoleAPICalled' && m.params.type === 'error') consoleErrors.push(m.params.args.map((a) => a.value ?? a.description).join(' '));
	else if (m.method === 'Runtime.exceptionThrown') consoleErrors.push(m.params.exceptionDetails.exception?.description ?? 'exception');
});
const send = (method, params = {}) => new Promise((r) => { const i = ++seq; pending.set(i, r); ws.send(JSON.stringify({ id: i, method, params })); });
const evaluate = async (expression) => (await send('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true })).result.result?.value;
await send('Page.enable'); await send('Runtime.enable');
await send('Emulation.setDeviceMetricsOverride', { width, height, deviceScaleFactor: 1, mobile: width < 768 });

// the state of the stage after each click
const MEASURE = `(() => {
	const r = (el) => { const b = el.getBoundingClientRect(); return { left: Math.round(b.left), right: Math.round(b.right), top: Math.round(b.top), bottom: Math.round(b.bottom), width: Math.round(b.width) }; };
	const portraits = [...document.querySelectorAll('.character-portrait')].map((el) => ({ ...r(el), depth: Number(el.dataset.depth), crowded: el.dataset.crowded === 'true', active: el.classList.contains('active'), glow: /drop-shadow/.test(el.style.filter), alt: el.querySelector('img')?.alt, src: el.querySelector('img')?.getAttribute('src') }));
	const box = document.querySelector('.vn-message');
	const slot = box?.querySelector('.avatar-slot .character-avatar');
	return {
		href: location.href,
		speaker: box?.querySelector('.character-name')?.textContent?.trim() ?? null,
		text: box?.querySelector('.text')?.textContent?.trim() ?? '',
		box: box ? r(box) : null,
		avatar: slot ? { ...r(slot), alt: slot.querySelector('img')?.alt, glow: /drop-shadow/.test(slot.style.filter), src: slot.querySelector('img')?.getAttribute('src') } : null,
		portraits,
		moodBadges: document.querySelectorAll('.mood-badge').length,
		waiting: !!document.querySelector('.continue-overlay')
	};
})()`;

const failures = [];
const fail = (msg) => { failures.push(msg); console.log('  FAIL ' + msg); };
const byCount = {}; // portraits on stage → one example measurement
let transitions = 0, steps = 0, avatarSteps = 0;

// the viewer's locale comes from the paraglide cookie
await send('Network.setCookie', { name: 'PARAGLIDE_LOCALE', value: locale, url: base });
const prefix = '';
const chapterFile = async (n) => (await fetch(`${base}/api/${toon}/${locale}/chapter${n}.json`)).json();
const season = await (await fetch(`${base}/api/${toon}/${locale}/season1.json`)).json();
const last = Number(opt.to || Math.max(...season.sequence.map((i) => i.number)));
const groups = [];
for (let n = Number(opt.from || 1); n <= last; n++) {
	const ch = await chapterFile(n);
	for (const g of ch.groups) {
		const group = g.ref ? await (await fetch(`${base}/api/${toon}/${locale}/${g.ref.src}`)).json() : g;
		groups.push({ chapter: n, label: group.label, waited: (group.events ?? []).filter((e) => e.type === 'showMessage' && e.params?.wait).length, events: group.events ?? [] });
	}
}

function checkStep(s, group) {
	steps++;
	if (s.moodBadges) fail(`${group}: ${s.moodBadges} mood badge(s) on screen`);
	const ps = s.portraits;
	for (let i = 0; i < ps.length; i++)
		for (let j = i + 1; j < ps.length; j++) {
			const a = ps[i], b = ps[j];
			const ox = Math.min(a.right, b.right) - Math.max(a.left, b.left), oy = Math.min(a.bottom, b.bottom) - Math.max(a.top, b.top);
			if (ox > 0 && oy > 0 && !(a.crowded && b.crowded)) fail(`${group}: portraits overlap (${a.alt} / ${b.alt}, ${ox}×${oy}px)`);
		}
	const sorted = [...ps].sort((a, b) => a.depth - b.depth);
	for (let k = 1; k < sorted.length; k++) if (sorted[k].left <= sorted[k - 1].left) fail(`${group}: depth ${k} is not right of depth ${k - 1}`);
	for (const p of ps) if (p.left < 0 || p.right > width) fail(`${group}: portrait ${p.alt} off screen (${p.left}–${p.right})`);
	if (ps.some((p) => p.active && !p.glow)) fail(`${group}: the speaking portrait has no glow`);
	if (ps.some((p) => p.active && p.depth !== 0)) fail(`${group}: the speaker is not in front`);
	if (s.avatar) {
		avatarSteps++;
		if (!s.avatar.glow) fail(`${group}: the avatar in the box has no glow`);
		if (s.box && (s.avatar.left < s.box.left || s.avatar.right > s.box.left + s.box.width / 2)) fail(`${group}: the avatar is not in the left of the box`);
		if (!s.speaker) fail(`${group}: an avatar without a speaker`);
	}
	if ([s.avatar?.src, ...ps.map((p) => p.src)].some((src) => /transition/.test(src ?? ''))) transitions++;
	const n = ps.length;
	if (n && !byCount[n]) byCount[n] = ps.map(({ alt, depth, left, right, top, bottom, width, active, crowded }) => ({ alt, depth, left, right, top, bottom, width, active, crowded }));
}

const shots = opt.shots ? String(opt.shots) : null;
if (shots) mkdirSync(shots, { recursive: true });
const report = [];
for (const g of groups) {
	const name = `chapter ${g.chapter}${g.label ? ` · ${g.label}` : ''}`;
	const url = `${base}${prefix}/${toon}/chapter/${g.chapter}${g.label ? `?label=${encodeURIComponent(g.label)}` : ''}`;
	await send('Page.navigate', { url });
	// the group starts once the chapter and its scene files are loaded (the first load in dev compiles modules)
	for (let t = 0; t < 40 && !(await evaluate(`!!document.querySelector('.continue-overlay')`)); t++) await sleep(250);
	const start = await evaluate('location.href');
	const lang = await evaluate('document.documentElement.lang');
	if (lang !== locale) fail(`${name}: page language ${lang}, expected ${locale}`);
	let clicks = 0, left = false;
	for (let i = 0; i <= g.waited + 3; i++) {
		await sleep(420); // the stage moves in 0.3 s
		const s = await evaluate(MEASURE);
		if (s.href !== start) { left = true; break; }
		checkStep(s, name);
		if (shots && (s.portraits.length >= 2 || (s.avatar && i < 4))) {
			const r = await send('Page.captureScreenshot', { format: 'png' });
			writeFileSync(join(shots, `${locale}-${width}-c${g.chapter}-${g.label}-${String(i).padStart(3, '0')}.png`), Buffer.from(r.result.data, 'base64'));
		}
		if (!s.waiting) {
			// the next message may take a moment (a busy dev server): up to 3 s before the group counts as done
			let waiting = false;
			for (let t = 0; t < 12 && !waiting; t++) { await sleep(250); waiting = (await evaluate(MEASURE)).waiting; }
			if (!waiting) break;
		}
		await evaluate(`document.querySelector('.continue-overlay')?.click()`);
		clicks++;
	}
	await sleep(600);
	if (!left) left = (await evaluate('location.href')) !== start;
	const endsWithJump = g.events.at(-1)?.type === 'jumpTo';
	const boxOpen = left ? null : await evaluate(`!!document.querySelector('.vn-message.visible')`);
	const ok = clicks === g.waited && (endsWithJump ? left : !boxOpen);
	if (!ok) fail(`${name}: ${clicks} clicks for ${g.waited} waited messages (${endsWithJump ? (left ? 'jumped' : 'did not jump') : boxOpen ? 'box still open' : 'box closed'})`);
	report.push({ group: name, waited: g.waited, clicks, jumped: left });
	console.log(`${ok ? 'ok  ' : 'FAIL'} ${name.padEnd(40)} ${clicks}/${g.waited} clicks${endsWithJump ? (left ? ', jumped' : ', no jump') : ''}`);
}

console.log(`\n${locale} ${width}×${height}: ${groups.length} groups, ${steps} steps, ${avatarSteps} with the avatar slot, ${transitions} with a transition image, console errors: ${consoleErrors.length}`);
for (const [n, ps] of Object.entries(byCount)) {
	console.log(`  ${n} portrait(s)${ps[0]?.crowded ? ' (stacked)' : ''}: ` + ps.sort((a, b) => a.depth - b.depth).map((p) => `d${p.depth} ${p.alt} x ${p.left}–${p.right} (w ${p.width}) y ${p.top}–${p.bottom}${p.active ? ' *' : ''}`).join(' | '));
}
for (const e of consoleErrors.slice(0, 10)) console.log('  console: ' + e);
if (opt.json) writeFileSync(String(opt.json), JSON.stringify({ locale, width, height, report, byCount, steps, avatarSteps, transitions, consoleErrors, failures }, null, '\t'));
ws.close();
chrome.kill();
if (failures.length) { console.log(`${failures.length} failure(s)`); process.exit(1); }
