// Wardrobe check (agents/ITEMS.md "Wardrobe", owner 2026-10-01): headless Chrome via CDP (own temp profile, no
// dependencies; Node 22 WebSocket), like scripts/check-next-bar.mjs.
//
// Per width (390, 1440), as a guest (fresh device) and as the test user (`npm run dev:test-user -- --reset` first):
//   1. the default = Cat Pope's default body (the A-pose of its regular default look) in the basic dress: the T-shirt,
//      shorts and sandals in their slot tiles – nothing drawn on the body (owner 2026-10-12: "additional items dont get
//      overlayed and we just put them in the slots"); the swim pants wait in the inventory;
//   2. "Anziehen" on the swim pants (legs) → in the legs tile, the shorts back in the inventory (note + the Items tab
//      lists the shorts; worn items are not listed there – owner 2026-10-01: they live in the Avatar tab only);
//   3. "Ausziehen" on the swim pants → legs empty, the base stays; the item detail: shorts on → note, shorts off → legs
//      empty again;
//   member only:
//   4. "Hide sensitive content" off and on again → the avatar is not touched (legs stay empty);
//   5. Top Toy (unlocked for the check through the admin API, revoked at the end), the swim pants on first: the twins
//      wear them in the left figure's legs tile, the right figure's legs stay empty, the shirt in torso-left;
//   6. back to Cat Pope: one legs slot, the swim pants.
// The member's avatar is put back to the default at the end (body null, the default wearables on).
//
// usage: node scripts/check-wardrobe.mjs [--viewer=http://localhost:8000] [--api=http://localhost:9000/api]
//        [--shots=<dir>] [--only=390] [--verbose]
// Needs the viewer, the API, the CDN (the A-poses, the item icons) and the test user.
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
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const de = JSON.parse(readFileSync(join(root, 'viewer/messages/de.json'), 'utf8'));
const fill = (s, v) => s.replace(/\{(\w+)\}/g, (_, k) => String(v[k]));
const catalog = await (await fetch(`${api}/items/`)).json(); // the local catalog (production's, npm run dev:db)
const nameOf = (id) => catalog.items.find((i) => i.id === id).name.de;

// ── the test user ───────────────────────────────────────────────────────────────────────────────────────────────────
const sessionFile = join(root, 'logs/dev/test-user.json');
const token = existsSync(sessionFile) ? JSON.parse(readFileSync(sessionFile, 'utf8')).token : null;
if (!token || !(await fetch(`${api}/me/`, { headers: { Authorization: `Bearer ${token}` } })).ok) { console.error('no working test-user session (run `npm run dev:test-user -- --reset`)'); process.exit(2); }
const auth = { Authorization: `Bearer ${token}`, 'Content-Type': 'application/json' };
const call = async (method, path, body) => { const r = await fetch(api + path, { method, headers: auth, body: body === undefined ? undefined : JSON.stringify(body) }); return { status: r.status, json: await r.json().catch(() => null) }; };
const me = (await call('GET', '/me/')).json;
const userId = me.user.id;
const hadToptoy = (me.avatar?.unlocked ?? []).some((u) => u.character === 'toptoy');

// ── Chrome ──────────────────────────────────────────────────────────────────────────────────────────────────────────
const CHROME = process.env.CHROME || ['C:/Program Files/Google/Chrome/Application/chrome.exe', 'C:/Program Files (x86)/Google/Chrome/Application/chrome.exe', 'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe', '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome', '/usr/bin/google-chrome'].find((p) => existsSync(p));
if (!CHROME) { console.error('Chrome not found (set CHROME)'); process.exit(2); }
const port = 9400 + Math.floor(Math.random() * 400);
const profile = mkdtempSync(join(tmpdir(), 'check-wardrobe-'));
const chrome = spawn(CHROME, ['--headless=new', `--remote-debugging-port=${port}`, `--user-data-dir=${profile}`, '--no-first-run', '--no-default-browser-check', '--disable-extensions', '--window-size=1440,900', 'about:blank'], { stdio: 'ignore' });
async function cleanup() {
	await call('PUT', '/me/', { hide_sensitive: true }).catch(() => {});
	if (!hadToptoy) await call('DELETE', `/admin/users/${userId}/characters/toptoy`).catch(() => {});
	await call('PUT', '/avatar/', { body: null }).catch(() => {});
	// the basic dress on again
	const m = (await call('GET', '/me/')).json;
	const id = (item) => m?.inventory?.find((e) => e.item_id === item)?.id ?? null;
	if (id('scutoons_tshirt')) await call('PUT', '/avatar/', { slots: { torso: id('scutoons_tshirt'), legs: id('scutoons_shorts'), feet: id('scutoons_sandals') } }).catch(() => {});
}
async function done(code) {
	await cleanup();
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
const send = (method, params = {}) => new Promise((r) => { const i = ++seq; pending.set(i, r); ws.send(JSON.stringify({ id: i, method, params })); });
ws.addEventListener('message', (e) => {
	const m = JSON.parse(e.data);
	if (m.id && pending.has(m.id)) { pending.get(m.id)(m); pending.delete(m.id); return; }
	if (m.method?.startsWith('Network.')) lastNet = Date.now();
});
const evaluate = async (expression) => { const r = await send('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true }); return r.result?.exceptionDetails ? { error: r.result.exceptionDetails.exception?.description ?? r.result.exceptionDetails.text } : r.result?.result?.value; };
await send('Page.enable'); await send('Runtime.enable'); await send('Network.enable');
async function idle(quietMs = 700, maxMs = 15000) { const t0 = Date.now(); while (Date.now() - t0 < maxMs) { if (Date.now() - lastNet > quietMs) return; await sleep(100); } }
async function waitFor(js, ms = 10000) { const t0 = Date.now(); let r; while (Date.now() - t0 < ms) { r = await evaluate(js); if (r && !r.error) return r; await sleep(150); } return null; }
async function open(url) { lastNet = Date.now(); await send('Page.navigate', { url }); await sleep(300); await idle(); }
async function shot(name) {
	if (!shots) return;
	const r = await send('Page.captureScreenshot', { format: 'png' });
	writeFileSync(join(shots, `${name}.png`), Buffer.from(r.result.data, 'base64'));
}
/** a real click in the middle of the element (the first match of `js`, an expression returning an element) */
async function clickEl(js) {
	const box = await evaluate(`(() => { const el = ${js}; if (!el) return null; el.scrollIntoView({ block: 'center' }); const b = el.getBoundingClientRect(); return { x: b.left + b.width / 2, y: b.top + b.height / 2 }; })()`);
	if (!box || box.error) return false;
	for (const type of ['mousePressed', 'mouseReleased']) await send('Input.dispatchMouseEvent', { type, x: box.x, y: box.y, button: 'left', clickCount: 1 });
	await sleep(250);
	await idle(400);
	return true;
}
const q = (s) => `document.querySelector(${JSON.stringify(s)})`;
/** the panel toggles in the header (since the avatar got its own panel): 0 = Items, 1 = Avatar */
const tab = (i) => (i ? `document.querySelector('[data-testid="avatar-toggle"]')` : `document.querySelector('.inventory-toggle')`);
/** the button of an item row in the avatar picker */
const pickerButton = (name) => `[...document.querySelectorAll('.picker li')].find((li) => li.querySelector('.name')?.textContent.includes(${JSON.stringify(name)}))?.querySelector('button')`;
/** the avatar: the base (the body's A-pose), what is drawn on it (must be nothing), the slot tiles (instance → item) */
const FIGURE = `(() => {
	const f = document.querySelector('.figure');
	if (!f) return null;
	const tiles = {};
	for (const t of document.querySelectorAll('.avatar [data-slot]')) tiles[t.dataset.slot] = t.dataset.item ?? null;
	const img = f.querySelector('[data-layer="base"]');
	const base = img?.getAttribute('src') ?? null;
	return { base, loaded: !!img && img.complete && img.naturalWidth > 0, layers: f.querySelectorAll('img').length, drawn: f.querySelectorAll('[data-overlay], [data-worn], .item-icon').length, tiles,
		note: document.querySelector('[data-testid="avatar-note"]')?.textContent.trim() ?? null,
		bodies: [...document.querySelectorAll('[data-body]')].map((b) => b.dataset.body) };
})()`;
const until = (pred, ms = 8000) => waitFor(`(() => { const s = ${FIGURE}; return s && (${pred}) ? s : null; })()`, ms);

let failed = 0, passed = 0;
function report(name, problems, detail = '') {
	if (problems.length) { failed++; console.log(`  FAIL  ${name}`); for (const p of problems) console.log(`        ${p}`); }
	else { passed++; console.log(`  ok    ${name}${opt.verbose && detail ? `  ${detail}` : ''}`); }
}

/** opens a panel (0 = Items, 1 = Avatar) through its header toggle and waits until it has slid in (PullPanel, 0.3 s) */
async function openPanel(i) {
	const panel = i ? '[data-testid="avatar-panel"]' : '[data-testid="inventory"]';
	const ready = `(() => { const p = document.querySelector('${panel}'); return !!p && p.classList.contains('visible') && /^(none|matrix\\(1, 0, 0, 1, 0, 0\\))$/.test(getComputedStyle(p).transform); })()`;
	for (let n = 0; n < 3; n++) {
		if (await evaluate(ready)) return true;
		if (!(await evaluate(`document.querySelector('${panel}')?.classList.contains('visible')`))) await evaluate(`(${tab(i)}?.click(), true)`);
		if (await waitFor(ready, 3000)) return true;
	}
	return false;
}
async function openAvatar() {
	await open(`${viewer}/de/catpope`);
	await waitFor(`${tab(1)}`);
	if (!(await openPanel(1))) console.log('        (the avatar panel did not open)');
	return until(`s.tiles.legs || s.tiles['legs-left'] || s.tiles.torso`, 12000);
}

/** steps 1–3 (both guest and member) */
async function basics(who, w) {
	const p1 = [];
	const s1 = await openAvatar();
	if (!s1) p1.push('no avatar figure / no worn tiles');
	else {
		if (s1.tiles.legs !== 'scutoons_shorts' || s1.tiles.torso !== 'scutoons_tshirt' || s1.tiles.feet !== 'scutoons_sandals') p1.push(`tiles ${JSON.stringify(s1.tiles)}`);
		if (!s1.base || !s1.loaded || s1.layers !== 1) p1.push(`base ${s1.base} (loaded ${s1.loaded}, ${s1.layers} images): one A-pose expected`);
		if (s1.drawn) p1.push(`${s1.drawn} item(s) drawn on the body – items belong in the slot tiles only`);
		// the body picker shows only with more than one body (the member has Top Toy too)
		if (s1.bodies.length && !s1.bodies.includes('catpope/regular/default')) p1.push(`bodies ${s1.bodies}: Cat Pope's default body expected`);
		if (!/\/catpope\//.test(s1.base ?? '')) p1.push(`base ${s1.base}: Cat Pope expected`);
	}
	await shot(`${who}-${w}-1-default`);
	report(`${who} ${w}: default = Cat Pope's default body, the basic dress (T-shirt, shorts, sandals) in the slot tiles`, p1, JSON.stringify(s1));

	const p2 = [];
	await clickEl(q('[data-slot="legs"]'));
	await clickEl(pickerButton(nameOf('scutoons_swim_pants')));
	const s2 = await until(`s.tiles.legs === 'scutoons_swim_pants'`);
	if (!s2) p2.push('the swim pants did not appear in the legs tile');
	else if (s2.drawn) p2.push('an item is drawn on the body');
	else if (s2.note !== fill(de.items.returned, { name: nameOf('scutoons_shorts') })) p2.push(`note "${s2.note}"`);
	await shot(`${who}-${w}-2-swim-pants`);
	const shortsRow = await evaluate(`[...document.querySelectorAll('.picker li')].map((li) => li.textContent.replace(/\\s+/g, ' ').trim())`);
	if (!shortsRow?.some?.((t) => t.includes(nameOf('scutoons_shorts')) && t.includes(de.items.put_on))) p2.push(`picker rows ${JSON.stringify(shortsRow)}`);
	// the Items tab: the shorts are listed again; the worn swim pants, T-shirt and sandals are not (one plain list)
	await openPanel(0);
	const lists = await waitFor(`(() => { const s = document.querySelector('[data-testid="inventory-items"]'); return s ? { items: [...s.querySelectorAll('.item')].map((b) => b.dataset.item), sections: document.querySelectorAll('[data-testid="inventory"] [data-section], [data-testid="inventory"] .section-title').length } : null; })()`);
	const worn = ['scutoons_swim_pants', 'scutoons_tshirt', 'scutoons_sandals'];
	if (!lists?.items?.includes('scutoons_shorts') || worn.some((id) => lists?.items?.includes(id)) || lists?.sections) p2.push(`items tab ${JSON.stringify(lists)}`);
	await shot(`${who}-${w}-2-items`);
	report(`${who} ${w}: swim pants on → the shorts back in the inventory (Items lists them, not the worn items)`, p2, JSON.stringify(lists));

	const p3 = [];
	await openPanel(1);
	await until('true');
	// the legs tile may still be selected from step 2 (the panel stays mounted): a second tap would close the picker
	if (!(await evaluate(`!!document.querySelector('[data-slot="legs"].active')`))) await clickEl(q('[data-slot="legs"]'));
	await clickEl(pickerButton(nameOf('scutoons_swim_pants')));
	const s3 = await until(`!s.tiles.legs`);
	if (!s3) p3.push(`after "Ausziehen" on the swim pants: ${JSON.stringify(await evaluate(FIGURE))}`);
	else if (!s3.base) p3.push('no base');
	await shot(`${who}-${w}-3-legs-empty`);
	report(`${who} ${w}: swim pants off → legs empty, the A-pose stays the base`, p3);

	// the item detail ("Anziehen" / "Ausziehen") swaps the same way
	const p3b = [];
	await openPanel(0);
	await clickEl(`[...document.querySelectorAll('[data-testid="inventory-items"] .item')].find((b) => b.querySelector('.name')?.textContent.trim() === ${JSON.stringify(nameOf('scutoons_shorts'))})`);
	const detailBtn = `[...document.querySelectorAll('.item-detail .actions button')].find((b) => b.textContent.trim() === %s)`;
	await clickEl(detailBtn.replace('%s', JSON.stringify(de.items.put_on)));
	const n1 = await waitFor(`document.querySelector('[data-testid="item-note"]')?.textContent.trim()`);
	if (!n1) p3b.push('put on: no note');
	if (!(await waitFor(`document.querySelector('[data-testid="item-worn"]')`))) p3b.push('the detail does not say "angezogen"');
	await shot(`${who}-${w}-3b-detail`);
	await clickEl(detailBtn.replace('%s', JSON.stringify(de.items.take_off)));
	if (!(await waitFor(`!document.querySelector('[data-testid="item-worn"]')`))) p3b.push('take off: the detail still says "angezogen"');
	report(`${who} ${w}: item detail – Anziehen puts the shorts on, Ausziehen takes them off`, p3b);
	await clickEl(`document.querySelector('.item-detail .btn-back')`);
}

const widths = [390, 1440].filter((w) => !opt.only || String(opt.only) === String(w));
// warm-up: the first page load of a fresh dev server may reload once (dependency optimisation)
await open(`${viewer}/de/catpope`);
await sleep(1500);
await idle();
for (const w of widths) {
	await send('Emulation.setDeviceMetricsOverride', { width: w, height: w < 768 ? 844 : 900, deviceScaleFactor: 1, mobile: w < 768 });
	// guest: a fresh device – also without the previous width's member session: the viewer moves the token into the
	// API's single sign-on cookie, so the "guest" would otherwise still be the member (and dress the member's avatar)
	await send('Network.clearBrowserCookies');
	await open(`${viewer}/de`);
	await evaluate(`(localStorage.clear(), true)`);
	await basics('guest', w);
	const guestState = await evaluate(`JSON.parse(localStorage.getItem('scutoons_inventory') ?? 'null')?.avatar`);
	report(`guest ${w}: stored on the device`, guestState?.slots ? [] : ['no scutoons_inventory avatar'], JSON.stringify(guestState));

	// member
	await open(`${viewer}/de`);
	await evaluate(`(localStorage.clear(), localStorage.setItem('scutoons_session', ${JSON.stringify(token)}), true)`);
	await basics('member', w);

	const p4 = [];
	const off = await call('PUT', '/me/', { hide_sensitive: false });
	const on = await call('PUT', '/me/', { hide_sensitive: true });
	if (off.status !== 200 || on.status !== 200 || on.json?.user?.hide_sensitive !== true) p4.push(`PUT /me/ ${off.status} / ${on.status}`);
	if (on.json?.avatar?.slots?.legs) p4.push(`setting on again put something on: legs ${JSON.stringify(on.json?.avatar?.slots)}`);
	const s4 = await openAvatar().then(() => until(`!s.tiles.legs`));
	if (!s4?.base) p4.push(`after the setting: ${JSON.stringify(s4)}`);
	await shot(`member-${w}-4-setting`);
	report(`member ${w}: "Hide sensitive content" off / on → the avatar is not touched`, p4);

	const p5 = [];
	if (!hadToptoy) await call('PUT', `/admin/users/${userId}/characters/toptoy`, { source: 'admin', ref: 'check-wardrobe' });
	const pants = (await call('GET', '/me/')).json?.inventory?.find((e) => e.item_id === 'scutoons_swim_pants')?.id;
	await call('PUT', '/avatar/', { slots: { legs: pants } });
	await openAvatar();
	await clickEl(q('[data-body^="toptoy/"]'));
	const s5 = await until(`s.tiles['legs-left']`);
	if (!s5) p5.push(`twins: ${JSON.stringify((await evaluate(FIGURE)))}`);
	else {
		// the worn items move to the first instance of their type (the left figure); the right legs stay empty
		if (s5.tiles['legs-left'] !== 'scutoons_swim_pants') p5.push(`legs ${JSON.stringify(s5.tiles)}`);
		if (s5.tiles['legs-right']) p5.push(`legs-right is not empty: ${JSON.stringify(s5.tiles)}`);
		if (s5.tiles['torso-left'] !== 'scutoons_tshirt') p5.push(`the T-shirt in torso-left expected: ${JSON.stringify(s5.tiles)}`);
		if (!s5.base || s5.drawn) p5.push(`base ${s5.base}, ${s5.drawn} drawn on the body`);
	}
	await shot(`member-${w}-5-toptoy`);
	report(`member ${w}: Top Toy twins – pants in the left legs tile, right legs empty, shirt in torso-left`, p5, JSON.stringify(s5));

	const p6 = [];
	await clickEl(q('[data-body^="catpope/"]'));
	const s6 = await until(`s.tiles.legs && !('legs-left' in s.tiles)`);
	if (s6?.tiles.legs !== 'scutoons_swim_pants') p6.push(`catpope again: ${JSON.stringify(s6)}`);
	await shot(`member-${w}-6-back`);
	report(`member ${w}: back to Cat Pope – one legs slot, the swim pants`, p6);
	await cleanup();
}

console.log(`\n${passed} passed, ${failed} failed`);
await done(failed ? 1 : 0);
