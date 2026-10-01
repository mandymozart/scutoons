// Substances check (agents/ITEMS.md "Substances", owner 2026-10-01): headless Chrome via CDP (own temp profile, no
// dependencies; Node 22 WebSocket), like scripts/check-wardrobe.mjs. At 390 px, as a guest (fresh device) and as the
// test user (`npm run dev:test-user -- --reset` first):
//   1. pick up the testo in the sandbox (chapter 2, slice 6: a click on the ledge → "Einsammeln");
//   2. the testo bar is in the Avatar tab (owner 2026-10-01: "the testo is part of the avatar"), appetite in the Items
//      tab; it shows "Testo 50"; "Benutzen" on the testo → 70, the tube is used up;
//   2b. "Essen" on a cucumber → appetite −5, the cucumber is eaten;
//   3. the decay: the stored time moved back three days (guests: localStorage; the member: the local SQLite DB) → 60;
//   4. testo + camera combined in the inventory → the testo camera; its scan grant (members: the API's scan grant with the
//      tool, then the cooldown – the camera's on-device detection itself is not driven here);
//   5. a magic mushroom (psychedelic): the effect bar + the slices' hue/wobble animation, off with reduced motion,
//      "Effekt beenden" ends it.
// The member's inventory, stats and effects are put back with `npm run dev:test-user -- --reset` at the end.
//
// usage: node scripts/check-testo.mjs [--viewer=http://localhost:8000] [--api=http://localhost:9000/api]
//        [--db=server/db/local.sqlite] [--shots=<dir>] [--verbose]
import { execFileSync, spawn } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const opt = Object.fromEntries(process.argv.slice(2).filter((a) => a.startsWith('--')).map((a) => { const [k, ...v] = a.slice(2).split('='); return [k, v.length ? v.join('=') : true]; }));
const viewer = String(opt.viewer || 'http://localhost:8000').replace(/\/$/, '');
const api = String(opt.api || 'http://localhost:9000/api').replace(/\/$/, '');
const db = resolve(root, String(opt.db || 'server/db/local.sqlite'));
const shots = opt.shots ? String(opt.shots) : null;
if (shots) mkdirSync(shots, { recursive: true });
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const de = JSON.parse(readFileSync(join(root, 'viewer/messages/de.json'), 'utf8'));
const catalog = JSON.parse(readFileSync(join(root, 'server/config/items.json'), 'utf8'));
const nameOf = (id) => catalog.items.find((i) => i.id === id).name.de;
const isLocal = (u) => /^https?:\/\/(localhost|127\.0\.0\.1|\[::1\])(:\d+)?(\/|$)/.test(u);
if (!isLocal(api) || !isLocal(viewer)) { console.error('local servers only'); process.exit(2); }

const sessionFile = join(root, 'logs/dev/test-user.json');
const token = existsSync(sessionFile) ? JSON.parse(readFileSync(sessionFile, 'utf8')).token : null;
if (!token || !(await fetch(`${api}/me/`, { headers: { Authorization: `Bearer ${token}` } })).ok) { console.error('no working test-user session (run `npm run dev:test-user -- --reset`)'); process.exit(2); }
const auth = { Authorization: `Bearer ${token}`, 'Content-Type': 'application/json' };
const call = async (method, path, body) => { const r = await fetch(api + path, { method, headers: auth, body: body === undefined ? undefined : JSON.stringify(body) }); return { status: r.status, json: await r.json().catch(() => null) }; };
const me = (await call('GET', '/me/')).json;
/** raw SQL on the local dev DB (the decay test moves the stored time back) */
function sql(statement, params) {
	const php = `$d=new PDO('sqlite:'.getenv('CT_DB'));$s=$d->prepare(getenv('CT_SQL'));$s->execute(json_decode(getenv('CT_PARAMS'),true));echo $s->rowCount();`;
	return execFileSync(process.env.PHP_BINARY || 'php', ['-r', php], { env: { ...process.env, CT_DB: db, CT_SQL: statement, CT_PARAMS: JSON.stringify(params) }, encoding: 'utf8' });
}

// ── Chrome ──────────────────────────────────────────────────────────────────────────────────────────────────────────
const CHROME = process.env.CHROME || ['C:/Program Files/Google/Chrome/Application/chrome.exe', 'C:/Program Files (x86)/Google/Chrome/Application/chrome.exe', 'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe', '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome', '/usr/bin/google-chrome'].find((p) => existsSync(p));
if (!CHROME) { console.error('Chrome not found (set CHROME)'); process.exit(2); }
const port = 9400 + Math.floor(Math.random() * 400);
const profile = mkdtempSync(join(tmpdir(), 'check-testo-'));
const chrome = spawn(CHROME, ['--headless=new', `--remote-debugging-port=${port}`, `--user-data-dir=${profile}`, '--no-first-run', '--no-default-browser-check', '--disable-extensions', '--window-size=390,844', 'about:blank'], { stdio: 'ignore' });
async function done(code) {
	try { execFileSync(process.execPath, [join(root, 'scripts/dev/test-user.mjs'), '--reset'], { env: { ...process.env, SCUTOONS_API: api }, stdio: 'ignore' }); } catch { console.log('  (could not reset the test user)'); }
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
await send('Emulation.setDeviceMetricsOverride', { width: 390, height: 844, deviceScaleFactor: 1, mobile: true });
async function idle(quietMs = 700, maxMs = 15000) { const t0 = Date.now(); while (Date.now() - t0 < maxMs) { if (Date.now() - lastNet > quietMs) return; await sleep(100); } }
async function waitFor(js, ms = 10000) { const t0 = Date.now(); let r; while (Date.now() - t0 < ms) { r = await evaluate(js); if (r && !r.error) return r; await sleep(150); } return null; }
async function open(url) { lastNet = Date.now(); await send('Page.navigate', { url }); await sleep(300); await idle(); }
async function shot(name) {
	if (!shots) return;
	const r = await send('Page.captureScreenshot', { format: 'png' });
	writeFileSync(join(shots, `${name}.png`), Buffer.from(r.result.data, 'base64'));
}
async function clickAt(x, y) {
	for (const type of ['mousePressed', 'mouseReleased']) await send('Input.dispatchMouseEvent', { type, x, y, button: 'left', clickCount: 1 });
	await sleep(300);
	await idle(400);
}
async function clickEl(js) {
	const box = await evaluate(`(() => { const el = ${js}; if (!el) return null; el.scrollIntoView({ block: 'center' }); const b = el.getBoundingClientRect(); return { x: b.left + b.width / 2, y: b.top + b.height / 2 }; })()`);
	if (!box || box.error) return false;
	await clickAt(box.x, box.y);
	return true;
}
const tab = (i) => `document.querySelectorAll('.inventory .tabs button')[${i}]`;
const tile = (name) => `[...document.querySelectorAll('.inventory .item')].find((b) => b.querySelector('.name')?.textContent.trim() === ${JSON.stringify(name)})`;
const testoLevel = `Number(document.querySelector('[data-testid="avatar-testo"] [data-stat="testo"]')?.dataset.level ?? NaN)`;
const appetiteLevel = `Number(document.querySelector('.inventory [data-stat="appetite"]')?.dataset.level ?? NaN)`;
/** the testo level as the Avatar tab shows it (waits for `want` when given) */
async function testoNow(want) {
	await clickEl(tab(1));
	await waitFor(`document.querySelector('[data-testid="avatar-testo"] [data-stat="testo"]')`);
	if (want !== undefined) await waitFor(`(() => { const l = ${testoLevel}; return l === ${want} ? l : null; })()`);
	return evaluate(testoLevel);
}
async function openInventory() {
	await open(`${viewer}/de/sandbox?inventory=open`);
	await waitFor(tab(0));
	await clickEl(tab(0));
	await waitFor(`document.querySelector('[data-testid="reader-stats"]')`);
}

let failed = 0, passed = 0;
function report(name, problems, detail = '') {
	if (problems.length) { failed++; console.log(`  FAIL  ${name}`); for (const p of problems) console.log(`        ${p}`); }
	else { passed++; console.log(`  ok    ${name}${opt.verbose && detail ? `  ${detail}` : ''}`); }
}

/** 1: the pickup in the sandbox (chapter 2, slice 6, the ledge) */
async function pickup(who) {
	const p = [];
	await open(`${viewer}/de/sandbox/chapter/2`);
	const img = await waitFor(`(() => { const i = [...document.querySelectorAll('[data-slice-index] img')].find((x) => /s1c2-slice-6\\./.test(x.src)); if (!i) return null; i.scrollIntoView({ block: 'center' }); return true; })()`, 20000);
	if (!img) p.push('slice 6 not found');
	await sleep(1200);
	await idle();
	const box = await evaluate(`(() => { const i = [...document.querySelectorAll('[data-slice-index] img')].find((x) => /s1c2-slice-6\\./.test(x.src)); const b = i.getBoundingClientRect(); return { x: b.left + b.width * 0.88, y: b.top + b.height * 0.875 }; })()`);
	// earlier VN messages of the group may wait for clicks: click the ledge until the offer shows (at most 25 times)
	let offer = null;
	for (let i = 0; i < 25 && !offer; i++) {
		await clickAt(box.x, box.y);
		offer = await evaluate(`[...document.querySelectorAll('.vn-choices.visible button')].find((b) => b.textContent.trim() === ${JSON.stringify('Einsammeln')}) ? true : null`);
	}
	if (!offer) p.push('no "Einsammeln" offer after clicking the ledge');
	else await clickEl(`[...document.querySelectorAll('.vn-choices.visible button')].find((b) => b.textContent.trim() === 'Einsammeln')`);
	await sleep(800);
	await shot(`testo-${who}-1-pickup`);
	return p;
}

const owned = async (who, item) => who === 'guest'
	? await evaluate(`(JSON.parse(localStorage.getItem('scutoons_inventory') ?? '{}').entries ?? []).filter((e) => e.item_id === ${JSON.stringify(item)}).reduce((n, e) => n + e.qty, 0)`)
	: ((await call('GET', '/me/')).json?.inventory ?? []).filter((e) => e.item_id === item).reduce((n, e) => n + e.qty, 0);

async function flow(who) {
	// 1
	const p1 = await pickup(who);
	const n1 = await owned(who, 'testo');
	if (n1 !== 1) p1.push(`testo owned: ${n1}`);
	report(`${who}: picked up the testo in the sandbox (chapter 2, slice 6)`, p1);

	// 2
	const p2 = [];
	await openInventory();
	const itemsTab = await evaluate(`[...document.querySelectorAll('.inventory [data-stat]')].map((s) => s.dataset.stat)`);
	if (JSON.stringify(itemsTab) !== '["appetite"]') p2.push(`Items tab stat bars ${JSON.stringify(itemsTab)} (only appetite expected)`);
	const before = await testoNow();
	if (before !== 50) p2.push(`start level ${before} (50 expected)`);
	const label = await evaluate(`document.querySelector('[data-testid="avatar-testo"] [data-stat="testo"] .label')?.textContent.trim()`);
	if (label !== 'Testo 50') p2.push(`bar label "${label}"`);
	const avatarTab = await evaluate(`[...document.querySelectorAll('.inventory [data-stat]')].map((s) => s.dataset.stat)`);
	if (JSON.stringify(avatarTab) !== '["testo"]') p2.push(`Avatar tab stat bars ${JSON.stringify(avatarTab)} (only testo expected)`);
	await shot(`testo-${who}-2-bar`);
	await clickEl(tab(0));
	await clickEl(tile(nameOf('testo')));
	const useText = await evaluate(`document.querySelector('[data-testid="item-use"]')?.textContent.trim()`);
	if (useText !== de.items.use) p2.push(`substance button "${useText}"`);
	await clickEl(`document.querySelector('[data-testid="item-use"]')`);
	const after = await testoNow(70);
	if (after !== 70) p2.push(`after "Benutzen": ${after}`);
	if ((await owned(who, 'testo')) !== 0) p2.push('the tube was not used up');
	await shot(`testo-${who}-2-used`);
	report(`${who}: the testo bar in the Avatar tab, appetite in Items; "Benutzen" raises testo 50 → 70, the tube is used up`, p2);

	// 2b – eat a cucumber (setup: the guest's device / the API's story grant)
	const p2b = [];
	if (who === 'guest') await evaluate(`(() => { const s = JSON.parse(localStorage.getItem('scutoons_inventory')); s.entries.push({ id: 'local-check-cucumber', item_id: 'cucumber', variant: null, qty: 1, source: 'story', code: null, edition_number: null, acquired_at: new Date().toISOString() }); localStorage.setItem('scutoons_inventory', JSON.stringify(s)); return true; })()`);
	else await call('POST', '/inventory/grant/', { item_id: 'cucumber', source: 'story', ref: `check-testo:cucumber:${Date.now()}` });
	await openInventory();
	const hungry = await evaluate(appetiteLevel);
	await clickEl(tile(nameOf('cucumber')));
	const eatText = await evaluate(`document.querySelector('[data-testid="item-use"]')?.textContent.trim()`);
	if (eatText !== de.items.eat) p2b.push(`food button "${eatText}" ("${de.items.eat}" expected)`);
	await clickEl(`document.querySelector('[data-testid="item-use"]')`);
	await clickEl(`document.querySelector('.item-detail .btn-back')`);
	const fed = await waitFor(`(() => { const l = ${appetiteLevel}; return l === ${hungry - 5} ? l : null; })()`);
	if (!Number.isFinite(hungry) || fed !== hungry - 5) p2b.push(`appetite ${hungry} → ${await evaluate(appetiteLevel)} (−5 expected)`);
	if ((await owned(who, 'cucumber')) !== 0) p2b.push('the cucumber was not eaten');
	await shot(`testo-${who}-2b-eaten`);
	report(`${who}: "Essen" on a cucumber → appetite ${hungry} → ${hungry - 5}, eaten`, p2b);

	// 3
	const p3 = [];
	const back = new Date(Date.now() - 3 * 86_400_000);
	if (who === 'guest') await evaluate(`(() => { const s = JSON.parse(localStorage.getItem('scutoons_reader')); s.stats.testo.updated_at = ${JSON.stringify(back.toISOString())}; localStorage.setItem('scutoons_reader', JSON.stringify(s)); return true; })()`);
	else sql("UPDATE member_stats SET updated_at = :at WHERE user_id = :id AND stat = 'testo'", { at: back.toISOString().slice(0, 19).replace('T', ' '), id: me.user.id });
	await openInventory();
	const decayed = await testoNow();
	if (decayed !== 60) p3.push(`three days later: ${decayed} (60 expected)`);
	report(`${who}: three days later (half-life 3 days) the level is halfway back: 60`, p3);

	// 4 – a second testo (setup: the guest's device / the API's story grant), combined with the camera
	const p4 = [];
	if (who === 'guest') await evaluate(`(() => { const s = JSON.parse(localStorage.getItem('scutoons_inventory')); s.entries.push({ id: 'local-check-testo', item_id: 'testo', variant: null, qty: 1, source: 'story', code: null, edition_number: null, acquired_at: new Date().toISOString() }); localStorage.setItem('scutoons_inventory', JSON.stringify(s)); return true; })()`);
	else await call('POST', '/inventory/grant/', { item_id: 'testo', source: 'story', ref: `check-testo:${Date.now()}` });
	await openInventory();
	await clickEl(tile(nameOf('testo')));
	await clickEl(`[...document.querySelectorAll('.item-detail .actions button')].find((b) => b.textContent.trim() === ${JSON.stringify(de.items.combine)})`);
	await clickEl(tile(nameOf('camera')));
	await clickEl(`[...document.querySelectorAll('.inventory button')].find((b) => b.textContent.trim() === ${JSON.stringify(de.items.craft)} && !b.closest('.item-detail'))`);
	await sleep(800);
	const cam = await owned(who, 'testo_camera');
	if (cam !== 1 || (await owned(who, 'camera')) !== 0) p4.push(`testo camera ${cam}, camera ${await owned(who, 'camera')}`);
	await shot(`testo-${who}-4-camera`);
	if (who === 'member') {
		const g1 = await call('POST', '/inventory/grant/', { item_id: 'testo', source: 'scan' });
		const g2 = await call('POST', '/inventory/grant/', { item_id: 'testo', source: 'scan' });
		if (g1.json?.granted !== 1 || g2.status !== 429) p4.push(`scan grants ${g1.status}/${g1.json?.granted}, again ${g2.status}`);
	}
	report(`${who}: testo + camera → testo camera${who === 'member' ? ', its scan grant + cooldown' : ''}`, p4);

	// 5 – a psychedelic
	const p5 = [];
	if (who === 'guest') await evaluate(`(() => { const s = JSON.parse(localStorage.getItem('scutoons_inventory')); s.entries.push({ id: 'local-check-mushroom', item_id: 'psilocybin', variant: null, qty: 1, source: 'story', code: null, edition_number: null, acquired_at: new Date().toISOString() }); localStorage.setItem('scutoons_inventory', JSON.stringify(s)); return true; })()`);
	else await call('POST', '/inventory/grant/', { item_id: 'psilocybin', source: 'story', ref: `check-testo:mushroom:${Date.now()}` });
	await openInventory();
	await clickEl(tile(nameOf('psilocybin')));
	await clickEl(`document.querySelector('[data-testid="item-use"]')`);
	const bar = await waitFor(`document.querySelector('[data-testid="reader-effect"]')?.textContent.trim()`);
	if (!bar?.startsWith(de.items.effect_psychedelic)) p5.push(`effect bar "${bar}"`);
	await open(`${viewer}/de/sandbox/chapter/2`);
	await waitFor(`document.querySelector('[data-slice-index] img')`, 20000);
	const anim = await evaluate(`[document.documentElement.dataset.effect, getComputedStyle(document.querySelector('[data-slice-index] img')).animationName]`);
	if (anim?.[0] !== 'psychedelic' || !/psychedelic-hue/.test(anim?.[1] ?? '')) p5.push(`html/effect animation ${JSON.stringify(anim)}`);
	await shot(`testo-${who}-5-psychedelic`);
	await send('Emulation.setEmulatedMedia', { features: [{ name: 'prefers-reduced-motion', value: 'reduce' }] });
	const reduced = await evaluate(`getComputedStyle(document.querySelector('[data-slice-index] img')).animationName`);
	if (reduced !== 'none') p5.push(`reduced motion: animation ${reduced}`);
	await send('Emulation.setEmulatedMedia', { features: [] });
	await clickEl(`document.querySelector('[data-testid="reader-effect-end"]')`);
	const gone = await waitFor(`!document.querySelector('[data-testid="reader-effect"]') && !document.documentElement.dataset.effect ? true : null`);
	if (!gone) p5.push('"Effekt beenden" did not end it');
	report(`${who}: magic mushroom → colour rush on the slices (off with reduced motion), "Effekt beenden" ends it`, p5);

	// 6 – the Check It camera: the mandatory notice + the multi-label scan (a fake camera stream and fake model
	// predictions through the DEV-only hook `__scutoonsFakePredictions`; MobileNet itself is not downloaded)
	const p6 = [];
	// the recipe (owner 2026-10-01): a fresh camera + testo (setup: the guest's device / the local DB + a story grant),
	// combined in the inventory, the choice "→ Check It Kamera" (the same two items also make the testo camera)
	if (who === 'guest') await evaluate(`(() => { const s = JSON.parse(localStorage.getItem('scutoons_inventory')); const at = new Date().toISOString(); s.entries.push({ id: 'local-check-camera', item_id: 'camera', variant: null, qty: 1, source: 'starter', code: null, edition_number: null, acquired_at: at }, { id: 'local-check-testo-2', item_id: 'testo', variant: null, qty: 1, source: 'story', code: null, edition_number: null, acquired_at: at }); localStorage.setItem('scutoons_inventory', JSON.stringify(s)); return true; })()`);
	else {
		const at = new Date().toISOString().slice(0, 19).replace('T', ' ');
		sql("INSERT INTO inventory (id, user_id, item_id, variant, qty, source, code, edition_number, acquired_at, updated_at) VALUES (:eid, :id, 'camera', NULL, 1, 'starter', NULL, NULL, :at, :at)", { eid: crypto.randomUUID(), id: me.user.id, at });
		await call('POST', '/inventory/grant/', { item_id: 'testo', source: 'story', ref: `check-testo:checkit:${Date.now()}` });
	}
	await openInventory();
	await clickEl(tile(nameOf('testo')));
	await clickEl(`[...document.querySelectorAll('.item-detail .actions button')].find((b) => b.textContent.trim() === ${JSON.stringify(de.items.combine)})`);
	await clickEl(tile(nameOf('camera')));
	await clickEl(`document.querySelector('.recipe [data-recipe="checkit_camera"]')`);
	await clickEl(`[...document.querySelectorAll('.recipe .actions button')].find((b) => b.textContent.trim() === ${JSON.stringify(de.items.craft)})`);
	await sleep(800);
	if ((await owned(who, 'checkit_camera')) !== 1) p6.push(`crafted Check It camera: ${await owned(who, 'checkit_camera')}`);
	const before6 = await owned(who, 'semaglutide');
	await openInventory();
	await evaluate(`(() => {
		globalThis.__scutoonsFakePredictions = [{ className: 'syringe', probability: 0.95 }, { className: 'pill bottle', probability: 0.02 }];
		const c = document.createElement('canvas'); c.width = 640; c.height = 480;
		const g = c.getContext('2d'); setInterval(() => { g.fillStyle = '#' + Math.floor(Math.random() * 0xffffff).toString(16).padStart(6, '0'); g.fillRect(0, 0, 640, 480); }, 100);
		navigator.mediaDevices.getUserMedia = async () => c.captureStream(10);
		return true;
	})()`);
	await clickEl(tile(nameOf('checkit_camera')));
	await clickEl(`document.querySelector('[data-testid="item-use"]')`);
	const notice = await waitFor(`(() => { const n = document.querySelector('[data-testid="camera-notice"]'); return n ? { text: n.textContent.replace(/\\s+/g, ' ').trim(), href: n.querySelector('a')?.href } : null; })()`);
	const expected = `${de.camera.game_notice} ${de.camera.checkit}`;
	if (notice?.text !== expected || notice?.href !== 'https://checkit.wien/') p6.push(`notice ${JSON.stringify(notice)}`);
	const hitText = await waitFor(`document.querySelector('[data-testid="camera-hit"][data-grants="semaglutide"]')?.textContent.trim()`, 15000);
	const wantHit = de.camera.recognised.replace('{object}', 'Spritze').replace('{item}', nameOf('semaglutide'));
	if (hitText !== wantHit) p6.push(`hit "${hitText}" (want "${wantHit}")`);
	await shot(`testo-${who}-6-checkit`);
	await sleep(800);
	if ((await owned(who, 'semaglutide')) !== before6 + 1) p6.push(`semaglutide ${before6} → ${await owned(who, 'semaglutide')}`);
	if (/analys|enthält|contains|safe|sicher/i.test(hitText ?? '')) p6.push('the hit text sounds like an analysis');
	await evaluate(`(document.querySelector('.camera .close')?.click(), delete globalThis.__scutoonsFakePredictions, true)`);
	report(`${who}: camera + testo → Check It camera (recipe choice); notice with the checkit! link on open, "Erkannt: Spritze → ${nameOf('semaglutide')}", granted`, p6);
}

await open(`${viewer}/de/sandbox`);
await sleep(1500);
await idle();
// guest: a fresh device
await evaluate(`(localStorage.clear(), true)`);
await flow('guest');
// member
await open(`${viewer}/de`);
await evaluate(`(localStorage.clear(), localStorage.setItem('scutoons_session', ${JSON.stringify(token)}), true)`);
await flow('member');

console.log(`\n${passed} passed, ${failed} failed`);
await done(failed ? 1 : 0);
