// Follow-up bar + player autoplay check (agents/DESIGN.md "Follow-up bar" / "Player video slice", owner 2026-10-01; since
// BACKLOG 10.48 the end actions are choices of the VN message box – the testids stay "next-bar"):
// headless Chrome via CDP (own temp profile, no dependencies; Node 22 WebSocket), like scripts/check-prolog.mjs. Chrome
// runs with the normal autoplay policy (`document-user-activation-required`): sound only after a click in the document.
//
// Per width (390, 1440) and locale (de, en):
//   guest  1. a direct load of the prologue (/catpope/chapter/0) autoplays muted with a "Ton an" button; the button unmutes
//             and it keeps playing;
//          2. the in-app way (a real click on the toon page's start button) autoplays with sound, no button;
//          3. at its end the bar asks for a membership: hint, "Anmelden →" (→ /account?return=<chapter 1>) + "Code
//             einlösen" (opens the QR camera), no countdown, it stays;
//   member (the test user agent@scutoons.test, `npm run dev:test-user`)
//          4. the prologue's end: "S1 K1 lesen →" + "↺ Nochmal ansehen" + the countdown → chapter 1 after 10 s;
//          5. "Abbrechen" stops it (the bar stays, no navigation); "Nochmal ansehen" plays the video again, the bar goes;
//          6. chapter 1 scrolled to its end: "Kapitel 2: Ärger →" + countdown; scrolling back up stops it; opening the
//             chapter list (an overlay) stops it; "↺ Nochmal" starts the chapter from the top;
//          7. chapter 2 scene-15's end: "Weiter →" + countdown → ?label=scene-16;
//          8. chapter 2's last group: "Kapitel 3 erscheint am …" + "Zur Übersicht", no countdown;
//          9. the season's end (the check drops chapters 3–5 from the toon answer): "Staffel 1 geschafft" + "Zur
//             Übersicht" + "Musik herunterladen", no countdown;
//         10. signed in without a membership (the check strips the access from /me/): "Code einlösen →" + "Wo bekomme ich
//             einen Code?", no countdown;
//         11. the sandbox VN chapter 2 (the check drops its final jump): no bar while a message waits, the bar after the
//             last message was clicked away (→ sandbox chapter 3, countdown);
//   once   12. guest at 390 de: "Anmelden" → the login (the test user, OTP from server/logs/mail.log) → back in chapter 1,
//             readable; a guest redeeming a code (`?code=promo_test`) gets the member bar with the countdown.
// Reduced motion (390 de): no autoplay, no countdown (the check presses play muted: no click, no sound allowed). The member's server progress is restored afterwards.
//
// usage: node scripts/check-next-bar.mjs [--viewer=http://localhost:8000] [--api=http://localhost:9000/api]
//        [--shots=<dir>] [--only=390-de] [--verbose]
// Needs the dev servers (npm run dev) and the test user. Exit code 1 on any failure.
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
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const messages = Object.fromEntries(['de', 'en'].map((l) => [l, JSON.parse(readFileSync(join(root, `viewer/messages/${l}.json`), 'utf8'))]));
const fill = (s, v) => s.replace(/\{(\w+)\}/g, (_, k) => String(v[k]));

// ── the test user ───────────────────────────────────────────────────────────────────────────────────────────────────
const sessionFile = join(root, 'logs/dev/test-user.json');
const token = existsSync(sessionFile) ? JSON.parse(readFileSync(sessionFile, 'utf8')).token : null;
if (!token || !(await fetch(`${api}/me/`, { headers: { Authorization: `Bearer ${token}` } })).ok) { console.error('no working test-user session (run `npm run dev:test-user`)'); process.exit(2); }
const auth = { Authorization: `Bearer ${token}`, 'Content-Type': 'application/json' };
// the member's save game (one for every toon – progress row `scutoons`), put back exactly afterwards
const saveGame = saveGameRow(JSON.parse(readFileSync(sessionFile, 'utf8')).email);
const titles = {};
for (const l of ['de', 'en']) {
	const bundle = await (await fetch(`${api}/library/toons/${toon}?locale=${l}`)).json();
	titles[l] = Object.fromEntries((bundle.seasons.find((s) => s.number === 1) ?? bundle.seasons[0]).chapters.map((c) => [c.number, c]));
}

// ── Chrome ──────────────────────────────────────────────────────────────────────────────────────────────────────────
const CHROME = process.env.CHROME || ['C:/Program Files/Google/Chrome/Application/chrome.exe', 'C:/Program Files (x86)/Google/Chrome/Application/chrome.exe', 'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe', '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome', '/usr/bin/google-chrome'].find((p) => existsSync(p));
if (!CHROME) { console.error('Chrome not found (set CHROME)'); process.exit(2); }
const port = 9400 + Math.floor(Math.random() * 400);
const profile = mkdtempSync(join(tmpdir(), 'check-next-bar-'));
// the browser's normal autoplay rule (sound only after a click in the document); sound is decoded but not heard
const chrome = spawn(CHROME, ['--headless=new', `--remote-debugging-port=${port}`, `--user-data-dir=${profile}`, '--no-first-run', '--no-default-browser-check', '--disable-extensions', '--mute-audio', '--autoplay-policy=document-user-activation-required', '--window-size=1440,900', 'about:blank'], { stdio: 'ignore' });
let restored = false;
async function restoreProgress() {
	if (restored) return;
	restored = true;
	if (!saveGame.restore()) throw new Error('the save game row differs');
}
async function done(code) {
	await restoreProgress().catch((e) => console.log(`  could not restore the member's progress: ${e}`));
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
/** answer rewrites (CDP Fetch at the response stage) */
const rewrite = { seasonEnd: false, noAccess: false, sandboxNoJump: false };
const send = (method, params = {}) => new Promise((r) => { const i = ++seq; pending.set(i, r); ws.send(JSON.stringify({ id: i, method, params })); });
async function fulfillJson(p, change) {
	const body = await send('Fetch.getResponseBody', { requestId: p.requestId });
	const raw = body.result.base64Encoded ? Buffer.from(body.result.body, 'base64').toString('utf8') : body.result.body;
	const json = change(JSON.parse(raw));
	const headers = p.responseHeaders.filter((h) => !/^(content-length|etag|content-encoding)$/i.test(h.name));
	return send('Fetch.fulfillRequest', { requestId: p.requestId, responseCode: 200, responseHeaders: headers, body: Buffer.from(JSON.stringify(json)).toString('base64') });
}
ws.addEventListener('message', async (e) => {
	const m = JSON.parse(e.data), p = m.params;
	if (m.id && pending.has(m.id)) { pending.get(m.id)(m); pending.delete(m.id); return; }
	if (m.method?.startsWith('Network.')) lastNet = Date.now();
	if (m.method === 'Fetch.requestPaused') {
		const url = new URL(p.request.url);
		if (p.responseStatusCode !== 200 || p.request.method !== 'GET') return send('Fetch.continueResponse', { requestId: p.requestId });
		// the season's end: the season lists nothing after chapter 2
		if (rewrite.seasonEnd && url.pathname.endsWith(`/library/toons/${toon}`)) {
			return fulfillJson(p, (b) => { for (const s of b.seasons) { s.chapters = s.chapters.filter((c) => c.number <= 2); s.season.sequence = s.season.sequence.filter((c) => Number(c.number) <= 2); } return b; });
		}
		// signed in without a membership
		if (rewrite.noAccess && /\/api\/me\/?$/.test(url.pathname)) {
			return fulfillJson(p, (me) => ({ ...me, access: { ...(me.access ?? {}), permissions: [], unlocked: false, codes: [], signup_code: null }, memberships: [] }));
		}
		// a VN group whose scene ends without a jump
		if (rewrite.sandboxNoJump && url.pathname.endsWith('/library/toons/sandbox/chapters/2')) {
			return fulfillJson(p, (c) => { for (const g of c.groups) if (g.events?.at(-1)?.type === 'jumpTo') g.events.pop(); return c; });
		}
		return send('Fetch.continueResponse', { requestId: p.requestId });
	}
});
const evaluate = async (expression) => { const r = await send('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true }); return r.result?.exceptionDetails ? { error: r.result.exceptionDetails.exception?.description ?? r.result.exceptionDetails.text } : r.result?.result?.value; };
await send('Page.enable'); await send('Runtime.enable'); await send('Network.enable');
await send('Fetch.enable', { patterns: [{ urlPattern: '*/api/*', requestStage: 'Response' }] });
async function idle(quietMs = 700, maxMs = 15000) {
	const t0 = Date.now();
	while (Date.now() - t0 < maxMs) { if (Date.now() - lastNet > quietMs) return; await sleep(100); }
}
async function waitFor(js, ms = 15000) {
	const t0 = Date.now();
	let r;
	while (Date.now() - t0 < ms) { r = await evaluate(js); if (r && !r.error) return r; await sleep(150); }
	return null;
}
async function open(url) { lastNet = Date.now(); await send('Page.navigate', { url }); await sleep(300); await idle(); }
async function shot(name) {
	if (!shots) return;
	const r = await send('Page.captureScreenshot', { format: 'png' });
	writeFileSync(join(shots, `${name}.png`), Buffer.from(r.result.data, 'base64'));
}
/** a real click (a user gesture) in the middle of the element */
async function click(selector) {
	const box = await evaluate(`(() => { const el = document.querySelector(${JSON.stringify(selector)}); if (!el) return null; el.scrollIntoView({ block: 'nearest' }); const b = el.getBoundingClientRect(); return { x: b.left + b.width / 2, y: b.top + b.height / 2 }; })()`);
	if (!box || box.error) return false;
	for (const type of ['mousePressed', 'mouseReleased']) await send('Input.dispatchMouseEvent', { type, x: box.x, y: box.y, button: 'left', clickCount: 1 });
	return true;
}

// ── checks ──────────────────────────────────────────────────────────────────────────────────────────────────────────
let failed = 0, passed = 0;
function report(name, problems, detail = '') {
	if (problems.length) { failed++; console.log(`  FAIL  ${name}`); for (const p of problems) console.log(`        ${p}`); }
	else { passed++; console.log(`  ok    ${name}${opt.verbose && detail ? `  ${detail}` : ''}`); }
}
const VIDEO = `(() => {
	const v = document.querySelector('video[data-player]');
	return v ? { paused: v.paused, ended: v.ended, muted: v.muted, time: v.currentTime, duration: v.duration, readyState: v.readyState,
		soundOn: !!document.querySelector('[data-testid="player-sound-on"]'), soundOnText: document.querySelector('[data-testid="player-sound-on"]')?.textContent.trim() ?? null } : null;
})()`;
const BAR = `(() => {
	const bar = document.querySelector('[data-testid="next-bar"]');
	if (!bar) return { shown: false, path: location.pathname + location.search };
	const primary = bar.querySelector('[data-testid="next-bar-primary"]');
	const secondary = bar.querySelector('[data-testid="next-bar-secondary"], [data-testid="next-bar-replay"]');
	const box = bar.closest('.vn-message');
	const b = (box ?? bar).getBoundingClientRect();
	const cs = getComputedStyle(box ?? bar);
	return { shown: true, path: location.pathname + location.search, kind: bar.dataset.kind, counting: bar.dataset.counting === 'true',
		message: bar.querySelector('[data-testid="next-bar-message"]')?.textContent.trim() ?? null,
		primary: primary?.textContent.trim() ?? null, primaryHref: primary?.getAttribute('href') ?? null, primaryTag: primary?.tagName,
		secondary: secondary?.textContent.trim().replace(/\\s+/g, ' ') ?? null, secondaryHref: secondary?.getAttribute('href') ?? null,
		countdown: bar.querySelector('[data-testid="next-bar-countdown"]')?.textContent.trim() ?? null,
		inBox: !!box && box.classList.contains('visible'), rem: parseFloat(getComputedStyle(document.documentElement).fontSize),
		bottom: box ? Math.round(parseFloat(cs.bottom)) : Math.round(innerHeight - b.bottom), width: Math.round(b.width), vw: document.documentElement.clientWidth, z: Number(cs.zIndex), border: cs.borderTopWidth + ' ' + cs.borderTopStyle,
		font: cs.fontFamily, radius: getComputedStyle(primary).borderRadius, choiceLook: [...bar.querySelectorAll('a, button:not(.cancel)')].every((c) => c.classList.contains('vn-end-choice')),
		actions: bar.querySelectorAll('a, button:not(.cancel)').length };
})()`;
const playToEnd = `(() => { const v = document.querySelector('video[data-player]'); return new Promise((ok, no) => {
	const go = () => { v.addEventListener('seeked', () => v.play().catch(() => ((v.muted = true), v.play())).then(() => ok(true), no), { once: true }); v.currentTime = v.duration - 1.2; };
	v.readyState >= 1 ? go() : v.addEventListener('loadedmetadata', go, { once: true });
}); })()`;
const scrollToEnd = `(window.scrollTo(0, document.documentElement.scrollHeight), true)`;
/** scroll to the end until the bar shows (lazy slices grow the page) */
async function reachEnd(ms = 20000) {
	const t0 = Date.now();
	while (Date.now() - t0 < ms) {
		await evaluate(scrollToEnd);
		const s = await evaluate(BAR);
		if (s?.shown) return s;
		await sleep(400);
	}
	return evaluate(BAR);
}
// the end actions are choices of the VN message box (BACKLOG 10.48): the box 1 rem above the bottom, 1 rem in from the sides
function barLook(s, problems) {
	if (!s.inBox) problems.push('the end actions are not in the shown message box (.vn-message.visible)');
	if (Math.abs(s.bottom - s.rem) > 1 || Math.abs(s.width - (s.vw - 2 * s.rem)) > 1) problems.push(`box not at the bottom (bottom ${s.bottom}, width ${s.width}/${s.vw})`);
	if (!/IBM Plex Mono/.test(s.font) || s.border !== '2px solid' || s.radius !== '0px' || !s.choiceLook) problems.push(`look: font ${s.font}, border ${s.border}, radius ${s.radius}, choices ${s.choiceLook}`);
	if (s.actions > 2) problems.push(`${s.actions} actions (at most 2)`);
}
async function setLocale(locale) {
	await send('Network.setCookie', { name: 'PARAGLIDE_LOCALE', value: locale, url: viewer, path: '/' });
}
async function asGuest(locale) {
	await send('Network.clearBrowserCookies');
	await setLocale(locale);
	await open(`${viewer}/`);
	await evaluate('localStorage.clear(), sessionStorage.clear(), true');
}
async function asMember(locale) {
	await send('Network.clearBrowserCookies');
	await setLocale(locale);
	await open(`${viewer}/`);
	await evaluate(`localStorage.clear(), sessionStorage.clear(), localStorage.setItem('scutoons_session', ${JSON.stringify(token)}), true`);
}

async function run(width, locale) {
	const prefix = locale === 'en' ? '/en' : '';
	const msg = messages[locale];
	const name = `${width}-${locale}`;
	const nb = msg.next_bar;
	const chapterName = (n) => (n === 0 ? msg.chapter.prologue : fill(msg.common.chapter, { number: n }));
	await send('Emulation.setDeviceMetricsOverride', { width, height: width < 768 ? 844 : 900, deviceScaleFactor: 1, mobile: width < 768 });
	await send('Emulation.setEmulatedMedia', { features: [{ name: 'prefers-reduced-motion', value: 'no-preference' }] });

	console.log(`${name} guest`);
	await asGuest(locale);
	// 1. direct load: muted + "Ton an"
	await open(`${viewer}${prefix}/${toon}/chapter/0`);
	let v = await waitFor(`(() => { const s = ${VIDEO}; return s && !s.paused && s.time > 0.3 ? s : null; })()`, 20000);
	{
		const problems = [];
		if (!v) problems.push(`not autoplaying: ${JSON.stringify(await evaluate(VIDEO))}`);
		else if (!v.muted || !v.soundOn || v.soundOnText !== msg.player.sound_on) problems.push(`expected muted + "${msg.player.sound_on}": ${JSON.stringify(v)}`);
		report('direct load: the prologue autoplays muted with the "Ton an" button', problems, JSON.stringify(v));
	}
	await shot(`next-${name}-1-direct-muted`);
	await click('[data-testid="player-sound-on"]');
	v = await waitFor(`(() => { const s = ${VIDEO}; return s && !s.muted && !s.paused && !s.soundOn ? s : null; })()`, 5000);
	report('"Ton an" unmutes, it keeps playing, the button goes', v ? [] : [JSON.stringify(await evaluate(VIDEO))]);

	// 2. the in-app way: a click on the toon page's start button → with sound
	await open(`${viewer}${prefix}/${toon}`);
	await waitFor(`document.querySelector('[data-testid="start-button"], [data-testid="hero-start"]') ? true : null`);
	await evaluate(`localStorage.removeItem('scutoons_game_scutoons'), true`);
	const startSel = width >= 1024 ? '[data-testid="hero-start"]' : '[data-testid="start-button"]';
	await click(startSel);
	await waitFor(`location.pathname.endsWith('/chapter/0') ? true : null`, 10000);
	v = await waitFor(`(() => { const s = ${VIDEO}; return s && !s.paused && s.time > 0.3 ? s : null; })()`, 20000);
	report('in-app navigation (a click on the start button): autoplays with sound, no button', v && !v.muted && !v.soundOn ? [] : [JSON.stringify(v ?? (await evaluate(VIDEO)))]);
	await shot(`next-${name}-2-inapp-sound`);

	// 3. its end: the membership case (not signed in)
	await evaluate(playToEnd);
	let s = await waitFor(`(() => { const s = ${BAR}; return s.shown ? s : null; })()`, 20000);
	{
		const problems = [];
		const ch1 = `${prefix}/${toon}/chapter/1`;
		if (!s) problems.push('no bar after the end');
		else {
			if (s.kind !== 'members' || s.counting || s.countdown) problems.push(`kind ${s.kind}, counting ${s.counting}`);
			if (s.message !== fill(nb.members_hint, { chapter: chapterName(1) })) problems.push(`message "${s.message}"`);
			if (s.primary !== nb.sign_in || s.primaryHref !== `${prefix}/account?return=${encodeURIComponent(ch1)}`) problems.push(`primary "${s.primary}" → ${s.primaryHref}`);
			if (s.secondary !== nb.redeem) problems.push(`secondary "${s.secondary}"`);
			barLook(s, problems);
		}
		report(`prologue end (guest): "${nb.sign_in} →" + "${nb.redeem}", hint, no countdown`, problems, JSON.stringify(s));
	}
	await shot(`next-${name}-3-guest-members`);
	await sleep(6000);
	s = await evaluate(BAR);
	report('…it stays (no navigation)', s.shown && s.path.endsWith('/chapter/0') ? [] : [JSON.stringify(s)]);
	await click('[data-testid="next-bar-secondary"]');
	const camera = await waitFor(`document.querySelector('.camera[role="dialog"]') ? true : null`, 5000);
	report(`"${nb.redeem}" opens the QR camera (the claim flow)`, camera ? [] : ['no camera dialog']);
	await evaluate(`document.querySelector('.camera .close')?.click(), true`);

	console.log(`${name} member (agent@scutoons.test)`);
	await asMember(locale);
	// 4. the prologue's end: on to chapter 1 with the countdown
	await open(`${viewer}${prefix}/${toon}/chapter/0`);
	await waitFor(VIDEO);
	await evaluate(playToEnd);
	s = await waitFor(`(() => { const s = ${BAR}; return s.shown ? s : null; })()`, 20000);
	{
		const problems = [];
		if (!s) problems.push('no bar');
		else {
			const label = fill(msg.start.read, { season: 1, chapter: 1 });
			if (s.kind !== 'chapter' || !s.counting) problems.push(`kind ${s.kind}, counting ${s.counting}`);
			if (s.primary !== label || s.primaryHref !== `${prefix}/${toon}/chapter/1`) problems.push(`primary "${s.primary}" → ${s.primaryHref}, expected "${label}"`);
			if (s.secondary !== `↺ ${nb.replay_video}`) problems.push(`secondary "${s.secondary}"`);
			if (!s.countdown?.startsWith(fill(nb.countdown, { seconds: 10 }).replace(/10 s$/, '')) || !s.countdown.includes(nb.cancel)) problems.push(`countdown "${s.countdown}"`);
			barLook(s, problems);
		}
		report(`prologue end (member): "${fill(msg.start.read, { season: 1, chapter: 1 })} →" + "↺ ${nb.replay_video}" + countdown`, problems, JSON.stringify(s));
	}
	await sleep(1500);
	await shot(`next-${name}-4-member-countdown`);
	const went = await waitFor(`location.pathname === '${prefix}/${toon}/chapter/1' ? true : null`, 12000);
	report('…after 10 s it goes to chapter 1', went ? [] : [`still at ${(await evaluate('location.pathname'))}`]);

	// 5. "Abbrechen", then "Nochmal ansehen"
	await open(`${viewer}${prefix}/${toon}/chapter/0`);
	await waitFor(VIDEO);
	await evaluate(playToEnd);
	await waitFor(`(() => { const s = ${BAR}; return s.shown && s.counting ? s : null; })()`, 20000);
	await click('[data-testid="next-bar-cancel"]');
	await sleep(6000);
	s = await evaluate(BAR);
	report(`"${nb.cancel}" stops the countdown (the bar and its button stay)`, s.shown && !s.counting && !s.countdown && s.path === `${prefix}/${toon}/chapter/0` && s.primary ? [] : [JSON.stringify(s)]);
	await shot(`next-${name}-5-cancelled`);
	await click('[data-testid="next-bar-replay"]');
	v = await waitFor(`(() => { const s = ${VIDEO}; return s && !s.paused && !s.ended && s.time < 5 && !document.querySelector('[data-testid="next-bar"]') ? s : null; })()`, 8000);
	s = await evaluate(BAR);
	report(`"↺ ${nb.replay_video}" plays the video again from the start, the bar goes`, v && !s.shown ? [] : [`video ${JSON.stringify(await evaluate(VIDEO))}, bar ${s.shown}`]);

	// 6. chapter 1's end → chapter 2 (a draft: readable in dev)
	await open(`${viewer}${prefix}/${toon}/chapter/1`);
	await waitFor(`document.querySelector('[data-slice-index="0"]') ? true : null`, 20000);
	s = await reachEnd(60000);
	{
		const problems = [];
		const label = fill(nb.chapter, { chapter: chapterName(2), title: titles[locale][2].title });
		if (!s?.shown) problems.push('no bar at the end of chapter 1');
		else {
			if (s.kind !== 'chapter' || !s.counting) problems.push(`kind ${s.kind}, counting ${s.counting}`);
			if (s.primary !== label || s.primaryHref !== `${prefix}/${toon}/chapter/2`) problems.push(`primary "${s.primary}" → ${s.primaryHref}, expected "${label}"`);
			if (s.secondary !== `↺ ${nb.replay}`) problems.push(`secondary "${s.secondary}"`);
			barLook(s, problems);
		}
		report(`chapter 1's end: "${label} →" + "↺ ${nb.replay}" + countdown`, problems, JSON.stringify(s));
	}
	await shot(`next-${name}-6-ch1-end`);
	await evaluate('window.scrollBy(0, -3000), true');
	await sleep(600);
	s = await evaluate(BAR);
	report('scrolling back up stops the countdown', s.shown && !s.counting && s.path === `${prefix}/${toon}/chapter/1` ? [] : [JSON.stringify(s)]);
	await sleep(10500);
	report('…and nothing navigates', (await evaluate('location.pathname')) === `${prefix}/${toon}/chapter/1` ? [] : ['navigated']);
	// opening an overlay stops it: a fresh showing (replay, then the end again)
	await click('[data-testid="next-bar-replay"]');
	const top = await waitFor(`scrollY < 5 && !document.querySelector('[data-testid="next-bar"]') ? true : null`, 5000);
	report(`"↺ ${nb.replay}" starts the chapter from the top`, top ? [] : [`scrollY ${await evaluate('scrollY')}, bar ${(await evaluate(BAR)).shown}`]);
	s = await reachEnd(60000);
	const countingAgain = s?.counting;
	await click('.sequence-navigation-toggle');
	await sleep(500);
	s = await evaluate(BAR);
	report('the end again counts; opening the chapter list stops it', countingAgain && !s.counting ? [] : [`counting before ${countingAgain}, after ${s.counting}`]);
	await evaluate(`document.querySelector('.sequence-navigation-toggle')?.click(), true`);

	// 7. a group with a group after it
	await open(`${viewer}${prefix}/${toon}/chapter/2?label=scene-15`);
	await waitFor(`document.querySelector('[data-slice-index="0"]') ? true : null`, 20000);
	s = await reachEnd();
	report(`a group's end (chapter 2 scene-15): "${nb.next} →" + countdown`, s?.shown && s.kind === 'group' && s.counting && s.primary === nb.next && s.primaryHref === `${prefix}/${toon}/chapter/2?label=scene-16` && s.secondary === `↺ ${nb.replay}` ? [] : [JSON.stringify(s)]);
	await shot(`next-${name}-7-group`);
	const nextGroup = await waitFor(`location.search === '?label=scene-16' ? true : null`, 12500);
	report('…after 10 s the next group (?label=scene-16)', nextGroup ? [] : [`at ${await evaluate('location.pathname + location.search')}`]);

	// 8. coming soon
	await open(`${viewer}${prefix}/${toon}/chapter/2?label=scene-20`);
	await waitFor(`document.querySelector('[data-slice-index="0"]') ? true : null`, 20000);
	s = await reachEnd();
	{
		const c3 = titles[locale][3] ?? { release: '1970-01-01' };
		const date = new Intl.DateTimeFormat(locale, { dateStyle: 'long', timeZone: 'UTC' }).format(new Date(`${c3.release}T00:00:00Z`));
		const text = fill(nb.coming_soon, { chapter: chapterName(3), date });
		report(`coming soon: "${text}" + "${nb.overview}", no countdown`, s?.shown && s.kind === 'coming-soon' && !s.counting && s.message === text && s.primary === nb.overview && s.primaryHref === `${prefix || ''}/${toon}` && !s.secondary ? [] : [JSON.stringify(s)]);
	}
	await shot(`next-${name}-8-coming-soon`);

	// 9. the season's end
	rewrite.seasonEnd = true;
	await open(`${viewer}${prefix}/${toon}/chapter/2?label=scene-20`);
	await waitFor(`document.querySelector('[data-slice-index="0"]') ? true : null`, 20000);
	s = await reachEnd();
	rewrite.seasonEnd = false;
	{
		const text = fill(nb.season_end, { season: 1 });
		report(`season end: "${text}" + "${nb.overview}" + "${msg.season.download}", no countdown`, s?.shown && s.kind === 'season-end' && !s.counting && s.message === text && s.primary === nb.overview && s.secondary === msg.season.download && s.secondaryHref === `${prefix}/${toon}/download` ? [] : [JSON.stringify(s)]);
	}
	await shot(`next-${name}-9-season-end`);

	// 10. signed in without a membership
	rewrite.noAccess = true;
	await open(`${viewer}${prefix}/${toon}/chapter/0`);
	await waitFor(VIDEO);
	await evaluate(playToEnd);
	s = await waitFor(`(() => { const s = ${BAR}; return s.shown ? s : null; })()`, 20000);
	rewrite.noAccess = false;
	report(`signed in, no membership: "${nb.redeem} →" + "${nb.where_code}", no countdown`, s?.kind === 'members' && !s.counting && s.primary === nb.redeem && s.primaryTag === 'BUTTON' && s.secondary === nb.where_code && /^https?:\/\//.test(s.secondaryHref ?? '') && s.message === fill(nb.members_hint_signed_in, { chapter: chapterName(1) }) ? [] : [JSON.stringify(s)]);
	await shot(`next-${name}-10-signed-in-no-membership`);

	// 11. a VN group: the bar only after the scene finished
	rewrite.sandboxNoJump = true;
	await open(`${viewer}${prefix}/sandbox/chapter/2`);
	await waitFor(`document.querySelector('.continue-overlay') ? true : null`, 30000);
	{
		const problems = [];
		let clicks = 0, early = 0, shownAfter = null;
		for (let i = 0; i < 60; i++) {
			await evaluate(scrollToEnd);
			await sleep(250);
			const st = await evaluate(`({ bar: !!document.querySelector('[data-testid="next-bar"]'), waiting: !!document.querySelector('.continue-overlay'), choices: !!document.querySelector('.vn-choices button, .choices button') })`);
			if (st.bar && (st.waiting || st.choices)) early++;
			if (st.bar) { shownAfter = clicks; break; }
			if (st.waiting) { await evaluate(`document.querySelector('.continue-overlay')?.click(), true`); clicks++; await sleep(350); }
			else await sleep(400);
		}
		s = await evaluate(BAR);
		if (early) problems.push(`the bar showed ${early}× while a message waited`);
		if (shownAfter === null) problems.push(`no bar after ${clicks} clicks`);
		else if (s.kind !== 'chapter' || !s.counting || s.primaryHref !== `${prefix}/sandbox/chapter/3`) problems.push(JSON.stringify(s));
		report(`sandbox VN chapter 2 (final jump dropped): no bar while messages wait, the bar after the last click (${clicks} clicks) → chapter 3`, problems);
	}
	await shot(`next-${name}-11-vn-finished`);
	rewrite.sandboxNoJump = false;
	await open(`${viewer}/`);
}

async function once() {
	console.log('390-de login round trip + redeem');
	await send('Emulation.setDeviceMetricsOverride', { width: 390, height: 844, deviceScaleFactor: 1, mobile: true });
	const msg = messages.de;
	await asGuest('de');
	await open(`${viewer}/${toon}/chapter/0`);
	await waitFor(VIDEO);
	await evaluate(playToEnd);
	await waitFor(`(() => { const s = ${BAR}; return s.shown ? s : null; })()`, 20000);
	await click('[data-testid="next-bar-primary"]');
	const onAccount = await waitFor(`location.pathname === '/account' && document.querySelector('input[type="email"]') ? location.search : null`, 10000);
	const mailLog = join(root, 'server/logs/mail.log');
	const before = existsSync(mailLog) ? readFileSync(mailLog, 'utf8').length : 0;
	await evaluate(`(() => { const i = document.querySelector('input[type="email"]'); i.value = 'agent@scutoons.test'; i.dispatchEvent(new Event('input', { bubbles: true })); i.form.requestSubmit(); return true; })()`);
	let otp = null;
	for (let i = 0; i < 40 && !otp; i++) {
		await sleep(250);
		const tail = existsSync(mailLog) ? readFileSync(mailLog, 'utf8').slice(before) : '';
		otp = /\b(\d{6})\b/.exec(tail)?.[1] ?? null;
	}
	await waitFor(`document.querySelector('input[name="otp"]') ? true : null`, 10000);
	await evaluate(`(() => { const i = document.querySelector('input[name="otp"]'); i.value = ${JSON.stringify(otp ?? '')}; i.dispatchEvent(new Event('input', { bubbles: true })); i.form.requestSubmit(); return true; })()`);
	const back = await waitFor(`location.pathname === '/${toon}/chapter/1' && document.querySelector('[data-slice-index="0"]') && !document.querySelector('.view-membership-blocker') ? true : null`, 20000);
	report(`"${msg.next_bar.sign_in}" → /account${onAccount ?? '?'} → signed in (OTP) → back in chapter 1, readable`, onAccount && otp && back ? [] : [`account ${onAccount}, otp ${!!otp}, at ${await evaluate('location.pathname')}`]);
	await shot('next-390-de-12-after-login-ch1');

	// redeeming a code as a guest: the member bar
	await asGuest('de');
	await open(`${viewer}/${toon}/chapter/0?code=promo_test`);
	await waitFor(`document.querySelector('.claim-backdrop') ? true : null`, 10000);
	await waitFor(`JSON.parse(localStorage.getItem('scutoons_access') || 'null')?.permissions?.includes('catalog') ? true : null`, 10000);
	await evaluate(`document.querySelector('.claim-backdrop')?.click(), true`);
	await waitFor(VIDEO);
	await evaluate(playToEnd);
	const s = await waitFor(`(() => { const s = ${BAR}; return s.shown ? s : null; })()`, 20000);
	report('a guest who redeemed a code (?code=promo_test) gets the member bar: "S1 K1 lesen →" + countdown', s?.kind === 'chapter' && s.counting ? [] : [JSON.stringify(s)]);

	// reduced motion: no autoplay, no countdown
	console.log('390-de reduced motion (member)');
	await send('Emulation.setEmulatedMedia', { features: [{ name: 'prefers-reduced-motion', value: 'reduce' }] });
	await asMember('de');
	await open(`${viewer}/${toon}/chapter/0`);
	await waitFor(`(() => { const s = ${VIDEO}; return s && s.readyState >= 1 ? s : null; })()`, 15000);
	await sleep(1500);
	const still = await evaluate(VIDEO);
	report('reduced motion: no autoplay (poster + native play)', still?.paused && still.time === 0 && !still.soundOn ? [] : [JSON.stringify(still)]);
	await evaluate(playToEnd);
	const r = await waitFor(`(() => { const s = ${BAR}; return s.shown ? s : null; })()`, 20000);
	report('reduced motion: the bar without a countdown', r?.kind === 'chapter' && !r.counting && !r.countdown ? [] : [JSON.stringify(r)]);
	await shot('next-390-de-13-reduced-motion');
	await send('Emulation.setEmulatedMedia', { features: [] });
}

const combos = [[390, 'de'], [390, 'en'], [1440, 'de'], [1440, 'en']].filter(([w, l]) => !opt.only || String(opt.only).split(',').includes(`${w}-${l}`));
for (const [w, l] of combos) await run(w, l);
if (!opt.only || opt.once) await once();

console.log(`\n${passed} passed, ${failed} failed`);
await done(failed ? 1 : 0);
