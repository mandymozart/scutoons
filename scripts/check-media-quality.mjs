// Media quality + video placement check (owner 2026-10-01, agents/DESIGN.md "Media quality" / "Video placement"):
// headless Chrome via CDP (own temp profile, no dependencies; Node 22 WebSocket), like scripts/check-prolog.mjs.
//
//   1. guest, 390 and 1440 px: the setting low / medium / high (localStorage `scutoons_media_quality`) → the prologue
//      (catpope chapter 0) loads exactly the 480p / 720p / 1080p file (`currentSrc` and the network log);
//   2. Automatic: with the Network Information API (Chrome) unthrottled → the level for `navigator.connection.downlink`;
//      throttled by CDP (Network.emulateNetworkConditions, a slow 3G) → low; without the API (hidden by a script, like
//      Safari/Firefox) the measured download of the low file decides: unthrottled → high, throttled → low;
//   3. a quality switch while it plays (another tab changes the setting: a `storage` event) → the other file, same
//      position (± 2 s), still playing;
//   4. the account page as a guest: the four buttons, "Hoch" → stored on the device, kept after a reload;
//   5. the test user (agent@scutoons.test, `npm run dev:test-user`): the account page shows the server value; choosing
//      "Niedrig" → `PUT /me/` → `GET /me/ user.media_quality` = low; the chapter then loads 480p although this device says
//      high (the account wins); the member's previous value is restored;
//   6. placement (`video.viewport`, injected into the chapter-0 answer by CDP Fetch): 3 positions × 4 fits at 390 × 844
//      and 1440 × 900: the box = the whole viewport (no sideways scroll), contain = the video's own box placed top / centre
//      / bottom, cover = filling with object-position; the poster too (reduced motion: the poster only); the "Ton an"
//      button on the video; screenshots;
//   7. controls on / off (a player without controls: a click plays / pauses);
//   8. the follow-up bar after the full-viewport player ended: shown, fixed at the bottom of the viewport; autoplay muted +
//      "Ton an" under the normal autoplay policy;
//   9. the creator's "Hintergrund" tab (test user, nothing saved): the 480p / 720p / 1080p fields with the default, player
//      + controls, "Bildschirmfüllend" → the 4 × 3 placement picker, the phone / desktop preview and "Füllt den Bildschirm ·
//      Seitenverhältnis …"; in the column the ratio select (9:16 → 390 × 693, custom 1:5 → clamped to 1:4).
//
// usage: node scripts/check-media-quality.mjs [--viewer=http://localhost:8000] [--api=http://localhost:9000/api]
//        [--creator=http://localhost:10000] [--shots=<dir>] [--verbose]
//        [--only=network,auto,switch,account,member,placement,controls,bar,creator]
// Needs the dev servers (npm run dev) and the media in media/. Exit code 1 on any failure.
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
const only = opt.only ? String(opt.only).split(',') : null;
const step = (name) => !only || only.includes(name);
const toon = 'catpope';

// a video request's path in the toon's CDN folder (agents/ASSETS.md "Folder convention": videos live in their season /
// chapter folder, no `videos/` folder any more) – compared with the content's toon-relative paths
const isVideo = (url) => /\.(mp4|webm)(\?|#|$)/.test(url);
const toonPath = (url) => url.replace(/[?#].*$/, '').replace(new RegExp(`^.*?/${toon}/`), '');
const KEY = 'scutoons_media_quality';
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const messages = Object.fromEntries(['de', 'en'].map((l) => [l, JSON.parse(readFileSync(join(root, `viewer/messages/${l}.json`), 'utf8'))]));

const chapter0 = await (await fetch(`${api}/library/toons/${toon}/chapters/0?locale=de`)).json();
const playerSlice = chapter0?.groups?.[0]?.slices?.[0];
const variants = playerSlice?.video?.variants;
if (!playerSlice?.video?.player || !variants?.low || !variants?.medium || !variants?.high) {
	console.error(`chapter 0 has no player video with low/medium/high variants (is the content migrated? ${JSON.stringify(playerSlice?.video)})`);
	process.exit(2);
}
const ratio = playerSlice.width / playerSlice.height;
const file = (level) => variants[level];

// ── Chrome ──────────────────────────────────────────────────────────────────────────────────────────────────────────
const CHROME = process.env.CHROME || ['C:/Program Files/Google/Chrome/Application/chrome.exe', 'C:/Program Files (x86)/Google/Chrome/Application/chrome.exe', 'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe', '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome', '/usr/bin/google-chrome'].find((p) => existsSync(p));
if (!CHROME) { console.error('Chrome not found (set CHROME)'); process.exit(2); }
const port = 9400 + Math.floor(Math.random() * 400);
const profile = mkdtempSync(join(tmpdir(), 'check-media-quality-'));
// the normal autoplay rule (sound needs a gesture): an autoplaying player starts muted with "Ton an"
const chrome = spawn(CHROME, ['--headless=new', `--remote-debugging-port=${port}`, `--user-data-dir=${profile}`, '--no-first-run', '--no-default-browser-check', '--disable-extensions', '--mute-audio', '--autoplay-policy=document-user-activation-required', '--hide-scrollbars', '--window-size=1440,900', 'about:blank'], { stdio: 'ignore' });
let restoreMember = null;
async function done(code) {
	if (restoreMember) await restoreMember().catch((e) => console.log(`  could not restore the member's setting: ${e}`));
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
let seq = 0; const pending = new Map(); let lastNet = Date.now();
/** video requests of the current page: { url, range } */
let videoRequests = [];
/** what the chapter-0 answer gets patched with (null = unchanged) */
let patchVideo = null;
/** slice fields to patch with it (a frame's width / height) */
let patchSlice = null;
const send = (method, params = {}) => new Promise((r) => { const i = ++seq; pending.set(i, r); ws.send(JSON.stringify({ id: i, method, params })); });
ws.addEventListener('message', async (e) => {
	const m = JSON.parse(e.data), p = m.params;
	if (m.id && pending.has(m.id)) { pending.get(m.id)(m); pending.delete(m.id); return; }
	if (m.method?.startsWith('Network.')) lastNet = Date.now();
	if (m.method === 'Network.requestWillBeSent' && isVideo(p.request.url)) videoRequests.push({ url: p.request.url, range: p.request.headers?.Range ?? p.request.headers?.range ?? null, type: p.type });
	if (m.method === 'Fetch.requestPaused') {
		if (p.responseStatusCode === undefined || p.responseStatusCode !== 200 || !patchVideo) return send('Fetch.continueResponse', { requestId: p.requestId });
		const body = await send('Fetch.getResponseBody', { requestId: p.requestId });
		const raw = body.result.base64Encoded ? Buffer.from(body.result.body, 'base64').toString('utf8') : body.result.body;
		const chapter = JSON.parse(raw);
		Object.assign(chapter.groups[0].slices[0].video, patchVideo);
		for (const [k, v] of Object.entries(patchVideo)) if (v === undefined) delete chapter.groups[0].slices[0].video[k];
		if (patchSlice) Object.assign(chapter.groups[0].slices[0], patchSlice);
		const headers = p.responseHeaders.filter((h) => !/^(content-length|etag|content-encoding)$/i.test(h.name));
		return send('Fetch.fulfillRequest', { requestId: p.requestId, responseCode: 200, responseHeaders: headers, body: Buffer.from(JSON.stringify(chapter)).toString('base64') });
	}
});
const evaluate = async (expression) => { const r = await send('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true }); return r.result?.exceptionDetails ? { error: r.result.exceptionDetails.exception?.description ?? r.result.exceptionDetails.text } : r.result?.result?.value; };
await send('Page.enable'); await send('Runtime.enable'); await send('Network.enable');
await send('Network.setCacheDisabled', { cacheDisabled: true });
await send('Fetch.enable', { patterns: [{ urlPattern: `*/library/toons/${toon}/chapters/0*`, requestStage: 'Response' }] });
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
async function shot(name) {
	if (!shots) return;
	const r = await send('Page.captureScreenshot', { format: 'png' });
	writeFileSync(join(shots, `${name}.png`), Buffer.from(r.result.data, 'base64'));
}
async function viewport(width, height) {
	await send('Emulation.setDeviceMetricsOverride', { width, height, deviceScaleFactor: 1, mobile: width < 768 });
}
async function click(x, y) {
	for (const type of ['mousePressed', 'mouseReleased']) await send('Input.dispatchMouseEvent', { type, x, y, button: 'left', clickCount: 1 });
}
/** a fresh guest: no session, the locale cookie, an empty localStorage (+ the given setting) */
async function guest(setting = null, locale = 'de') {
	await send('Network.clearBrowserCookies');
	await send('Network.setCookie', { name: 'PARAGLIDE_LOCALE', value: locale, url: viewer, path: '/' });
	await open(`${viewer}/`);
	await evaluate(`localStorage.clear(), sessionStorage.clear(), ${setting ? `localStorage.setItem('${KEY}', '${setting}'),` : ''} true`);
}
const throttle = (on) => send('Network.emulateNetworkConditions', on
	? { offline: false, latency: 400, downloadThroughput: (400 * 1000) / 8, uploadThroughput: (400 * 1000) / 8, connectionType: 'cellular3g' }
	: { offline: false, latency: 0, downloadThroughput: -1, uploadThroughput: -1 });

// ── checks ──────────────────────────────────────────────────────────────────────────────────────────────────────────
let failed = 0, passed = 0;
function report(name, problems, detail = '') {
	if (problems.length) { failed++; console.log(`  FAIL  ${name}`); for (const p of problems) console.log(`        ${p}`); }
	else { passed++; console.log(`  ok    ${name}${opt.verbose && detail ? `  ${detail}` : ''}`); }
}
const VIDEO = `(() => {
	const v = document.querySelector('video[data-player]');
	if (!v) return null;
	const r = v.getBoundingClientRect();
	const box = v.closest('.video-box')?.getBoundingClientRect();
	const btn = document.querySelector('[data-testid="player-sound-on"]')?.getBoundingClientRect();
	const cs = getComputedStyle(v);
	return { src: v.currentSrc || v.getAttribute('src') || '', readyState: v.readyState, paused: v.paused, ended: v.ended, muted: v.muted,
		time: v.currentTime, duration: v.duration, controls: v.controls, poster: v.poster,
		rect: { x: r.x, y: r.y, w: r.width, h: r.height }, box: box && { x: box.x, y: box.y, w: box.width, h: box.height },
		fit: cs.objectFit, objectPosition: cs.objectPosition,
		button: btn && { x: btn.x + btn.width / 2, y: btn.y + btn.height / 2 },
		vw: document.documentElement.clientWidth, vh: window.innerHeight, scrollWidth: document.documentElement.scrollWidth };
})()`;
/** the files the `<video>` requested (type Media), and the "Automatic" probe downloads (a fetch with a Range) */
const fetched = () => [...new Set(videoRequests.filter((r) => r.type === 'Media').map((r) => toonPath(r.url)))];
const probes = () => videoRequests.filter((r) => r.type === 'Fetch').map((r) => toonPath(r.url));
async function chapterVideo(prefix = '') {
	videoRequests = [];
	await open(`${viewer}${prefix}/${toon}/chapter/0`);
	return waitFor(`(() => { const s = ${VIDEO}; return s && s.src && s.readyState >= 1 ? s : null; })()`, 20000);
}
const near = (a, b, tol) => Math.abs(a - b) <= tol;
/**
 * The toon page loaded at full speed (the dev server's hundreds of modules would take minutes on a slow 3G), then the
 * network throttled (or not) and the start button "Prolog ansehen" → chapter 0 inside the app; "Automatic" is decided
 * once per page load, when the first video slice needs it – i.e. now, on the throttled network.
 */
let lastNetInfo = null;
async function viaToonPage(slow) {
	await open(`${viewer}/${toon}`);
	await waitFor(`document.querySelector('a[href$="/${toon}/chapter/0"]') ? true : null`);
	await throttle(slow);
	await sleep(500);
	lastNetInfo = await evaluate(`(() => { const c = navigator.connection; return c ? { effectiveType: c.effectiveType, downlink: c.downlink } : null; })()`);
	videoRequests = [];
	await evaluate(`document.querySelector('a[href$="/${toon}/chapter/0"]').click(), true`);
	const v = await waitFor(`(() => { const s = ${VIDEO}; return s && s.src && s.readyState >= 1 ? s : null; })()`, slow ? 60000 : 20000);
	await throttle(false);
	return v;
}

// 1. each level loads its file, at a phone and a desktop width
if (step('network')) {
	for (const [w, h] of [[390, 844], [1440, 900]]) {
		console.log(`${w} px, guest: the setting decides the file`);
		await viewport(w, h);
		for (const level of ['low', 'medium', 'high']) {
			await guest(level);
			const v = await chapterVideo();
			const got = fetched();
			const problems = [];
			if (!v || v.error) problems.push(`no player: ${JSON.stringify(v)}`);
			else if (!v.src.endsWith(file(level))) problems.push(`currentSrc ${v.src}, expected ${file(level)}`);
			if (got.length !== 1 || got[0] !== file(level)) problems.push(`network: ${got.join(', ') || 'no video request'}, expected only ${file(level)}`);
			if (probes().length) problems.push(`a bandwidth probe although the setting is ${level}`);
			report(`${level} → ${file(level).replace(/^.*\//, '')}`, problems, v?.src);
			await shot(`mq-network-${w}-${level}`);
		}
	}
}

// 2. Automatic
if (step('auto')) {
	console.log('Automatic (guest, 1440 px)');
	await viewport(1440, 900);
	// with the API: a slow connection says "low" by itself (no probe); a 4G-class one is measured
	{
		await guest('auto');
		await throttle(true);
		await sleep(1000);
		const net = await evaluate(`(() => { const c = navigator.connection; return c ? { effectiveType: c.effectiveType, downlink: c.downlink, saveData: c.saveData } : null; })()`);
		await throttle(false);
		const problems = [];
		if (!net) problems.push('no navigator.connection in this Chrome');
		else if (net.effectiveType === '4g' && !(net.downlink < 1)) problems.push(`throttled, but navigator.connection says ${JSON.stringify(net)}`);
		report(`throttled (400 kbit/s, 400 ms): navigator.connection ${JSON.stringify(net)} = slow`, problems);
	}
	for (const slow of [false, true]) {
		await guest('auto');
		const v = await viaToonPage(slow);
		const want = slow ? 'low' : 'high';
		const problems = [];
		if (!v || v.error || !v.src.endsWith(file(want))) problems.push(`loads ${v?.src}, expected ${file(want)} (connection while deciding: ${JSON.stringify(lastNetInfo)})`);
		const probed = probes().length > 0;
		if (!slow && !probed) problems.push('no probe on a 4G-class connection');
		report(`Network Information API${slow ? ', throttled' : ''}: ${JSON.stringify(lastNetInfo)} → ${want}${probed ? ' (measured)' : ' (by the API alone)'}`, problems, JSON.stringify(videoRequests.map((r) => [r.url.replace(/^.*\//, ''), r.type, r.range])));
	}
	await throttle(false);
	// without the API (Safari, Firefox): the measured download of the first media
	const hide = await send('Page.addScriptToEvaluateOnNewDocument', { source: `Object.defineProperty(Navigator.prototype, 'connection', { get: () => undefined, configurable: true });` });
	for (const slow of [false, true]) {
		await guest('auto');
		const v = await viaToonPage(slow);
		const want = slow ? 'low' : 'high';
		const problems = [];
		if (!probes().length || probes()[0] !== file('low')) problems.push(`no 512 KB probe of the low file (requests: ${JSON.stringify(videoRequests.map((r) => [r.url.replace(/^.*\//, ''), r.range]))})`);
		if (!v || v.error || !v.src.endsWith(file(want))) problems.push(`loads ${v?.src}, expected ${file(want)}`);
		report(`no Network Information API, measured download${slow ? ' throttled' : ''} → ${want}`, problems, v?.src);
	}
	await throttle(false);
	await send('Page.removeScriptToEvaluateOnNewDocument', { identifier: hide.result.identifier });
}

// 3. switching while it plays
if (step('switch')) {
	console.log('switch while playing (guest, 390 px)');
	await viewport(390, 844);
	await guest('low');
	let v = await chapterVideo();
	await evaluate(`(() => { const v = document.querySelector('video[data-player]'); v.muted = true; return new Promise((ok) => { v.addEventListener('seeked', () => v.play().then(() => ok(true), () => ok(false)), { once: true }); v.currentTime = 30; }); })()`);
	const before = await waitFor(`(() => { const s = ${VIDEO}; return s && !s.paused && s.time > 30.5 ? s : null; })()`, 20000);
	await evaluate(`localStorage.setItem('${KEY}', 'medium'), window.dispatchEvent(new StorageEvent('storage', { key: '${KEY}', newValue: 'medium' })), true`);
	const after = await waitFor(`(() => { const s = ${VIDEO}; return s && s.src.endsWith('${file('medium')}') && !s.paused && s.time > 29 ? s : null; })()`, 20000);
	const problems = [];
	if (!before || before.error) problems.push(`not playing at 30 s on low: ${JSON.stringify(await evaluate(VIDEO))}`);
	if (!after || after.error) problems.push(`after the switch: ${JSON.stringify(await evaluate(VIDEO))}`);
	else if (!near(after.time, before?.time ?? 30, 3)) problems.push(`position ${after.time.toFixed(1)} s, was ${before?.time?.toFixed(1)} s`);
	report('low → medium mid-video: the 720p file, same position, still playing', problems, after && !after.error ? `${before?.time?.toFixed(1)} s → ${after.time.toFixed(1)} s on ${after.src.replace(/^.*\//, '')}` : '');
}

// 4. the account page as a guest
if (step('account')) {
	for (const [w, h, locale] of [[390, 844, 'de'], [1440, 900, 'en']]) {
		console.log(`account page, guest, ${w} px ${locale}`);
		await viewport(w, h);
		await guest(null, locale);
		const prefix = locale === 'en' ? '/en' : '';
		await open(`${viewer}${prefix}/account/reading`);
		const STATE = `(() => { const f = document.querySelector('[data-testid="media-quality"]'); if (!f) return null;
			return { labels: [...f.querySelectorAll('label')].map((l) => l.textContent.trim()), active: f.querySelector('label.active')?.textContent.trim(),
				legend: f.querySelector('legend')?.textContent.trim(), stored: localStorage.getItem('${KEY}'), hint: f.querySelector('small')?.textContent.replace(/\\s+/g, ' ').trim() }; })()`;
		const s = await waitFor(STATE);
		const msg = messages[locale].media_quality;
		const problems = [];
		if (!s || s.error) problems.push('no media quality setting on the account page');
		else {
			if (JSON.stringify(s.labels) !== JSON.stringify([msg.auto, msg.low, msg.medium, msg.high])) problems.push(`labels ${JSON.stringify(s.labels)}`);
			if (s.legend !== msg.title) problems.push(`legend "${s.legend}"`);
			if (s.active !== msg.auto) problems.push(`active "${s.active}", expected ${msg.auto} by default`);
			if (!s.hint?.includes(msg.saved_device)) problems.push(`hint "${s.hint}"`);
		}
		await shot(`mq-account-guest-${w}-${locale}`);
		const box = await evaluate(`(() => { const r = document.querySelector('[data-testid="media-quality-high"]').closest('label').getBoundingClientRect(); window.scrollTo(0, window.scrollY + r.top - 200); const q = document.querySelector('[data-testid="media-quality-high"]').closest('label').getBoundingClientRect(); return { x: q.x + q.width / 2, y: q.y + q.height / 2 }; })()`);
		if (box && !box.error) await click(box.x, box.y);
		const chosen = await waitFor(`(() => { const s = ${STATE}; return s && s.stored === 'high' && s.active === '${msg.high}' ? s : null; })()`, 5000);
		if (!chosen || chosen.error) problems.push(`after a click on "${msg.high}": ${JSON.stringify(await evaluate(STATE))}`);
		await open(`${viewer}${prefix}/account/reading`);
		const kept = await waitFor(`(() => { const s = ${STATE}; return s && s.active === '${msg.high}' ? s : null; })()`, 8000);
		if (!kept || kept.error) problems.push(`after a reload: ${JSON.stringify(await evaluate(STATE))}`);
		await shot(`mq-account-guest-${w}-${locale}-high`);
		report(`four buttons (${s?.labels?.join(' / ')}), Automatic by default, "${msg.high}" stored on the device and kept after a reload`, problems);
	}
}

// 5. a member (the agents' test user)
if (step('member')) {
	console.log('member (agent@scutoons.test), 390 px de');
	const sessionFile = join(root, 'logs/dev/test-user.json');
	const token = existsSync(sessionFile) ? JSON.parse(readFileSync(sessionFile, 'utf8')).token : null;
	const auth = token ? { Authorization: `Bearer ${token}`, 'Content-Type': 'application/json' } : {};
	const me = token ? await fetch(`${api}/me/`, { headers: auth }) : null;
	if (!me?.ok) report('member', ['no working test-user session (run `npm run dev:test-user`)']);
	else {
		const before = (await me.json()).user?.media_quality ?? 'auto';
		restoreMember = async () => { restoreMember = null; await fetch(`${api}/me/`, { method: 'PUT', headers: auth, body: JSON.stringify({ media_quality: before }) }); };
		await fetch(`${api}/me/`, { method: 'PUT', headers: auth, body: JSON.stringify({ media_quality: 'medium' }) });
		await viewport(390, 844);
		await guest('high');
		await evaluate(`localStorage.setItem('scutoons_session', ${JSON.stringify(token)}), true`);
		await open(`${viewer}/account/reading`);
		const msg = messages.de.media_quality;
		const STATE = `(() => { const f = document.querySelector('[data-testid="media-quality"]'); return f ? { active: f.querySelector('label.active')?.textContent.trim(), hint: f.querySelector('small')?.textContent.replace(/\\s+/g, ' ').trim() } : null; })()`;
		const s = await waitFor(`(() => { const s = ${STATE}; return s && s.active === '${msg.medium}' ? s : null; })()`, 10000);
		const problems = [];
		if (!s || s.error) problems.push(`the account page shows ${JSON.stringify(await evaluate(STATE))}, expected the server's "${msg.medium}" (this device says high)`);
		else if (!s.hint.includes(msg.saved_account)) problems.push(`hint "${s.hint}"`);
		const box = await evaluate(`(() => { const l = document.querySelector('[data-testid="media-quality-low"]').closest('label'); l.scrollIntoView({ block: 'center' }); const q = l.getBoundingClientRect(); return { x: q.x + q.width / 2, y: q.y + q.height / 2 }; })()`);
		if (box && !box.error) await click(box.x, box.y);
		await waitFor(`(() => { const s = ${STATE}; return s && s.active === '${msg.low}' ? s : null; })()`, 5000);
		await sleep(500);
		const server = (await (await fetch(`${api}/me/`, { headers: auth })).json()).user?.media_quality;
		if (server !== 'low') problems.push(`GET /me/ media_quality = ${server} after choosing "${msg.low}"`);
		await shot('mq-account-member-390-de');
		report(`the account page shows the server's value; "${msg.low}" → PUT /me/ → GET /me/ media_quality low`, problems);
		await evaluate(`localStorage.setItem('${KEY}', 'high'), true`);
		const v = await chapterVideo();
		report('the member\'s chapter 0 loads 480p (the account wins over this device\'s "high")', v && !v.error && v.src.endsWith(file('low')) && fetched().length === 1 ? [] : [`loads ${v?.src}, network ${fetched().join(', ')}`], v?.src);
		await restoreMember();
		const restored = (await (await fetch(`${api}/me/`, { headers: auth })).json()).user?.media_quality;
		report(`the member's previous setting is restored (${before})`, restored === before ? [] : [`now ${restored}`]);
	}
}

// 6. placement: full viewport, all 4 fits × 3 positions; frames in the column
const FITS = ['contain', 'width', 'height', 'cover'];
const POSITIONS = ['top', 'center', 'bottom'];
/** contain or cover for a box (fit width / height depend on the ratios) – @scutoons/content viewportObjectFit */
const objectFitFor = (fit, media, box) => fit === 'cover' ? 'cover' : fit === 'width' ? (media >= box ? 'contain' : 'cover') : fit === 'height' ? (media <= box ? 'contain' : 'cover') : 'contain';
const objectPositionFor = (position) => position === 'top' ? '50% 0%' : position === 'bottom' ? '50% 100%' : '50% 50%';
const PLACED = `(() => { const s = ${VIDEO}; if (!s) return null; const b = document.querySelector('.video-box'); const sl = b.closest('.slice').getBoundingClientRect();
	return { ...s, fitAttr: b.dataset.fit, objectFitAttr: b.dataset.objectFit, slice: { w: sl.width, h: sl.height } }; })()`;
const MEDIA_RATIO = `(() => { const v = document.querySelector('video[data-player]'); return v && v.videoWidth ? v.videoWidth / v.videoHeight : null; })()`;
if (step('placement')) {
	for (const reduced of [false, true]) {
		await send('Emulation.setEmulatedMedia', { features: [{ name: 'prefers-reduced-motion', value: reduced ? 'reduce' : 'no-preference' }] });
		for (const [w, h] of [[390, 844], [1440, 900]]) {
			console.log(`full viewport ${w} × ${h}${reduced ? ' (reduced motion: the poster)' : ''}`);
			await viewport(w, h);
			for (const fit of FITS) {
				for (const position of POSITIONS) {
					patchVideo = { viewport: { position, fit } };
					await guest('low');
					await open(`${viewer}/${toon}/chapter/0`);
					await evaluate(`document.querySelector('[data-testid="viewport-video"]')?.scrollIntoView({ block: 'start' }), true`);
					await sleep(300);
					const v = await waitFor(`(() => { const s = ${PLACED}; return s && s.box && s.objectFitAttr && (${reduced} ? s.readyState >= 1 : (s.button && !s.paused)) ? s : null; })()`, 20000);
					const problems = [];
					if (!v || v.error) { report(`${fit} ${position}`, [`no full-viewport player: ${JSON.stringify(await evaluate(PLACED))}`]); continue; }
					const media = (await evaluate(MEDIA_RATIO)) || ratio;
					const want = objectFitFor(fit, media, v.vw / v.vh);
					if (v.fitAttr !== fit || v.objectFitAttr !== want) problems.push(`fit ${v.fitAttr} → ${v.objectFitAttr}, expected ${fit} → ${want}`);
					if (!near(v.box.x, 0, 1) || !near(v.box.w, v.vw, 1) || !near(v.box.h, v.vh, 1)) problems.push(`box ${JSON.stringify(v.box)}, expected the viewport ${v.vw} × ${v.vh}`);
					if (v.scrollWidth > v.vw) problems.push(`sideways scroll: ${v.scrollWidth} > ${v.vw}`);
					if (want === 'contain') {
						const vw = Math.min(v.vw, v.vh * media), vh = vw / media;
						const top = position === 'top' ? 0 : position === 'bottom' ? v.vh - vh : (v.vh - vh) / 2;
						if (!near(v.rect.w, vw, 2) || !near(v.rect.h, vh, 2) || !near(v.rect.y - v.box.y, top, 2) || !near(v.rect.x, (v.vw - vw) / 2, 2)) problems.push(`video ${JSON.stringify(v.rect)}, expected ${Math.round(vw)} × ${Math.round(vh)} at y ${Math.round(top)}`);
					} else {
						if (!near(v.rect.w, v.vw, 1) || !near(v.rect.h, v.vh, 1)) problems.push(`video ${JSON.stringify(v.rect)}, expected the whole viewport`);
						if (v.fit !== 'cover' || v.objectPosition !== objectPositionFor(position)) problems.push(`object-fit ${v.fit} / object-position ${v.objectPosition}, expected cover / ${objectPositionFor(position)}`);
					}
					if (!reduced) {
						const r = v.rect;
						if (!(v.button.x > r.x && v.button.x < r.x + r.w && v.button.y > r.y && v.button.y < r.y + r.h && v.button.y > 0 && v.button.y < v.vh)) problems.push(`"Ton an" at ${JSON.stringify(v.button)}, not on the video ${JSON.stringify(r)}`);
					}
					report(`${fit} ${position} → ${want}${reduced ? ' (poster)' : ' + "Ton an" on the video'}`, problems, `video ${Math.round(v.rect.w)} × ${Math.round(v.rect.h)} at ${Math.round(v.rect.x)},${Math.round(v.rect.y - v.box.y)}`);
					await shot(`mq-placement-${w}-${fit}-${position}${reduced ? '-poster' : ''}`);
				}
			}
		}
	}
	await send('Emulation.setEmulatedMedia', { features: [{ name: 'prefers-reduced-motion', value: 'no-preference' }] });
	// frames in the column: each preset + custom 1:4, whole and cropped (+ fit width / height)
	const FRAMES = [['5:8', [5, 8]], ['6:13', [6, 13]], ['9:16', [9, 16]], ['1:1', [1, 1]], [[1, 4], [1, 4]]];
	for (const [w, h] of [[390, 844], [1440, 900]]) {
		console.log(`frames in the column ${w} × ${h}`);
		await viewport(w, h);
		for (const [frameRatio, [fw, fh]] of FRAMES) {
			for (const [fit, position] of [['contain', 'center'], ['cover', 'top'], ['width', 'center'], ['height', 'bottom']]) {
				patchVideo = { frame: { ratio: frameRatio, fit, position } };
				patchSlice = { width: 390, height: Math.round((390 * fh) / fw) };
				await guest('low');
				await open(`${viewer}/${toon}/chapter/0`);
				await evaluate(`document.querySelector('[data-testid="framed-video"]')?.scrollIntoView({ block: 'start' }), true`);
				const v = await waitFor(`(() => { const s = ${PLACED}; return s && s.box && s.objectFitAttr && s.readyState >= 1 ? s : null; })()`, 20000);
				const problems = [];
				const label = `${Array.isArray(frameRatio) ? frameRatio.join(':') + ' (custom)' : frameRatio} ${fit} ${position}`;
				if (!v || v.error) { report(`frame ${label}`, [`no framed player: ${JSON.stringify(await evaluate(PLACED))}`]); continue; }
				const media = (await evaluate(MEDIA_RATIO)) || ratio;
				const want = objectFitFor(fit, media, fw / fh);
				if (!near(v.box.h, (v.box.w * fh) / fw, 2)) problems.push(`box ${Math.round(v.box.w)} × ${Math.round(v.box.h)}, expected ${fw}:${fh}`);
				if (!near(v.slice.h, v.box.h, 2)) problems.push(`slice ${Math.round(v.slice.h)} px high, the box ${Math.round(v.box.h)} (the reserved space doesn't match)`);
				if (!near(v.rect.w, v.box.w, 1) || !near(v.rect.h, v.box.h, 1)) problems.push(`video ${JSON.stringify(v.rect)} doesn't fill the box ${JSON.stringify(v.box)}`);
				if (v.objectFitAttr !== want || v.fit !== want || v.objectPosition !== objectPositionFor(position)) problems.push(`object-fit ${v.fit} (${v.objectFitAttr}) / ${v.objectPosition}, expected ${want} / ${objectPositionFor(position)}`);
				if (v.scrollWidth > v.vw) problems.push(`sideways scroll: ${v.scrollWidth} > ${v.vw}`);
				report(`frame ${label} → ${want}`, problems, `box ${Math.round(v.box.w)} × ${Math.round(v.box.h)}`);
				if (fit === 'contain' || fit === 'cover') await shot(`mq-frame-${w}-${Array.isArray(frameRatio) ? 'custom-1-4' : frameRatio.replace(':', '-')}-${fit}`);
			}
		}
	}
	patchVideo = null;
	patchSlice = null;
}

// 7. controls on / off
if (step('controls')) {
	console.log('controls (390 px)');
	await viewport(390, 844);
	for (const controls of [true, false]) {
		patchVideo = { controls };
		await guest('low');
		const v = await chapterVideo();
		const problems = [];
		if (!v || v.error) problems.push('no player');
		else if (v.controls !== controls) problems.push(`video.controls ${v.controls}`);
		if (!controls && v && !v.error) {
			await evaluate(`document.querySelector('video[data-player]').scrollIntoView({ block: 'center' }), true`);
			await sleep(300);
			const playing = await waitFor(`(() => { const s = ${VIDEO}; return s && !s.paused ? s : null; })()`, 10000);
			const r = playing?.rect;
			if (r) await click(r.x + r.w / 4, r.y + r.h / 4);
			const paused = await waitFor(`(() => { const s = ${VIDEO}; return s && s.paused ? s : null; })()`, 3000);
			if (r) await click(r.x + r.w / 4, r.y + r.h / 4);
			const again = await waitFor(`(() => { const s = ${VIDEO}; return s && !s.paused ? s : null; })()`, 3000);
			if (!playing || !paused || !again) problems.push(`click toggles: playing ${!!playing}, paused after a click ${!!paused}, playing after another ${!!again}`);
		}
		report(controls ? 'controls: true → native controls' : 'controls: false → no controls, a click pauses / plays', problems);
		await shot(`mq-controls-${controls ? 'on' : 'off'}`);
	}
	patchVideo = null;
}

// 8. the follow-up bar after a full-viewport player
if (step('bar')) {
	for (const [w, h] of [[390, 844], [1440, 900]]) {
		console.log(`follow-up bar after a full-viewport player (${w} px)`);
		await viewport(w, h);
		patchVideo = { viewport: { position: 'center', fit: 'contain' } };
		await guest('low');
		const v = await chapterVideo();
		await evaluate(`document.querySelector('[data-testid="viewport-video"]').scrollIntoView({ block: 'start' }), true`);
		const auto = await waitFor(`(() => { const s = ${VIDEO}; return s && !s.paused && s.muted && s.button ? s : null; })()`, 15000);
		await evaluate(`(() => { const v = document.querySelector('video[data-player]'); return new Promise((ok) => { v.addEventListener('seeked', () => v.play().then(() => ok(true), () => ok(false)), { once: true }); v.currentTime = v.duration - 1.5; }); })()`);
		const bar = await waitFor(`(() => { const b = document.querySelector('[data-testid="next-bar"]'); const v = document.querySelector('video[data-player]'); if (!b || !v?.ended) return null; const r = b.getBoundingClientRect(); return { top: r.top, bottom: r.bottom, h: r.height, vh: window.innerHeight, kind: b.dataset.kind, position: getComputedStyle(b).position }; })()`, 20000);
		const problems = [];
		if (!v || v.error) problems.push('no player');
		if (!auto || auto.error) problems.push(`no muted autoplay + "Ton an": ${JSON.stringify(await evaluate(VIDEO))}`);
		if (!bar || bar.error) problems.push('no follow-up bar after the end');
		else if (bar.position !== 'fixed' || !near(bar.bottom, bar.vh, 1)) problems.push(`bar ${JSON.stringify(bar)}, expected fixed at the bottom`);
		await shot(`mq-bar-${w}`);
		report(`autoplay muted + "Ton an", ended → the follow-up bar fixed at the bottom (${bar?.kind ?? '–'})`, problems);
	}
	patchVideo = null;
}

// 9. the creator's background panel (as the test user; edits are never saved – the temp profile is thrown away)
if (step('creator')) {
	const creator = String(opt.creator || 'http://localhost:10000').replace(/\/$/, '');
	console.log(`creator background panel (${creator}, test user, 1440 px)`);
	const sessionFile = join(root, 'logs/dev/test-user.json');
	const token = existsSync(sessionFile) ? JSON.parse(readFileSync(sessionFile, 'utf8')).token : null;
	if (!token) report('creator', ['no test-user session (run `npm run dev:test-user`)']);
	else {
		await viewport(1440, 900);
		await send('Network.clearBrowserCookies');
		await open(`${creator}/`);
		await evaluate(`localStorage.clear(), localStorage.setItem('scutoons_session', ${JSON.stringify(token)}), true`);
		await open(`${creator}/${toon}/chapter/0?group=roar-little-kitty&slice=1`);
		const tab = await waitFor(`document.querySelector('[data-testid="tab-background"]') ? document.querySelector('[data-testid="tab-background"]').textContent.trim() : null`, 30000);
		await evaluate(`document.querySelector('[data-testid="tab-background"]').click(), true`);
		const PANEL = `(() => { const p = document.querySelector('[data-testid="slice-media"]'); if (!p) return null;
			const val = (id) => p.querySelector('[data-testid="' + id + '"]');
			// the resolution slot (media-variants-slot): one chip per level, its file in data-file, the default marked
			return { low: val('video-file-low')?.dataset.file, medium: val('video-file-medium')?.dataset.file, high: val('video-file-high')?.dataset.file,
				def: ['low', 'medium', 'high'].find((l) => val('video-file-' + l)?.dataset.default === 'true') ?? null,
				labels: ['low', 'medium', 'high'].map((l) => val('video-file-' + l)?.textContent.replace(/\\s+/g, ' ').trim() ?? ''),
				player: val('video-player')?.checked, controls: val('video-controls')?.checked,
				column: val('video-in-column')?.checked, full: val('video-full-viewport')?.checked,
				cells: p.querySelectorAll('[data-testid^="fit-"]:not([data-testid="fit-help"])').length, active: p.querySelector('[data-testid^="fit-"][aria-checked="true"]')?.dataset.testid ?? null,
				positions: p.querySelectorAll('[data-testid^="position-"]:not([data-testid="position-none"])').length, positionNone: !!p.querySelector('[data-testid="position-none"]'),
				help: p.querySelector('[data-testid="fit-help"]')?.textContent.replace(/\\s+/g, ' ').trim() ?? null,
				preview: [...p.querySelectorAll('[data-testid="placement-preview"] img')].map((i) => getComputedStyle(i).objectFit + ' ' + getComputedStyle(i).objectPosition) }; })()`;
		const s = await waitFor(`(() => { const s = ${PANEL}; return s && s.low !== undefined ? s : null; })()`, 15000);
		const problems = [];
		if (!['Hintergrund', 'Background'].includes(tab)) problems.push(`tab "${tab}", expected Hintergrund / Background`);
		if (!s || s.error) problems.push(`no background panel: ${JSON.stringify(s)}`);
		else {
			if (s.low !== file('low') || s.medium !== file('medium') || s.high !== file('high')) problems.push(`files ${JSON.stringify([s.low, s.medium, s.high])}`);
			if (s.def !== 'medium') problems.push(`default ${s.def}, expected medium (src = the 720p file)`);
			if (!s.labels[0]?.startsWith('480p') || !s.labels[1]?.startsWith('720p') || !s.labels[2]?.startsWith('1080p')) problems.push(`labels ${JSON.stringify(s.labels)}`);
			// (whatever placement a draft in the store has – the owner may be trying it out in a preview – the steps below set their own)
			if (!s.player) problems.push(`flags ${JSON.stringify(s)}`);
		}
		await shot('mq-creator-1-panel');
		// full viewport → the picker (3 × 2) + the preview; "fit height, top"; controls off
		await evaluate(`document.querySelector('[data-testid="video-full-viewport"]').click(), true`);
		await waitFor(`document.querySelector('[data-testid="fit-cover"]') ? true : null`, 5000);
		await evaluate(`document.querySelector('[data-testid="fit-cover"]').click(), (document.querySelector('[data-testid="video-controls"]').checked && document.querySelector('[data-testid="video-controls"]').click()), true`);
		const s2 = await waitFor(`(() => { const s = ${PANEL}; return s && s.active === 'fit-cover' && !s.controls ? s : null; })()`, 5000);
		if (!s2 || s2.error) problems.push(`after full viewport + cover top + controls off: ${JSON.stringify(await evaluate(PANEL))}`);
		else {
			if (s2.cells !== 4 || !s2.full) problems.push(`fit tiles ${s2.cells}`);
			// a landscape video filling a phone or a 16:10 desktop is cropped at the sides only: no position buttons
			if (s2.positions !== 0) problems.push(`position buttons for cover in full viewport: ${s2.positions}`);
			if (s2.preview.length !== 2 || !s2.preview.every((p) => p === 'cover 50% 50%')) problems.push(`preview ${JSON.stringify(s2.preview)}`);
		}
		const SIZE = `document.querySelector('[data-testid="slice-size"]')?.textContent.replace(/\\s+/g, ' ').trim()`;
		const sizeLine = await evaluate(SIZE);
		if (!/^(Bildschirm|Screen) · 1[.,]89:1$/.test(sizeLine ?? '')) problems.push(`size line in full viewport "${sizeLine}"`);
		if (await evaluate(`!!document.querySelector('[data-testid="video-frame"]')`)) problems.push('the ratio select shows in full viewport');
		await shot('mq-creator-2-full-viewport');
		// back to the column: 9:16 → the slice is 390 × 693, the picker + a framed preview; custom 1:5 → clamped to 1:4
		await evaluate(`document.querySelector('[data-testid="video-in-column"]').click(), true`);
		await waitFor(`document.querySelector('[data-testid="video-frame"]') ? true : null`, 5000);
		await evaluate(`(() => { const s = document.querySelector('[data-testid="video-frame"]'); s.value = 'hd-portrait'; s.dispatchEvent(new Event('change', { bubbles: true })); return true; })()`);
		const framed = await waitFor(`(() => { const t = ${SIZE}; return t && t.includes('390 × 693') ? { t, cells: document.querySelectorAll('[data-testid^="fit-"]').length, positions: document.querySelectorAll('[data-testid^="position-"]').length, preview: document.querySelectorAll('[data-testid="placement-preview"] img').length } : null; })()`, 5000);
		if (!framed || framed.error || framed.cells !== 4 || framed.positions !== 3 || framed.preview !== 1) problems.push(`9:16 frame: ${JSON.stringify(framed ?? (await evaluate(SIZE)))}`);
		await evaluate(`document.querySelector('[data-testid="placement-preview"]')?.scrollIntoView({ block: 'end' }), true`);
		await evaluate(`document.querySelector('[data-testid="position-top"]')?.click(), true`);
		const top = await waitFor(`(() => { const i = document.querySelector('[data-testid="placement-preview"] img'); return i && getComputedStyle(i).objectPosition === '50% 0%' && getComputedStyle(i).objectFit === 'contain' ? true : null; })()`, 5000);
		if (top !== true) problems.push('9:16 + "Oben": the framed preview does not move the video to the top');
		await evaluate(`document.querySelector('[data-testid="placement-picker"]')?.scrollIntoView({ block: 'start' }), true`);
		await sleep(300);
		await shot('mq-creator-4-frame-9-16');
		await evaluate(`(() => { const s = document.querySelector('[data-testid="video-frame"]'); s.value = 'custom'; s.dispatchEvent(new Event('change', { bubbles: true })); return true; })()`);
		await waitFor(`document.querySelector('[data-testid="video-frame-h"]') ? true : null`, 5000);
		await evaluate(`(() => { const w = document.querySelector('[data-testid="video-frame-w"]'); w.value = '1'; w.dispatchEvent(new Event('change', { bubbles: true })); const h = document.querySelector('[data-testid="video-frame-h"]'); h.value = '5'; h.dispatchEvent(new Event('change', { bubbles: true })); return true; })()`);
		const clamped = await waitFor(`(() => { const t = ${SIZE}; return t && t.includes('390 × 1560') ? t : null; })()`, 5000);
		if (!clamped || clamped.error) problems.push(`custom 1:5 → expected clamped to 1:4 (390 × 1560): "${await evaluate(SIZE)}"`);
		await shot('mq-creator-5-custom-1-4');
		// the 720p chip's × → the level is gone, the default moves to the LOWER level first (480p, less data; owner 2026-10-03)
		await evaluate(`document.querySelector('[data-testid="video-file-medium-clear"]').click(), true`);
		const s3 = await waitFor(`(() => { const s = ${PANEL}; return s && s.medium === '' ? s : null; })()`, 5000);
		if (!s3 || s3.error || s3.def !== 'low') problems.push(`after clearing 720p: ${JSON.stringify(await evaluate(PANEL))}`);
		report(`"${tab}" tab: one resolution slot with 480p / 720p / 1080p chips (default 720p), player + controls, fit row Ganz/Breite/Höhe/Füllen + position row only where it matters, the small preview, the short size lines, frame 9:16 → 390 × 693, custom 1:5 clamped to 1:4, clearing 720p makes 480p the default`, problems);
	}
}

console.log(`\n${passed} passed, ${failed} failed`);
await done(failed ? 1 : 0);
