// Toon page desktop check (owner 2026-10-01, agents/DESIGN.md "Toon page desktop"): headless Chrome via CDP (own temp
// profile, no dependencies), like scripts/check-prolog.mjs.
//
// Desktop (1440 × 900, 1280 × 800, 1024 × 768, and 1920 × 1080 for the large loop), de + en, a fresh guest:
//   - the hero is the first screen (≤ 900 px high), the trailer loop autoplays muted (never unmuted), `playsinline`,
//     loops; the network loads exactly one loop file (1280w, 1920w from 1600 px) and nothing of the prologue video;
//   - the loop pauses when the hero leaves the screen and when the tab is hidden, and plays again when it is back;
//   - reduced motion: no video, the poster image, no loop request;
//   - the season line, title, tagline; the start button (inverted, "Prolog ansehen" for a fresh guest) and
//     "Trailer ansehen" + "Ton an" → /<toon>/chapter/0;
//   - the logo + the round top icons are white over the hero (`html.on-hero`) and ink below it;
//   - below: season + the credits colophon (as wide as the column) in cols 1–5 left of the chapter list (cols 7–12),
//     thumbnails 112 × 72, "Prolog" / "#n" right-aligned.
// Phone / tablet (390 × 844, 768 × 1024): no hero, no loop request, the cover + start button in the season column as before.
// All widths: the menu button (top right, "Menü" / "Menu") opens the slide-out (also on the home page), and every menu
// entry has the same font size (the toons too).
// The test user (agent@scutoons.test) at 1440: the hero's start button follows the member's progress (synced state).
//
// usage: node scripts/check-toon-hero.mjs [--viewer=http://localhost:8000] [--api=http://localhost:9000/api]
//        [--shots=<dir>] [--no-member] [--verbose]
import { spawn } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const opt = Object.fromEntries(process.argv.slice(2).filter((a) => a.startsWith('--')).map((a) => { const [k, ...v] = a.slice(2).split('='); return [k, v.length ? v.join('=') : true]; }));
const viewer = String(opt.viewer || 'http://localhost:8000').replace(/\/$/, '');
const api = String(opt.api || 'http://localhost:9000/api').replace(/\/$/, '');
const shots = opt.shots ? String(opt.shots) : null;
if (shots) mkdirSync(shots, { recursive: true });
const toon = 'catpope';
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const messages = Object.fromEntries(['de', 'en'].map((l) => [l, JSON.parse(readFileSync(join(root, `viewer/messages/${l}.json`), 'utf8'))]));
const fill = (s, v) => s.replace(/\{(\w+)\}/g, (_, k) => String(v[k]));
const bundles = {};
for (const l of ['de', 'en']) bundles[l] = await (await fetch(`${api}/library/toons/${toon}?locale=${l}`)).json();
const trailer = bundles.de.seasons?.[0]?.season?.trailer;
if (!trailer?.loop) { console.error('season 1 has no trailer (is the content imported?)'); process.exit(2); }

// ── Chrome ──────────────────────────────────────────────────────────────────────────────────────────────────────────
const CHROME = process.env.CHROME || ['C:/Program Files/Google/Chrome/Application/chrome.exe', 'C:/Program Files (x86)/Google/Chrome/Application/chrome.exe', 'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe', '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome', '/usr/bin/google-chrome'].find((p) => existsSync(p));
if (!CHROME) { console.error('Chrome not found (set CHROME)'); process.exit(2); }
const port = 9400 + Math.floor(Math.random() * 400);
const profile = mkdtempSync(join(tmpdir(), 'check-hero-'));
// default autoplay policy: a muted video may autoplay, sound would need a gesture
const chrome = spawn(CHROME, ['--headless=new', `--remote-debugging-port=${port}`, `--user-data-dir=${profile}`, '--no-first-run', '--no-default-browser-check', '--disable-extensions', '--mute-audio', '--window-size=1440,900', 'about:blank'], { stdio: 'ignore' });
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
let seq = 0; const pending = new Map(); let lastNet = Date.now(); let videoUrls = []; let loader = null;
ws.addEventListener('message', (e) => {
	const m = JSON.parse(e.data), p = m.params;
	if (m.id && pending.has(m.id)) { pending.get(m.id)(m); pending.delete(m.id); return; }
	if (m.method?.startsWith('Network.')) lastNet = Date.now();
	// only the current document's requests (a request of the page before can still be logged after a navigation)
	if (m.method === 'Network.requestWillBeSent' && /\/videos\//.test(p.request.url) && p.loaderId === loader) videoUrls.push(p.request.url.replace(/^.*\/videos\//, 'videos/'));
});
const send = (method, params = {}) => new Promise((r) => { const i = ++seq; pending.set(i, r); ws.send(JSON.stringify({ id: i, method, params })); });
const evaluate = async (expression) => { const r = await send('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true }); return r.result?.exceptionDetails ? { error: r.result.exceptionDetails.exception?.description ?? r.result.exceptionDetails.text } : r.result?.result?.value; };
await send('Page.enable'); await send('Runtime.enable'); await send('Network.enable');
await send('Network.setCacheDisabled', { cacheDisabled: true });
async function idle(quietMs = 700, maxMs = 15000) { const t0 = Date.now(); while (Date.now() - t0 < maxMs) { if (Date.now() - lastNet > quietMs) return; await sleep(100); } }
async function waitFor(js, ms = 12000) { const t0 = Date.now(); let r; while (Date.now() - t0 < ms) { r = await evaluate(js); if (r && !r.error) return r; await sleep(200); } return r ?? null; }
async function open(url, locale) {
	videoUrls = []; loader = null; lastNet = Date.now();
	// the locale cookie decides unprefixed URLs (a visit to /en/… writes it): set it for every page
	if (locale) await send('Network.setCookie', { name: 'PARAGLIDE_LOCALE', value: locale, url: viewer, path: '/' });
	const r = await send('Page.navigate', { url });
	loader = r.result?.loaderId ?? null;
	await sleep(400); await idle();
}
async function shot(name) { if (!shots) return; const r = await send('Page.captureScreenshot', { format: 'png' }); writeFileSync(join(shots, `${name}.png`), Buffer.from(r.result.data, 'base64')); }
async function size(width, height) { await send('Emulation.setDeviceMetricsOverride', { width, height, deviceScaleFactor: 1, mobile: width < 768 }); }
let failed = 0, passed = 0;
function report(name, problems, detail = '') {
	if (problems.length) { failed++; console.log(`  FAIL  ${name}`); for (const p of problems) console.log(`        ${p}`); }
	else { passed++; console.log(`  ok    ${name}${opt.verbose && detail ? `  ${detail}` : ''}`); }
}

const HERO = `(() => {
	const hero = document.querySelector('[data-testid="hero"]');
	const v = document.querySelector('[data-testid="hero-video"]');
	const img = (sel) => { const e = document.querySelector(sel); return e ? getComputedStyle(e).filter : null; };
	const box = (sel) => { const e = document.querySelector(sel); if (!e) return null; const r = e.getBoundingClientRect(); return { x: Math.round(r.x), y: Math.round(r.y), w: Math.round(r.width), h: Math.round(r.height) }; };
	const start = document.querySelector('[data-testid="hero-start"]');
	return {
		hero: box('[data-testid="hero"]'), viewport: document.documentElement.clientWidth, video: v ? { muted: v.muted, defaultMuted: v.defaultMuted, loop: v.loop, playsinline: v.hasAttribute('playsinline'), paused: v.paused, time: v.currentTime, src: v.currentSrc.replace(/^.*\\/videos\\//, 'videos/'), volume: v.volume } : null,
		posterImg: !!hero?.querySelector('img.hero-media'),
		season: hero?.querySelector('.hero-season')?.textContent.trim(), title: hero?.querySelector('h1')?.textContent.trim(), tagline: hero?.querySelector('.hero-tagline')?.textContent.trim(),
		start: start ? { text: start.textContent.trim(), state: start.dataset.state, href: new URL(start.href).pathname, bg: getComputedStyle(start).backgroundColor } : null,
		trailer: document.querySelector('[data-testid="hero-trailer"]') ? { text: document.querySelector('[data-testid="hero-trailer"]').textContent.trim(), href: new URL(document.querySelector('[data-testid="hero-trailer"]').href).pathname } : null,
		sound: document.querySelector('[data-testid="hero-sound"]') ? { text: document.querySelector('[data-testid="hero-sound"]').textContent.trim(), href: new URL(document.querySelector('[data-testid="hero-sound"]').href).pathname } : null,
		onHero: document.documentElement.classList.contains('on-hero'), logoFilter: img('.main-menu-toggle img'), iconFilter: img('.map-toggle img'), menuFilter: img('.menu-toggle img'),
		mobileHeader: !!document.querySelector('.toon-header') && getComputedStyle(document.querySelector('.toon-header')).display !== 'none',
		mobileStart: !!document.querySelector('[data-testid="start-button"]'),
		season: hero?.querySelector('.hero-season')?.textContent.trim(),
		seasonCol: box('.season-column'), listCol: box('.list-column'), thumb: box('.list-column .chapter-item img'),
		colophon: box('.season-column [data-testid="credits-colophon"]'), colophonAnywhere: document.querySelectorAll('[data-testid="credits-colophon"]').length,
		firstNumber: document.querySelector('.list-column .chapter-item [data-testid="chapter-number"]')?.textContent.trim(),
		numberAlign: (() => { const row = document.querySelector('.list-column .chapter-item'); const n = row?.querySelector('[data-testid="chapter-number"]'); const t = n?.firstElementChild || n; if (!row || !t) return null; return Math.round(row.getBoundingClientRect().right - t.getBoundingClientRect().right); })()
	};
})()`;

async function desktop(width, height, locale) {
	const prefix = locale === 'en' ? '/en' : '';
	const msg = messages[locale];
	const season = bundles[locale].seasons[0].season;
	const toonTitle = bundles[locale].toon.title;
	console.log(`${width}×${height} ${locale} (guest)`);
	await size(width, height);
	await open(`${viewer}${prefix}/${toon}`, locale);
	let s = await waitFor(`(() => { const s = ${HERO}; return s.video && !s.video.paused && s.video.time > 0.3 ? s : null; })()`, 15000);
	if (!s || s.error) s = await evaluate(HERO);
	const wantLoop = width >= 1600 ? (trailer.sources?.[0]?.src ?? trailer.loop) : trailer.loop;
	{
		const p = [];
		if (!s.hero) p.push('no hero');
		else if (s.hero.y !== 0 || s.hero.w !== s.viewport || s.hero.h !== Math.min(height, 900)) p.push(`hero box ${JSON.stringify(s.hero)}, expected 0/0 ${s.viewport} × ${Math.min(height, 900)}`);
		if (!s.video) p.push('no loop video');
		else {
			if (!s.video.muted || !s.video.loop || !s.video.playsinline) p.push(`video ${JSON.stringify(s.video)}`);
			if (s.video.paused) p.push('the loop does not play');
			if (s.video.src !== wantLoop) p.push(`plays ${s.video.src}, expected ${wantLoop}`);
		}
		const files = [...new Set(videoUrls)];
		if (files.length !== 1 || files[0] !== wantLoop) p.push(`network loaded ${files.join(', ') || 'no video'}, expected only ${wantLoop}`);
		report('hero = first screen, the loop autoplays muted + loops, one loop file on the network', p, `${JSON.stringify(s.hero)} ${s.video?.src}, network ${[...new Set(videoUrls)].join(', ')}`);
	}
	{
		const p = [];
		if (s.season?.toUpperCase() !== fill(msg.hero.season, { number: 1, title: season.title }).toUpperCase()) p.push(`season line "${s.season}"`);
		if (s.title !== toonTitle) p.push(`title "${s.title}"`);
		if (!s.start || s.start.state !== 'prologue' || s.start.text !== msg.start.prologue || s.start.href !== `${prefix}/${toon}/chapter/0` || s.start.bg !== 'rgb(255, 255, 255)') p.push(`start button ${JSON.stringify(s.start)}`);
		if (!s.trailer || !s.trailer.text.startsWith(msg.hero.trailer) || s.trailer.href !== `${prefix}/${toon}/chapter/0`) p.push(`trailer button ${JSON.stringify(s.trailer)}`);
		if (!s.sound || !s.sound.text.endsWith(msg.hero.sound_on) || s.sound.href !== `${prefix}/${toon}/chapter/0`) p.push(`sound button ${JSON.stringify(s.sound)}`);
		if (s.mobileHeader || s.mobileStart) p.push('the phone header / start button is shown too');
		report(`hero text + buttons ("${msg.start.prologue}" inverted, "${msg.hero.trailer}", "${msg.hero.sound_on}" → chapter 0)`, p);
	}
	{
		const p = [];
		if (!s.onHero || !/invert\(1\)/.test(s.logoFilter ?? '') || !/invert\(1\)/.test(s.iconFilter ?? '') || !/invert\(1\)/.test(s.menuFilter ?? '')) p.push(`on the hero: on-hero ${s.onHero}, logo ${s.logoFilter}, icons ${s.iconFilter}, menu ${s.menuFilter}`);
		await shot(`hero-${width}-${locale}-1-hero`);
		await evaluate(`window.scrollTo(0, document.querySelector('[data-testid="hero"]').offsetHeight + 40), true`);
		const below = await waitFor(`(() => { const s = ${HERO}; return s.video && s.video.paused && !s.onHero && s.colophon ? s : null; })()`, 8000);
		if (!below || below.error) p.push(`below the hero: ${JSON.stringify(await evaluate(`(() => { const s = ${HERO}; return { paused: s.video?.paused, onHero: s.onHero }; })()`))}`);
		else if (/invert/.test(below.logoFilter ?? '') || /invert/.test(below.iconFilter ?? '')) p.push(`below the hero the icons stay inverted: ${below.logoFilter} ${below.iconFilter}`);
		report('icons white on the hero, ink below; the loop pauses offscreen', p);
		// the grid below the hero
		const g = below && !below.error ? below : await evaluate(HERO);
		const q = [];
		if (!g.seasonCol || !g.listCol || !(g.seasonCol.x < g.listCol.x) || g.seasonCol.x + g.seasonCol.w > g.listCol.x) q.push(`columns ${JSON.stringify({ season: g.seasonCol, list: g.listCol })}`);
		if (!g.thumb || g.thumb.w !== 112 || g.thumb.h !== 72) q.push(`thumbnail ${JSON.stringify(g.thumb)}`);
		if (!g.colophon || g.colophonAnywhere !== 1 || g.colophon.x < g.seasonCol.x - 1 || g.colophon.x + g.colophon.w > g.seasonCol.x + g.seasonCol.w + 1 || Math.abs(g.colophon.w - g.seasonCol.w) > 2) q.push(`colophon ${JSON.stringify(g.colophon)} (${g.colophonAnywhere} on the page) not inside the season column ${JSON.stringify(g.seasonCol)}`);
		if (g.firstNumber !== msg.chapter.prologue || g.numberAlign === null || g.numberAlign > 2) q.push(`first row "${g.firstNumber}", right gap ${g.numberAlign}`);
		report('below: season + credits colophon left (cols 1–5, the colophon as wide as the column), list right (cols 7–12), thumbnails 112 × 72, "Prolog" right-aligned', q, JSON.stringify({ season: g.seasonCol, list: g.listCol }));
		await shot(`hero-${width}-${locale}-2-below`);
		await evaluate('window.scrollTo(0, 0), true');
		const back = await waitFor(`(() => { const s = ${HERO}; return s.video && !s.video.paused && s.onHero ? s : null; })()`, 5000);
		const r = [];
		if (!back || back.error) r.push('the loop did not play again back on the hero');
		await evaluate(`Object.defineProperty(document, 'visibilityState', { value: 'hidden', configurable: true }), document.dispatchEvent(new Event('visibilitychange')), true`);
		const hidden = await waitFor(`(() => { const s = ${HERO}; return s.video?.paused ? s : null; })()`, 3000);
		if (!hidden || hidden.error) r.push('the loop keeps playing in a hidden tab');
		await evaluate(`Object.defineProperty(document, 'visibilityState', { value: 'visible', configurable: true }), document.dispatchEvent(new Event('visibilitychange')), true`);
		const visible = await waitFor(`(() => { const s = ${HERO}; return s.video && !s.video.paused ? s : null; })()`, 3000);
		if (!visible || visible.error) r.push('the loop does not resume in a visible tab');
		report('plays again on the hero, pauses in a hidden tab, resumes when visible', r);
	}
}

for (const [w, h] of [[1440, 900], [1280, 800], [1024, 768]]) for (const l of ['de', 'en']) await desktop(w, h, l);
await desktop(1920, 1080, 'en');

// reduced motion: the poster only, no loop download
console.log('1440×900 de (prefers-reduced-motion)');
await size(1440, 900);
await send('Emulation.setEmulatedMedia', { features: [{ name: 'prefers-reduced-motion', value: 'reduce' }] });
await open(`${viewer}/${toon}`, 'de');
{
	const s = await waitFor(`(() => { const s = ${HERO}; return s.hero ? s : null; })()`);
	report('reduced motion: poster image, no video, no loop request', s && !s.video && s.posterImg && !videoUrls.length ? [] : [`video ${JSON.stringify(s?.video)}, poster ${s?.posterImg}, network ${videoUrls.join(', ')}`]);
	await shot('hero-1440-reduced-motion');
}
await send('Emulation.setEmulatedMedia', { features: [] });

// phone + tablet: unchanged single column, no hero
for (const [w, h] of [[390, 844], [768, 1024]]) for (const l of ['de', 'en']) {
	console.log(`${w}×${h} ${l} (guest)`);
	await size(w, h);
	await open(`${viewer}${l === 'en' ? '/en' : ''}/${toon}`, l);
	const s = await waitFor(`(() => { const s = ${HERO}; return s.mobileHeader && s.mobileStart ? s : null; })()`);
	const bottom = await waitFor(`(() => { const c = document.querySelector('[data-testid="credits-colophon"]'); const l = document.querySelector('.chapters-list'); return c && l && c.getBoundingClientRect().top > l.getBoundingClientRect().bottom && !c.closest('.season-column') ? true : null; })()`, 8000);
	report('no hero, no loop request; cover + start button as before, the colophon at the bottom', s && !s.error && !s.hero && !videoUrls.length && bottom === true ? [] : [`hero ${JSON.stringify(s?.hero)}, header ${s?.mobileHeader}, start ${s?.mobileStart}, network ${videoUrls.join(', ')}, colophon at the bottom ${bottom}`]);
	await shot(`hero-${w}-${l}-toon`);
}

// the menu button + the menu's type sizes, on a toon page and the home page
const MENU = `(() => {
	const sizes = [...document.querySelectorAll('.main-menu .nav-btn')].map((e) => { const t = e.querySelector('span') || e; return getComputedStyle(t).fontSize; });
	return { open: !!document.querySelector('.main-menu .menu.open'), sizes: [...new Set(sizes)], label: document.querySelector('[data-testid="menu-toggle"]')?.getAttribute('aria-label') };
})()`;
for (const [w, h] of [[1440, 900], [390, 844]]) for (const [l, path] of [['de', `/${toon}`], ['en', '/en']]) {
	await size(w, h);
	await open(`${viewer}${path}`, l);
	await evaluate(`document.querySelector('[data-testid="menu-toggle"]').click(), true`);
	const s = await waitFor(`(() => { const s = ${MENU}; return s.open ? s : null; })()`, 3000);
	const p = [];
	if (!s || s.error) p.push('the menu button does not open the slide-out');
	else {
		if (s.label !== messages[l].menu.open) p.push(`aria-label "${s.label}"`);
		if (s.sizes.length !== 1) p.push(`menu entries in ${s.sizes.join(' / ')}`);
	}
	report(`${w} ${l} ${path}: the menu button opens the slide-out, every entry one size`, p, s?.sizes?.join(','));
	await shot(`menu-${w}-${l}${path.replace(/\//g, '-')}`);
}

// the test user: the hero's start button follows the member's (synced) progress
if (!opt['no-member']) {
	console.log('1440×900 de (member: agent@scutoons.test)');
	const sessionFile = join(root, 'logs/dev/test-user.json');
	const token = existsSync(sessionFile) ? JSON.parse(readFileSync(sessionFile, 'utf8')).token : null;
	const auth = { Authorization: `Bearer ${token}`, 'Content-Type': 'application/json' };
	const me = token ? await fetch(`${api}/me/`, { headers: auth }) : null;
	if (!me?.ok) report('member', ['no working test-user session (run `npm run dev:test-user`)']);
	else {
		const before = await (await fetch(`${api}/progress/${toon}`, { headers: auth })).json();
		const now = Date.now();
		const iso = (t) => new Date(t).toISOString();
		const cp = (n, position, isComplete, t) => ({ uri: `${toon}/chapter/${n}`, isComplete, position, created_at: iso(t), updated_at: iso(t) });
		const states = [
			{ name: 'prologue watched → "S1 K1 lesen"', progress: [cp(0, 1, true, now - 3000)], kind: 'start', text: fill(messages.de.start.read, { season: 1, chapter: 1 }), href: `/${toon}/chapter/1` },
			{ name: 'chapter 1 at 40 % → "S1 K1 weiterlesen" + rule', progress: [cp(0, 1, true, now - 3000), cp(1, 0.4, false, now - 2000)], kind: 'continue', text: fill(messages.de.start.continue, { season: 1, chapter: 1 }), href: `/${toon}/chapter/1` }
		];
		try {
			for (const st of states) {
				await fetch(`${api}/progress/${toon}`, { method: 'PUT', headers: auth, body: JSON.stringify({ state: { variables: {}, progress: st.progress } }) });
				await size(1440, 900);
				await open(`${viewer}/`, 'de');
				await evaluate(`localStorage.clear(), localStorage.setItem('scutoons_session', ${JSON.stringify(token)}), true`);
				await open(`${viewer}/${toon}`, 'de');
				const s = await waitFor(`(() => { const b = document.querySelector('[data-testid="hero-start"]'); return b && b.dataset.state === ${JSON.stringify(st.kind)} ? { text: b.textContent.trim(), href: new URL(b.href).pathname, rule: !!document.querySelector('[data-testid="hero-start-progress"]') } : null; })()`, 12000);
				const p = [];
				if (!s || s.error) p.push(`start button ${JSON.stringify(await evaluate(`(() => { const b = document.querySelector('[data-testid="hero-start"]'); return b && { state: b.dataset.state, text: b.textContent.trim() }; })()`))}`);
				else if (s.text !== st.text || s.href !== st.href || s.rule !== (st.kind === 'continue')) p.push(JSON.stringify(s));
				report(`member: ${st.name}`, p);
				await shot(`hero-member-${st.kind}`);
			}
		} finally {
			await fetch(`${api}/progress/${toon}`, { method: 'PUT', headers: auth, body: JSON.stringify({ state: before?.state ?? {} }) });
		}
	}
}

console.log(`\n${passed} passed, ${failed} failed`);
await done(failed ? 1 : 0);
