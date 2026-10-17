// Landing page check (owner 2026-10-01, design "D · Brutal", agents/DESIGN.md "Landing page"): headless Chrome via CDP
// (own temp profile, no dependencies), like scripts/check-toon-hero.mjs.
//
// 390 × 844, 768 × 1024, 1280 × 800, 1440 × 900, de + en, a fresh guest:
//   - the fixed animated logo top left (a link to the localized landing page) and the menu button top right;
//   - the big static wordmark (/SCUTOONS-logo.svg) below it, as wide as the page, + the slogan in bold;
//   - TOONS = the API's list (/library/toons: published + announced; never a draft or the sandbox), published first:
//     square tiles in the star's colour (/library/characters, or the settings' tile.color) with the tile image
//     (Mausboss = the helmet portrait), a 2px ink frame, the name in bold; published → a link to the toon page with
//     "Staffel 1 · <title>" (its newest season with a published chapter) + "Lesen"; announced → "bald", no link, no date;
//     ≥ 768 px three tiles in a row, 390 px the published tile across the row and the announced ones two by two;
//   - LIMITED EDITIONS: intro + "Digitaler Download inklusive · weltweiter Versand", the featured editions in the API's
//     order (/products/featured/ resolved against /products/) with sizes (sold out struck), colours, price, "Limitiert
//     auf N Stück", "Kaufen" (the Bandcamp link), the slider (product-slides.svelte on the endless band's utility:
//     1 slide < 768 px, 3 from 768 px, the slides out of view aria-hidden; autoplay on, prev / pause / next with their
//     labels – pause stops it, next / prev move it one slide and loop round);
//     "Alle Editionen" → the shop; the footer links (Wo kaufen – only with a listed store –, Termine, Quellenangaben); no release date anywhere on the page;
//   - the main menu: the published toons are links, the announced ones greyed "bald" without a link (the helmet image),
//     no sandbox.
// The test user (agent@scutoons.test, role admin), 1280 de: the admin app's "Startseite" page (localhost:11000/homepage)
// changes the featured list (remove the first edition, add the vinyl, save) → the landing page shows the new list; the
// previous list is restored afterwards.
//
// usage: node scripts/check-landing.mjs [--viewer=http://localhost:8000] [--admin=http://localhost:11000]
//        [--api=http://localhost:9000/api] [--shots=<dir>] [--no-member] [--verbose]
import { spawn } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { local } from './dev/ports.mjs';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const opt = Object.fromEntries(process.argv.slice(2).filter((a) => a.startsWith('--')).map((a) => { const [k, ...v] = a.slice(2).split('='); return [k, v.length ? v.join('=') : true]; }));
const viewer = String(opt.viewer || local('viewer')).replace(/\/$/, '');
const adminApp = String(opt.admin || local('admin')).replace(/\/$/, '');
const api = String(opt.api || local('api', '/api')).replace(/\/$/, '');
const shots = opt.shots ? String(opt.shots) : null;
if (shots) mkdirSync(shots, { recursive: true });
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const messages = Object.fromEntries(['de', 'en'].map((l) => [l, JSON.parse(readFileSync(join(root, `viewer/messages/${l}.json`), 'utf8'))]));
const fill = (s, v) => s.replace(/\{(\w+)\}/g, (_, k) => String(v[k]));
const getJson = async (path, init) => { const r = await fetch(`${api}${path}`, init); if (!r.ok) throw new Error(`${path} → ${r.status}`); return r.json(); };

// ── what the API says the page must show ────────────────────────────────────────────────────────────────────────────
const toonList = await getJson('/library/toons');
const listed = [...toonList.filter((t) => t.status === 'published' && !t.announced), ...toonList.filter((t) => t.announced)];
if (!listed.some((t) => t.announced)) { console.error('no announced toon in /library/toons (is the content imported?)'); process.exit(2); }
const registry = await getJson('/library/characters');
const CSS = { blue: 'rgb(0, 0, 255)', yellow: 'rgb(255, 255, 0)', black: 'rgb(0, 0, 0)', red: 'rgb(255, 0, 0)' };
// hex colours (the registry writes #rrggbb since 2026-10-01) → the computed rgb()
const hexRgb = (c) => { const m = /^#([0-9a-f]{3}|[0-9a-f]{6})$/i.exec(c ?? ''); if (!m) return c; const h = m[1].length === 3 ? [...m[1]].map((x) => x + x).join('') : m[1]; return `rgb(${[0, 2, 4].map((i) => parseInt(h.slice(i, i + 2), 16)).join(', ')})`; };
const tileColor = (t) => { const c = t.tile?.color ?? registry.characters.find((x) => x.own_toon === t.id)?.color; return CSS[c] ?? hexRgb(c); };
const seasons = {};
for (const l of ['de', 'en']) {
	seasons[l] = {};
	for (const t of listed.filter((x) => !x.announced)) {
		const bundle = await getJson(`/library/toons/${t.id}?locale=${l}`);
		const s = bundle.seasons.filter((o) => o.chapters.some((c) => c.status === 'published')).sort((a, b) => b.number - a.number)[0];
		seasons[l][t.id] = s ? fill(messages[l].home.season, { number: s.number, title: s.season.title }) : null;
	}
}
const catalog = await getJson('/products/');
// "Wo kaufen" in the footer only once a store is listed (agents/EMBASSY.md "Stores")
const hasStores = ((await getJson('/stockists/').catch(() => ({ stores: [] }))).stores ?? []).length > 0;
async function expectedEditions() {
	const featured = await getJson('/products/featured/');
	return featured.flatMap((f) => {
		const p = catalog.products.find((x) => x.id === f.product);
		if (!p) return [];
		const byId = (id) => p.variants.find((v) => v.id === id);
		const wanted = byId(f.variant) ?? byId(p.defaultVariant);
		const v = wanted && !wanted.soldOut ? wanted : (p.variants.find((x) => !x.soldOut) ?? wanted ?? p.variants[0]);
		return [{ id: p.id, variant: v.id, name: p.name, link: p.link, images: v.images.length, pieces: p.limited ?? p.edition_size, soldOutSizes: p.soldOutSizes }];
	});
}
const shopUrl = (l) => `${local('shop')}${l === 'en' ? '/en' : ''}`;

// ── Chrome ──────────────────────────────────────────────────────────────────────────────────────────────────────────
const CHROME = process.env.CHROME || ['C:/Program Files/Google/Chrome/Application/chrome.exe', 'C:/Program Files (x86)/Google/Chrome/Application/chrome.exe', 'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe', '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome', '/usr/bin/google-chrome'].find((p) => existsSync(p));
if (!CHROME) { console.error('Chrome not found (set CHROME)'); process.exit(2); }
const port = 9400 + Math.floor(Math.random() * 400);
const profile = mkdtempSync(join(tmpdir(), 'check-landing-'));
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
	const box = (e) => { if (!e) return null; const r = e.getBoundingClientRect(); return { x: Math.round(r.x), y: Math.round(r.y + scrollY), w: Math.round(r.width), h: Math.round(r.height) }; };
	const logo = document.querySelector('.logo-link');
	const word = document.querySelector('.wordmark img');
	return {
		vw: document.documentElement.clientWidth,
		logo: logo ? { href: new URL(logo.href).pathname, src: logo.querySelector('img')?.getAttribute('src'), pos: getComputedStyle(logo).position, box: box(logo) } : null,
		menu: box(document.querySelector('[data-testid="menu-toggle"]')),
		wordmark: word ? { src: word.getAttribute('src'), box: box(word), loaded: word.complete && word.naturalWidth > 0 } : null,
		slogan: (() => { const s = document.querySelector('.slogan'); return s ? { text: s.textContent.trim(), weight: getComputedStyle(s).fontWeight } : null; })(),
		tiles: [...document.querySelectorAll('[data-testid="toon-tile"]')].map((t) => {
			const img = t.querySelector('.tile-image img'), frame = t.querySelector('.tile-image');
			const read = t.querySelector('.read');
			return {
				toon: t.dataset.toon, state: t.dataset.state, tag: t.tagName, href: t.tagName === 'A' ? new URL(t.href).pathname : null,
				links: t.querySelectorAll('a').length + (t.closest('a') ? 1 : 0),
				bg: getComputedStyle(frame).backgroundColor, img: img?.getAttribute('src'), loaded: !!img && img.complete && img.naturalWidth > 0,
				imgBox: box(img), box: box(t), border: getComputedStyle(t).borderTopWidth + ' ' + getComputedStyle(t).borderTopColor,
				name: t.querySelector('.tile-name')?.textContent.trim(), nameWeight: t.querySelector('.tile-name') ? getComputedStyle(t.querySelector('.tile-name')).fontWeight : null,
				meta: t.querySelector('.tile-meta')?.textContent.trim() ?? null, read: read?.textContent.trim() ?? null, readBg: read ? getComputedStyle(read).backgroundColor : null,
				text: t.textContent.replace(/\\s+/g, ' ').trim()
			};
		}),
		editions: [...document.querySelectorAll('[data-testid="edition"]')].map((a) => {
			const sw = a.querySelector('[data-slides]');
			return {
				id: a.dataset.product, variant: a.dataset.variant, name: a.querySelector('h3')?.textContent.trim(),
				lines: [...a.querySelectorAll('.line')].map((l) => l.textContent.replace(/\\s+/g, ' ').trim()),
				struck: [...a.querySelectorAll('del')].map((d) => d.firstChild?.textContent.trim()),
				buy: (() => { const b = a.querySelector('.buy'); return b ? { text: b.textContent.trim(), href: b.getAttribute('href'), target: b.getAttribute('target') } : null; })(),
				slides: sw ? sw.querySelectorAll('.slide').length : 0,
				shown: sw ? sw.querySelectorAll('.slide[aria-hidden="false"]').length : 0,
				currentPerView: sw ? Number(sw.dataset.perView) : null,
				sliderBox: box(sw), info: box(a.querySelector('.product-info'))
			};
		}),
		intro: document.querySelector('.editions .intro')?.textContent.trim(), note: document.querySelector('.editions .note')?.textContent.trim(),
		all: (() => { const a = document.querySelector('.editions .all'); return a ? { text: a.textContent.trim(), href: a.href } : null; })(),
		footer: (() => { const f = document.querySelector('.footer'); return f ? { claim: f.querySelector('.claim')?.textContent.trim(), links: [...f.querySelectorAll('a')].map((a) => [a.textContent.trim(), new URL(a.href).pathname]) } : null; })(),
		text: document.querySelector('[data-testid="landing"]')?.innerText ?? '',
		scrollW: document.documentElement.scrollWidth
	};
})()`;

async function guest(width, height, locale) {
	const prefix = locale === 'en' ? '/en' : '';
	const msg = messages[locale];
	console.log(`${width}×${height} ${locale} (guest)`);
	await size(width, height);
	await open(`${viewer}${prefix}/`, locale);
	const editions = await expectedEditions();
	let s = await waitFor(`(() => { const s = ${PAGE}; return s.tiles.length && s.editions.length === ${editions.length} && s.tiles.every((t) => t.loaded) ? s : null; })()`, 15000);
	if (!s || s.error) s = await evaluate(PAGE);
	{
		const p = [];
		if (!s.logo || s.logo.src !== '/SCUTOONS-logo-animated.svg' || s.logo.pos !== 'fixed' || s.logo.box.x > 8 || s.logo.box.y > 16 || s.logo.href !== `${prefix}/`) p.push(`animated logo ${JSON.stringify(s.logo)}`);
		if (!s.menu || s.menu.x + s.menu.w < s.vw - 24 || s.menu.y > 16) p.push(`menu button ${JSON.stringify(s.menu)}`);
		if (!s.wordmark || s.wordmark.src !== '/SCUTOONS-logo.svg' || !s.wordmark.loaded || s.wordmark.box.w < s.vw - 2 * 48 - 2 || s.wordmark.box.y < s.logo?.box.y + s.logo?.box.h) p.push(`wordmark ${JSON.stringify(s.wordmark)} (logo ${JSON.stringify(s.logo?.box)})`);
		if (!s.slogan || s.slogan.text !== msg.slogan || Number(s.slogan.weight) < 700) p.push(`slogan ${JSON.stringify(s.slogan)}`);
		if (s.scrollW > s.vw) p.push(`horizontal scroll: ${s.scrollW} > ${s.vw}`);
		report('the animated logo top left (→ home), the menu button top right, the big wordmark + bold slogan', p, JSON.stringify({ logo: s.logo?.box, word: s.wordmark?.box }));
	}
	{
		const p = [];
		const ids = s.tiles.map((t) => t.toon);
		if (JSON.stringify(ids) !== JSON.stringify(listed.map((t) => t.id))) p.push(`tiles ${ids.join(', ')}, expected ${listed.map((t) => t.id).join(', ')}`);
		if (ids.includes('sandbox')) p.push('the sandbox has a tile');
		for (const t of s.tiles) {
			const want = listed.find((x) => x.id === t.toon);
			if (!want) continue;
			const name = want.title?.[locale] ?? want.i18n?.[locale]?.name;
			if (t.bg !== tileColor(want)) p.push(`${t.toon}: background ${t.bg}, expected ${tileColor(want)}`);
			if (!t.loaded || !t.img?.endsWith(want.tile?.image?.replace(/^\.\.\//, '') ?? '\u0000')) p.push(`${t.toon}: image ${t.img} (loaded ${t.loaded}), expected …${want.tile?.image}`);
			if (!t.imgBox || Math.abs(t.imgBox.w - t.imgBox.h) > 1) p.push(`${t.toon}: image not square ${JSON.stringify(t.imgBox)}`);
			if (t.border !== '2px rgb(0, 0, 0)') p.push(`${t.toon}: frame ${t.border}`);
			if (t.name !== name || Number(t.nameWeight) < 700) p.push(`${t.toon}: name "${t.name}" (${t.nameWeight})`);
			if (/\b(19|20)\d\d\b|\d{1,2}\.\s?\d{1,2}\./.test(t.text)) p.push(`${t.toon}: a date on the tile "${t.text}"`);
			if (want.announced) {
				if (t.tag === 'A' || t.links) p.push(`${t.toon}: announced but a link`);
				if (t.meta !== msg.home.soon || t.read) p.push(`${t.toon}: meta "${t.meta}", read "${t.read}"`);
			} else {
				if (t.tag !== 'A' || t.href !== `${prefix}/${t.toon}`) p.push(`${t.toon}: link ${t.href}`);
				if (t.meta !== seasons[locale][t.toon]) p.push(`${t.toon}: meta "${t.meta}", expected "${seasons[locale][t.toon]}"`);
				if (t.read !== msg.home.read || t.readBg !== 'rgb(0, 0, 0)') p.push(`${t.toon}: read button "${t.read}" ${t.readBg}`);
			}
		}
		if (s.tiles.find((t) => t.toon === 'yellowmouse') && !s.tiles.find((t) => t.toon === 'yellowmouse').img.includes('yellowmouse_helmet_portrait-neutral')) p.push('Mausboss without the helmet');
		// layout
		const boxes = s.tiles.map((t) => t.box);
		if (width >= 768) {
			const row = boxes.slice(0, 3);
			if (row.some((b) => Math.abs(b.y - row[0].y) > 1 || Math.abs(b.w - row[0].w) > 2)) p.push(`not three equal tiles in a row: ${JSON.stringify(row)}`);
		} else {
			const pub = s.tiles.filter((t) => t.state === 'published').map((t) => t.box);
			const soon = s.tiles.filter((t) => t.state === 'soon').map((t) => t.box);
			if (pub.some((b) => b.w < s.vw - 2 * 16 - 2)) p.push(`a published tile is not across the row: ${JSON.stringify(pub)}`);
			if (soon.length >= 2 && (Math.abs(soon[0].y - soon[1].y) > 1 || soon[0].w > s.vw / 2)) p.push(`the announced tiles are not two by two: ${JSON.stringify(soon)}`);
		}
		report(`TOONS: ${ids.join(', ')} – colours, images (Mausboss helmet), frames, "${msg.home.read}" / "${msg.home.soon}", no dates, ${width >= 768 ? '3 in a row' : 'published across, announced 2 × 1'}`, p, JSON.stringify(s.tiles.map((t) => [t.toon, t.bg, t.box])));
	}
	{
		const p = [];
		if (s.intro !== msg.home.editions_intro || s.note !== msg.home.editions_note) p.push(`intro "${s.intro}" / note "${s.note}"`);
		const got = s.editions.map((e) => `${e.id}/${e.variant}`), want = editions.map((e) => `${e.id}/${e.variant}`);
		if (JSON.stringify(got) !== JSON.stringify(want)) p.push(`editions ${got.join(', ')}, expected ${want.join(', ')}`);
		for (const e of s.editions) {
			const w = editions.find((x) => x.id === e.id);
			if (!w) continue;
			if (e.name !== w.name) p.push(`${e.id}: name "${e.name}"`);
			if (w.pieces && !e.lines.includes(fill(msg.home.limited, { count: w.pieces }))) p.push(`${e.id}: no "${fill(msg.home.limited, { count: w.pieces })}" in ${JSON.stringify(e.lines)}`);
			if (!e.lines.some((l) => /€/.test(l) && /\d+[.,]\d\d/.test(l))) p.push(`${e.id}: no price in ${JSON.stringify(e.lines)}`);
			for (const size of w.soldOutSizes) if (!e.struck.includes(size)) p.push(`${e.id}: sold-out size ${size} not struck`);
			if (!e.buy || e.buy.text !== msg.home.buy || e.buy.href !== w.link || e.buy.target !== '_blank') p.push(`${e.id}: buy ${JSON.stringify(e.buy)}`);
			const perView = width >= 768 ? 3 : 1;
			if (e.slides !== w.images || e.currentPerView !== perView) p.push(`${e.id}: slider ${e.slides} slides, ${e.currentPerView} per view (expected ${w.images}, ${perView})`);
			if (e.shown !== Math.min(perView, e.slides)) p.push(`${e.id}: ${e.shown} slides not aria-hidden (expected ${Math.min(perView, e.slides)})`);
			if (width >= 1024 && !(e.info && e.sliderBox && e.info.x < e.sliderBox.x && Math.abs(e.info.y - e.sliderBox.y) < 4)) p.push(`${e.id}: text not left of the slider ${JSON.stringify([e.info, e.sliderBox])}`);
			if (width < 1024 && !(e.info && e.sliderBox && e.sliderBox.y > e.info.y)) p.push(`${e.id}: slider not below the text ${JSON.stringify([e.info, e.sliderBox])}`);
		}
		if (!s.all || s.all.text !== msg.home.all_editions || !s.all.href.startsWith(shopUrl(locale))) p.push(`all editions ${JSON.stringify(s.all)}`);
		report(`LIMITED EDITIONS: ${got.join(', ')} – sizes, price, "${fill(msg.home.limited, { count: 'N' })}", "${msg.home.buy}", slider ${width >= 768 ? '3-up' : '1-up'}, "${msg.home.all_editions}"`, p);
	}
	{
		// the slider's buttons (product-slides.svelte): the first edition with more images than fit in view (2 images in a
		// 3-up slider are whole and still – no buttons); pause first so only the clicks count, then next, prev, and prev
		// from the first slide round to the last
		const pick = `[...document.querySelectorAll('[data-testid="edition"] [data-slides]')].find((sw) => sw.querySelector('[data-slides-next]'))`;
		const slider = () => evaluate(`(() => { const sw = ${pick}; return { index: Number(sw.dataset.index), playing: sw.dataset.playing === 'true', count: sw.querySelectorAll('.slide').length, labels: ['prev', 'toggle', 'next'].map((k) => sw.querySelector('[data-slides-' + k + ']')?.getAttribute('aria-label')) }; })()`);
		const press = async (sel) => {
			const b = await evaluate(`(() => { const b = (${pick}).querySelector('${sel}').getBoundingClientRect(); return { x: Math.round(b.x + b.width / 2), y: Math.round(b.y + b.height / 2) }; })()`);
			await clickAt(b.x, b.y);
			await sleep(700);
			return slider();
		};
		const p = [];
		const r = await evaluate(`(async () => {
			const sw = ${pick};
			if (!sw) return { error: 'no slider with buttons' };
			sw.scrollIntoView({ block: 'center' });
			await new Promise((r) => setTimeout(r, 400));
			return true;
		})()`);
		if (!r || r.error) p.push(JSON.stringify(r));
		else {
			const start = await slider();
			if (!start.playing) p.push('autoplay is not on at the start');
			if (JSON.stringify(start.labels) !== JSON.stringify([msg.home.slides_prev, msg.home.slides_pause, msg.home.slides_next])) p.push(`button labels ${JSON.stringify(start.labels)}`);
			const paused = await press('[data-slides-toggle]');
			if (paused.playing || paused.labels[1] !== msg.home.slides_play) p.push(`pause: playing ${paused.playing}, label "${paused.labels[1]}"`);
			const n = paused.count;
			const next = await press('[data-slides-next]');
			if (next.index !== (paused.index + 1) % n) p.push(`next: ${paused.index} → ${next.index}`);
			const prev = await press('[data-slides-prev]');
			if (prev.index !== paused.index) p.push(`prev: ${next.index} → ${prev.index}`);
			let now = prev;
			for (let i = 0; i < n && now.index !== 0; i++) now = await press('[data-slides-next]');
			const round = await press('[data-slides-prev]');
			if (now.index !== 0 || round.index !== n - 1) p.push(`prev from the first slide: ${now.index} → ${round.index} (expected ${n - 1})`);
			if (round.playing) p.push('a button started the autoplay again');
		}
		report('the slider: autoplay on, pause stops it, next / prev move one slide and loop round, the buttons\' labels', p);
	}
	{
		const p = [];
		if (!s.footer || s.footer.claim !== msg.home.footer) p.push(`footer claim ${JSON.stringify(s.footer)}`);
		const links = JSON.stringify(s.footer?.links ?? []);
		if (links !== JSON.stringify([...(hasStores ? [[msg.home.stores, `${prefix}/stores`]] : []), [msg.home.events, `${prefix}/events`], [msg.home.attributions, `${prefix}/attributions`]])) p.push(`footer links ${links}`);
		const dates = s.text.match(/\b\d{1,2}\.\s?\d{1,2}\.\s?(19|20)\d\d\b|\b(19|20)\d\d-\d\d-\d\d\b|\b(Januar|Februar|März|April|Mai|Juni|Juli|August|September|Oktober|November|Dezember|January|February|March|June|July|October|December)\s+\d/gi);
		if (dates) p.push(`dates on the page: ${dates.join(', ')}`);
		if (s.text.includes(msg.next_release)) p.push(`"${msg.next_release}" on the page`);
		report(`footer (${hasStores ? `${msg.home.stores} · ` : ''}${msg.home.events} · ${msg.home.attributions}), no dates / release announcements on the page`, p);
	}
	await evaluate('window.scrollTo(0, 0), true');
	await shot(`landing-${width}-${locale}`);
	await shot(`landing-${width}-${locale}-full`, true);
	// the main menu: published toons = links, announced = greyed "bald" (no link, the tile image), no sandbox
	{
		await evaluate(`document.querySelector('[data-testid="menu-toggle"]').click(), true`);
		const m = await waitFor(`(() => { const menu = document.querySelector('#main-menu.open'); if (!menu) return null; return {
			links: [...menu.querySelectorAll('.nav--toons a.nav-btn')].map((a) => [a.dataset.toon, new URL(a.href).pathname]),
			soon: [...menu.querySelectorAll('[data-testid="menu-toon-soon"]')].map((e) => ({ toon: e.dataset.toon, tag: e.tagName, text: e.textContent.replace(/\\s+/g, ' ').trim(), color: getComputedStyle(e).color, img: e.querySelector('img')?.getAttribute('src') ?? '', link: !!e.closest('a') || !!e.querySelector('a') })),
			names: menu.querySelector('.nav--toons')?.textContent ?? ''
		}; })()`, 5000);
		const p = [];
		if (!m || m.error) p.push(`menu ${JSON.stringify(m)}`);
		else {
			const pub = listed.filter((t) => !t.announced), soon = listed.filter((t) => t.announced);
			if (JSON.stringify(m.links) !== JSON.stringify(pub.map((t) => [t.id, `${prefix}/${t.id}`]))) p.push(`links ${JSON.stringify(m.links)}`);
			if (JSON.stringify(m.soon.map((x) => x.toon)) !== JSON.stringify(soon.map((t) => t.id))) p.push(`announced ${JSON.stringify(m.soon.map((x) => x.toon))}`);
			for (const x of m.soon) {
				if (x.tag === 'A' || x.link || !x.text.endsWith(msg.home.soon) || x.color !== 'rgb(107, 107, 107)') p.push(`${x.toon}: ${JSON.stringify(x)}`);
				if (x.toon === 'yellowmouse' && !x.img.includes('yellowmouse_helmet_portrait-neutral')) p.push(`Mausboss in the menu without the helmet: ${x.img}`);
			}
			if (/Sandkasten|Sandbox/.test(m.names)) p.push('the sandbox is in the menu');
		}
		report('menu: published toons are links, announced ones greyed "bald" without a link (Mausboss helmet), no sandbox', p, JSON.stringify(m));
		if (width === 390 || width === 1440) await shot(`menu-${width}-${locale}`);
		await evaluate(`document.querySelector('[data-testid="menu-close"]').click(), true`);
		await sleep(300);
	}
}

for (const [w, h] of [[390, 844], [768, 1024], [1280, 800], [1440, 900]]) for (const l of ['de', 'en']) await guest(w, h, l);

// the test user (role admin) changes the featured list in the admin app → the landing page shows it
if (!opt['no-member']) {
	console.log('1280×800 de (admin: agent@scutoons.test) – admin "Startseite" → landing page');
	const sessionFile = join(root, 'logs/dev/test-user.json');
	const token = existsSync(sessionFile) ? JSON.parse(readFileSync(sessionFile, 'utf8')).token : null;
	const auth = { Authorization: `Bearer ${token}`, 'Content-Type': 'application/json' };
	const before = token ? await fetch(`${api}/admin/featured`, { headers: auth }) : null;
	if (!before?.ok) report('admin', [`no working admin session of the test user (${before?.status ?? 'no session'}; run \`npm run dev:test-user\`)`]);
	else {
		const prev = await before.json();
		restore = async () => {
			await fetch(`${api}/admin/featured`, { method: 'PUT', headers: auth, body: JSON.stringify({ items: prev.stored ? prev.items : null }) });
			restore = null;
		};
		await size(1280, 800);
		await send('Page.navigate', { url: `${adminApp}/` }); await sleep(400); await idle();
		await evaluate(`localStorage.setItem('scutoons_session', ${JSON.stringify(token)}), true`);
		await send('Page.navigate', { url: `${adminApp}/homepage` }); await sleep(400); await idle();
		const rows = await waitFor(`(() => { const r = [...document.querySelectorAll('[data-testid="featured-row"]')]; return r.length ? r.map((x) => [x.dataset.product, x.dataset.variant]) : null; })()`, 12000);
		const p = [];
		if (!rows || rows.error) p.push(`admin page rows ${JSON.stringify(rows)}`);
		else {
			const first = rows[0][0];
			const pick = catalog.products.find((x) => !rows.some((r) => r[0] === x.id) && x.variants.some((v) => !v.soldOut) && x.variants.length === 1) ?? catalog.products.find((x) => !rows.some((r) => r[0] === x.id));
			// remove the first, add another product (its default colour), save – through the UI
			const ui = await evaluate(`(async () => {
				const tick = () => new Promise((r) => setTimeout(r, 150));
				document.querySelector('[data-testid="featured-row"] [data-testid="featured-remove"]').click(); await tick();
				const sel = document.querySelector('[data-testid="featured-add-product"]');
				sel.value = ${JSON.stringify(pick.id)}; sel.dispatchEvent(new Event('change', { bubbles: true })); await tick();
				document.querySelector('[data-testid="featured-add"]').click(); await tick();
				const rows = [...document.querySelectorAll('[data-testid="featured-row"]')].map((x) => [x.dataset.product, x.dataset.variant]);
				document.querySelector('[data-testid="featured-save"]').click();
				return rows;
			})()`);
			await sleep(1200);
			await shot('admin-homepage-1280');
			const stored = await (await fetch(`${api}/admin/featured`, { headers: auth })).json();
			const wantIds = ui?.error ? [] : ui.map((r) => r[0]);
			if (ui?.error || wantIds.includes(first) && rows.filter((r) => r[0] === first).length === 1 || !wantIds.includes(pick.id)) p.push(`UI edit ${JSON.stringify(ui)}`);
			if (!stored.stored || JSON.stringify(stored.items.map((i) => i.product)) !== JSON.stringify(wantIds) || !String(stored.updated_by).includes('agent@scutoons.test')) p.push(`stored ${JSON.stringify(stored)}`);
			report(`admin "Startseite": remove ${first}, add ${pick.id}, save → stored by the test user`, p, JSON.stringify(stored.items));
			const q = [];
			await open(`${viewer}/`, 'de');
			const shown = await waitFor(`(() => { const e = [...document.querySelectorAll('[data-testid="edition"]')].map((a) => a.dataset.product); return e.length === ${wantIds.length} ? e : null; })()`, 12000);
			if (JSON.stringify(shown) !== JSON.stringify(wantIds)) q.push(`landing editions ${JSON.stringify(shown)}, expected ${JSON.stringify(wantIds)}`);
			report('the landing page shows the new list in its order', q);
			await shot('landing-1280-de-after-admin', true);
		}
		await restore?.();
		const after = await (await fetch(`${api}/admin/featured`, { headers: auth })).json();
		report('the previous featured list is restored', JSON.stringify([after.stored, after.items]) === JSON.stringify([prev.stored, prev.items]) ? [] : [`${JSON.stringify(after.items)} vs ${JSON.stringify(prev.items)}`]);
	}
}

console.log(`\n${passed} passed, ${failed} failed`);
await done(failed ? 1 : 0);
