// Prologue check (chapter 0 of catpope season 1 = the music video "Roar Little Kitty", owner 2026-10-01): headless Chrome
// via CDP (own temp profile, no dependencies; Node 22 WebSocket), like scripts/check-locale.mjs.
//
// Per width (375, 1280) and locale (de, en), as a guest without a membership, from a fresh browser:
//   1. toon page: the start button offers the prologue ("Prolog ansehen" / "Watch the prologue" → /catpope/chapter/0),
//      the chapter list's first row is labelled "Prolog" / "Prologue" (not "#0"), no progress bar yet;
//   2. /catpope/chapter/0: no membership blocker, one player video (controls, sound, no loop / autoplay attribute, playsinline,
//      preload=metadata, the poster), the slice box has the video's ratio (no 390 × 625 crop), and the browser loads the
//      file of the reader's media quality (`video.variants`; the guest's setting is set per run: low at 375 → 480p,
//      medium at 1280 → 720p, high at 1600 → 1080p – checked in the network log too; Automatic, the account page and the
//      placement: scripts/check-media-quality.mjs);
//   3. it starts by itself once in view (this Chrome allows sound without a click; the muted start + "Ton an" under the
//      normal policy is scripts/check-next-bar.mjs): sound (decoded audio bytes); the chapter's music waits while it plays,
//      starts after a pause and pauses again when it plays (chapter 0 has no audio of its own: the check injects chapter
//      1's music into the chapter-0 answer);
//   4. seeking to 200 s works (the CDN answers ranges with 206; `video.seekable` covers the video);
//   5. back on the toon page: still the prologue, the list's progress bar shows how far it was watched;
//   6. watched to its end (`ended`) → the start button says "S1 K1 lesen" / "Read S1 C1" (→ chapter 1), full bar.
// Then the test user (agent@scutoons.test, `npm run dev:test-user`) as a member: chapter 0 plays, its finished
// checkpoint reaches /api/progress/catpope; the member's previous server progress is restored afterwards.
//
// The local CDN (scripts/dev/media-serve.mjs) answers ranges.
//
// usage: node scripts/check-prolog.mjs [--viewer=http://localhost:8000] [--api=http://localhost:9000/api]
//        [--cdn=http://localhost:8001] [--shots=<dir>] [--only=375-de] [--no-member] [--verbose]
// Needs the dev servers (npm run dev). Exit code 1 on any failure.
import { spawn } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { local } from './dev/ports.mjs';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const opt = Object.fromEntries(process.argv.slice(2).filter((a) => a.startsWith('--')).map((a) => { const [k, ...v] = a.slice(2).split('='); return [k, v.length ? v.join('=') : true]; }));
const viewer = String(opt.viewer || local('viewer')).replace(/\/$/, '');
const api = String(opt.api || local('api', '/api')).replace(/\/$/, '');
const cdn = String(opt.cdn || local('cdn')).replace(/\/$/, '');
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

// ── the CDN: ranges? ────────────────────────────────────────────────────────────────────────────────────────────────
const chapter0 = await (await fetch(`${api}/library/toons/${toon}/chapters/0?locale=de`)).json();
const playerSlice = chapter0?.groups?.[0]?.slices?.[0];
if (!playerSlice?.video?.player) { console.error('chapter 0 has no player video slice (is the content imported?)'); process.exit(2); }
const videoPath = `${toon}/${playerSlice.video.src}`;
{
	const res = await fetch(`${cdn}/${videoPath}`, { headers: { Range: 'bytes=0-99' } });
	await res.body?.cancel();
	if (res.status !== 206) console.log(`note: ${cdn} answers ranges with ${res.status} – seeking will fail (is the dev CDN scripts/dev/media-serve.mjs running?)`);
}
// chapter 1's music, injected into chapter 0 to check that the player pauses it
const chapter1 = await (await fetch(`${api}/library/toons/${toon}/chapters/1?locale=de`)).json();
const music = chapter1.audio;
const sliceMusic = chapter1.groups?.[0]?.slices?.[0]?.audio;

// ── Chrome ──────────────────────────────────────────────────────────────────────────────────────────────────────────
const CHROME = process.env.CHROME || ['C:/Program Files/Google/Chrome/Application/chrome.exe', 'C:/Program Files (x86)/Google/Chrome/Application/chrome.exe', 'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe', '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome', '/usr/bin/google-chrome'].find((p) => existsSync(p));
if (!CHROME) { console.error('Chrome not found (set CHROME)'); process.exit(2); }
const port = 9400 + Math.floor(Math.random() * 400);
const profile = mkdtempSync(join(tmpdir(), 'check-prolog-'));
// sound is decoded (and counted) but not heard; media may start without a gesture (the check presses play by script)
const chrome = spawn(CHROME, ['--headless=new', `--remote-debugging-port=${port}`, `--user-data-dir=${profile}`, '--no-first-run', '--no-default-browser-check', '--disable-extensions', '--mute-audio', '--autoplay-policy=no-user-gesture-required', '--window-size=1280,900', 'about:blank'], { stdio: 'ignore' });
let restoreMember = null;
async function done(code) {
	if (restoreMember) await restoreMember().catch((e) => console.log(`  could not restore the member's progress: ${e}`));
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
let seq = 0; const pending = new Map(); let lastNet = Date.now(); let videoStatuses = []; let injectMusic = false;
const send = (method, params = {}) => new Promise((r) => { const i = ++seq; pending.set(i, r); ws.send(JSON.stringify({ id: i, method, params })); });
ws.addEventListener('message', async (e) => {
	const m = JSON.parse(e.data), p = m.params;
	if (m.id && pending.has(m.id)) { pending.get(m.id)(m); pending.delete(m.id); return; }
	if (m.method?.startsWith('Network.')) lastNet = Date.now();
	if (m.method === 'Network.responseReceived' && isVideo(p.response.url)) videoStatuses.push(p.response.status);
	if (m.method === 'Network.requestWillBeSent' && isVideo(p.request.url)) videoUrls.push(p.request.url);
	if (m.method === 'Fetch.requestPaused') {
		const url = p.request.url;
		if (url.includes(`/library/toons/${toon}/chapters/0`) && p.responseStatusCode !== undefined) {
			if (!injectMusic || p.responseStatusCode !== 200) return send('Fetch.continueResponse', { requestId: p.requestId });
			const body = await send('Fetch.getResponseBody', { requestId: p.requestId });
			const raw = body.result.base64Encoded ? Buffer.from(body.result.body, 'base64').toString('utf8') : body.result.body;
			const chapter = JSON.parse(raw);
			chapter.audio = music;
			chapter.groups[0].slices[0].audio = sliceMusic;
			const headers = p.responseHeaders.filter((h) => !/^(content-length|etag|content-encoding)$/i.test(h.name));
			return send('Fetch.fulfillRequest', { requestId: p.requestId, responseCode: 200, responseHeaders: headers, body: Buffer.from(JSON.stringify(chapter)).toString('base64') });
		}
		return send('Fetch.continueRequest', { requestId: p.requestId });
	}
});
const evaluate = async (expression) => { const r = await send('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true }); return r.result?.exceptionDetails ? { error: r.result.exceptionDetails.exception?.description ?? r.result.exceptionDetails.text } : r.result?.result?.value; };
await send('Page.enable'); await send('Runtime.enable'); await send('Network.enable');
await send('Network.setCacheDisabled', { cacheDisabled: true });
await send('Fetch.enable', { patterns: [
	{ urlPattern: `*/library/toons/${toon}/chapters/0*`, requestStage: 'Response' }
] });
// every media element that ever plays (the music's <audio> elements are not in the DOM)
await send('Page.addScriptToEvaluateOnNewDocument', { source: `(() => {
	window.__media = [];
	const play = HTMLMediaElement.prototype.play;
	HTMLMediaElement.prototype.play = function () { if (!window.__media.includes(this)) window.__media.push(this); return play.apply(this, arguments); };
})();` });
async function idle(quietMs = 700, maxMs = 15000) {
	const t0 = Date.now();
	while (Date.now() - t0 < maxMs) { if (Date.now() - lastNet > quietMs) return; await sleep(100); }
}
async function waitFor(js, ms = 15000) {
	const t0 = Date.now();
	let r;
	while (Date.now() - t0 < ms) { r = await evaluate(js); if (r && !r.error) return r; await sleep(200); }
	return r ?? null;
}
async function open(url) { lastNet = Date.now(); await send('Page.navigate', { url }); await sleep(300); await idle(); }
async function shot(name, { list = false } = {}) {
	if (!shots) return;
	// the toon page: the start button and the chapter list in view (a full-page capture draws the fixed overlays wrongly)
	if (list) { await evaluate(`document.querySelector('[data-testid="start-button"], .list-column')?.scrollIntoView({ block: 'start' }), window.scrollBy(0, -80), true`); await sleep(400); }
	const r = await send('Page.captureScreenshot', { format: 'png' });
	writeFileSync(join(shots, `${name}.png`), Buffer.from(r.result.data, 'base64'));
}

// ── checks ──────────────────────────────────────────────────────────────────────────────────────────────────────────
let failed = 0, passed = 0;
function report(name, problems, detail = '') {
	if (problems.length) { failed++; console.log(`  FAIL  ${name}`); for (const p of problems) console.log(`        ${p}`); }
	else { passed++; console.log(`  ok    ${name}${opt.verbose && detail ? `  ${detail}` : ''}`); }
}
const TOON_STATE = `(() => {
	const b = document.querySelector('[data-testid="start-button"], [data-testid="hero-start"]');
	const rows = [...document.querySelectorAll('.toon-overview .chapter-item')];
	const first = rows[0];
	const bar = first?.querySelector('[data-testid="chapter-progress"]');
	return b && rows.length ? { state: b.dataset.state, text: b.textContent.trim(), href: new URL(b.href).pathname, title: b.title,
		firstLabel: first.querySelector('[data-testid="chapter-number"]')?.textContent.trim(), firstTitle: first.querySelector('.chapter-title')?.textContent.trim(),
		secondLabel: rows[1]?.querySelector('[data-testid="chapter-number"]')?.textContent.trim(),
		bar: bar ? Number(bar.dataset.percent) : null, lang: document.documentElement.lang } : null;
})()`;
const VIDEO_STATE = `(() => {
	const v = document.querySelector('video[data-player]');
	if (!v) return null;
	const box = v.getBoundingClientRect();
	const music = (window.__media || []).filter((m) => m !== v && /\\.(mp3|m4a|ogg|opus)(\\?|$)/i.test(m.currentSrc || m.src));
	return { controls: v.controls, muted: v.muted, loop: v.loop, autoplay: v.autoplay, playsinline: v.hasAttribute('playsinline'),
		preload: v.preload, poster: v.poster, src: v.currentSrc || v.querySelector('source')?.src, paused: v.paused, ended: v.ended,
		time: v.currentTime, duration: v.duration, readyState: v.readyState, audioBytes: v.webkitAudioDecodedByteCount ?? null,
		seekableEnd: v.seekable.length ? v.seekable.end(v.seekable.length - 1) : 0, width: box.width, height: box.height,
		blocker: !!document.querySelector('.view-membership-blocker'), music: music.map((m) => ({ paused: m.paused })) };
})()`;
const checkpoint = (n) => `(() => { try { const s = JSON.parse(localStorage.getItem('scutoons_game_${toon}') || '{}'); return (s.progress || []).find((c) => c.uri === '${toon}/chapter/${n}') || 'none'; } catch (e) { return 'none'; } })()`;
const near = (a, b, tol) => Math.abs(a - b) <= tol;

async function toonPage(prefix, locale, label, expect) {
	await open(`${viewer}${prefix}/${toon}`);
	const s = await waitFor(TOON_STATE);
	const msg = messages[locale];
	const problems = [];
	if (!s || s.error) return report(label, [`toon page: ${s?.error ?? 'no start button / chapter list'}`]);
	if (s.lang !== locale) problems.push(`<html lang="${s.lang}">`);
	if (s.firstLabel !== msg.chapter.prologue) problems.push(`first row labelled "${s.firstLabel}", expected "${msg.chapter.prologue}"`);
	if (s.firstTitle !== 'Roar Little Kitty') problems.push(`first row title "${s.firstTitle}"`);
	if (s.secondLabel !== '#1') problems.push(`second row labelled "${s.secondLabel}", expected "#1"`);
	if (expect.state === 'prologue') {
		if (s.state !== 'prologue' || s.text !== msg.start.prologue || s.href !== `${prefix}/${toon}/chapter/0`) problems.push(`start button ${s.state} "${s.text}" → ${s.href}, expected "${msg.start.prologue}" → ${prefix}/${toon}/chapter/0`);
		if (s.title !== fill(msg.start.prologue_title, { season: 1, title: 'Roar Little Kitty' })) problems.push(`title "${s.title}"`);
	} else {
		const text = fill(msg.start.read, { season: 1, chapter: 1 });
		if (s.state !== expect.state || s.text !== text || s.href !== `${prefix}/${toon}/chapter/1`) problems.push(`start button ${s.state} "${s.text}" → ${s.href}, expected ${expect.state} "${text}" → ${prefix}/${toon}/chapter/1`);
	}
	if (expect.bar === null ? s.bar !== null : s.bar === null || !near(s.bar, expect.bar, expect.tol ?? 0)) problems.push(`first row progress bar ${s.bar}, expected ${expect.bar}`);
	report(label, problems, `${s.state} "${s.text}", row "${s.firstLabel}" bar ${s.bar}`);
}

async function run(width, locale) {
	const prefix = locale === 'en' ? '/en' : '';
	const name = `${width}-${locale}`;
	console.log(`${name} (guest, no membership)`);
	await send('Emulation.setDeviceMetricsOverride', { width, height: width < 768 ? 812 : 900, deviceScaleFactor: 1, mobile: width < 768 });
	await send('Network.clearBrowserCookies');
	await send('Network.setCookie', { name: 'PARAGLIDE_LOCALE', value: locale, url: viewer, path: '/' });
	await open(`${viewer}/`);
	await evaluate(`localStorage.clear(), sessionStorage.clear(), localStorage.setItem('scutoons_media_quality', '${qualityFor(width)}'), true`);

	// 1. fresh: the prologue first
	await toonPage(prefix, locale, 'fresh toon page: start button "prologue", list row "Prolog", no bar', { state: 'prologue', bar: null });
	await shot(`prolog-${name}-1-toon-fresh`, { list: true });

	// 2. the chapter as a guest
	injectMusic = true;
	videoUrls = [];
	await open(`${viewer}${prefix}/${toon}/chapter/0`);
	let v = await waitFor(`(() => { const s = ${VIDEO_STATE}; return s && s.readyState >= 1 ? s : null; })()`);
	const wanted = expectedSource(width);
	{
		const problems = [];
		if (!v || v.error) problems.push(`no player video: ${v?.error ?? (await evaluate('document.body.innerText.slice(0, 200)'))}`);
		else {
			if (v.blocker) problems.push('membership blocker shown');
			if (!v.controls || v.muted || v.loop || v.autoplay || !v.playsinline || v.preload !== 'metadata') problems.push(`attributes ${JSON.stringify({ controls: v.controls, muted: v.muted, loop: v.loop, autoplay: v.autoplay, playsinline: v.playsinline, preload: v.preload })}`);
			if (!v.poster.endsWith(playerSlice.image)) problems.push(`poster ${v.poster}`);
			if (!v.src?.endsWith(wanted)) problems.push(`loads ${v.src}, expected ${wanted} at ${width} px`);
			const fetched = [...new Set(videoUrls.map(toonPath))];
			if (fetched.length !== 1 || fetched[0] !== wanted) problems.push(`network: ${fetched.join(', ') || 'no video request'}, expected only ${wanted}`);
			const ratio = playerSlice.height / playerSlice.width;
			if (!near(v.height, v.width * ratio, 2)) problems.push(`box ${v.width} × ${v.height}, expected the video's ratio (${playerSlice.width} × ${playerSlice.height})`);
		}
		report(`chapter 0 as a guest: no blocker, one player video (controls, sound, no loop/autoplay attribute, playsinline, metadata), video ratio, ${wanted.replace(/^.*\//, '')}`, problems, v && !v.error ? `${Math.round(v.width)} × ${Math.round(v.height)}, ${v.src}` : '');
	}
	await shot(`prolog-${name}-2-chapter`);
	if (!v || v.error) { injectMusic = false; return; }

	// 3. it starts by itself with sound; the music waits while it plays, starts after a pause, pauses when it plays again
	videoStatuses = [];
	let playing = await waitFor(`(() => { const s = ${VIDEO_STATE}; return s && !s.paused && s.time > 1.5 && s.audioBytes > 0 ? s : null; })()`, 25000);
	{
		const problems = [];
		if (!playing || playing.error) problems.push(`not playing by itself with sound: ${JSON.stringify(await evaluate(VIDEO_STATE))}`);
		else if (playing.music.some((m) => !m.paused)) problems.push(`music while the video plays: ${JSON.stringify(playing.music)}`);
		report('starts by itself with sound (decoded audio), no music meanwhile', problems, playing ? `t=${playing.time.toFixed(1)} s, ${playing.audioBytes} audio bytes` : '');
	}
	await evaluate(`document.querySelector('video[data-player]').pause(), true`);
	const musicOn = await waitFor(`(() => { const s = ${VIDEO_STATE}; return s && s.paused && s.music.length && s.music.every((m) => !m.paused) ? s : null; })()`, 25000);
	await evaluate(`document.querySelector('video[data-player]').play().then(() => true)`);
	playing = await waitFor(`(() => { const s = ${VIDEO_STATE}; return s && !s.paused && s.music.length && s.music.every((m) => m.paused) ? s : null; })()`, 10000);
	{
		const problems2 = [];
		if (!musicOn || musicOn.error) problems2.push('the injected chapter music never started after a pause');
		else if (!playing || playing.error) problems2.push(`music while the video plays: ${JSON.stringify((await evaluate(VIDEO_STATE))?.music)}`);
		report('the chapter music starts after a pause and pauses while the video plays', problems2, playing ? JSON.stringify(playing.music) : '');
	}
	await shot(`prolog-${name}-3-playing`);

	// 4. seeking
	await evaluate(`(() => { const v = document.querySelector('video[data-player]'); return new Promise((ok) => { v.addEventListener('seeked', () => ok(true), { once: true }); v.currentTime = 200; }); })()`);
	const seeked = await waitFor(`(() => { const s = ${VIDEO_STATE}; return s && s.time >= 200 && s.time < 215 && !s.paused ? s : null; })()`, 20000);
	{
		const problems = [];
		if (!seeked || seeked.error) problems.push(`after seeking to 200 s: ${JSON.stringify(await evaluate(VIDEO_STATE))}`);
		else if (seeked.seekableEnd < seeked.duration - 1) problems.push(`seekable only up to ${seeked.seekableEnd} of ${seeked.duration} s`);
		if (!videoStatuses.includes(206)) problems.push(`no 206 answer for the video (statuses ${[...new Set(videoStatuses)].join(', ')})`);
		report('seeking to 200 s works (206 ranges, seekable to the end)', problems, seeked ? `t=${seeked.time.toFixed(1)} s, seekable ${seeked.seekableEnd.toFixed(0)} s, statuses ${[...new Set(videoStatuses)].join(',')}` : '');
	}
	// the pause gives the music back
	await evaluate(`document.querySelector('video[data-player]').pause(), true`);
	const resumed = await waitFor(`(() => { const s = ${VIDEO_STATE}; return s && s.paused && s.music.length && s.music.every((m) => !m.paused) ? s : null; })()`, 5000);
	report('the music resumes after a pause', resumed && !resumed.error ? [] : [`music after the pause: ${JSON.stringify((await evaluate(VIDEO_STATE))?.music)}`]);
	const partial = await waitFor(`(() => { const c = ${checkpoint(0)}; return c !== 'none' ? c : null; })()`, 3000);
	const expectedShare = Math.floor((200 / (seeked?.duration || 475)) * 100);
	report('progress while watching: a checkpoint, not finished', partial && !partial.error && !partial.isComplete && near(partial.position * 100, expectedShare, 3) ? [] : [`checkpoint ${JSON.stringify(partial)}`], JSON.stringify(partial));
	injectMusic = false;

	// 5. back on the toon page: still the prologue, the bar shows how far
	await toonPage(prefix, locale, 'toon page after watching part of it: still "prologue", row bar = how far', { state: 'prologue', bar: Math.max(1, Math.round((partial?.position ?? 0) * 100)), tol: 1 });
	await shot(`prolog-${name}-4-toon-partial`, { list: true });

	// 6. watched to its end
	await open(`${viewer}${prefix}/${toon}/chapter/0`);
	await waitFor(VIDEO_STATE);
	await evaluate(`(() => { const v = document.querySelector('video[data-player]'); return new Promise((ok, no) => {
		const go = () => { v.addEventListener('seeked', () => v.play().then(() => ok(true), no), { once: true }); v.currentTime = v.duration - 1.5; };
		v.readyState >= 1 ? go() : v.addEventListener('loadedmetadata', go, { once: true });
		v.load?.call && v.readyState < 1 && v.load();
	}); })()`);
	const ended = await waitFor(`(() => { const s = ${VIDEO_STATE}; return s && s.ended ? s : null; })()`, 20000);
	const finished = await waitFor(`(() => { const c = ${checkpoint(0)}; return c !== 'none' && c.isComplete && c.position === 1 ? c : null; })()`, 3000);
	report('watched to its end → chapter 0 finished (checkpoint position 1)', ended && finished && !finished.error ? [] : [`ended ${!!ended}, checkpoint ${JSON.stringify(await evaluate(checkpoint(0)))}`]);
	await shot(`prolog-${name}-5-ended`);
	await toonPage(prefix, locale, 'toon page after the prologue: "S1 K1 lesen" → chapter 1, row bar 100', { state: 'start', bar: 100 });
	await shot(`prolog-${name}-6-toon-after`, { list: true });

	// chapter 1 stays members-only
	await open(`${viewer}${prefix}/${toon}/chapter/1`);
	const blocked = await waitFor(`document.querySelector('.view-membership-blocker') ? true : null`, 8000);
	report('chapter 1 still shows the membership blocker to a guest', blocked === true ? [] : ['no blocker on chapter 1']);
}

// the media quality each run sets for the guest (localStorage `scutoons_media_quality`), and the file it loads
function qualityFor(width) {
	return width < 768 ? 'low' : width < 1600 ? 'medium' : 'high';
}
function expectedSource(width) {
	return playerSlice.video.variants?.[qualityFor(width)] ?? playerSlice.video.src;
}
let videoUrls = [];

const combos = [[375, 'de'], [375, 'en'], [1280, 'de'], [1280, 'en']].filter(([w, l]) => !opt.only || String(opt.only).split(',').includes(`${w}-${l}`));
for (const [w, l] of combos) await run(w, l);

// the high setting takes 1080p
if (!opt.only) {
	console.log('1600-en (resolution only)');
	await send('Emulation.setDeviceMetricsOverride', { width: 1600, height: 1000, deviceScaleFactor: 1, mobile: false });
	await evaluate(`localStorage.setItem('scutoons_media_quality', '${qualityFor(1600)}'), true`);
	videoUrls = [];
	await open(`${viewer}/en/${toon}/chapter/0`);
	const v = await waitFor(`(() => { const s = ${VIDEO_STATE}; return s && s.readyState >= 1 ? s : null; })()`);
	const wanted = expectedSource(1600);
	const fetched = [...new Set(videoUrls.map(toonPath))];
	report(`1600 px loads ${wanted.replace(/^.*\//, '')}`, v && !v.error && v.src.endsWith(wanted) && fetched.length === 1 && fetched[0] === wanted ? [] : [`loads ${v?.src}, network ${fetched.join(', ')}`]);
	await shot('prolog-1600-en-chapter');
}

// ── a member (the agents' test user) ───────────────────────────────────────────────────────────────────────────────
if (!opt['no-member']) {
	console.log('375-de (member: agent@scutoons.test)');
	const sessionFile = join(root, 'logs/dev/test-user.json');
	const token = existsSync(sessionFile) ? JSON.parse(readFileSync(sessionFile, 'utf8')).token : null;
	const me = token ? await fetch(`${api}/me/`, { headers: { Authorization: `Bearer ${token}` } }) : null;
	if (!me?.ok) report('member', [`no working test-user session (run \`npm run dev:test-user\`)`]);
	else {
		const auth = { Authorization: `Bearer ${token}`, 'Content-Type': 'application/json' };
		const before = await (await fetch(`${api}/progress/${toon}`, { headers: auth })).json();
		restoreMember = async () => {
			restoreMember = null;
			await fetch(`${api}/progress/${toon}`, { method: 'PUT', headers: auth, body: JSON.stringify({ state: before?.state ?? {} }) });
		};
		await send('Emulation.setDeviceMetricsOverride', { width: 375, height: 812, deviceScaleFactor: 1, mobile: true });
		await send('Network.clearBrowserCookies');
		await send('Network.setCookie', { name: 'PARAGLIDE_LOCALE', value: 'de', url: viewer, path: '/' });
		await open(`${viewer}/`);
		await evaluate(`localStorage.clear(), localStorage.setItem('scutoons_session', ${JSON.stringify(token)}), true`);
		await open(`${viewer}/${toon}/chapter/0`);
		const v = await waitFor(VIDEO_STATE);
		await evaluate(`(() => { const v = document.querySelector('video[data-player]'); return new Promise((ok, no) => {
			const go = () => { v.addEventListener('seeked', () => v.play().then(() => ok(true), no), { once: true }); v.currentTime = v.duration - 1.5; };
			v.readyState >= 1 ? go() : v.addEventListener('loadedmetadata', go, { once: true });
		}); })()`);
		const ended = await waitFor(`(() => { const s = ${VIDEO_STATE}; return s && s.ended ? s : null; })()`, 20000);
		await shot('prolog-member-375-de-ended');
		// the member's progress reaches the server (debounced push)
		let server = null;
		for (let i = 0; i < 30 && !server; i++) {
			const p = await (await fetch(`${api}/progress/${toon}`, { headers: auth })).json();
			const state = typeof p.state === 'string' ? JSON.parse(p.state) : p.state;
			server = (state?.progress ?? []).find((c) => c.uri === `${toon}/chapter/0` && c.isComplete) ?? null;
			if (!server) await sleep(500);
		}
		report('member: chapter 0 plays to its end, the finished checkpoint reaches /api/progress', v && !v.error && !v.blocker && ended && server ? [] : [`video ${!!v && !v.error}, ended ${!!ended}, server checkpoint ${JSON.stringify(server)}`]);
		await restoreMember();
	}
}

console.log(`\n${passed} passed, ${failed} failed`);
await done(failed ? 1 : 0);
