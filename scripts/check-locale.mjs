// Locale check of the viewer: which language a URL renders in. Headless Chrome via CDP (own temp profile, no cookies,
// no dependencies; Node 22 WebSocket), like scripts/check-fonts.mjs. agents/MEMORY.md 2026-09-30 "viewer locale".
//
// The rule (viewer/paraglide.config.js):
//   1. an explicit prefix `/en/…` or `/de/…` always wins (cookie and browser language are ignored);
//   2. an unprefixed URL (every printed / QR URL) uses the cookie PARAGLIDE_LOCALE, then the browser language
//      (Accept-Language / navigator.languages), then `de`;
//   3. the main menu's de/en switch sets the cookie; it survives reloads and internal navigation, links stay in the
//      language (`/en/…` for English, unprefixed for German), no redirects;
//   4. the dev server's HTML (`<html lang>`) and the page after hydration agree.
// Per page it checks: the dev server's `<html lang>` for the same cookie/Accept-Language (200, no redirect), and in the
// browser `document.documentElement.lang`, the UI text (viewer/messages de vs en) and the `?locale=` of the API calls.
// The printed URLs come from the qr app: `/?c=<CODE>&skip=1` must redirect to the unchanged `/<toon>?code=<CODE>`.
//
// usage: node scripts/check-locale.mjs [--viewer=http://localhost:8000] [--qr=http://localhost:9001] [--width=375]
//        [--height=812] [--static] [--verbose]
// --static: the viewer is a plain static host of viewer/build (SPA fallback index.html, like production): the raw HTML
// is the build-time shell (`<html lang="de">`, no text; ssr = false), so only its status is checked there – the
// language is set on the client (routes/+layout.svelte) and checked after hydration as usual.
// Needs the dev servers (npm run dev). Exit code 1 on any failure.
import { spawn } from 'node:child_process';
import { existsSync, mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { localSql } from './dev/db-env.mjs';
import { local } from './dev/ports.mjs';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const opt = Object.fromEntries(process.argv.slice(2).filter((a) => a.startsWith('--')).map((a) => { const [k, ...v] = a.slice(2).split('='); return [k, v.length ? v.join('=') : true]; }));
const viewer = String(opt.viewer || local('viewer')).replace(/\/$/, '');
const qr = String(opt.qr || local('qr')).replace(/\/$/, '');
const width = Number(opt.width || 375), height = Number(opt.height || 812);
const COOKIE = 'PARAGLIDE_LOCALE';
const toon = 'catpope';
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

// ── the language of a page: UI strings that exist in only one of viewer/messages/{de,en}.json ─────────────────────────
const flat = (o, p = '', out = {}) => { for (const [k, v] of Object.entries(o)) { if (k === '$schema') continue; if (typeof v === 'string') out[p + k] = v; else if (v && typeof v === 'object') flat(v, p + k + '.', out); } return out; };
const messages = Object.fromEntries(['de', 'en'].map((l) => [l, flat(JSON.parse(readFileSync(join(root, `viewer/messages/${l}.json`), 'utf8')))]));
const fragments = (l) => new Set(Object.values(messages[l]).flatMap((v) => v.split(/\{[^}]*\}|<[^>]*>|\n/)).map((s) => s.trim().toLowerCase()).filter((s) => s.length >= 5 && /[a-zäöüß]/.test(s)));
const markers = (() => { const de = fragments('de'), en = fragments('en'); return { de: [...de].filter((s) => ![...en].some((e) => e.includes(s) || s.includes(e))), en: [...en].filter((s) => ![...de].some((d) => d.includes(s) || s.includes(d))) }; })();
const DETECT = `(() => {
	const text = [document.body.innerText, ...[...document.querySelectorAll('[alt],[aria-label],[title],[placeholder]')].map((e) => [e.getAttribute('alt'), e.getAttribute('aria-label'), e.getAttribute('title'), e.getAttribute('placeholder')].join(' '))].join('\\n').toLowerCase();
	const m = ${JSON.stringify(markers)};
	const hits = (l) => m[l].filter((s) => text.includes(s));
	return { lang: document.documentElement.lang, path: location.pathname, search: location.search, cookie: document.cookie, languages: navigator.languages, de: hits('de'), en: hits('en') };
})()`;

// ── Chrome ──────────────────────────────────────────────────────────────────────────────────────────────────────────
const CHROME = process.env.CHROME || ['C:/Program Files/Google/Chrome/Application/chrome.exe', 'C:/Program Files (x86)/Google/Chrome/Application/chrome.exe', 'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe', '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome', '/usr/bin/google-chrome'].find((p) => existsSync(p));
if (!CHROME) { console.error('Chrome not found (set CHROME)'); process.exit(2); }
const port = 9400 + Math.floor(Math.random() * 400);
const profile = mkdtempSync(join(tmpdir(), 'check-locale-'));
const chrome = spawn(CHROME, ['--headless=new', `--remote-debugging-port=${port}`, `--user-data-dir=${profile}`, '--no-first-run', '--no-default-browser-check', '--disable-extensions', '--mute-audio', `--window-size=${width},${height}`, 'about:blank'], { stdio: 'ignore' });
async function done(code) {
	const exited = chrome.exitCode !== null ? Promise.resolve() : new Promise((r) => chrome.once('exit', r));
	chrome.kill();
	await Promise.race([exited, sleep(5000)]);
	try { rmSync(profile, { recursive: true, force: true, maxRetries: 10, retryDelay: 200 }); } catch {}
	process.exit(code);
}
process.on('uncaughtException', (e) => { console.error(e); done(2); });

let ws;
for (let i = 0; i < 50 && !ws; i++) {
	try { const u = (await (await fetch(`http://127.0.0.1:${port}/json/list`)).json()).find((t) => t.type === 'page')?.webSocketDebuggerUrl; if (u) ws = new WebSocket(u); } catch {}
	if (!ws) await sleep(200);
}
await new Promise((r) => ws.addEventListener('open', r));
let seq = 0; const pending = new Map(); let requests = []; let lastNet = Date.now(); let docs = [];
ws.addEventListener('message', (e) => {
	const m = JSON.parse(e.data), p = m.params;
	if (m.id && pending.has(m.id)) { pending.get(m.id)(m); pending.delete(m.id); return; }
	if (m.method === 'Network.requestWillBeSent') { requests.push(p.request.url); if (p.type === 'Document') docs.push({ url: p.request.url, redirect: p.redirectResponse?.status }); lastNet = Date.now(); }
	else if (m.method?.startsWith('Network.')) lastNet = Date.now();
});
const send = (method, params = {}) => new Promise((r) => { const i = ++seq; pending.set(i, r); ws.send(JSON.stringify({ id: i, method, params })); });
const evaluate = async (expression) => { const r = await send('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true }); return r.result?.exceptionDetails ? { error: r.result.exceptionDetails.exception?.description } : r.result?.result?.value; };
await send('Page.enable'); await send('Runtime.enable'); await send('Network.enable');
await send('Network.setCacheDisabled', { cacheDisabled: true });
await send('Emulation.setDeviceMetricsOverride', { width, height, deviceScaleFactor: 1, mobile: true });
const userAgent = (await send('Browser.getVersion')).result.userAgent.replace('HeadlessChrome', 'Chrome');
async function idle(quietMs = 800, maxMs = 20000) {
	const t0 = Date.now();
	while (Date.now() - t0 < maxMs) { if (Date.now() - lastNet > quietMs) return; await sleep(100); }
}
async function browser({ cookie, acceptLanguage }) {
	await send('Network.clearBrowserCookies');
	if (cookie) await send('Network.setCookie', { name: COOKIE, value: cookie, url: viewer, path: '/' });
	await send('Emulation.setUserAgentOverride', { userAgent, acceptLanguage });
}
async function settle() {
	await sleep(400); await idle();
	for (let i = 0; i < 40; i++) { const r = await evaluate(DETECT); if (r && !r.error && (r.de.length || r.en.length)) return r; await sleep(250); }
	return evaluate(DETECT);
}
async function open(url) { requests = []; docs = []; lastNet = Date.now(); await send('Page.navigate', { url }); return settle(); }
async function action(js) { requests = []; docs = []; lastNet = Date.now(); const r = await evaluate(js); if (r !== true) return { error: `action failed: ${JSON.stringify(r)}` }; await sleep(600); return settle(); }
// the locale the API calls asked for (library endpoints: `?locale=`)
const apiLocales = () => [...new Set(requests.map((u) => { try { return new URL(u).searchParams.get('locale'); } catch { return null; } }).filter(Boolean))];

// ── checks ──────────────────────────────────────────────────────────────────────────────────────────────────────────
let failed = 0, passed = 0;
function report(name, problems, detail) {
	if (problems.length) { failed++; console.log(`  FAIL  ${name}`); for (const p of problems) console.log(`        ${p}`); }
	else { passed++; console.log(`  ok    ${name}${opt.verbose ? `  ${detail}` : ''}`); }
}
function judge(r, expect, { path } = {}) {
	const problems = [];
	if (!r || r.error) return [`page: ${r?.error ?? 'no result'}`];
	if (r.lang !== expect) problems.push(`<html lang="${r.lang}"> after hydration, expected "${expect}"`);
	const other = expect === 'en' ? 'de' : 'en';
	if (r[expect].length < 1 || r[other].length > r[expect].length) problems.push(`UI text looks ${other}: ${expect} ${JSON.stringify(r[expect].slice(0, 4))} vs ${other} ${JSON.stringify(r[other].slice(0, 4))}`);
	const api = apiLocales();
	if (api.some((l) => l !== expect)) problems.push(`API asked for ?locale=${api.join(',')}`);
	if (path && r.path !== path) problems.push(`path ${r.path}, expected ${path}`);
	const redirects = docs.filter((d) => d.redirect);
	if (redirects.length) problems.push(`redirected: ${redirects.map((d) => `${d.redirect} → ${d.url}`).join(', ')}`);
	return problems;
}
// the dev server's HTML for the same request: 200 (no redirect) and `<html lang>`
async function server(path, { cookie, acceptLanguage }, expect) {
	const headers = { 'sec-fetch-dest': 'document', accept: 'text/html' };
	if (acceptLanguage) headers['accept-language'] = acceptLanguage;
	if (cookie) headers.cookie = `${COOKIE}=${cookie}`;
	const res = await fetch(viewer + path, { headers, redirect: 'manual' });
	const lang = (await res.text()).match(/<html[^>]*\slang="([^"]*)"/)?.[1];
	const problems = [];
	if (res.status !== 200) problems.push(`server answered ${res.status}${res.headers.get('location') ? ` → ${res.headers.get('location')}` : ''}`);
	if (lang !== expect && !opt.static) problems.push(`server <html lang="${lang}">, expected "${expect}"`);
	return problems;
}
async function page(name, path, ctx, expect) {
	const problems = await server(path, ctx, expect);
	await browser(ctx);
	const r = await open(viewer + path);
	problems.push(...judge(r, expect, { path: path.split('?')[0] }));
	report(name, problems, `(${r?.lang}, api ${apiLocales().join(',') || '-'}, navigator ${r?.languages?.join(',')})`);
	return r;
}

console.log(`Locale check · viewer ${viewer} · qr ${qr} · ${width}×${height}\n`);
const EN = 'en-US,en;q=0.9', DE = 'de-DE,de;q=0.9';

// 1. an explicit prefix wins
await page('/en/catpope (fresh, Accept-Language de) → en', `/en/${toon}`, { acceptLanguage: DE }, 'en');
await page('/en/catpope with cookie de → en', `/en/${toon}`, { cookie: 'de', acceptLanguage: DE }, 'en');
await page('/en/catpope/chapter/1 with cookie de → en', `/en/${toon}/chapter/1`, { cookie: 'de', acceptLanguage: DE }, 'en');
await page('/en/account with cookie de → en', '/en/account', { cookie: 'de', acceptLanguage: DE }, 'en');
await page('/de/catpope with cookie en → de', `/de/${toon}`, { cookie: 'en', acceptLanguage: EN }, 'de');
// 2. unprefixed: cookie → browser language → de
await page('/catpope, no cookie, Accept-Language en → en', `/${toon}`, { acceptLanguage: EN }, 'en');
await page('/catpope, no cookie, Accept-Language de → de', `/${toon}`, { acceptLanguage: DE }, 'de');
await page('/catpope, no cookie, Accept-Language fr → de', `/${toon}`, { acceptLanguage: 'fr-FR,fr;q=0.9' }, 'de');
await page('/catpope, cookie en, Accept-Language de → en', `/${toon}`, { cookie: 'en', acceptLanguage: DE }, 'en');
await page('/catpope, cookie de, Accept-Language en → de', `/${toon}`, { cookie: 'de', acceptLanguage: EN }, 'de');
await page('/catpope/chapter/1, cookie en → en', `/${toon}/chapter/1`, { cookie: 'en', acceptLanguage: DE }, 'en');

// 3. the menu switch: cookie, reload, internal navigation (links in the language, no flip, no redirect)
{
	const menu = (locale) => `(async () => {
		document.querySelector('[data-testid="menu-toggle"]')?.click();
		await new Promise((r) => setTimeout(r, 300));
		const b = [...document.querySelectorAll('.nav--language button')].find((e) => e.textContent.trim() === ${JSON.stringify(locale)});
		if (!b) return 'no ${locale} button';
		b.click();
		return true;
	})()`;
	const chapterLink = `(() => { const a = [...document.querySelectorAll('a[href*="/chapter/"]')][0]; if (!a) return 'no chapter link'; a.click(); return true; })()`;
	const cookieIs = (r, l) => (new RegExp(`(^|; )${COOKIE}=${l}(;|$)`).test(r?.cookie ?? '') ? [] : [`cookie "${r?.cookie}", expected ${COOKIE}=${l}`]);
	await browser({ acceptLanguage: DE });
	let r = await open(`${viewer}/${toon}`);
	report('menu: /catpope starts in de (Accept-Language de)', judge(r, 'de', { path: `/${toon}` }));
	r = await action(menu('en'));
	report('menu: switch to en → en, cookie en, same URL', [...judge(r, 'en', { path: `/${toon}` }), ...cookieIs(r, 'en')]);
	await send('Page.reload'); r = await settle();
	report('menu: reload → still en', judge(r, 'en', { path: `/${toon}` }));
	r = await action(chapterLink);
	report('menu: internal link → /en/catpope/chapter/N in en', [...judge(r, 'en'), ...(r?.path?.startsWith(`/en/${toon}/chapter/`) ? [] : [`path ${r?.path}`])]);
	await send('Page.reload'); r = await settle();
	report('menu: reload the chapter → still en', judge(r, 'en'));
	r = await action(menu('de'));
	report('menu: switch to de on /en/… → de, unprefixed URL, cookie de', [...judge(r, 'de'), ...cookieIs(r, 'de'), ...(r?.path?.startsWith('/en/') ? [`path ${r?.path} kept /en/`] : [])]);
	await send('Page.reload'); r = await settle();
	report('menu: reload → still de', judge(r, 'de'));
	r = await open(`${viewer}/${toon}`);
	r = await action(chapterLink);
	report('menu: internal link → unprefixed chapter in de', [...judge(r, 'de'), ...(r?.path?.startsWith(`/${toon}/chapter/`) ? [] : [`path ${r?.path}`])]);
	// the language a prefixed visit chose sticks for unprefixed links followed from there
	await browser({ cookie: 'de', acceptLanguage: DE });
	await open(`${viewer}/en/${toon}`);
	r = await open(`${viewer}/${toon}`);
	report('/en/ visit, then unprefixed /catpope → en (no flip back)', judge(r, 'en', { path: `/${toon}` }));
}

// 4. printed URLs via the qr app: unchanged redirect target, language by the rule
{
	let codes = [];
	try {
		const [rows] = localSql(["SELECT MIN(code) code, redirect_to FROM qr WHERE revoked_at IS NULL AND redirect_to LIKE 'https://%scutoons.com/%' AND type IN ('membership', 'page') GROUP BY redirect_to"]);
		codes = rows;
	} catch (e) { report('qr: printed codes from the local database', [e.message.split('\n')[0]]); }
	if (!codes.length) report('qr: printed codes from the local database', ['none found']);
	for (const { code, redirect_to } of codes) {
		const res = await fetch(`${qr}/?c=${encodeURIComponent(code)}&skip=1`, { redirect: 'manual' });
		const location = res.headers.get('location') ?? '';
		const want = `${redirect_to}?code=${encodeURIComponent(code)}`;
		report(`qr: ?c=${code}&skip=1 → ${want}`, res.status >= 300 && res.status < 400 && location === want ? [] : [`${res.status} → ${location}`]);
		if (location !== want) continue;
		const target = new URL(location);
		const local = target.pathname + target.search; // the printed target on the local viewer
		await page(`qr → ${target.host}${local}, Accept-Language en → en`, local, { acceptLanguage: EN }, 'en');
		await page(`qr → ${target.host}${local}, Accept-Language de → de`, local, { acceptLanguage: DE }, 'de');
		await page(`qr → ${target.host}${local}, cookie en, Accept-Language de → en`, local, { cookie: 'en', acceptLanguage: DE }, 'en');
	}
}

console.log(`\n${passed} ok, ${failed} failed`);
await done(failed ? 1 : 0);
