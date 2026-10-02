// "Where to buy" check (owner 2026-10-02, agents/EMBASSY.md "Stores"): headless Chrome via CDP (own temp profile, no
// dependencies), like scripts/check-landing.mjs. Needs at least one listed store in the API (locally:
// `php server/bin/console sample-stores`).
//
// 390 × 844 and 1440 × 900, de + en, a fresh guest:
//   - /stores: the title, the API's stores (/stockists/) – the independent ones in the first block, the others in the
//     second –, each with name, the products it carries (catalog names), its website (new tab); no sideways scroll, no
//     text under the caption size;
//   - ?product=<id>: only the stores carrying it (the filter select shows it); ?country=<code>: only that country;
//     a product no store carries: the empty state with "Online kaufen" (the product's link) and "Alle Läden zeigen";
//   - the toon page's related row and the landing page's editions: "Läden vor Ort" → /stores?product=<id> exactly for the
//     products a store carries; the main menu has "Wo kaufen" → /stores.
//
// usage: node scripts/check-stores.mjs [--viewer=http://localhost:8000] [--api=http://localhost:9000/api] [--toon=catpope]
//        [--shots=<dir>] [--verbose]
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
const toon = String(opt.toon || 'catpope');
const shots = opt.shots ? String(opt.shots) : null;
if (shots) mkdirSync(shots, { recursive: true });
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const messages = Object.fromEntries(['de', 'en'].map((l) => [l, JSON.parse(readFileSync(join(root, `viewer/messages/${l}.json`), 'utf8'))]));
const fill = (s, v) => s.replace(/\{(\w+)\}/g, (_, k) => String(v[k]));
const getJson = async (path) => { const r = await fetch(`${api}${path}`); if (!r.ok) throw new Error(`${path} → ${r.status}`); return r.json(); };

// ── what the API says ───────────────────────────────────────────────────────────────────────────────────────────────
const list = await getJson('/stockists/');
if (!list.stores.length) { console.error('no listed store in /stockists/ (locally: php server/bin/console sample-stores)'); process.exit(2); }
const catalog = await getJson('/products/');
const items = (await getJson('/items/')).items;
const nameOf = (id) => catalog.products.find((p) => p.id === id)?.name ?? id;
const carried = Object.keys(list.products);
const rare = [...carried].sort((a, b) => list.products[a] - list.products[b])[0];
const notCarried = catalog.products.find((p) => !carried.includes(p.id));
const someCountry = list.countries[0];

// ── Chrome ──────────────────────────────────────────────────────────────────────────────────────────────────────────
const CHROME = process.env.CHROME || ['C:/Program Files/Google/Chrome/Application/chrome.exe', 'C:/Program Files (x86)/Google/Chrome/Application/chrome.exe', 'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe', '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome', '/usr/bin/google-chrome'].find((p) => existsSync(p));
if (!CHROME) { console.error('Chrome not found (set CHROME)'); process.exit(2); }
const port = 9400 + Math.floor(Math.random() * 400);
const profile = mkdtempSync(join(tmpdir(), 'check-stores-'));
const chrome = spawn(CHROME, ['--headless=new', `--remote-debugging-port=${port}`, `--user-data-dir=${profile}`, '--no-first-run', '--no-default-browser-check', '--disable-extensions', '--mute-audio', '--window-size=1440,900', 'about:blank'], { stdio: 'ignore' });
let restore = null;
async function done(code) {
	if (restore) { try { await restore(); } catch (e) { console.error('restore failed', e); code = code || 1; } }
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
ws.addEventListener('message', (e) => {
	const m = JSON.parse(e.data);
	if (m.id && pending.has(m.id)) { pending.get(m.id)(m); pending.delete(m.id); return; }
	if (m.method?.startsWith('Network.')) lastNet = Date.now();
});
const send = (method, params = {}) => new Promise((r) => { const i = ++seq; pending.set(i, r); ws.send(JSON.stringify({ id: i, method, params })); });
const evaluate = async (expression) => { const r = await send('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true }); return r.result?.exceptionDetails ? { error: r.result.exceptionDetails.exception?.description ?? r.result.exceptionDetails.text } : r.result?.result?.value; };
await send('Page.enable'); await send('Runtime.enable'); await send('Network.enable');
await send('Network.setCacheDisabled', { cacheDisabled: true });
async function idle(quietMs = 700, maxMs = 15000) { const t0 = Date.now(); while (Date.now() - t0 < maxMs) { if (Date.now() - lastNet > quietMs) return; await sleep(100); } }
async function waitFor(js, ms = 12000) { const t0 = Date.now(); let r; while (Date.now() - t0 < ms) { r = await evaluate(js); if (r && !r.error) return r; await sleep(200); } return r ?? null; }
async function open(url, locale) {
	lastNet = Date.now();
	if (locale) await send('Network.setCookie', { name: 'PARAGLIDE_LOCALE', value: locale, url: viewer, path: '/' });
	await send('Page.navigate', { url });
	await sleep(400); await idle();
}
async function shot(name, full = false) {
	if (!shots) return;
	const params = { format: 'png' };
	if (full) {
		const m = await send('Page.getLayoutMetrics');
		const c = m.result.cssContentSize ?? m.result.contentSize;
		params.clip = { x: 0, y: 0, width: c.width, height: Math.min(c.height, 8000), scale: 1 };
		params.captureBeyondViewport = true;
	}
	const r = await send('Page.captureScreenshot', params);
	writeFileSync(join(shots, `${name}.png`), Buffer.from(r.result.data, 'base64'));
}
async function size(width, height) { await send('Emulation.setDeviceMetricsOverride', { width, height, deviceScaleFactor: 1, mobile: width < 768 }); }
async function clickAt(x, y) {
	await send('Input.dispatchMouseEvent', { type: 'mouseMoved', x, y });
	await send('Input.dispatchMouseEvent', { type: 'mousePressed', x, y, button: 'left', clickCount: 1 });
	await send('Input.dispatchMouseEvent', { type: 'mouseReleased', x, y, button: 'left', clickCount: 1 });
}
let failed = 0, passed = 0;
function report(name, problems, detail = '') {
	if (problems.length) { failed++; console.log(`  FAIL  ${name}`); for (const p of problems) console.log(`        ${p}`); }
	else { passed++; console.log(`  ok    ${name}${opt.verbose && detail ? `  ${detail}` : ''}`); }
}

const PAGE = `(() => {
	const page = document.querySelector('[data-testid="stores-page"]');
	if (!page || !page.querySelector('[data-testid="store"], [data-testid="stores-empty"]')) return null;
	const texts = [...page.querySelectorAll('*')].filter((e) => [...e.childNodes].some((n) => n.nodeType === 3 && n.textContent.trim()) && e.getClientRects().length);
	const empty = page.querySelector('[data-testid="stores-empty"]');
	return {
		title: page.querySelector('h1')?.textContent.trim(),
		independent: [...page.querySelectorAll('[data-testid="stores-independent"] [data-testid="store"]')].map((e) => e.dataset.store),
		others: [...page.querySelectorAll('[data-testid="stores-others"] [data-testid="store"]')].map((e) => e.dataset.store),
		stores: [...page.querySelectorAll('[data-testid="store"]')].map((e) => {
			const a = e.querySelector('[data-testid="store-link"]');
			return { id: e.dataset.store, name: e.querySelector('.name')?.textContent.trim(), products: [...e.querySelectorAll('.carries li')].map((li) => li.textContent.trim()),
				link: a ? { href: a.getAttribute('href'), target: a.getAttribute('target') } : null };
		}),
		product: page.querySelector('[data-testid="filter-product"]')?.value ?? null,
		country: page.querySelector('[data-testid="filter-country"]')?.value ?? null,
		empty: empty ? { text: empty.querySelector('p')?.textContent.trim(), links: [...empty.querySelectorAll('a')].map((a) => [a.textContent.trim(), a.getAttribute('href')]) } : null,
		overflow: document.documentElement.scrollWidth - document.documentElement.clientWidth,
		minFont: Math.min(...texts.map((e) => parseFloat(getComputedStyle(e).fontSize))),
		caption: parseFloat(getComputedStyle(document.documentElement).fontSize) * 0.75
	};
})()`;
const sameSet = (a, b) => JSON.stringify([...a].sort()) === JSON.stringify([...b].sort());

for (const [w, h] of [[390, 844], [1440, 900]]) for (const l of ['de', 'en']) {
	console.log(`${w}×${h} ${l} (guest)`);
	const msg = messages[l];
	const prefix = l === 'en' ? '/en' : '';
	await size(w, h);

	await open(`${viewer}${prefix}/stores`, l);
	let s = await waitFor(PAGE);
	let p = [];
	if (!s || s.error) p.push(`page ${JSON.stringify(s)}`);
	else {
		if (s.title !== msg.stores.title) p.push(`title "${s.title}"`);
		const ind = list.stores.filter((x) => x.independent).map((x) => x.id);
		if (!sameSet(s.independent, ind)) p.push(`independent block ${JSON.stringify(s.independent)} ≠ ${JSON.stringify(ind)}`);
		if (!sameSet([...s.independent, ...s.others], list.stores.map((x) => x.id))) p.push(`stores ${JSON.stringify(s.stores.map((x) => x.name))}`);
		for (const store of list.stores) {
			const got = s.stores.find((x) => x.id === store.id);
			if (!got) continue;
			if (got.name !== store.name) p.push(`${store.id}: name "${got.name}"`);
			if (JSON.stringify(got.products) !== JSON.stringify(store.carries.map(nameOf))) p.push(`${store.name}: products ${JSON.stringify(got.products)}`);
			if (store.website && (got.link?.href !== store.website || got.link?.target !== '_blank')) p.push(`${store.name}: link ${JSON.stringify(got.link)}`);
		}
		if (s.overflow > 0) p.push(`sideways scroll ${s.overflow}px`);
		if (s.minFont < s.caption - 0.5) p.push(`text at ${s.minFont}px (caption ${s.caption}px)`);
	}
	report(`/stores: ${list.stores.length} stores, the ${list.stores.filter((x) => x.independent).length} independent first, names, products, links, no sideways scroll`, p, JSON.stringify(s?.stores?.map((x) => x.name)));
	await shot(`stores-${w}-${l}`, true);

	await open(`${viewer}${prefix}/stores?product=${rare}`, l);
	s = await waitFor(PAGE);
	p = [];
	const want = list.stores.filter((x) => x.carries.includes(rare)).map((x) => x.id);
	if (!s || s.error) p.push(`page ${JSON.stringify(s)}`);
	else {
		if (!sameSet(s.stores.map((x) => x.id), want)) p.push(`stores ${JSON.stringify(s.stores.map((x) => x.name))}`);
		if (s.product !== rare) p.push(`product filter shows "${s.product}"`);
	}
	report(`?product=${rare} → ${want.length} store(s), the filter shows it`, p);

	await open(`${viewer}${prefix}/stores?country=${someCountry}`, l);
	s = await waitFor(PAGE);
	const inCountry = list.stores.filter((x) => x.country === someCountry).map((x) => x.id);
	p = !s || s.error || !sameSet(s.stores.map((x) => x.id), inCountry) || s.country !== someCountry ? [`${JSON.stringify(s && { stores: s.stores?.map((x) => x.name), country: s.country })}`] : [];
	report(`?country=${someCountry} → ${inCountry.length} store(s)`, p);

	if (notCarried) {
		await open(`${viewer}${prefix}/stores?product=${notCarried.id}`, l);
		s = await waitFor(PAGE);
		p = [];
		const text = fill(msg.stores.empty_product, { product: notCarried.name });
		if (!s?.empty || s.empty.text !== text) p.push(`empty ${JSON.stringify(s?.empty)}`);
		const links = JSON.stringify(s?.empty?.links ?? []);
		if (links !== JSON.stringify([[msg.stores.buy_online, notCarried.link], [msg.stores.show_all, `${prefix}/stores`]])) p.push(`links ${links}`);
		report(`?product=${notCarried.id} (no store) → "${text}", "${msg.stores.buy_online}", "${msg.stores.show_all}"`, p);
	}

	// "Läden vor Ort" next to "Kaufen": the toon page's related row (a guest owns nothing), the landing page's editions
	await open(`${viewer}${prefix}/${toon}`, l);
	const related = await waitFor(`(() => { const r = document.querySelector('[data-testid="season-related"]'); if (!r || !r.querySelector('[data-testid="related-buy"]')) return null;
		return [...r.querySelectorAll('li[data-item]')].map((li) => { const a = li.querySelector('[data-testid="related-stores"]');
			return { item: li.dataset.item, stores: a?.getAttribute('href') ?? null, text: a?.textContent.trim() ?? null }; }); })()`, 15000);
	await sleep(500);
	const related2 = await evaluate(`[...document.querySelectorAll('[data-testid="season-related"] li[data-item]')].map((li) => { const a = li.querySelector('[data-testid="related-stores"]');
		return { item: li.dataset.item, stores: a?.getAttribute('href') ?? null, text: a?.textContent.trim() ?? null }; })`);
	p = [];
	if (!Array.isArray(related)) p.push(`related row ${JSON.stringify(related)}`);
	else for (const r of related2) {
		const product = items.find((i) => i.id === r.item)?.product;
		const expect = product && list.products[product] ? `${prefix}/stores?product=${product}` : null;
		if (r.stores !== expect || (expect && r.text !== msg.season.local_stores)) p.push(`${r.item}: ${JSON.stringify(r)} (expected ${expect})`);
	}
	const menu = await evaluate(`document.querySelector('[data-testid="menu-stores"]')?.getAttribute('href') ?? null`);
	if (menu !== `${prefix}/stores`) p.push(`menu entry ${menu}`);
	report(`/${toon}: "${msg.season.local_stores}" in the related row exactly for carried products; menu "${msg.menu.stores}"`, p, JSON.stringify(related2));

	await open(`${viewer}${prefix}/`, l);
	await waitFor(`document.querySelector('[data-testid="edition"]') ? true : null`, 12000);
	await sleep(800);
	const editions = await evaluate(`[...document.querySelectorAll('[data-testid="edition"]')].map((a) => ({ product: a.dataset.product, stores: a.querySelector('[data-testid="edition-stores"]')?.getAttribute('href') ?? null }))`);
	p = [];
	if (!Array.isArray(editions) || !editions.length) p.push(`editions ${JSON.stringify(editions)}`);
	else for (const e of editions) {
		const expect = list.products[e.product] ? `${prefix}/stores?product=${e.product}` : null;
		if (e.stores !== expect) p.push(`${e.product}: ${e.stores} (expected ${expect})`);
	}
	report(`landing editions: "${msg.home.local_stores}" exactly for carried products`, p, JSON.stringify(editions));
}

console.log(`\n${passed} passed, ${failed} failed`);
await done(failed ? 1 : 0);
