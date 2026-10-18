// Toon page desktop check (owner 2026-10-01, agents/DESIGN.md "Toon page desktop"): headless Chrome via CDP (own temp
// profile, no dependencies), like scripts/check-prolog.mjs.
//
// Desktop (1440 × 900, 1280 × 800, 1024 × 768, and 1920 × 1080 for the large loop), de + en, a fresh guest:
//   - the hero is the first screen (≤ 900 px high), the trailer loop autoplays muted, `playsinline`, loops; the network
//     loads exactly one loop file (1280w, 1920w from 1600 px) and nothing of the prologue video;
//   - the loop pauses when the hero leaves the screen and when the tab is hidden, and plays again when it is back;
//   - reduced motion: no video, the poster image, no loop request; "Ton an" then plays the loop with sound, "Ton aus"
//     stops it again;
//   - the season line, title, tagline; the start button (inverted, "Prolog ansehen" for a fresh guest) and
//     "Trailer ansehen" → /<toon>/chapter/0; "Ton an" is a toggle button (aria-pressed false, speaker icon), no link;
//   - 1440 / 1280 (real CDP mouse clicks): a click on the video / hero background does nothing (URL unchanged, still
//     muted, no pointer cursor); "Ton an" unmutes the playing loop in place (URL unchanged, "Ton aus", speaker-muted
//     icon, aria-pressed true, audio decoded); scrolled away it pauses, back on the hero it resumes still unmuted;
//     "Ton aus" mutes again; a reload starts muted; "Trailer ansehen" opens /<toon>/chapter/0;
//   - the logo + the round top icons are white over the hero (`html.on-hero`) and ink below it;
//   - below: season + the credits colophon (as wide as the column) in cols 1–5 left of the chapter list (cols 7–12),
//     thumbnails 112 × 72, "Prolog" / "#n" right-aligned.
// Phone / tablet (390 × 844, 768 × 1024): no hero, no loop request, the cover + start button in the season column as before.
// All widths: the menu button (top right, "Menü" / "Menu") opens the slide-out (also on the home page), and every menu
// entry has the same font size (the toons too). 1440 / 390, de + en: the toons are the first entries, no "SCUTOONS"
// entry, the closed slide-out is inert, opening focuses the first toon, Escape closes it and focuses the menu button;
// the logo is a link to the (localized) landing page and never opens the menu.
// 390 / 768 / 1280 / 1440, de + en: the record cut-out (season.download.image, decorative, lazy) beside the music button
// (desktop: 120-160 px, left of it in the season column; phone / tablet: smaller, the pair centred).
// The test user (agent@scutoons.test) at 1440: the hero's start button follows the member's progress (synced state).
//
// usage: node scripts/check-toon-hero.mjs [--viewer=http://localhost:8000] [--api=http://localhost:9000/api]
//        [--shots=<dir>] [--no-member] [--verbose]
import { spawn } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { local } from './dev/ports.mjs';
import { saveGameRow } from './dev/save-game-row.mjs';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const opt = Object.fromEntries(process.argv.slice(2).filter((a) => a.startsWith('--')).map((a) => { const [k, ...v] = a.slice(2).split('='); return [k, v.length ? v.join('=') : true]; }));
const viewer = String(opt.viewer || local('viewer')).replace(/\/$/, '');
const api = String(opt.api || local('api', '/api')).replace(/\/$/, '');
const shots = opt.shots ? String(opt.shots) : null;
if (shots) mkdirSync(shots, { recursive: true });
const toon = 'catpope';

// a video request's path in the toon's CDN folder (agents/ASSETS.md "Folder convention": videos live in their season /
// chapter folder, no `videos/` folder any more) – compared with the content's toon-relative paths
const isVideo = (url) => /\.(mp4|webm)(\?|#|$)/.test(url);
const toonPath = (url) => url.replace(/[?#].*$/, '').replace(new RegExp(`^.*?/${toon}/`), '');
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
	if (m.method === 'Network.requestWillBeSent' && isVideo(p.request.url) && p.loaderId === loader) videoUrls.push(toonPath(p.request.url));
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
		hero: box('[data-testid="hero"]'), viewport: document.documentElement.clientWidth, video: v ? { muted: v.muted, defaultMuted: v.defaultMuted, loop: v.loop, playsinline: v.hasAttribute('playsinline'), paused: v.paused, time: v.currentTime, src: v.currentSrc.replace(/[?#].*$/, '').replace(/^.*?\\/${toon}\\//, ''), volume: v.volume } : null,
		posterImg: !!hero?.querySelector('img.hero-media'),
		season: hero?.querySelector('.hero-season')?.textContent.trim(), title: hero?.querySelector('h1')?.textContent.trim(), tagline: hero?.querySelector('.hero-tagline')?.textContent.trim(),
		start: start ? { text: start.textContent.trim(), state: start.dataset.state, href: new URL(start.href).pathname, bg: getComputedStyle(start).backgroundColor } : null,
		trailer: document.querySelector('[data-testid="hero-trailer"]') ? { text: document.querySelector('[data-testid="hero-trailer"]').textContent.trim(), href: new URL(document.querySelector('[data-testid="hero-trailer"]').href).pathname } : null,
		sound: (() => { const b = document.querySelector('[data-testid="hero-sound"]'); return b ? { tag: b.tagName, text: b.textContent.trim(), pressed: b.getAttribute('aria-pressed'), icon: b.querySelector('svg')?.dataset.icon, href: b.getAttribute('href') } : null; })(),
		path: location.pathname,
		onHero: document.documentElement.classList.contains('on-hero'), logoFilter: img('.logo-link img'), iconFilter: img('.map-toggle img'), menuWhite: (() => { const b = document.querySelector('.menu-toggle'); const i = b?.querySelector('svg'); return i ? getComputedStyle(i).color === getComputedStyle(b).borderTopColor && getComputedStyle(i).color !== getComputedStyle(document.body).color : null; })(),
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
	// the account's media quality picks the encoding (owner 2026-10-02: no resolution switching); the check's fresh guest = Automatic → the loop
	const wantLoop = trailer.variants ? trailer.loop : width >= 1600 ? (trailer.sources?.[0]?.src ?? trailer.loop) : trailer.loop;
	// with quality variants any one of the trailer's files is right (the reader's quality picks it)
	const okLoop = (file) => file === wantLoop || Object.values(trailer.variants ?? {}).includes(file);
	{
		const p = [];
		if (!s.hero) p.push('no hero');
		else if (s.hero.y !== 0 || s.hero.w !== s.viewport || s.hero.h !== Math.min(height, 900)) p.push(`hero box ${JSON.stringify(s.hero)}, expected 0/0 ${s.viewport} × ${Math.min(height, 900)}`);
		if (!s.video) p.push('no loop video');
		else {
			if (!s.video.muted || !s.video.loop || !s.video.playsinline) p.push(`video ${JSON.stringify(s.video)}`);
			if (s.video.paused) p.push('the loop does not play');
			if (!okLoop(s.video.src)) p.push(`plays ${s.video.src}, expected ${wantLoop}`);
		}
		const files = [...new Set(videoUrls)];
		if (files.length !== 1 || !okLoop(files[0])) p.push(`network loaded ${files.join(', ') || 'no video'}, expected only ${wantLoop}`);
		report('hero = first screen, the loop autoplays muted + loops, one loop file on the network', p, `${JSON.stringify(s.hero)} ${s.video?.src}, network ${[...new Set(videoUrls)].join(', ')}`);
	}
	{
		const p = [];
		if (s.season?.toUpperCase() !== fill(msg.hero.season, { number: 1, title: season.title }).toUpperCase()) p.push(`season line "${s.season}"`);
		if (s.title !== toonTitle) p.push(`title "${s.title}"`);
		if (!s.start || s.start.state !== 'prologue' || s.start.text !== msg.start.prologue || s.start.href !== `${prefix}/${toon}/chapter/0` || s.start.bg !== 'rgb(255, 255, 255)') p.push(`start button ${JSON.stringify(s.start)}`);
		if (!s.trailer || !s.trailer.text.startsWith(msg.hero.trailer) || s.trailer.href !== `${prefix}/${toon}/chapter/0`) p.push(`trailer button ${JSON.stringify(s.trailer)}`);
		if (!s.sound || s.sound.tag !== 'BUTTON' || s.sound.text !== msg.hero.sound_on || s.sound.pressed !== 'false' || s.sound.icon !== 'speaker' || s.sound.href !== null) p.push(`sound button ${JSON.stringify(s.sound)}`);
		if (s.mobileHeader || s.mobileStart) p.push('the phone header / start button is shown too');
		report(`hero text + buttons ("${msg.start.prologue}" inverted, "${msg.hero.trailer}" → chapter 0, "${msg.hero.sound_on}" a toggle, muted)`, p);
	}
	{
		const p = [];
		if (!s.onHero || !/invert\(1\)/.test(s.logoFilter ?? '') || !/invert\(1\)/.test(s.iconFilter ?? '') || !s.menuWhite) p.push(`on the hero: on-hero ${s.onHero}, logo ${s.logoFilter}, icons ${s.iconFilter}, menu (Feather, paper) ${s.menuWhite}`);
		await shot(`hero-${width}-${locale}-1-hero`);
		await evaluate(`window.scrollTo(0, document.querySelector('[data-testid="hero"]').offsetHeight + 40), true`);
		const below = await waitFor(`(() => { const s = ${HERO}; return s.video && s.video.paused && !s.onHero && s.colophon ? s : null; })()`, 8000);
		if (!below || below.error) p.push(`below the hero: ${JSON.stringify(await evaluate(`(() => { const s = ${HERO}; return { paused: s.video?.paused, onHero: s.onHero }; })()`))}`);
		else if (/invert/.test(below.logoFilter ?? '') || /invert/.test(below.iconFilter ?? '') || below.menuWhite) p.push(`below the hero the icons stay inverted: ${below.logoFilter} ${below.iconFilter} menu ${below.menuWhite}`);
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
	if (width === 1440 || width === 1280) await clicks(width, height, locale);
}

// a real mouse click (CDP input = a user gesture, so the browser lets the loop play with sound)
async function clickAt(x, y) {
	await send('Input.dispatchMouseEvent', { type: 'mouseMoved', x, y });
	await send('Input.dispatchMouseEvent', { type: 'mousePressed', x, y, button: 'left', clickCount: 1 });
	await send('Input.dispatchMouseEvent', { type: 'mouseReleased', x, y, button: 'left', clickCount: 1 });
}
async function clickOn(sel) {
	const b = await evaluate(`(() => { const e = document.querySelector(${JSON.stringify(sel)}); if (!e) return null; const r = e.getBoundingClientRect(); return { x: r.x + r.width / 2, y: r.y + r.height / 2 }; })()`);
	if (!b || b.error) return false;
	await clickAt(b.x, b.y);
	return true;
}

// the hero's click targets: the background does nothing, the sound button toggles in place, the trailer navigates
async function clicks(width, height, locale) {
	const prefix = locale === 'en' ? '/en' : '';
	const msg = messages[locale];
	const page = `${prefix}/${toon}`;
	const playing = (cond) => `(() => { const s = ${HERO}; return s.video && !s.video.paused && s.onHero && (${cond}) ? s : null; })()`;
	const state = `(() => { const s = ${HERO}; return { video: s.video, sound: s.sound, path: s.path }; })()`;
	await evaluate('window.scrollTo(0, 0), true');
	await waitFor(playing('true'), 5000);
	{
		// the background: a click in the empty upper middle of the hero (the scrim over the video)
		const p = [];
		const hit = await evaluate(`(() => { const x = innerWidth / 2, y = 300; const e = document.elementFromPoint(x, y); const v = document.querySelector('[data-testid="hero-video"]'); return { x, y, cls: String(e?.className), inHero: !!e?.closest('[data-testid="hero"]'), cursor: e ? getComputedStyle(e).cursor : null, videoCursor: v && getComputedStyle(v).cursor, link: !!e?.closest('a, button'), videoLink: !!v?.closest('a, button'), onclick: !!(v?.onclick || e?.onclick) }; })()`);
		await clickAt(hit.x, hit.y);
		await sleep(800);
		const s = await evaluate(HERO);
		if (!hit.inHero || hit.link || hit.videoLink || hit.onclick || hit.cursor === 'pointer' || hit.videoCursor === 'pointer') p.push(`background ${JSON.stringify(hit)}`);
		if (s.path !== page) p.push(`a click on the video navigated to ${s.path}`);
		if (!s.video?.muted || s.sound?.pressed !== 'false') p.push(`a click on the video changed the sound: ${JSON.stringify({ muted: s.video?.muted, pressed: s.sound?.pressed })}`);
		report(`${width} ${locale}: a click on the video / hero background does nothing (no link, no pointer, URL + mute unchanged)`, p, JSON.stringify(hit));
	}
	{
		const p = [];
		await clickOn('[data-testid="hero-sound"]');
		const on = await waitFor(playing(`!s.video.muted && s.sound?.pressed === 'true'`), 4000);
		if (!on || on.error) p.push(`after "${msg.hero.sound_on}": ${JSON.stringify(await evaluate(state))}`);
		else {
			if (on.path !== page) p.push(`navigated to ${on.path}`);
			if (on.sound.text !== msg.hero.sound_off || on.sound.icon !== 'speaker-muted') p.push(`sound button ${JSON.stringify(on.sound)}`);
			if (on.video.volume === 0) p.push('volume 0');
		}
		await sleep(800);
		const decoded = await evaluate(`document.querySelector('[data-testid="hero-video"]')?.webkitAudioDecodedByteCount ?? null`);
		if (decoded !== null && !(decoded > 0)) p.push(`no audio decoded (${decoded} bytes)`);
		await shot(`hero-${width}-${locale}-3-sound-on`);
		report(`${width} ${locale}: "${msg.hero.sound_on}" unmutes the playing loop in place → "${msg.hero.sound_off}", speaker-muted icon, aria-pressed true, audio decoded`, p, `audio ${decoded} B`);
	}
	{
		const p = [];
		await evaluate(`window.scrollTo(0, document.querySelector('[data-testid="hero"]').offsetHeight + 40), true`);
		const away = await waitFor(`(() => { const s = ${HERO}; return s.video && s.video.paused ? s : null; })()`, 5000);
		if (!away || away.error) p.push('keeps playing scrolled away');
		await evaluate('window.scrollTo(0, 0), true');
		const back = await waitFor(playing(`!s.video.muted && s.sound?.pressed === 'true'`), 5000);
		if (!back || back.error) p.push(`back on the hero: ${JSON.stringify(await evaluate(state))}`);
		await clickOn('[data-testid="hero-sound"]');
		const off = await waitFor(playing(`s.video.muted && s.sound?.pressed === 'false' && s.sound.icon === 'speaker'`), 4000);
		if (!off || off.error) p.push(`after "${msg.hero.sound_off}": ${JSON.stringify(await evaluate(state))}`);
		else if (off.path !== page || off.sound.text !== msg.hero.sound_on) p.push(`after "${msg.hero.sound_off}": ${off.path} "${off.sound.text}"`);
		report(`${width} ${locale}: unmuted, scrolled away it pauses, back it resumes unmuted; "${msg.hero.sound_off}" mutes again (still playing, URL unchanged)`, p);
	}
	{
		// unmute, then reload: always muted on load
		const p = [];
		await clickOn('[data-testid="hero-sound"]');
		await waitFor(playing('!s.video.muted'), 4000);
		await open(`${viewer}${page}`, locale);
		const s = await waitFor(playing('true'), 10000);
		if (!s || s.error || !s.video.muted || s.sound?.pressed !== 'false') p.push(`after a reload: ${JSON.stringify(s?.video ?? s)} ${JSON.stringify(s?.sound)}`);
		report(`${width} ${locale}: a reload starts muted again (no remembered unmute)`, p);
	}
	{
		const p = [];
		await clickOn('[data-testid="hero-trailer"]');
		const went = await waitFor(`location.pathname === ${JSON.stringify(`${prefix}/${toon}/chapter/0`)} ? location.pathname : null`, 8000);
		if (!went || went.error) p.push(`"${msg.hero.trailer}" → ${await evaluate('location.pathname')}`);
		report(`${width} ${locale}: "${msg.hero.trailer}" opens ${prefix}/${toon}/chapter/0`, p);
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
	// "Ton an" is the explicit request: the loop plays with sound (in place); "Ton aus" stops it again
	const state = `(() => { const s = ${HERO}; return { video: s.video, sound: s.sound, path: s.path }; })()`;
	const p = [];
	await clickOn('[data-testid="hero-sound"]');
	const on = await waitFor(`(() => { const s = ${HERO}; return s.video && !s.video.paused && !s.video.muted && s.sound?.pressed === 'true' ? s : null; })()`, 10000);
	if (!on || on.error) p.push(`after "Ton an": ${JSON.stringify(await evaluate(state))}`);
	else if (on.path !== `/${toon}`) p.push(`navigated to ${on.path}`);
	await clickOn('[data-testid="hero-sound"]');
	const off = await waitFor(`(() => { const s = ${HERO}; return s.video && s.video.paused && s.video.muted && s.sound?.pressed === 'false' ? s : null; })()`, 4000);
	if (!off || off.error) p.push(`after "Ton aus": ${JSON.stringify(await evaluate(state))}`);
	report('reduced motion: "Ton an" plays the loop with sound (in place), "Ton aus" stops it', p);
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

// the slide-out: account + language, then the toons, no "SCUTOONS" entry; closed = inert; opens on the first toon, Escape → back to the
// menu button; the logo is the link to the landing page (never opens the menu)
const MENU_ORDER = `(() => {
	const menu = document.getElementById('main-menu');
	const entries = [...menu.querySelectorAll('.nav-btn')];
	// the toons sit under the account + language header (owner 2026-10-01); focus goes to the first toon
	const first = menu.querySelector('.nav--toons .nav-btn');
	const toggle = document.querySelector('[data-testid="menu-toggle"]');
	const logo = document.querySelector('[data-testid="logo-link"]');
	return {
		open: menu.classList.contains('open'), inert: menu.inert,
		controls: toggle?.getAttribute('aria-controls'), expanded: toggle?.getAttribute('aria-expanded'),
		first: first ? { tag: first.tagName, href: first.getAttribute('href'), text: first.textContent.trim(), toon: !!first.closest('.nav--toons') } : null,
		toonsFirst: !!menu.firstElementChild?.matches('.menu-header') && !!menu.firstElementChild?.nextElementSibling?.matches('.nav--toons'),
		scutoonsEntry: entries.some((e) => e.textContent.trim().toUpperCase() === 'SCUTOONS'),
		focusFirst: document.activeElement === first,
		focusToggle: document.activeElement === toggle,
		logo: logo ? { tag: logo.tagName, href: logo.getAttribute('href'), alt: logo.querySelector('img')?.alt } : null
	};
})()`;
for (const [w, h] of [[1440, 900], [390, 844]]) for (const l of ['de', 'en']) {
	const prefix = l === 'en' ? '/en' : '';
	const home = l === 'en' ? ['/en', '/en/'] : ['/'];
	await size(w, h);
	await open(`${viewer}${prefix}/${toon}`, l);
	const closed = await waitFor(`(() => { const s = ${MENU_ORDER}; return s.logo ? s : null; })()`, 5000);
	const p = [];
	if (!closed || closed.error) p.push(`no menu / logo ${JSON.stringify(closed)}`);
	else {
		if (closed.open || !closed.inert || closed.expanded !== 'false' || closed.controls !== 'main-menu') p.push(`closed state ${JSON.stringify({ open: closed.open, inert: closed.inert, expanded: closed.expanded, controls: closed.controls })}`);
		if (closed.logo.tag !== 'A' || !home.includes(closed.logo.href) || closed.logo.alt !== messages[l].menu.home) p.push(`logo ${JSON.stringify(closed.logo)}, expected a link to ${home.join(' / ')}`);
	}
	await evaluate(`document.querySelector('[data-testid="menu-toggle"]').click(), true`);
	const s = await waitFor(`(() => { const s = ${MENU_ORDER}; return s.open && s.focusFirst ? s : null; })()`, 3000);
	if (!s || s.error) p.push(`after opening: ${JSON.stringify(await evaluate(MENU_ORDER))}`);
	else {
		if (s.inert || s.expanded !== 'true') p.push(`open but inert ${s.inert} / aria-expanded ${s.expanded}`);
		if (!s.toonsFirst || !s.first?.toon || s.first.tag !== 'A' || s.first.href !== `${prefix}/${toon}`) p.push(`first entry ${JSON.stringify(s.first)}, toons first ${s.toonsFirst}`);
		if (s.scutoonsEntry) p.push('the "SCUTOONS" entry is still there');
	}
	await shot(`menu-order-${w}-${l}`);
	await send('Input.dispatchKeyEvent', { type: 'keyDown', key: 'Escape', code: 'Escape', windowsVirtualKeyCode: 27 });
	await send('Input.dispatchKeyEvent', { type: 'keyUp', key: 'Escape', code: 'Escape', windowsVirtualKeyCode: 27 });
	const esc = await waitFor(`(() => { const s = ${MENU_ORDER}; return !s.open && s.inert ? s : null; })()`, 3000);
	if (!esc || esc.error || !esc.focusToggle) p.push(`Escape: ${JSON.stringify(esc && { open: esc.open, focusToggle: esc.focusToggle })}`);
	// the logo goes to the landing page and never opens the menu
	await evaluate(`document.querySelector('[data-testid="logo-link"]').click(), true`);
	const landed = await waitFor(`(() => { const path = location.pathname; return ${JSON.stringify(home)}.includes(path) ? { path, open: document.getElementById('main-menu').classList.contains('open') } : null; })()`, 6000);
	if (!landed || landed.error || landed.open) p.push(`logo click → ${JSON.stringify(landed ?? (await evaluate('location.pathname')))}`);
	report(`${w} ${l}: menu = account/language, then toons, no "SCUTOONS" entry, inert when closed, focus on the first toon, Escape → menu button; logo → the landing page`, p);
}

// the season's music in its record's row (owner 2026-10-02): the record's cut-out (season.download.image) as the row's
// picture; desktop in the season column; decorative (alt ""), lazy. "Download the music" only for a reader who may
// download (owner 2026-10-06) – this fresh guest has no code: the record without the button
const musicImage = bundles.de.seasons[0].season.download?.image;
const MUSIC = `(async () => {
	const pic = document.querySelector('[data-testid="season-download-image"]');
	if (!pic) return null;
	const wrap = pic.closest('li') ?? pic.parentElement;
	const btn = wrap.querySelector('[data-testid="season-download"]') ?? document.querySelector('a[data-testid="season-download"]');
	wrap.scrollIntoView({ block: 'center' });
	const img = wrap.querySelector('[data-testid="season-download-image"]');
	for (let i = 0; i < 40 && img && !(img.complete && img.naturalWidth); i++) await new Promise((r) => setTimeout(r, 100));
	const box = (e) => { if (!e) return null; const r = e.getBoundingClientRect(); return { x: Math.round(r.x), y: Math.round(r.y), w: Math.round(r.width), h: Math.round(r.height) }; };
	return {
		img: img ? { ...box(img), src: img.currentSrc || img.src, alt: img.getAttribute('alt'), loading: img.loading, loaded: img.complete && img.naturalWidth > 0 } : null,
		btn: box(btn), text: btn?.textContent.trim(), href: btn && new URL(btn.href).pathname,
		inSeasonColumn: !!wrap.closest('.season-column'), viewport: document.documentElement.clientWidth
	};
})()`;
if (!musicImage) report('season 1 has a download.image (is the content imported?)', ['no download.image in the library answer']);
else for (const [w, h] of [[390, 844], [768, 1024], [1280, 800], [1440, 900]]) for (const l of ['de', 'en']) {
	const prefix = l === 'en' ? '/en' : '';
	await size(w, h);
	await open(`${viewer}${prefix}/${toon}`, l);
	const s = await waitFor(MUSIC, 8000);
	const p = [];
	if (!s || s.error || !s.img) p.push(`music block ${JSON.stringify(s)}`);
	else {
		if (!s.img.src.endsWith(`/${toon}/${musicImage}`) || s.img.alt !== '' || s.img.loading !== 'lazy' || !s.img.loaded) p.push(`image ${JSON.stringify(s.img)}`);
		if (s.btn) p.push(`a guest without a code sees "${s.text}" → ${s.href}`);
		if (w >= 1024) {
			if (!s.inSeasonColumn) p.push('not in the season column');
			if (s.img.w < 120 || s.img.w > 160) p.push(`record ${s.img.w} px wide (120-160 on desktop)`);
		} else {
			if (s.img.w < 96 || s.img.w > 200) p.push(`record ${s.img.w} px wide (96-200 on phone / tablet: the row's big picture)`);
		}
	}
	report(`${w} ${l}: the record (${musicImage}), no "${messages[l].season.download}" without a code`, p, JSON.stringify(s?.img));
	await shot(`music-${w}-${l}`);
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
		// the member's save game (one for every toon – progress row `scutoons`): set per state, put back exactly (a PUT merges)
		const saveGame = saveGameRow(JSON.parse(readFileSync(sessionFile, 'utf8')).email);
		const now = Date.now();
		const iso = (t) => new Date(t).toISOString();
		const cp = (n, position, isComplete, t) => ({ uri: `${toon}/chapter/${n}`, isComplete, position, created_at: iso(t), updated_at: iso(t) });
		const states = [
			{ name: 'prologue watched → "S1 K1 lesen"', progress: [cp(0, 1, true, now - 3000)], kind: 'start', text: fill(messages.de.start.read, { season: 1, chapter: 1 }), href: `/${toon}/chapter/1` },
			{ name: 'chapter 1 at 40 % → "S1 K1 weiterlesen" + rule', progress: [cp(0, 1, true, now - 3000), cp(1, 0.4, false, now - 2000)], kind: 'continue', text: fill(messages.de.start.continue, { season: 1, chapter: 1 }), href: `/${toon}/chapter/1` }
		];
		try {
			for (const st of states) {
				saveGame.set({ variables: {}, progress: st.progress, history: {}, map: { places: {}, routes: {}, positions: {} } });
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
			if (!saveGame.restore()) report('member: the save game is put back', ['the row differs']);
		}
	}
}

console.log(`\n${passed} passed, ${failed} failed`);
await done(failed ? 1 : 0);
