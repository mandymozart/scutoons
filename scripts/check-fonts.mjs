// Font check: every page of every app renders in IBM Plex Mono, loaded from our own woff2 (no font CDN). Headless
// Chrome via CDP (own temp profile, no dependencies; Node 22 WebSocket). agents/DESIGN.md "Type", shared/tokens/README.md.
//
// usage: node scripts/check-fonts.mjs [--apps=viewer,creator,admin,shop,qr] [--only=<text in the page name>]
//        [--viewer=http://localhost:8000] [--creator=…:10000] [--admin=…:11000] [--shop=…:12000] [--qr=…:9001]
//        [--api=http://localhost:9000] [--token=<session>] [--session] [--width=390] [--height=844] [--verbose] [--list]
//
// Per page (and per opened dialog/sheet), after the fonts settled:
//   - document.fonts.check('1em "IBM Plex Mono"') and a loaded FontFace 'IBM Plex Mono'
//   - the network: a Plex Mono woff2 of @scutoons/tokens answered 200, no request to a font CDN (Google Fonts & co.)
//   - every visible element with its own text and every form control (input, select, textarea, button): the computed
//     font-family starts with 'IBM Plex Mono'; 'Bangers' only as the lettering stack (--font-lettering, text boxes)
//   - the font Chrome really rendered with (CSS.getPlatformFontsForNode) for the form controls and a sample of texts
// Signed-in pages (creator, admin, viewer account) need a session: --token= / SCUTOONS_TOKEN, or --session, which
// creates a throw-away admin + creator user in the LOCAL database (server/tests/support/legacy.php) and removes it at
// the end. Without one, creator and admin are checked on their sign-in screen only.
// Needs the dev servers (npm run dev:up). Exit code 1 on any failure.
import { execFileSync, spawn } from 'node:child_process';
import { existsSync, mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const args = process.argv.slice(2);
const opt = Object.fromEntries(args.filter((a) => a.startsWith('--')).map((a) => { const [k, ...v] = a.slice(2).split('='); return [k, v.length ? v.join('=') : true]; }));
const base = {
	viewer: opt.viewer || 'http://localhost:8000',
	creator: opt.creator || 'http://localhost:10000',
	admin: opt.admin || 'http://localhost:11000',
	shop: opt.shop || 'http://localhost:12000',
	qr: opt.qr || 'http://localhost:9001'
};
const api = String(opt.api || 'http://localhost:9000').replace(/\/$/, '');
const apps = String(opt.apps || 'viewer,creator,admin,shop,qr').split(',');
const width = Number(opt.width || 390), height = Number(opt.height || 844);
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

// the Plex Mono / Bangers files of @scutoons/tokens (Vite keeps the name stem and adds a hash)
const fontsCss = readFileSync(join(root, 'shared/tokens/fonts.css'), 'utf8');
const fontFiles = { plex: [], bangers: [] };
for (const m of fontsCss.matchAll(/font-family:\s*'([^']+)'[\s\S]*?url\(\.\/fonts\/([^)]+)\.woff2\)/g)) (m[1] === 'Bangers' ? fontFiles.bangers : fontFiles.plex).push(m[2]);
const isPlexFile = (url) => fontFiles.plex.some((stem) => decodeURIComponent(url).includes(stem));
const FONT_CDN = /fonts\.googleapis\.com|fonts\.gstatic\.com|use\.typekit|fonts\.bunny\.net|fontawesome|cdnjs\.cloudflare\.com\/ajax\/libs\/font/i;

// ── session ─────────────────────────────────────────────────────────────────────────────────────────────────────────
let token = opt.token || process.env.SCUTOONS_TOKEN || null;
let minted = null;
const php = (code) => execFileSync('php', ['-r', `chdir(${JSON.stringify(join(root, 'server'))}); require 'tests/support/legacy.php'; ${code}`], { encoding: 'utf8' }).trim();
if (!token && opt.session) {
	const email = `check-fonts-${Date.now()}@example.com`;
	token = php(`$u = Users::create('${email}', 'en'); Roles::grant($u['id'], Roles::ADMIN, 'check-fonts'); Roles::grant($u['id'], Roles::CREATOR, 'check-fonts'); echo Auth::createSession($u['id']);`);
	minted = email;
}
function dropSession() {
	if (!minted) return;
	php(`$id = Db::run('SELECT id FROM users WHERE email = ?', ['${minted}'])->fetchColumn(); if ($id) { foreach (['sessions', 'user_roles', 'inventory', 'access_devices'] as $t) { try { Db::run("DELETE FROM $t WHERE user_id = ?", [$id]); } catch (Throwable) {} } Db::run('DELETE FROM users WHERE id = ?', [$id]); }`);
	minted = null;
}
const apiGet = async (path) => {
	try { const r = await fetch(api + path, { headers: token ? { Authorization: `Bearer ${token}` } : {} }); return r.ok ? await r.json() : null; } catch { return null; }
};

// ── the pages ───────────────────────────────────────────────────────────────────────────────────────────────────────
// `auth`: needs a session (signed-out: only one sign-in screen per app); `open`: JS run after the load that opens a
// dialog/sheet (returns true when it could), `wait`: selector that must appear; `signedOut`: forces no session
const click = (sel) => `(() => { const el = document.querySelector(${JSON.stringify(sel)}); if (!el) return false; el.click(); return true; })()`;
const clickText = (sel, re) => `(() => { const el = [...document.querySelectorAll(${JSON.stringify(sel)})].find((e) => ${re}.test(e.textContent)); if (!el) return false; el.click(); return true; })()`;
async function pages() {
	const batch = (await apiGet('/api/admin/batches'))?.items?.[0]?.filename;
	const code = (await apiGet('/api/admin/codes?limit=1'))?.items?.[0]?.code;
	const user = (await apiGet('/api/admin/users?limit=1'))?.items?.[0]?.id;
	const toon = 'catpope';
	return [
		// viewer (scutoons.com)
		{ app: 'viewer', name: 'home', path: '/' },
		{ app: 'viewer', name: 'toon page', path: `/${toon}` },
		{ app: 'viewer', name: 'toon: inventory / avatar', path: `/${toon}?inventory=open`, wait: '.inventory, [class*="inventory"]' },
		{ app: 'viewer', name: 'chapter 1', path: `/${toon}/chapter/1` },
		{ app: 'viewer', name: 'chapter (VN dialogue box)', path: '/sandbox/chapter/1', open: `(async () => { for (let i = 0; i < 40 && !document.querySelector('.vn-message.visible'); i++) { document.elementFromPoint(innerWidth / 2, innerHeight / 2)?.click(); await new Promise((r) => setTimeout(r, 400)); } return !!document.querySelector('.vn-message.visible'); })()`, wait: '.vn-message.visible' },
		{ app: 'viewer', name: 'credits', path: `/${toon}/credits` },
		{ app: 'viewer', name: 'download', path: `/${toon}/download` },
		{ app: 'viewer', name: 'events', path: '/events' },
		{ app: 'viewer', name: 'attributions', path: '/attributions' },
		{ app: 'viewer', name: 'account (signed out: login)', path: '/account', signedOut: true },
		{ app: 'viewer', name: 'account (signed in)', path: '/account', auth: true },
		{ app: 'viewer', name: 'account lab', path: '/account/lab', auth: true },
		{ app: 'viewer', name: 'claim sheet (?code=)', path: `/${toon}?code=FONTCHK0`, wait: '[class*="claim"]' },
		{ app: 'viewer', name: 'main menu', path: '/', open: click('.main-menu-toggle button'), wait: '.main-menu' },
		{ app: 'viewer', name: '404 (unknown toon)', path: '/no-such-toon-fontcheck' },
		{ app: 'viewer', name: '404 (unknown chapter)', path: `/${toon}/chapter/999` },
		// creator
		{ app: 'creator', name: 'sign-in screen', path: '/', signedOut: true },
		{ app: 'creator', name: 'toons', path: '/', auth: true },
		{ app: 'creator', name: 'Info panel', path: '/', auth: true, open: click('[data-testid="info-toggle"]'), wait: '.sc-info-panel, [class*="info-panel"]' },
		{ app: 'creator', name: 'characters', path: '/characters', auth: true },
		{ app: 'creator', name: 'guide', path: '/guide', auth: true },
		{ app: 'creator', name: 'toon overview', path: `/${toon}`, auth: true },
		{ app: 'creator', name: 'toon settings', path: `/${toon}/toon`, auth: true },
		{ app: 'creator', name: 'cast', path: `/${toon}/cast`, auth: true },
		{ app: 'creator', name: 'cast: character picker', path: `/${toon}/cast`, auth: true, open: click('[data-testid="cast-add"]'), wait: '[data-testid="character-picker-sheet"]' },
		{ app: 'creator', name: 'season', path: `/${toon}/season/1`, auth: true },
		{ app: 'creator', name: 'chapter editor', path: `/${toon}/chapter/1`, auth: true },
		{ app: 'creator', name: 'draft preview', path: `/${toon}/chapter/1/preview`, auth: true },
		{ app: 'creator', name: 'credits', path: `/${toon}/credits`, auth: true },
		{ app: 'creator', name: '404', path: '/no-such-toon-fontcheck/chapter/999', auth: true },
		// admin
		{ app: 'admin', name: 'sign-in screen', path: '/', signedOut: true },
		{ app: 'admin', name: 'dashboard', path: '/', auth: true },
		{ app: 'admin', name: 'Info panel', path: '/', auth: true, open: click('[data-testid="info-toggle"]'), wait: '.sc-info-panel, [class*="info-panel"]' },
		{ app: 'admin', name: 'codes', path: '/codes', auth: true },
		{ app: 'admin', name: 'generate (certificate preview)', path: '/codes/new', auth: true },
		...(code ? [{ app: 'admin', name: 'code detail', path: `/codes/${encodeURIComponent(code)}`, auth: true },
			{ app: 'admin', name: 'code: certificate', path: `/codes/${encodeURIComponent(code)}`, auth: true, open: clickText('button, [role="tab"], a', '/certificate|zertifikat/i') }] : []),
		{ app: 'admin', name: 'batches', path: '/batches', auth: true },
		...(batch ? [{ app: 'admin', name: 'batch (print sheet)', path: `/batches/${encodeURIComponent(batch)}`, auth: true }] : []),
		{ app: 'admin', name: 'catalog', path: '/catalog', auth: true },
		{ app: 'admin', name: 'content', path: '/content', auth: true },
		{ app: 'admin', name: 'events', path: '/events', auth: true },
		{ app: 'admin', name: 'settings', path: '/settings', auth: true },
		{ app: 'admin', name: 'users', path: '/users', auth: true },
		...(user ? [{ app: 'admin', name: 'user detail', path: `/users/${user}`, auth: true }] : []),
		{ app: 'admin', name: '404', path: '/no-such-page-fontcheck', auth: true },
		// shop
		{ app: 'shop', name: 'products', path: '/' },
		{ app: 'shop', name: '404', path: '/no-such-page-fontcheck' },
		// qr (qr.scutoons.com, plain PHP)
		{ app: 'qr', name: 'code entry form', path: '/' },
		{ app: 'qr', name: 'unknown code (404 page)', path: '/?c=FONTCHK0' },
		...(code ? [{ app: 'qr', name: 'code page (skip=1)', path: `/?c=${encodeURIComponent(code)}&skip=1` }] : []),
		{ app: 'qr', name: 'verify page', path: '/verify.html' }
	];
}

// ── Chrome ──────────────────────────────────────────────────────────────────────────────────────────────────────────
const CHROME = process.env.CHROME || ['C:/Program Files/Google/Chrome/Application/chrome.exe', 'C:/Program Files (x86)/Google/Chrome/Application/chrome.exe', 'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe', '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome', '/usr/bin/google-chrome'].find((p) => existsSync(p));
if (!CHROME) { console.error('Chrome not found (set CHROME)'); process.exit(2); }
const port = 9400 + Math.floor(Math.random() * 400);
const profile = mkdtempSync(join(tmpdir(), 'check-fonts-'));
const chrome = spawn(CHROME, ['--headless=new', `--remote-debugging-port=${port}`, `--user-data-dir=${profile}`, '--no-first-run', '--no-default-browser-check', '--disable-extensions', '--mute-audio', `--window-size=${width},${height}`, 'about:blank'], { stdio: 'ignore' });
async function done(code) {
	try { dropSession(); } catch (e) { console.error('session cleanup failed:', e.message); }
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
let seq = 0; const pending = new Map(); let requests = new Map(); let lastNet = Date.now();
ws.addEventListener('message', (e) => {
	const m = JSON.parse(e.data), p = m.params;
	if (m.id && pending.has(m.id)) { pending.get(m.id)(m); pending.delete(m.id); return; }
	if (m.method === 'Network.requestWillBeSent') { requests.set(p.requestId, { url: p.request.url, type: p.type, status: null }); lastNet = Date.now(); }
	else if (m.method === 'Network.responseReceived') { const r = requests.get(p.requestId); if (r) { r.status = p.response.status; r.type = p.type; } lastNet = Date.now(); }
	else if (m.method === 'Network.loadingFinished' || m.method === 'Network.loadingFailed') { const r = requests.get(p.requestId); if (r && m.method === 'Network.loadingFailed') r.failed = p.errorText; lastNet = Date.now(); }
});
const send = (method, params = {}) => new Promise((r) => { const i = ++seq; pending.set(i, r); ws.send(JSON.stringify({ id: i, method, params })); });
const evaluate = async (expression) => { const r = await send('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true }); return r.result?.exceptionDetails ? { error: r.result.exceptionDetails.exception?.description } : r.result?.result?.value; };
await send('Page.enable'); await send('Runtime.enable'); await send('Network.enable'); await send('DOM.enable'); await send('CSS.enable');
await send('Network.setCacheDisabled', { cacheDisabled: true });
await send('Emulation.setDeviceMetricsOverride', { width, height, deviceScaleFactor: 1, mobile: width < 768 });
async function idle(quietMs = 700, maxMs = 15000) {
	const t0 = Date.now();
	while (Date.now() - t0 < maxMs) { if (Date.now() - lastNet > quietMs) return; await sleep(100); }
}
let initId = null;
async function setSession(value, origins) {
	if (initId) await send('Page.removeScriptToEvaluateOnNewDocument', { identifier: initId });
	const src = `try { if (${JSON.stringify(origins)}.includes(location.origin)) { ${value ? `localStorage.setItem('scutoons_session', ${JSON.stringify(value)})` : `localStorage.removeItem('scutoons_session')`}; } } catch {}`;
	initId = (await send('Page.addScriptToEvaluateOnNewDocument', { source: src })).result.identifier;
}

// in the page: the fonts, and every element that shows text or is a form control
const INSPECT = `(async () => {
	await document.fonts.ready;
	const clean = (f) => f.trim().replace(/^["']|["']$/g, '');
	const plexFaces = [...document.fonts].filter((f) => clean(f.family) === 'IBM Plex Mono');
	const out = { check: document.fonts.check('1em "IBM Plex Mono"'), faces: plexFaces.length, loaded: plexFaces.filter((f) => f.status === 'loaded').length, bad: [], sample: 0, bangers: 0, texts: 0, controls: 0 };
	document.querySelectorAll('[data-fontcheck]').forEach((el) => { el.removeAttribute('data-fontcheck'); el.removeAttribute('data-fonttext'); });
	const visible = (el) => { if (!el.getClientRects().length) return false; const cs = getComputedStyle(el); return cs.visibility !== 'hidden' && cs.display !== 'none' && Number(cs.opacity) > 0; };
	const describe = (el) => { const cls = typeof el.className === 'string' && el.className.trim() ? '.' + el.className.trim().split(/\\s+/).filter((c) => !/^svelte-/.test(c)).slice(0, 2).join('.') : ''; const txt = (el.value || el.placeholder || el.textContent || '').trim().replace(/\\s+/g, ' ').slice(0, 30); return el.tagName.toLowerCase() + cls + (txt ? ' "' + txt + '"' : ''); };
	let marked = 0;
	for (const el of document.body.querySelectorAll('*')) {
		if (/^(SCRIPT|STYLE|NOSCRIPT|TEMPLATE|svg|path|g|defs)$/.test(el.tagName) || el.closest('noscript, template')) continue;
		const control = /^(INPUT|SELECT|TEXTAREA|BUTTON)$/.test(el.tagName) && !['hidden'].includes(el.type);
		const ownText = [...el.childNodes].some((n) => n.nodeType === 3 && n.textContent.trim());
		if (!control && !ownText) continue;
		if (!visible(el) && !(control && el.type === 'file')) continue;
		control ? out.controls++ : out.texts++;
		const ff = getComputedStyle(el).fontFamily;
		const first = clean(ff.split(',')[0]);
		const ok = first === 'IBM Plex Mono' || (first === 'Bangers' && /IBM Plex Mono/.test(ff));
		if (first === 'Bangers') out.bangers++;
		if (!ok) out.bad.push(describe(el) + ' → ' + ff);
		if (control || marked < 40) {
			el.setAttribute('data-fontcheck', first === 'Bangers' ? 'Bangers' : 'IBM Plex Mono');
			el.setAttribute('data-fonttext', [...el.childNodes].filter((n) => n.nodeType === 3).map((n) => n.textContent).join('')); // what getPlatformFontsForNode reports
			marked++;
		}
	}
	out.sample = marked;
	return out;
})()`;

async function rendered() {
	// the font Chrome really used for the marked elements' text
	const bad = [];
	const doc = (await send('DOM.getDocument', { depth: 0 })).result.root;
	const nodes = (await send('DOM.querySelectorAll', { nodeId: doc.nodeId, selector: '[data-fontcheck]' })).result?.nodeIds ?? [];
	for (const nodeId of nodes) {
		const fonts = (await send('CSS.getPlatformFontsForNode', { nodeId })).result?.fonts ?? [];
		if (!fonts.length) continue; // no text of its own (e.g. an empty input: the UA shadow text is not reported)
		const want = (await send('DOM.getAttributes', { nodeId })).result.attributes;
		const expected = want[want.indexOf('data-fontcheck') + 1];
		const text = want[want.indexOf('data-fonttext') + 1] ?? '';
		// glyphs the family doesn't have (☰ ⓘ ✕ →, emoji, CJK/Hangul) fall back to a system font by design; a text with
		// Latin/Cyrillic letters or digits (which the family covers) must be rendered in it
		if (!/[0-9A-Za-zÀ-ɏЀ-ӿ]/.test(text)) continue;
		if (!fonts.some((f) => f.familyName === expected)) {
			const desc = await evaluate(`(() => { const el = document.querySelectorAll('[data-fontcheck]')[${nodes.indexOf(nodeId)}]; return el ? el.tagName.toLowerCase() + ' "' + (el.value || el.textContent || '').trim().slice(0, 30) + '"' : '?'; })()`);
			bad.push(`${desc} rendered in ${fonts.map((f) => `${f.familyName} (${f.glyphCount})`).join(', ')}`);
		}
	}
	return bad;
}

// ── run ─────────────────────────────────────────────────────────────────────────────────────────────────────────────
const list = (await pages()).filter((p) => apps.includes(p.app) && (!opt.only || p.name.includes(opt.only) || p.app === opt.only));
if (opt.list) { for (const p of list) console.log(`${p.app.padEnd(8)} ${p.name.padEnd(34)} ${base[p.app]}${p.path}`); await done(0); }
const origins = [base.viewer, base.creator, base.admin, base.shop].map((u) => new URL(u).origin);
let failed = 0, passed = 0, skipped = 0;
console.log(`Font check (IBM Plex Mono) · ${width}×${height} · session: ${token ? (minted ? 'throw-away' : 'given') : 'none (signed-in pages: sign-in screens only)'}\n`);
for (const p of list) {
	const label = `${p.app.padEnd(8)} ${p.name}`;
	if (p.auth && !token) { skipped++; if (opt.verbose) console.log(`  SKIP  ${label} (needs a session)`); continue; }
	const url = base[p.app] + p.path;
	await setSession(p.signedOut ? null : token, origins);
	requests = new Map(); lastNet = Date.now();
	await send('Page.navigate', { url });
	await sleep(300); await idle();
	const problems = [];
	if (p.open) {
		const opened = await evaluate(p.open);
		if (opened !== true) problems.push(`could not open (${JSON.stringify(opened)})`);
		await sleep(300); await idle();
	}
	if (p.wait) {
		let ok = false;
		for (let i = 0; i < 30 && !ok; i++) { ok = await evaluate(`!!document.querySelector(${JSON.stringify(p.wait)})`); if (!ok) await sleep(200); }
		if (!ok) problems.push(`"${p.wait}" did not appear`);
	}
	const r = await evaluate(INSPECT);
	if (!r || r.error) { problems.push(`inspect failed: ${r?.error}`); }
	else {
		if (!r.check) problems.push('document.fonts.check(\'1em "IBM Plex Mono"\') is false');
		if (!r.faces) problems.push('no @font-face for IBM Plex Mono');
		else if (!r.loaded) problems.push('no IBM Plex Mono face loaded');
		for (const b of r.bad) problems.push(`computed: ${b}`);
		for (const b of await rendered()) problems.push(`rendered: ${b}`);
	}
	const reqs = [...requests.values()];
	const plex = reqs.filter((q) => q.type === 'Font' && isPlexFile(q.url));
	if (!plex.some((q) => q.status === 200)) problems.push(`no IBM Plex Mono woff2 loaded from the network (fonts: ${reqs.filter((q) => q.type === 'Font').map((q) => `${q.url.split('/').pop()} ${q.status ?? q.failed}`).join(', ') || 'none'})`);
	for (const q of reqs.filter((q) => FONT_CDN.test(q.url))) problems.push(`font CDN request: ${q.url}`);
	for (const q of plex.filter((q) => q.status !== 200)) problems.push(`font ${q.url.split('/').pop()} → ${q.status ?? q.failed}`);
	if (problems.length) {
		failed++;
		console.log(`  FAIL  ${label}  ${url}`);
		for (const x of problems.slice(0, opt.verbose ? 200 : 12)) console.log(`        ${x}`);
		if (problems.length > 12 && !opt.verbose) console.log(`        … ${problems.length - 12} more (--verbose)`);
	} else {
		passed++;
		console.log(`  PASS  ${label}  (${r.texts} texts, ${r.controls} controls${r.bangers ? `, ${r.bangers} lettering` : ''}, ${plex.filter((q) => q.status === 200).length} Plex woff2)`);
	}
}
console.log(`\n${passed} passed, ${failed} failed${skipped ? `, ${skipped} skipped (no session: --session or SCUTOONS_TOKEN)` : ''}`);
await done(failed ? 1 : 0);
