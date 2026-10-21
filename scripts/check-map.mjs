// World map check (agents/MAP.md M0–M2, §8.5a/b): the reader's map and the creator's map builder, headless Chrome via CDP
// (own temp profile, no dependencies; Node 22 WebSocket), like scripts/check-next-bar.mjs.
// Since 2026-10-11 a place is unlocked ONLY by an `unlockLocation` event (MAP.md §8.5b, owner: "Freischalten von Locations
// sollten wir an diesen Events machen"): the migration put every old place unlock on its slice as an intersect-triggered
// event `unlock-<place>`. No reading-progress rule, no `unlock` on a place, no Unlock tab / slice map section any more –
// the check reads the events from the chapter documents the API serves (`/library/toons/catpope/chapters/<n>?draft=1`,
// slices[].events with `type: unlockLocation`, `trigger.type: intersect`).
//
// Reader (guest; 390 de and 1440 en):
//   1. a fresh guest: the map opens (toggle), its sea is the toon's colour, it opens at the toon's start view, no place
//      is visible (catpope's places are hidden until an event unlocks them), "Orte" says nothing was discovered;
//   2. the guest's world state `scutoons_game_scutoons` gets the places of chapter 1 + 2's unlock events
//      ({ map: { places: { <id>: { unlocked } } } }, as if read): after a reload exactly those are in it, the toggle shows
//      the badge, the opened map shows them with the "new" look (`.place.new`), the ones on screen are marked seen (painted
//      in place, then `seen`) and the badge goes;
//   3. pan: a mouse drag moves the world by the drag; zoom: the "+" key (×1.5) and the wheel (at the cursor) change the
//      scale (no zoom buttons since 2026-10-11);
//   4. no two visible labels overlap (the phone overlap fix);
//   5. a click on a (seen) place in "Orte" opens its card (name; "Kapitel N lesen" only when that chapter is readable for
//      this reader) – a new one would play its reveal instead;
//   6. "Orte" lists the visible places A–Z;
//   7. universe-wide: in another public toon (none in the production data: skipped) the unlocked catpope places are on the
//      map as plain markers (no label);
//   8. markers are round (bbox width = height) and screen-sized after zooming;
// Reading (a fresh guest with a promo code redeemed on the device, 390 de): the unlock events fire as their slices scroll
// in (an intersect trigger: half the slice on screen; the map never opens for it – scrolling is never hijacked, §8.5a):
//  R1. chapter 1 scrolled slice by slice: each event's place enters the world state at its slice, not before, with the toast
//      "<Ort> freigeschaltet" (`notice-place`; none for a `silent` event), the map stays closed;
//  R2. chapter 2 group by group (?label=), the same – only when a guest can read it (the dev DB is production's: chapter 2
//      is "coming soon" since 2026-10-06, then R2 says so and is skipped);
//      then the opened map shows exactly the places unlocked while reading.
//  Practical limits: only slice events with an intersect trigger and no `if` are read (VN-script / click / item events
//  would need the story played); chapters a guest can't open are skipped.
// Member (the test user, `npm run dev:test-user`):
//   8. a guest's discoveries merge into the account's world state on sign-in (set merge, nothing lost on either side);
//      the member's server state is restored afterwards.
// Builder (creator /map as the test user):
//   9. "+ Ort": a click on the canvas adds a place, the de name sets its id; a drag moves it (x/y follow the drag);
//  10. "Weg zeichnen": the new place + a free point + Enter → a route with 2 waypoints (place, free point);
//  11. Save draft → the stored draft has the place and the route; the published map is restored afterwards (the map must
//      have no unpublished draft before the check, else it refuses to run – someone's work in progress is never touched);
//  12. a place image on its ground point (default 50 / 50 %) at the place's size (world units × zoom, default 4 – the hex
//      footprint, 2026-10-11) with the anchor editor and 3 previews; 13. reader view sea colours;
//  14. the place's unlock events (the builder reads `/creator/flow-index` placeUnlocks): the new place made hidden gets the
//      warning "… is hidden and no unlockLocation event unlocks it" in the problems table; none of the places the
//      events unlock has it;
//  15. Set start view → the toon entry's map { center, zoom }; 16. Wanted images → every place without an image;
//  17. every hidden place of the published map is unlocked by some event (flow-index), and every intersect unlock event
//      in the chapters names a place of the map (an unknown one fizzles in the reader).
//
// usage: node scripts/check-map.mjs [--viewer=http://localhost:8000] [--creator=http://localhost:10000]
//        [--api=http://localhost:9000/api] [--shots=<dir>] [--only=reader|reading|member|builder] [--verbose]
//        (the promo code and the member's save game are read / restored in the database SCUTOONS_DB_NAME, default
//        `scutoons` – set it for a worktree's own database)
// Exit code 1 on any failure.
import { execFileSync, spawn } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { local } from './dev/ports.mjs';
import { saveGameRow } from './dev/save-game-row.mjs';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const opt = Object.fromEntries(process.argv.slice(2).filter((a) => a.startsWith('--')).map((a) => { const [k, ...v] = a.slice(2).split('='); return [k, v.length ? v.join('=') : true]; }));
const viewer = String(opt.viewer || local('viewer')).replace(/\/$/, '');
const creator = String(opt.creator || local('creator')).replace(/\/$/, '');
const api = String(opt.api || local('api', '/api')).replace(/\/$/, '');
const shots = opt.shots ? String(opt.shots) : null;
if (shots) mkdirSync(shots, { recursive: true });
const only = opt.only ? String(opt.only) : null;
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const messages = Object.fromEntries(['de', 'en'].map((l) => [l, JSON.parse(readFileSync(join(root, `viewer/messages/${l}.json`), 'utf8'))]));
const creatorMessages = Object.fromEntries(['de', 'en'].map((l) => [l, JSON.parse(readFileSync(join(root, `creator/messages/${l}.json`), 'utf8'))]));
const fill = (s, v) => s.replace(/\{(\w+)\}/g, (_, k) => String(v[k]));

// ── the test user ───────────────────────────────────────────────────────────────────────────────────────────────────
const sessionFile = join(root, 'logs/dev/test-user.json');
const token = existsSync(sessionFile) ? JSON.parse(readFileSync(sessionFile, 'utf8')).token : null;
if (!token || !(await fetch(`${api}/me/`, { headers: { Authorization: `Bearer ${token}` } })).ok) { console.error('no working test-user session (run `npm run dev:test-user`)'); process.exit(2); }
const auth = { Authorization: `Bearer ${token}`, 'Content-Type': 'application/json' };
// the member's save game (map + every toon's progress, row `scutoons`): put back exactly (a PUT merges)
const saveGame = saveGameRow(JSON.parse(readFileSync(sessionFile, 'utf8')).email);
// a promo code of the dev database for a guest's reading access (`npm run dev:db` makes `devpromo`; the SQLite era's
// `promo_test` first when it is there – like scripts/dev/test-user.mjs)
const promo = (() => {
	try {
		const container = process.env.SCUTOONS_DB_CONTAINER ?? 'scutoons-mariadb-1';
		const db = process.env.SCUTOONS_DB_NAME ?? 'scutoons';
		const query = "SELECT code FROM qr WHERE type = 'promo' AND revoked_at IS NULL AND (expires_at IS NULL OR expires_at > NOW()) ORDER BY (code = 'promo_test') DESC, created_at LIMIT 1";
		return execFileSync('docker', ['exec', '-i', container, 'sh', '-c', `mariadb -uroot -p"$MARIADB_ROOT_PASSWORD" -N -r ${db}`], { input: query }).toString().trim() || 'promo_test';
	} catch {
		return 'promo_test';
	}
})();
const world = Object.fromEntries(await Promise.all(['de', 'en'].map(async (l) => [l, await (await fetch(`${api}/library/map?locale=${l}&toon=catpope`)).json()])));
const bundle = Object.fromEntries(await Promise.all(['de', 'en'].map(async (l) => [l, await (await fetch(`${api}/library/toons/catpope?locale=${l}`)).json()])));
const mapEntity = await (await fetch(`${api}/entities/map?draft=1&wrap=1`, { headers: auth })).json();

/** a #rrggbb / #rgb colour as the browser reports it */
function rgb(hex) {
	const h = hex.replace('#', '');
	const full = h.length === 3 ? [...h].map((c) => c + c).join('') : h;
	return `rgb(${parseInt(full.slice(0, 2), 16)}, ${parseInt(full.slice(2, 4), 16)}, ${parseInt(full.slice(4, 6), 16)})`;
}

const now = new Date().toISOString();
/** a catpope chapter's document as the creator / a draft reader gets it (groups → slices → events) */
const chapterDocs = {};
const chapterOf = async (n) => (chapterDocs[n] ??= await (await fetch(`${api}/library/toons/catpope/chapters/${n}?locale=de&draft=1`, { headers: auth })).json());
/**
 * a chapter's unlock events that fire on scrolling (MAP.md §8.5b: `unlockLocation` with an intersect trigger, no `if`),
 * in reading order: { group (label), groupIndex, slice (0-based in the group), image, location, silent }
 */
function sliceUnlocks(doc) {
	return (doc?.groups ?? []).flatMap((g, gi) => (g.slices ?? []).flatMap((s, si) => (s.events ?? [])
		.filter((e) => e?.type === 'unlockLocation' && e.trigger?.type === 'intersect' && !e.if && typeof e.params?.location === 'string')
		.map((e) => ({ group: g.label, groupIndex: gi, slice: si, image: s.image ?? '', location: e.params.location, silent: !!e.params.silent }))));
}

// ── Chrome ──────────────────────────────────────────────────────────────────────────────────────────────────────────
const CHROME = process.env.CHROME || ['C:/Program Files/Google/Chrome/Application/chrome.exe', 'C:/Program Files (x86)/Google/Chrome/Application/chrome.exe', 'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe', '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome', '/usr/bin/google-chrome'].find((p) => existsSync(p));
if (!CHROME) { console.error('Chrome not found (set CHROME)'); process.exit(2); }
const port = 9400 + Math.floor(Math.random() * 400);
const profile = mkdtempSync(join(tmpdir(), 'check-map-'));
const chrome = spawn(CHROME, ['--headless=new', `--remote-debugging-port=${port}`, `--user-data-dir=${profile}`, '--no-first-run', '--no-default-browser-check', '--disable-extensions', '--mute-audio', '--window-size=1440,900', 'about:blank'], { stdio: 'ignore' });
let restoreMap = null;
async function restore() {
	if (!saveGame.restore()) console.log('  could not restore the save game row');
	if (restoreMap) await restoreMap().catch((e) => console.log(`  could not restore the map: ${e}`));
}
async function done(code) {
	await restore();
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
const consoleErrors = [];
const send = (method, params = {}) => new Promise((r) => { const i = ++seq; pending.set(i, r); ws.send(JSON.stringify({ id: i, method, params })); });
ws.addEventListener('message', (e) => {
	const m = JSON.parse(e.data), p = m.params;
	if (m.id && pending.has(m.id)) { pending.get(m.id)(m); pending.delete(m.id); return; }
	if (m.method?.startsWith('Network.')) lastNet = Date.now();
	if (m.method === 'Runtime.exceptionThrown') consoleErrors.push(p.exceptionDetails?.exception?.description ?? p.exceptionDetails?.text);
	if (m.method === 'Runtime.consoleAPICalled' && p.type === 'error') consoleErrors.push(p.args?.map((a) => a.value ?? a.description).join(' '));
	if (m.method === 'Page.javascriptDialogOpening') send('Page.handleJavaScriptDialog', { accept: true });
});
const evaluate = async (expression) => { const r = await send('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true }); return r.result?.exceptionDetails ? { error: r.result.exceptionDetails.exception?.description ?? r.result.exceptionDetails.text } : r.result?.result?.value; };
await send('Page.enable'); await send('Runtime.enable'); await send('Network.enable');
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
const center = (selector) => evaluate(`(() => { const el = document.querySelector(${JSON.stringify(selector)}); if (!el) return null; const b = el.getBoundingClientRect(); return { x: b.left + b.width / 2, y: b.top + b.height / 2 }; })()`);
async function mouse(type, x, y, extra = {}) { await send('Input.dispatchMouseEvent', { type, x, y, button: 'left', clickCount: 1, ...extra }); }
async function click(selector) {
	const p = await center(selector);
	if (!p || p.error) return false;
	await mouse('mousePressed', p.x, p.y);
	await mouse('mouseReleased', p.x, p.y);
	return true;
}
/** Save draft; the in-page note dialog that follows (askDialog) is skipped */
async function saveDraft() {
	await click('[data-testid="save-draft"]');
	if (await waitFor(`document.querySelector('dialog.sc-dialog[open] button.primary') ? true : null`, 3000)) await click('dialog.sc-dialog[open] button.primary');
}
/** the toon filter: the pill at the end of the crumbs (owner 2026-10-11: like on phones) – `all` = every toon */
async function pickToon(t) {
	await click('[data-testid="toon-filter"]');
	await click(`[data-testid="toon-${t}"]`);
}
async function clickAt(x, y, clickCount = 1) { await mouse('mousePressed', x, y, { clickCount }); await mouse('mouseReleased', x, y, { clickCount }); }
async function drag(from, to, steps = 8) {
	await mouse('mousePressed', from.x, from.y);
	for (let i = 1; i <= steps; i++) await send('Input.dispatchMouseEvent', { type: 'mouseMoved', x: from.x + ((to.x - from.x) * i) / steps, y: from.y + ((to.y - from.y) * i) / steps, button: 'left', buttons: 1 });
	await mouse('mouseReleased', to.x, to.y);
}
async function typeText(text) { for (const ch of text) await send('Input.dispatchKeyEvent', { type: 'char', text: ch }); }
async function key(k, code = k) { for (const type of ['keyDown', 'keyUp']) await send('Input.dispatchKeyEvent', { type, key: k, code, windowsVirtualKeyCode: k === 'Enter' ? 13 : 0 }); }

let failed = 0, passed = 0;
function report(name, problems, detail = '') {
	if (problems.length) { failed++; console.log(`  FAIL  ${name}`); for (const p of problems) console.log(`        ${p}`); }
	else { passed++; console.log(`  ok    ${name}${opt.verbose && detail ? `  ${detail}` : ''}`); }
}
async function setLocale(locale, url) { await send('Network.setCookie', { name: 'PARAGLIDE_LOCALE', value: locale, url, path: '/' }); }

const MAP = `(() => {
	const o = document.querySelector('[data-testid="map-overlay"]');
	const w = document.querySelector('[data-testid="map-world"]');
	const t = w ? getComputedStyle(w).transform : null;
	const m = t && t !== 'none' ? t.match(/matrix\\(([^)]+)\\)/)[1].split(',').map(Number) : null;
	const labels = [...document.querySelectorAll('[data-testid^="map-label-"]')].map((l) => { const b = l.getBoundingClientRect(); return { id: l.dataset.testid.slice(10), x0: b.left, y0: b.top, x1: b.right, y1: b.bottom, text: l.textContent }; });
	return { open: !!o?.classList.contains('visible'), places: [...document.querySelectorAll('[data-testid^="map-place-"]')].map((b) => b.dataset.testid.slice(10)),
		newPlaces: [...document.querySelectorAll('.place.new [data-testid^="map-place-"]')].map((b) => b.dataset.testid.slice(10)),
		labels, scale: m ? m[0] : null, tx: m ? m[4] : null, ty: m ? m[5] : null, badge: !!document.querySelector('[data-testid="map-badge"]'),
		card: document.querySelector('[data-testid="map-card"] h2')?.textContent ?? null, cardLink: document.querySelector('[data-testid="map-card-chapter"]')?.textContent.trim() ?? null,
		list: [...document.querySelectorAll('[data-testid="map-list"] li button')].map((b) => b.firstChild?.textContent.trim()),
		world: JSON.parse(localStorage.getItem('scutoons_game_scutoons') || 'null') };
})()`;

/** opens the reader's map with its toggle – again while the page still hydrates (a cold dev server: the first click is lost) */
async function openMap() {
	for (let i = 0; i < 3; i++) {
		if (!(await waitFor(`document.querySelector('[data-testid="map-toggle"]') ? true : null`, 10000))) return;
		await click('[data-testid="map-toggle"]');
		if (await waitFor(`document.querySelector('[data-testid="map-overlay"].visible') ? true : null`, 5000)) return;
	}
}

// ── reader ──────────────────────────────────────────────────────────────────────────────────────────────────────────
async function reader(width, locale) {
	const prefix = locale === 'en' ? '/en' : '';
	const msg = messages[locale];
	const data = world[locale];
	console.log(`reader ${width} ${locale}`);
	await send('Emulation.setDeviceMetricsOverride', { width, height: width < 768 ? 844 : 900, deviceScaleFactor: 1, mobile: width < 768 });
	await send('Network.clearBrowserCookies');
	await setLocale(locale, viewer);
	await open(`${viewer}${prefix}/catpope`);
	await evaluate('localStorage.clear(), sessionStorage.clear(), true');
	await open(`${viewer}${prefix}/catpope`);

	// 1. fresh guest
	await openMap();
	let s = await waitFor(`(() => { const s = ${MAP}; return s.open && s.scale ? s : null; })()`);
	report('fresh guest: the map opens, no place visible yet (they unlock while reading)', s && s.places.length === 0 ? [] : [JSON.stringify(s && { open: s.open, places: s.places })]);
	{
		// the sea is the toon's colour (owner 2026-10-01: /library/map view.color = tile colour, else the star's)
		const bg = await evaluate(`getComputedStyle(document.querySelector('[data-testid="map-overlay"]')).backgroundColor`);
		const want = rgb(data.view?.color ?? '#00ff00');
		report(`the sea is the toon's colour (${data.view?.color ?? 'default'})`, bg === want ? [] : [`${bg} ≠ ${want}`]);
		// the map opens at the toon's start view (set in the creator; the toon entry's map.center / zoom)
		const vp = await evaluate(`(() => { const b = document.querySelector('[data-testid="map-viewport"]').getBoundingClientRect(); return { w: b.width, h: b.height }; })()`);
		const cx = (vp.w / 2 - s.tx) / s.scale, cy = (vp.h / 2 - s.ty) / s.scale;
		const v = data.view;
		report(`it opens at the toon's start view (${v.center.x}, ${v.center.y} · ${Math.round(v.zoom * 100)} %)`, Math.abs(s.scale - v.zoom) < 0.01 && Math.abs(cx - v.center.x) <= 3 && Math.abs(cy - v.center.y) <= 3 ? [] : [`centre ${Math.round(cx)}, ${Math.round(cy)} · ${s.scale}`]);
	}
	await click('[data-testid="map-list-toggle"]');
	const empty = await waitFor(`document.querySelector('[data-testid="map-list"] p')?.textContent ?? null`, 3000);
	report('"Orte" says nothing was discovered yet', empty === msg.map.places_empty ? [] : [`"${empty}"`]);
	await shot(`map-${width}-${locale}-1-empty`);

	// 2. unlocked places in the world state (as chapter 1 + 2's unlock events would have put them; the events themselves:
	//    the reading section)
	const ids = new Set(data.locations.filter((p) => p.hidden).map((p) => p.id));
	const expected = [...new Set([...sliceUnlocks(await chapterOf(1)), ...sliceUnlocks(await chapterOf(2))].map((u) => u.location))].filter((id) => ids.has(id)).sort();
	const seeded = { map: { places: Object.fromEntries(expected.map((id, i) => [id, { unlocked: Date.now() - 1000 * (i + 1) }])), routes: {}, positions: {} }, created_at: now, updated_at: now };
	await evaluate(`localStorage.setItem('scutoons_game_scutoons', ${JSON.stringify(JSON.stringify(seeded))}), true`);
	await open(`${viewer}${prefix}/catpope`);
	s = await waitFor(`(() => { const s = ${MAP}; return s.badge && s.world ? s : null; })()`, 10000);
	{
		const got = Object.keys(s?.world?.map?.places ?? {}).sort();
		report(`unlocked places in the world state (${expected.length}: ${expected.join(', ')}) are kept, the toggle shows the badge`, s && expected.length && JSON.stringify(got) === JSON.stringify(expected) ? [] : [`world state ${JSON.stringify(got)}`, `badge ${s?.badge}`]);
	}
	await openMap();
	s = await waitFor(`(() => { const s = ${MAP}; return s.open && s.places.length ? s : null; })()`);
	report('the opened map shows them, new ones with the halo', s && JSON.stringify([...s.places].sort()) === JSON.stringify(expected) && s.newPlaces.length > 0 ? [] : [JSON.stringify(s && { places: s.places, isNew: s.newPlaces })]);
	await shot(`map-${width}-${locale}-2-unlocked`);
	// a new place on screen is painted in place (≈ 1.5 s + 2.3 s), then seen; the badge goes once none is new any more –
	// places off screen stay new (and keep the badge) until they are looked at
	const seen = await waitFor(`(() => { const s = ${MAP}; const p = Object.values(s.world.map.places); return p.some((x) => x.seen) && s.badge === p.some((x) => x.seen === undefined) ? s : null; })()`, 8000);
	report('places on screen are marked seen (the badge stays only while one is still new)', seen ? [] : [JSON.stringify((({ badge, world }) => ({ badge, places: world?.map?.places }))(await evaluate(MAP)))]);
	await shot(`map-${width}-${locale}-2b-seen`);

	// 4. labels never overlap
	s = await evaluate(MAP);
	{
		const overlaps = [];
		for (let i = 0; i < s.labels.length; i++) for (let j = i + 1; j < s.labels.length; j++) {
			const a = s.labels[i], b = s.labels[j];
			if (a.x0 < b.x1 - 1 && a.x1 > b.x0 + 1 && a.y0 < b.y1 - 1 && a.y1 > b.y0 + 1) overlaps.push(`${a.text} × ${b.text}`);
		}
		report(`no overlapping labels (${s.labels.length} shown)`, s.labels.length && !overlaps.length ? [] : overlaps.length ? overlaps : ['no label shown']);
	}

	// 3. pan + zoom
	const vp = await center('[data-testid="map-viewport"]');
	const before = await evaluate(MAP);
	await drag({ x: vp.x - 40, y: vp.y + 60 }, { x: vp.x + 20, y: vp.y + 100 });
	let after = await evaluate(MAP);
	report('a drag pans the world by the drag (60, 40 px)', Math.abs(after.tx - before.tx - 60) <= 2 && Math.abs(after.ty - before.ty - 40) <= 2 ? [] : [`moved ${after.tx - before.tx}, ${after.ty - before.ty}`]);
	// no zoom buttons any more (owner 2026-10-11: fingers / mouse) – the "+" key still zooms (sent from the page: the key
	// handlers look at the event target's `closest`)
	await evaluate(`document.body.dispatchEvent(new KeyboardEvent('keydown', { key: '+', bubbles: true })), true`);
	await sleep(100);
	after = await evaluate(MAP);
	report('"+" (key) zooms ×1.5', Math.abs(after.scale / before.scale - 1.5) < 0.01 ? [] : [`scale ${before.scale} → ${after.scale}`]);
	await send('Input.dispatchMouseEvent', { type: 'mouseWheel', x: vp.x, y: vp.y, deltaX: 0, deltaY: 200 });
	await sleep(200);
	const wheeled = await evaluate(MAP);
	report('the wheel zooms out at the cursor', wheeled.scale < after.scale ? [] : [`scale ${after.scale} → ${wheeled.scale}`]);
	await shot(`map-${width}-${locale}-3-zoomed`);
	{
		// 8. markers stay round and screen-sized at every zoom (owner screenshot 2026-10-01: tall ellipses)
		const boxes = await evaluate(`[...document.querySelectorAll('[data-testid^="map-place-"]')].map((b) => { const r = b.getBoundingClientRect(); return [b.dataset.testid.slice(10), Math.round(r.width * 10) / 10, Math.round(r.height * 10) / 10]; })`);
		const bad = boxes.filter(([, w, h]) => !w || Math.abs(w - h) > 1 || h > 24);
		report(`markers are round (width = height, at most 24 px) at ${Math.round(wheeled.scale * 100)} % (${boxes.length})`, boxes.length && !bad.length ? [] : bad.length ? bad.map((b) => b.join(' ')) : ['no marker']);
	}

	// 5. card
	// a seen place (a new one's tap plays its reveal first – owner 2026-10-11, "der kleine Punkt")
	const seenIds = Object.entries((await evaluate(MAP)).world?.map?.places ?? {}).filter(([, p]) => p.seen !== undefined).map(([id]) => id);
	const first = data.locations.find((p) => p.id === (expected.find((id) => seenIds.includes(id)) ?? expected[0]));
	await evaluate(`(() => { document.querySelector('[data-testid="map-place-${first.id}"]').scrollIntoView(); return true; })()`);
	await click('[data-testid="map-list-toggle"]');
	const listed = await waitFor(`(() => { const s = ${MAP}; return s.list.length ? s.list : null; })()`, 3000);
	const names = expected.map((id) => data.locations.find((p) => p.id === id).name).sort((a, b) => a.localeCompare(b));
	report('"Orte" lists the visible places A–Z', JSON.stringify(listed) === JSON.stringify(names) ? [] : [JSON.stringify(listed), JSON.stringify(names)]);
	await evaluate(`(() => { [...document.querySelectorAll('[data-testid="map-list"] li button')].find((b) => b.firstChild?.textContent.trim() === ${JSON.stringify(first.name)})?.click(); return true; })()`);
	s = await waitFor(`(() => { const s = ${MAP}; return s.card ? s : null; })()`, 3000);
	const chapterItem = bundle[locale].seasons[0].chapters.find((c) => c.number === first.chapter?.number);
	const readable = !!chapterItem && ['published', 'draft'].includes(chapterItem.status) && !!chapterItem.isPreview;
	const link = readable ? fill(msg.map.read_chapter, { number: first.chapter.number }) : null;
	report(`a place's card: "${first.name}"${link ? ` + "${link}"` : ' (chapter not readable for a guest: no link)'}`, s?.card === first.name && s?.cardLink === link ? [] : [JSON.stringify({ card: s?.card, link: s?.cardLink })]);
	await shot(`map-${width}-${locale}-4-card`);

	// 7. universe-wide: another toon a guest can open shows catpope's unlocked places as plain markers. The dev DB is
	// production's (2026-10-06): the sandbox, Top Toy and Yellow Mouse are drafts there (a guest gets no toon page) – with
	// no other public toon there is nothing to look at, and it says so
	const toons = await (await fetch(`${api}/library/toons?locale=${locale}`)).json().catch(() => []);
	const other = (Array.isArray(toons) ? toons : []).find((t) => t?.id !== 'catpope' && t?.status === 'published')?.id;
	if (!other) {
		console.log('  skip  universe-wide: no other toon is public for a guest (sandbox, toptoy, yellowmouse are drafts)');
		return;
	}
	await open(`${viewer}${prefix}/${other}`);
	const ok = await waitFor(`document.querySelector('[data-testid="map-toggle"]') ? true : null`, 8000);
	if (ok) {
		await openMap();
		s = await waitFor(`(() => { const s = ${MAP}; return s.open && s.places.length ? s : null; })()`, 8000);
		report(`universe-wide: another toon (${other}) shows the discovered places, as markers without labels`, s && expected.every((id) => s.places.includes(id)) && s.labels.length === 0 ? [] : [JSON.stringify(s && { places: s.places, labels: s.labels.map((l) => l.text) })]);
	} else report(`universe-wide: the ${other} toon page`, [`no map toggle on /${other}`]);
}

// ── reading: a passed slice unlocks its places ──────────────────────────────────────────────────────────────────────
const worldPlaces = () => evaluate(`Object.keys(JSON.parse(localStorage.getItem('scutoons_game_scutoons') || 'null')?.map?.places ?? {})`);
/**
 * opens a chapter page and scrolls its slices one by one; returns the places that entered the world state per slice
 * (1-based, in the group; the first slice is on screen when the page opens), the place toasts' texts that showed and
 * whether the map overlay ever opened (an intersect trigger never opens it)
 */
async function readSlices(url) {
	let known = new Set(await worldPlaces());
	await open(url);
	const count = await waitFor(`document.querySelectorAll('[data-slice-index]').length || null`, 15000);
	await sleep(2500); // the map's data loads when the browser is idle (toon/map/map.svelte) – an unlock waits for it
	const appeared = {};
	const toasts = new Set();
	let mapOpened = false;
	const look = async () => {
		const r = await evaluate(`({ toasts: [...document.querySelectorAll('[data-testid="notice-place"]')].map((n) => n.textContent.trim()), open: !!document.querySelector('[data-testid="map-overlay"].visible') })`);
		for (const t of r?.toasts ?? []) toasts.add(t);
		if (r?.open) mapOpened = true;
	};
	const step = async (i) => {
		await look();
		const now = await worldPlaces();
		const fresh = now.filter((id) => !known.has(id));
		if (fresh.length) appeared[i + 1] = [...(appeared[i + 1] ?? []), ...fresh].sort();
		known = new Set(now);
	};
	await step(0); // what the opened page shows
	for (let i = 0; i < (count ?? 0); i++) {
		await evaluate(`(document.querySelectorAll('[data-slice-index]')[${i}].scrollIntoView({ block: 'end' }), true)`);
		await sleep(250);
		await step(i);
	}
	await sleep(800);
	await step(Math.max(0, (count ?? 1) - 1));
	return { count, appeared, toasts, mapOpened };
}
async function reading() {
	console.log('reading (fresh guest, 390 de)');
	const data = world.de;
	const nameOf = (id) => data.locations.find((p) => p.id === id)?.name ?? id;
	await send('Emulation.setDeviceMetricsOverride', { width: 390, height: 844, deviceScaleFactor: 1, mobile: true });
	await send('Network.clearBrowserCookies');
	await setLocale('de', viewer);
	await open(`${viewer}/catpope`);
	await evaluate('localStorage.clear(), sessionStorage.clear(), true');
	// a guest with reading access: a code redeemed on the device (no account), like check-next-bar.mjs
	await open(`${viewer}/catpope/chapter/0?code=${encodeURIComponent(promo)}`);
	const access = await waitFor(`JSON.parse(localStorage.getItem('scutoons_access') || 'null')?.permissions?.length ? true : null`, 10000);
	report('a fresh guest (no discoveries) with a code redeemed on the device', access && !(await worldPlaces()).length ? [] : [`access ${access}`, JSON.stringify(await worldPlaces())]);
	const onMap = new Set(data.locations.map((p) => p.id));
	/** the expected unlocks of these groups: "<label>#<slice>" → places (a place already unlocked earlier shows nothing) */
	const unlocked = new Set();
	const want = (doc, groups) => {
		const out = {};
		for (const u of sliceUnlocks(doc)) {
			if (!groups.includes(u.group) || !onMap.has(u.location) || unlocked.has(u.location)) continue;
			unlocked.add(u.location);
			(out[`${u.group}#${u.slice + 1}`] ??= []).push(u.location);
		}
		for (const k of Object.keys(out)) out[k].sort();
		return out;
	};
	const toastsOf = (doc, places) => places.filter((id) => !sliceUnlocks(doc).find((u) => u.location === id)?.silent).map((id) => fill(messages.de.map.place_unlocked, { name: nameOf(id) })).sort();
	const describe = (o) => Object.entries(o).map(([k, v]) => `${v.join('+')} @ ${k}`).join(', ');
	const checkRead = (label, doc, got, exp, r) => {
		const places = Object.values(exp).flat();
		report(`${label}: ${places.length} place(s) unlock as their slices scroll in (${describe(exp) || 'none'}), not before`,
			places.length && JSON.stringify(got) === JSON.stringify(exp) ? [] : [`got ${JSON.stringify(got)}`, `want ${JSON.stringify(exp)}`]);
		const toasts = [...r.toasts].sort();
		const wantToasts = toastsOf(doc, places);
		report(`${label}: a toast per unlocked place ("${wantToasts.join('", "')}"), the map stays closed`,
			JSON.stringify(toasts) === JSON.stringify(wantToasts) && !r.mapOpened ? [] : [`toasts ${JSON.stringify(toasts)}`, `map opened ${r.mapOpened}`]);
	};
	// R1. chapter 1 (its scene groups are one chain = one page, #145): page slice k → "<group>#<slice in it>"
	const c1 = await chapterOf(1);
	const r1 = await readSlices(`${viewer}/catpope/chapter/1`);
	const pageSlice = c1.groups.flatMap((g) => g.slices.map((_, i) => `${g.label}#${i + 1}`));
	const got1 = Object.fromEntries(Object.entries(r1.appeared).map(([k, v]) => [pageSlice[k - 1] ?? `?#${k}`, v]));
	const exp1 = want(c1, c1.groups.map((g) => g.label));
	checkRead('chapter 1', c1, got1, exp1, r1);
	await shot('map-reading-chapter-1');
	// R2. chapter 2, group by group (its walk is a VN choice: a group not visited unlocks nothing) – only when a guest can
	// read it: a coming-soon / draft chapter shows its teaser (the dev DB is production's: chapter 2 is "coming soon" there
	// since 2026-10-06 – then R2 has nothing to read and says so)
	const c2 = await chapterOf(2);
	if (!c2.groups) { report('chapter 2 is readable here', [JSON.stringify(c2).slice(0, 120)]); return; }
	const c2Status = bundle.de.seasons.flatMap((x) => x.chapters).find((c) => c.number === 2)?.status;
	let exp2 = {};
	if (c2Status !== 'published') console.log(`  skip  chapter 2 is ${c2Status ?? 'not listed'} for readers – nothing to read as a guest (R2)`);
	else {
		const got2 = {};
		const toasts = new Set();
		let mapOpened = false;
		for (const g of c2.groups) {
			const r = await readSlices(`${viewer}/catpope/chapter/2?label=${encodeURIComponent(g.label)}`);
			for (const [k, v] of Object.entries(r.appeared)) got2[`${g.label}#${k}`] = v;
			r.toasts.forEach((t) => toasts.add(t));
			mapOpened ||= r.mapOpened;
		}
		exp2 = want(c2, c2.groups.map((g) => g.label));
		checkRead('chapter 2', c2, got2, exp2, { toasts, mapOpened });
	}
	await open(`${viewer}/catpope`);
	await openMap();
	const s = await waitFor(`(() => { const s = ${MAP}; return s.open && s.places.length ? s : null; })()`, 10000);
	const all = [...new Set([...Object.values(exp1).flat(), ...Object.values(exp2).flat()])].sort();
	report(`the opened map shows those ${all.length} places`, s && JSON.stringify([...s.places].sort()) === JSON.stringify(all) ? [] : [JSON.stringify(s?.places)]);
	await shot('map-reading-chapters-1-2');
}

// ── member: guest discoveries merge on sign-in ──────────────────────────────────────────────────────────────────────
async function member() {
	console.log('member (test user)');
	await send('Emulation.setDeviceMetricsOverride', { width: 1440, height: 900, deviceScaleFactor: 1, mobile: false });
	saveGame.set({ map: { places: { monastery: { unlocked: 5 } }, routes: {}, positions: {} } });
	await send('Network.clearBrowserCookies');
	await setLocale('de', viewer);
	await open(`${viewer}/catpope`);
	await evaluate(`localStorage.clear(), localStorage.setItem('scutoons_game_scutoons', JSON.stringify({ map: { places: { apartment: { unlocked: 7, seen: 8 } }, routes: {}, positions: {} }, created_at: '${now}', updated_at: '${now}' })), localStorage.setItem('scutoons_session', ${JSON.stringify(token)}), true`);
	await open(`${viewer}/catpope`);
	const merged = await waitFor(`(() => { const w = JSON.parse(localStorage.getItem('scutoons_game_scutoons') || 'null'); return w && w.map.places.monastery && w.map.places.apartment ? w : null; })()`, 10000);
	let server = null;
	for (let i = 0; i < 20; i++) {
		server = await (await fetch(`${api}/progress/scutoons/`, { headers: auth })).json();
		if (server?.state?.map?.places?.apartment) break;
		await sleep(500);
	}
	const places = Object.keys(server?.state?.map?.places ?? {}).sort();
	report('sign-in merges the guest\'s discoveries with the account\'s (both devices keep everything)', merged && places.includes('apartment') && places.includes('monastery') ? [] : [`local ${JSON.stringify(merged?.map?.places)}`, `server ${JSON.stringify(places)}`]);
}

// ── builder ─────────────────────────────────────────────────────────────────────────────────────────────────────────
async function builder() {
	console.log('builder (creator /map)');
	const content = mapEntity.content; // the newest version: what the builder shows
	const unlocks = (await (await fetch(`${api}/creator/flow-index`, { headers: auth })).json())?.placeUnlocks ?? {};
	// 17. every hidden place is unlocked by some unlockLocation event (in any toon's documents – the flow index the builder
	//     reads), and every event names a place of the map (an unknown place fizzles in the reader, §8.5a) – read only.
	//     A hidden place no event unlocks yet is content still to be written (new places, the airport): listed, not a failure
	//     (the builder warns about it, 14)
	{
		const ids = new Set(content.locations.map((p) => p.id));
		const never = content.locations.filter((p) => p.hidden && !unlocks[p.id]?.length).map((p) => p.id);
		const unknown = Object.entries(unlocks).filter(([id]) => !ids.has(id)).map(([id, docs]) => `${id} (${docs.join(', ')})`);
		if (never.length) console.log(`  note  hidden, no unlockLocation event unlocks them yet (readers never see them): ${never.join(', ')}`);
		report(`every unlockLocation event names a place of the map (${Object.keys(unlocks).length} places in events; ${content.locations.filter((p) => p.hidden).length - never.length} of ${content.locations.filter((p) => p.hidden).length} hidden places unlocked by one)`,
			unknown.map((u) => `unknown place in an event: ${u}`));
	}
	await send('Emulation.setDeviceMetricsOverride', { width: 1440, height: 900, deviceScaleFactor: 1, mobile: false });
	await send('Network.clearBrowserCookies');
	await setLocale('de', creator);
	await open(`${creator}/`);
	await evaluate(`localStorage.clear(), localStorage.setItem('scutoons_session', ${JSON.stringify(token)}), true`);
	await open(`${creator}/map`);
	const ready = await waitFor(`document.querySelector('[data-testid="map-canvas"]') && document.querySelectorAll('[data-testid^="builder-place-"]').length ? true : null`, 15000);
	report("the builder opens with the world's places", ready ? [] : ['no canvas / places']);
	if (!ready) return;
	{
		// markers are round: the creator's global button min-height made them tall ellipses (owner screenshot 2026-10-01)
		const boxes = await evaluate(`[...document.querySelectorAll('[data-testid^="builder-place-"]')].map((b) => { const r = b.getBoundingClientRect(); return [b.dataset.testid.slice(14), Math.round(r.width), Math.round(r.height)]; })`);
		const bad = boxes.filter(([, w, h]) => !w || Math.abs(w - h) > 1);
		report(`builder markers are round (width = height, ${boxes.length})`, boxes.length && !bad.length ? [] : bad.map((b) => b.join(' ')));
	}
	await shot('builder-1-open');

	// 14. the places' unlock events (owner 2026-10-11: places are unlocked by events only; the builder reads them from
	//     /creator/flow-index placeUnlocks): the problems table warns "… is hidden and no unlockLocation event unlocks it"
	//     for exactly the hidden places no event unlocks; a place without events toggled "hidden" gets it / loses it, and
	//     back (nothing is saved: the page is reloaded without saving)
	{
		const warning = (p) => ['de', 'en'].map((l) => fill(creatorMessages[l].creator.map_hidden_never_unlocked, { name: p.name?.[l] || p.name?.de || p.name?.en || p.id }));
		const rows = () => waitFor(`(() => { const v = document.querySelector('[data-testid="map-validation"]'); return v ? [...v.querySelectorAll('tbody tr')].map((r) => r.textContent) : null; })()`, 5000);
		const warned = (list, p) => (list ?? []).some((r) => warning(p).some((w) => r.includes(w)));
		await waitFor(`document.querySelector('[data-testid="map-validation"] h3') ? true : null`, 5000);
		await sleep(1500); // the flow index loads after the map
		const now = await rows();
		const want = content.locations.filter((p) => p.hidden && !unlocks[p.id]?.length).map((p) => p.id).sort();
		const got = content.locations.filter((p) => warned(now, p)).map((p) => p.id).sort();
		report(`the problems table warns about exactly the hidden places no event unlocks (${want.length ? want.join(', ') : 'none'})`, now && JSON.stringify(got) === JSON.stringify(want) ? [] : [`warned ${JSON.stringify(got)}`, `rows ${now?.length ?? 'no table'}`]);
		const loose = content.locations.find((p) => !p.hidden && !unlocks[p.id]?.length) ?? content.locations.find((p) => !unlocks[p.id]?.length);
		if (!loose) console.log('  skip  a place without unlock events to toggle "hidden" (every place has one)');
		else {
			const toggle = async (on) => {
				await evaluate(`(document.querySelector('[data-testid="nav-place-${loose.id}"]')?.click(), true)`);
				await waitFor(`document.querySelector('[data-testid="place-hidden"]') ? true : null`, 3000);
				await evaluate(`(() => { const c = document.querySelector('[data-testid="place-hidden"]'); if (c.checked !== ${on}) c.click(); return true; })()`);
				await sleep(200);
				await key('Escape'); // no selection: the problems table shows
				return rows();
			};
			const was = !!loose.hidden;
			const flipped = await toggle(!was);
			await shot('builder-1b-hidden-warning');
			const back = await toggle(was);
			report(`"${loose.id}" (no event unlocks it) toggled ${was ? 'visible and back to hidden' : 'hidden and back'}: the warning follows ("${warning(loose)[0]}")`,
				warned(flipped, loose) === !was && warned(back, loose) === was ? [] : [JSON.stringify({ flipped: warned(flipped, loose), back: warned(back, loose) })]);
		}
	}

	if (mapEntity?.published?.id !== mapEntity?.version?.id) {
		report('the map has no unpublished draft (the check never touches work in progress)', [`newest version ${mapEntity?.version?.id} ≠ published ${mapEntity?.published?.id} – publish or discard it first`]);
		return;
	}
	const entryBefore = await (await fetch(`${api}/entities/toon:catpope?draft=1&wrap=1`, { headers: auth })).json();
	if (entryBefore?.published?.id !== entryBefore?.version?.id) {
		report('the toon entry toon:catpope has no unpublished draft', [`newest ${entryBefore?.version?.id} ≠ published ${entryBefore?.published?.id}`]);
		return;
	}
	const original = mapEntity.content;
	restoreMap = async () => {
		const put = await fetch(`${api}/entities/map`, { method: 'PUT', headers: auth, body: JSON.stringify({ content: original, message: 'check-map: restore' }) });
		if (put.ok) await fetch(`${api}/entities/map/publish`, { method: 'POST', headers: auth, body: '{}' });
		const now = await (await fetch(`${api}/entities/toon:catpope?draft=1&wrap=1`, { headers: auth })).json();
		if (now?.version?.id !== entryBefore.version.id) {
			const back = await fetch(`${api}/entities/toon:catpope`, { method: 'PUT', headers: auth, body: JSON.stringify({ content: entryBefore.content, message: 'check-map: restore' }) });
			if (back.ok) await fetch(`${api}/entities/toon:catpope/publish`, { method: 'POST', headers: auth, body: '{}' });
			entryBefore.version = (await (await fetch(`${api}/entities/toon:catpope?draft=1&wrap=1`, { headers: auth })).json()).version;
		}
	};
	// a fresh page: the ticked / unticked place of 14 is not carried into the saves below
	await open(`${creator}/map`);
	await waitFor(`document.querySelector('[data-testid="map-canvas"]') && document.querySelectorAll('[data-testid^="builder-place-"]').length ? true : null`, 15000);

	// 9. add a place, name it, drag it
	const canvas = await evaluate(`(() => { const b = document.querySelector('[data-testid="map-canvas"]').getBoundingClientRect(); return { x: b.left, y: b.top, w: b.width, h: b.height }; })()`);
	await click('[data-testid="tool-place"]');
	// an empty spot of the terrain: not on a place (its marker / diorama), a route's (wide) hit line or the tool strips –
	// a click on a route line adds the place but the route gets selected (map-canvas.svelte: the click after the tap)
	const at = await evaluate(`(() => {
		const c = document.querySelector('[data-testid="map-canvas"]');
		for (const fy of [0.3, 0.4, 0.5, 0.6, 0.25, 0.7, 0.2]) for (const fx of [0.3, 0.4, 0.5, 0.6, 0.25, 0.7, 0.2]) {
			const x = ${canvas.x} + ${canvas.w} * fx, y = ${canvas.y} + ${canvas.h} * fy;
			const el = document.elementFromPoint(x, y);
			if (el && c.contains(el) && !el.closest('[data-route], [data-handle], button, a, input, select, .strip')) return { x, y };
		}
		return null;
	})()`);
	if (!at) { report('"+ Ort": an empty spot on the canvas to click', ['every probed point is on a place, a route or a control']); return; }
	await clickAt(at.x, at.y);
	const idBefore = await waitFor(`document.querySelector('[data-testid="place-id"]')?.value || null`, 5000);
	const name = `Prüfort ${Date.now() % 100000}`;
	await waitFor(`document.activeElement?.dataset?.testid === 'place-name-de' ? true : null`, 3000);
	await typeText(name);
	const placeId = await waitFor(`(() => { const v = document.querySelector('[data-testid="place-id"]')?.value; return v && v !== ${JSON.stringify(idBefore)} ? v : null; })()`, 5000);
	const slug = name.toLowerCase().replace('ü', 'ue').replace(/[^a-z0-9]+/g, '-');
	report(`"+ Ort" adds a place at the click; the de name sets its id (${slug})`, placeId === slug ? [] : [`id ${idBefore} → ${placeId}`, `selection ${await evaluate('location.search')}`]);
	if (placeId !== slug) return; // the rest needs the new place (the map is restored on the way out)
	const pos0 = await evaluate(`({ x: Number(document.querySelector('[data-testid="place-x"]').value), y: Number(document.querySelector('[data-testid="place-y"]').value) })`);
	// the canvas zoom (no zoom display any more since the phone bar, 2026-10-11): two places' screen distance / world distance
	const zoom = await (async () => {
		const [a, b] = original.locations.filter((p, i, all) => all.findIndex((q) => q.x === p.x && q.y === p.y) === i);
		const [sa, sb] = [await center(`[data-testid="builder-place-${a?.id}"]`), await center(`[data-testid="builder-place-${b?.id}"]`)];
		const world = a && b ? Math.hypot(a.x - b.x, a.y - b.y) : 0;
		return sa && sb && !sa.error && !sb.error && world ? Math.hypot(sa.x - sb.x, sa.y - sb.y) / world : 1;
	})();
	const marker = await center(`[data-testid="builder-place-${placeId}"]`);
	await drag(marker, { x: marker.x + 60, y: marker.y + 40 });
	const pos1 = await waitFor(`(() => { const x = Number(document.querySelector('[data-testid="place-x"]').value); return x !== ${pos0.x} ? { x, y: Number(document.querySelector('[data-testid="place-y"]').value) } : null; })()`, 3000);
	report('a drag moves the place (world units = px / zoom)', pos1 && Math.abs(pos1.x - pos0.x - 60 / zoom) <= 2 && Math.abs(pos1.y - pos0.y - 40 / zoom) <= 2 ? [] : [`${JSON.stringify(pos0)} → ${JSON.stringify(pos1)} at zoom ${zoom}`]);

	// 10. draw a route: the new place → a free point → Enter
	await click('[data-testid="tool-route"]');
	const from = await center(`[data-testid="builder-place-${placeId}"]`);
	await clickAt(from.x, from.y);
	await clickAt(from.x + 90, from.y + 70);
	await key('Enter');
	const routeId = await waitFor(`document.querySelector('[data-testid="route-id"]')?.value || null`, 5000);
	const waypoints = await evaluate(`[...document.querySelectorAll('[data-testid="route-waypoints"] li')].length`);
	report('"Weg zeichnen": place + free point + Enter → a route with 2 waypoints, selected', routeId && waypoints === 2 ? [] : [`route ${routeId}, ${waypoints} waypoints`]);
	await shot('builder-2-route');

	// 11. save draft → stored
	await saveDraft();
	let stored = null;
	for (let i = 0; i < 20; i++) {
		stored = await (await fetch(`${api}/entities/map?draft=1&wrap=1`, { headers: auth })).json();
		if (stored?.version?.id !== mapEntity.version.id) break;
		await sleep(400);
	}
	const place = stored?.content?.locations?.find((p) => p.id === placeId);
	const route = stored?.content?.routes?.find((r) => r.id === routeId);
	report('Save draft stores the place (name, moved position) and the route (place + free waypoint)',
		place && place.name?.de === name && place.x === pos1?.x && route && route.waypoints.length === 2 && route.waypoints[0].location === placeId && typeof route.waypoints[1].x === 'number'
			? [] : [JSON.stringify({ place, route })]);

	// 12. a place image (any image of the CDN): it stands on the place's point (ground point = the default anchor 50 / 50 %),
	//     as wide as the place's size × zoom (the size slider, default 4 world units since the hex footprint), + previews
	await click('[data-testid="tool-select"]');
	await evaluate(`(document.querySelector('[data-testid="nav-place-${placeId}"]')?.click(), true)`);
	// the image is a slot with the asset picker (platform 04eb9e5): search the file, pick it, "Use"
	await waitFor(`document.querySelector('[data-testid="place-image-slot"] button') ? true : null`, 3000);
	await evaluate(`(document.querySelector('[data-testid="place-image-slot"] button').click(), true)`);
	await waitFor(`document.querySelector('[data-testid="asset-search"]') ? true : null`, 8000);
	await evaluate(`(() => { const i = document.querySelector('[data-testid="asset-search"]'); i.value = 'catpope_avatar-neutral'; i.dispatchEvent(new Event('input', { bubbles: true })); return true; })()`);
	await waitFor(`document.querySelector('[data-testid="asset-card"]') ? true : null`, 8000);
	await sleep(800);
	await evaluate(`(document.querySelector('[data-testid="asset-card"]').click(), true)`);
	await waitFor(`document.querySelector('[data-testid="asset-use"]') ? true : null`, 3000);
	await evaluate(`(document.querySelector('[data-testid="asset-use"]').click(), true)`);
	const img = await waitFor(`(() => {
		const marker = document.querySelector('[data-testid="builder-place-${placeId}"]'); const im = marker?.parentElement.querySelector('.place-image');
		if (!im || !im.complete || !im.naturalWidth) return null;
		const m = marker.getBoundingClientRect(), b = im.getBoundingClientRect();
		return { w: Math.round(b.width), groundX: Math.round(b.left + b.width * 0.5 - (m.left + m.width / 2)), groundY: Math.round(b.top + b.height * 0.5 - (m.top + m.height / 2)), size: Number(document.querySelector('[data-testid="place-size"]')?.value || 4), previews: document.querySelectorAll('.previews .preview img').length, anchor: !!document.querySelector('[data-testid="place-anchor"] img') };
	})()`, 8000);
	report(`a place image stands on the point (size ${img?.size} × zoom ${Math.round(zoom * 100) / 100}, ground point centre), with the ground point editor and 3 previews`,
		img && Math.abs(img.w - img.size * zoom) <= 2 && Math.abs(img.groundX) <= 2 && Math.abs(img.groundY) <= 2 && img.previews === 3 && img.anchor ? [] : [JSON.stringify(img)]);
	await evaluate(`(() => { const side = document.querySelector('.side'); const f = document.querySelector('[data-testid="place-image"]'); side.scrollTop = f.offsetTop - 20; return true; })()`);
	await shot('builder-3-image');

	// 13. reader view: the sea is the filtered toon's colour (tile colour, else its star's registry colour), else neutral
	const registry = await (await fetch(`${api}/library/characters`)).json();
	const list = await (await fetch(`${api}/library/toons?draft=1`, { headers: auth })).json();
	const colorOf = (t) => list.find((x) => x.id === t)?.tile?.color || registry.characters.find((c) => c.own_toon === t)?.color || null;
	const seas = {};
	await click('[data-testid="reader-view"]');
	for (const t of ['catpope', 'toptoy', 'yellowmouse']) {
		await pickToon(t);
		await sleep(150);
		seas[t] = await evaluate(`getComputedStyle(document.querySelector('[data-testid="map-canvas"]')).backgroundColor`);
		await shot(`builder-4-sea-${t}`);
	}
	await pickToon('all');
	const neutral = await evaluate(`getComputedStyle(document.querySelector('[data-testid="map-canvas"]')).backgroundColor`);
	const wrong = Object.entries(seas).filter(([t, bg]) => colorOf(t) && bg !== (colorOf(t).startsWith('#') ? rgb(colorOf(t)) : bg)).map(([t, bg]) => `${t}: ${bg} ≠ ${colorOf(t)}`);
	report(`reader view: the sea is the filtered toon's colour (${Object.keys(seas).map((t) => `${t} ${colorOf(t)}`).join(', ')}), neon green #00ff00 without a filter`, !wrong.length && neutral === rgb('#00ff00') ? [] : [...wrong, `neutral ${neutral}`]);

	// 16. Wanted images → Map places: every place without an image, derived from the map (owner 2026-10-01)
	await restoreMap();
	const current = (await (await fetch(`${api}/entities/map?draft=1&wrap=1`, { headers: auth })).json()).content;
	const lacking = current.locations.filter((p) => !p.image?.src).map((p) => p.id);
	await open(`${creator}/characters?tab=wanted`);
	const wantedPlaces = await waitFor(`(() => { const l = [...document.querySelectorAll('[data-testid="wanted-media"] li[data-testid^="wanted-map-"]')].map((li) => li.dataset.testid.slice(11)); return l.length ? l : null; })()`, 10000);
	const paths = await evaluate(`[...document.querySelectorAll('[data-testid="wanted-media"] li[data-testid^="wanted-map-"] code')].map((c) => c.textContent)`);
	const briefs = await evaluate(`document.querySelectorAll('[data-testid="wanted-media"] li[data-testid^="wanted-map-"] details pre').length`);
	report(`Wanted images (toon media) lists every map place without an image (${lacking.length} of ${current.locations.length}, the airport too) with its path <toon>/map/places/<id>.webp and the brief`,
		briefs === lacking.length && lacking.includes('airport') && JSON.stringify(wantedPlaces) === JSON.stringify(lacking) && paths.every((p, i) => p === `catpope/map/places/${lacking[i]}.webp`) ? [] : [JSON.stringify({ shown: wantedPlaces?.length, want: lacking.length, first: paths?.[0] })]);
	await shot('builder-8-wanted-map');
	await open(`${creator}/map`);
	await waitFor(`document.querySelector('[data-testid="map-canvas"]') ? true : null`, 10000);

	// 15. the toon's start view: set in the builder from the current view (toon filter), stored in the toon entry
	await pickToon('catpope');
	await waitFor(`document.querySelector('[data-testid="start-view-set"]:not([disabled])') ? true : null`, 8000);
	const canvasBox = await evaluate(`(() => { const b = document.querySelector('[data-testid="map-canvas"]').getBoundingClientRect(); return { x: b.left + b.width / 2, y: b.top + b.height / 2 }; })()`);
	await send('Input.dispatchMouseEvent', { type: 'mouseWheel', x: canvasBox.x, y: canvasBox.y, deltaX: 0, deltaY: -300 });
	await sleep(200);
	await click('[data-testid="start-view-set"]');
	const shown = await waitFor(`document.querySelector('[data-testid="phone-frame-box"]') && document.querySelector('[data-testid="start-view"] .hint')?.textContent.includes('%') ? document.querySelector('[data-testid="start-view"] .hint').textContent : null`, 3000);
	await shot('builder-7-start-view');
	await saveDraft();
	let entry = null;
	for (let i = 0; i < 20; i++) {
		entry = await (await fetch(`${api}/entities/toon:catpope?draft=1&wrap=1`, { headers: auth })).json();
		if (entry?.version?.id !== entryBefore.version.id) break;
		await sleep(400);
	}
	const sv = entry?.content?.map;
	report('"Set start view" stores the toon entry map { center, zoom } from the builder view (shown + phone frame)', shown && sv?.center && Number.isInteger(sv.center.x) && sv.zoom > 1 && entry.version.id !== entryBefore.version.id ? [] : [JSON.stringify({ shown, map: sv })]);

}

try {
	if (!only || only === 'reader') {
		await reader(390, 'de');
		await reader(1440, 'en');
	}
	if (!only || only === 'reading') await reading();
	if (!only || only === 'member') await member();
	if (!only || only === 'builder') await builder();
	const errors = consoleErrors.filter((e) => e && !/favicon|ResizeObserver loop/.test(e));
	report('no console errors', errors.length ? errors.slice(0, 5) : []);
} catch (e) {
	console.error(e);
	failed++;
}
console.log(`\n${passed} passed, ${failed} failed`);
await done(failed ? 1 : 0);
