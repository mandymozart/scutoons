// Plays a toon's VN groups in headless Chrome (own temp profile, CDP, no dependencies) and checks the VN layout
// (agents/DESIGN.md → "VN layout") and the one-click rule:
//   - one reader click per waited message, the last one included (the click on the last message jumps)
//   - everyone in the scene (replayed from the group's script) is on stage: `avatar` = left group, `portrait` = right
//   - the speaker grounded on the box top (+4 px), in front of the box (z) with the glow; the others behind it, dimmed,
//     1 rem lower
//   - order on each side: the speaker, then by recency of speaking, then the ones who haven't spoken (join order)
//   - left (avatars, 4 rem): the front one 0.25 rem inside the box's left border, each one behind +1 rem to the right
//   - right (portraits, 9 % smaller per step): further back = further right, overlapping the one in front by 1/3 of its
//     width when there is room, the same larger share up to 2/3 when crowded, beyond that the side is smaller
//   - centre (full view): the front one centred at full size, each one behind alternating left / right by 1.5 rem per step
//     and 9 % smaller, feet on the box's bottom edge, always under the text box; the speaker un-dimmed with the glow in
//     front of the stage, the others dimmed behind it
//   - no mood text
// usage: node scripts/vn/vn-check.mjs [--base=http://localhost:8000] [--api=http://localhost:9000/api] [--toon=sandbox] [--locale=de] [--width=375]
//        [--height=812] [--from=1] [--to=<last chapter>] [--shots=<dir>] [--json=<file>] [--cpu=<throttle rate>] [--latency=<ms>]
// Needs the dev viewer (and API / CDN) running. Exit code 1 on any failed check.
import { spawn } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

const opt = Object.fromEntries(process.argv.slice(2).filter((a) => a.startsWith('--')).map((a) => { const [k, v] = a.slice(2).split('='); return [k, v ?? true]; }));
const base = String(opt.base || 'http://localhost:8000').replace(/\/$/, '');
const toon = opt.toon || 'sandbox';
const locale = opt.locale || 'de';
const width = Number(opt.width || 375), height = Number(opt.height || 812);
const CHROME = process.env.CHROME || ['C:/Program Files/Google/Chrome/Application/chrome.exe', 'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe', '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome', '/usr/bin/google-chrome'].find((p) => existsSync(p));
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

const port = 9400 + Math.floor(Math.random() * 400);
// a throw-away profile, removed again when the check ends (each one is ~120 MB)
const profile = mkdtempSync(join(tmpdir(), 'vn-check-'));
const chrome = spawn(CHROME, ['--headless=new', `--remote-debugging-port=${port}`, `--user-data-dir=${profile}`, '--no-first-run', '--no-default-browser-check', '--disable-extensions', '--mute-audio', `--window-size=${width},${height}`, 'about:blank'], { stdio: 'ignore' });
/** Stops Chrome and deletes the temp profile (Chrome holds its files until it has exited). */
async function closeChrome() {
	const exited = chrome.exitCode !== null ? Promise.resolve() : new Promise((r) => chrome.once('exit', r));
	chrome.kill();
	await Promise.race([exited, sleep(5000)]);
	try { rmSync(profile, { recursive: true, force: true, maxRetries: 10, retryDelay: 200 }); } catch {}
}
process.on('uncaughtException', async (e) => { console.error(e); await closeChrome(); process.exit(1); });
let ws;
for (let i = 0; i < 50 && !ws; i++) {
	try { const u = (await (await fetch(`http://127.0.0.1:${port}/json/list`)).json()).find((t) => t.type === 'page')?.webSocketDebuggerUrl; if (u) ws = new WebSocket(u); } catch {}
	if (!ws) await sleep(200);
}
await new Promise((r) => ws.addEventListener('open', r));
let seq = 0, documents = 0; const pending = new Map(); const consoleErrors = []; const viteLog = [];
ws.addEventListener('message', (e) => {
	const m = JSON.parse(e.data);
	if (m.id && pending.has(m.id)) { pending.get(m.id)(m); pending.delete(m.id); }
	else if (m.method === 'Runtime.consoleAPICalled' && m.params.type === 'error') consoleErrors.push(m.params.args.map((a) => a.value ?? a.description).join(' '));
	else if (m.method === 'Runtime.exceptionThrown') consoleErrors.push(m.params.exceptionDetails.exception?.description ?? 'exception');
	// a new document in the main frame: our navigation, or a reload we didn't ask for (the dev server's full reload);
	// `documents` also counts the dev server's hot updates (below) – both restart a running group
	else if (m.method === 'Page.frameNavigated' && !m.params.frame.parentId) documents++;
	// the dev server's messages (a full reload after a file change / dependency optimization shows up here)
	if (m.method === 'Runtime.consoleAPICalled' && /^\[vite\]/.test(String(m.params.args?.[0]?.value ?? ''))) {
		const line = String(m.params.args[0].value);
		viteLog.push(line);
		// a hot update re-mounts the components: a running VN group starts again from its first event
		if (/hot updated|page reload|invalidate/.test(line)) documents++;
	}
});
const send = (method, params = {}) => new Promise((r) => { const i = ++seq; pending.set(i, r); ws.send(JSON.stringify({ id: i, method, params })); });
const evaluate = async (expression) => (await send('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true })).result.result?.value;
await send('Page.enable'); await send('Runtime.enable');
await send('Emulation.setDeviceMetricsOverride', { width, height, deviceScaleFactor: 1, mobile: width < 768 });
// stress options: --cpu=<rate> (CPU throttling), --latency=<ms> (a slow network)
if (opt.cpu) await send('Emulation.setCPUThrottlingRate', { rate: Number(opt.cpu) });
if (opt.latency) { await send('Network.enable'); await send('Network.emulateNetworkConditions', { offline: false, latency: Number(opt.latency), downloadThroughput: -1, uploadThroughput: -1 }); }

// the state of the stage after each click
const MEASURE = `(() => {
	const q = (x) => Math.round(x * 10) / 10; const r = (el) => { const b = el.getBoundingClientRect(); return { left: q(b.left), right: q(b.right), top: q(b.top), bottom: q(b.bottom), width: q(b.width) }; };
	// x from the layout (computed left / width): the speaker's wiggle (translateX) must not count
	const lx = (el) => { const cs = getComputedStyle(el); const left = q(parseFloat(cs.left)), w = q(parseFloat(cs.width)); return { left, right: q(left + w), width: w }; };
	const stage = [...document.querySelectorAll('.character-portrait')].map((el) => ({ ...r(el), ...lx(el), key: el.dataset.key, side: el.dataset.side, depth: Number(el.dataset.depth), z: Number(getComputedStyle(el).zIndex), active: el.classList.contains('active'), glow: /drop-shadow/.test(el.style.filter), dim: /brightness/.test(el.style.filter), alt: el.querySelector('img')?.alt, src: el.querySelector('img')?.getAttribute('src') }));
	// the centre group (full view): x / size / feet from the layout (computed left, width, height, bottom) – the speaker's
	// wiggle is a scale transform
	const full = [...document.querySelectorAll('.character-full')].map((el) => { const cs = getComputedStyle(el); return { ...lx(el), height: q(parseFloat(cs.height)), feet: q(innerHeight - parseFloat(cs.bottom)), key: el.dataset.key, depth: Number(el.dataset.depth), z: Number(cs.zIndex), active: el.classList.contains('active'), glow: /drop-shadow/.test(el.style.filter), dim: /brightness/.test(el.style.filter), alt: el.querySelector('img')?.alt }; });
	const box = document.querySelector('.vn-message');
	// the text box is on top of everything (hit test in its middle)
	const hit = box ? (() => { const b = box.getBoundingClientRect(); return !!document.elementFromPoint(b.left + b.width / 2, b.top + b.height / 2)?.closest('.vn-message'); })() : null;
	return {
		href: location.href,
		speaker: box?.querySelector('.character-name')?.textContent?.trim() ?? null,
		text: box?.textContent?.trim() ?? '',
		box: box ? { ...r(box), z: Number(getComputedStyle(box).zIndex), visible: box.classList.contains('visible') } : null,
		stage,
		full,
		hit,
		vw: innerWidth,
		rem: parseFloat(getComputedStyle(document.documentElement).fontSize),
		moodBadges: document.querySelectorAll('.mood-badge').length,
		waiting: !!document.querySelector('.continue-overlay')
	};
})()`;

/** The stage state; while the page is being replaced (a reload) there is no answer yet – ask again. */
async function measure() {
	for (let i = 0; i < 40; i++) {
		const value = await evaluate(MEASURE);
		if (value) return value;
		await sleep(250);
	}
	return { href: '', speaker: null, text: '', box: null, stage: [], full: [], hit: null, vw: width, rem: 16, moodBadges: 0, waiting: false };
}

/** The scene (keys → view) and the speaker at every waited message of a group's script: what must be on stage. */
function expectedScenes(events) {
	const scene = new Map();
	const spoke = new Map();
	let tick = 0;
	const out = [];
	const keyOf = (r) => `${r.character ?? r.name}:${r.persona || 'regular'}`;
	for (const e of events) {
		if (e.type === 'joinScene') for (const r of e.params?.characters ?? []) { if (!scene.has(keyOf(r))) spoke.delete(keyOf(r)); scene.delete(keyOf(r)); scene.set(keyOf(r), r.variant ?? r.position?.variant ?? 'avatar'); }
		if (e.type === 'leaveScene') for (const r of e.params?.characters ?? []) {
			if (r.persona) scene.delete(keyOf(r));
			else for (const k of [...scene.keys()]) if (k.startsWith(`${r.character ?? r.name}:`)) scene.delete(k);
		}
		if (e.type === 'showMessage') {
			const speaker = e.params?.character ? `${e.params.character}:${e.params.persona || 'regular'}` : null;
			if (speaker) spoke.set(speaker, ++tick);
			if (!e.params?.wait) continue;
			// front to back per side: the speaker, then by recency of speaking, then the silent ones in join order
			const order = (view) => {
				const keys = [...scene].filter(([, v]) => v === view).map(([k]) => k);
				const rest = keys.filter((k) => k !== speaker);
				return [...(keys.includes(speaker) ? [speaker] : []), ...rest.filter((k) => spoke.has(k)).sort((a, b) => spoke.get(b) - spoke.get(a)), ...rest.filter((k) => !spoke.has(k))];
			};
			out.push({ scene: new Map(scene), speaker, order: { left: order('avatar'), right: order('portrait'), full: order('full') } });
		}
	}
	return out;
}

const failures = [];
/** failures of the group being played go to `sink` first (a replayed group drops the ones of the reloaded attempt) */
let sink = null;
const fail = (msg) => { if (sink) return void sink.push(msg); failures.push(msg); console.log('  FAIL ' + msg); };
const byCount = {}; // "left+right" → one example measurement
const overlaps = []; // measured overlap / width of the one in front
let transitions = 0, steps = 0, leftSteps = 0;

// the viewer's locale comes from the paraglide cookie
await send('Network.setCookie', { name: 'PARAGLIDE_LOCALE', value: locale, url: base });
const prefix = '';
// the scripts come from the library API (content from the DB, one request per chapter with the scenes inlined)
const api = String(opt.api || 'http://localhost:9000/api').replace(/\/$/, '');
const library = async (path) => {
	// drafts (the sandbox) are served by the dev API (CONTENT_SHOW_DRAFTS); elsewhere pass a creator session in SCUTOONS_TOKEN
	const token = process.env.SCUTOONS_TOKEN;
	const r = await fetch(`${api}/library/toons/${toon}${path}?locale=${locale}${token ? '&draft=1' : ''}`, token ? { headers: { Authorization: `Bearer ${token}` } } : {});
	if (!r.ok) throw new Error(`library ${path}: HTTP ${r.status}`);
	return r.json();
};
const chapterFile = (n) => library(`/chapters/${n}`);
const season = (await library('/seasons/1')).season;
const last = Number(opt.to || Math.max(...season.sequence.map((i) => i.number)));
const groups = [];
for (let n = Number(opt.from || 1); n <= last; n++) {
	const ch = await chapterFile(n);
	for (const g of ch.groups) {
		const group = g.ref ? await library(`/chapters/${n}`).then((c) => c.groups.find((x) => !x.ref && x.label === g.ref.src.split('/').pop().replace(/\.json$/, '')) ?? g) : g;
		groups.push({ chapter: n, label: group.label, waited: (group.events ?? []).filter((e) => e.type === 'showMessage' && e.params?.wait).length, events: group.events ?? [], expected: expectedScenes(group.events ?? []) });
	}
}

/**
 * The centre group (full view): the front one exactly centred at full size, each one behind alternating left / right by
 * 1.5 rem per step (0, −1, +1, −2, +2 …) and 9 % smaller; feet on the box's bottom edge; always under the box (the text
 * stays readable on top); the speaker un-dimmed with the glow and in front of the whole stage, the others dimmed behind it.
 */
let fullSteps = 0, fullExample = null;
const fullStep = (k) => (k === 0 ? 0 : (k % 2 ? -1 : 1) * Math.ceil(k / 2));
function checkFull(s, group) {
	const g = [...s.full].sort((a, b) => a.depth - b.depth);
	if (!g.length) return;
	fullSteps++;
	if (!fullExample || g.length > fullExample.length) fullExample = g;
	const front = g[0];
	for (const p of g) {
		const center = p.left + p.width / 2;
		const want = s.vw / 2 + fullStep(p.depth) * 1.5 * s.rem;
		if (Math.abs(center - want) > 1) fail(`${group}: full ${p.alt} (depth ${p.depth}) centred at x ${center.toFixed(1)}, expected ${want}`);
		const h = front.height * Math.max(0.5, 1 - 0.09 * p.depth);
		if (Math.abs(p.height - h) > 1) fail(`${group}: full ${p.alt} (depth ${p.depth}) is ${p.height} px high, expected ${h.toFixed(1)}`);
		if (s.box?.visible && Math.abs(p.feet - s.box.bottom) > 3) fail(`${group}: full ${p.alt} not grounded on the box's bottom edge (feet ${p.feet}, box bottom ${s.box.bottom})`);
		if (s.box && !(p.z < s.box.z)) fail(`${group}: full ${p.alt} is above the text box (z ${p.z} / ${s.box.z})`);
		if (p.active) {
			if (p.depth !== 0) fail(`${group}: the speaker ${p.alt} is not in front of the centre group`);
			if (!p.glow || p.dim) fail(`${group}: the speaker ${p.alt} has no glow or is dimmed`);
			const others = s.stage.filter((x) => !x.active);
			if (others.some((x) => !(p.z > x.z))) fail(`${group}: the speaker ${p.alt} is not in front of the stage`);
		} else {
			if (!p.dim) fail(`${group}: full ${p.alt} (not speaking) is not dimmed`);
			if (s.stage.some((x) => !(p.z < x.z))) fail(`${group}: full ${p.alt} (not speaking) is not behind the stage`);
		}
	}
	if (s.box?.visible && s.hit === false) fail(`${group}: the text box is covered`);
}

function checkStep(s, group, expected) {
	steps++;
	if (s.moodBadges) fail(`${group}: ${s.moodBadges} mood badge(s) on screen`);
	const ps = s.stage;
	// everyone in the scene is shown (full ones are centred elsewhere)
	if (expected) {
		for (const [side, view] of [['left', 'avatar'], ['right', 'portrait']]) {
			const want = [...expected.scene].filter(([, v]) => v === view).map(([k]) => k);
			const got = ps.filter((p) => p.side === side).map((p) => p.key);
			if (got.length !== want.length || got.some((k) => !want.includes(k))) fail(`${group}: ${side} shows ${got.join(', ') || '–'}, expected ${want.join(', ') || '–'}`);
		}
		if (expected.speaker && expected.scene.has(expected.speaker) && ![...ps, ...s.full].some((p) => p.key === expected.speaker && p.active)) fail(`${group}: the speaker ${expected.speaker} is not shown as speaking`);
		const fullKeys = [...s.full].sort((a, b) => a.depth - b.depth).map((p) => p.key);
		if (fullKeys.join() !== expected.order.full.join()) fail(`${group}: centre (full) ${fullKeys.join(' > ') || '–'}, expected ${expected.order.full.join(' > ') || '–'}`);
	}
	checkFull(s, group);
	for (const side of ['left', 'right']) {
		const g = ps.filter((p) => p.side === side).sort((a, b) => a.depth - b.depth);
		if (expected && g.map((p) => p.key).join() !== expected.order[side].join()) fail(`${group}: ${side} order ${g.map((p) => p.key).join(' > ')}, expected ${expected.order[side].join(' > ')}`);
		if (side === 'left') {
			// the avatar stack: the front one 0.25 rem inside the box, each one behind +1 rem to the right, all 4 rem
			for (let k = 0; k < g.length; k++) {
				const want = (s.box?.left ?? 0) + 0.25 * s.rem + k * s.rem;
				if (Math.abs(g[k].left - want) > 0.6) fail(`${group}: avatar ${g[k].alt} (depth ${k}) at x ${g[k].left}, expected ${want}`);
				if (g[k].left < (s.box?.left ?? 0) + 0.25 * s.rem - 0.6) fail(`${group}: avatar ${g[k].alt} sticks out left of the box`);
			}
			continue;
		}
		const base = 1 / 3;
		const ratios = [];
		for (let k = 1; k < g.length; k++) {
			const front = g[k - 1], back = g[k];
			if (!(back.left > front.left)) fail(`${group}: ${side} depth ${k} is not further right than depth ${k - 1}`);
			if (!(back.width < front.width)) fail(`${group}: ${side} depth ${k} has the wrong size (${back.width} after ${front.width})`);
			const ov = front.right - back.left;
			ratios.push(ov / front.width);
		}
		if (!ratios.length) continue;
		overlaps.push(...ratios.map((r) => ({ side, r, crowded: false })));
		const lo = Math.min(...ratios), hi = Math.max(...ratios);
		if (hi - lo > 0.03) fail(`${group}: ${side} overlaps differ (${Math.round(lo * 100)}–${Math.round(hi * 100)} %)`);
		if (lo < base - 0.03 || hi > 2 / 3 + 0.03) fail(`${group}: ${side} overlap ${Math.round(lo * 100)}–${Math.round(hi * 100)} % outside ${Math.round(base * 100)}–67 %`);
		if (hi > base + 0.03) overlaps.slice(-ratios.length).forEach((o) => (o.crowded = true));
	}
	for (const p of ps) {
		if (p.left < 0 || p.right > width) fail(`${group}: ${p.alt} off screen (${p.left}–${p.right})`);
		if (p.active && !p.glow) fail(`${group}: the speaker ${p.alt} has no glow`);
		if (p.active && p.depth !== 0) fail(`${group}: the speaker ${p.alt} is not in front`);
		if (s.box) {
			if (p.active && !(p.z > s.box.z)) fail(`${group}: the speaker ${p.alt} is not in front of the box (z ${p.z} / ${s.box.z})`);
			if (!p.active && !(p.z < s.box.z)) fail(`${group}: ${p.alt} (not speaking) is not behind the box (z ${p.z} / ${s.box.z})`);
			if (!p.active && !p.dim) fail(`${group}: ${p.alt} (not speaking) is not dimmed`);
			const feet = s.box.top + 4 + (p.active ? 0 : s.rem);
			if (s.box.visible && Math.abs(p.bottom - feet) > 3) fail(`${group}: ${p.alt} ${p.active ? 'not grounded' : 'not 1 rem lower'} (bottom ${p.bottom}, box top ${s.box.top})`);
		}
	}
	const left = ps.filter((p) => p.side === 'left');
	if (left.length) {
		leftSteps++;
		for (const p of left) if (Math.abs(p.width - 4 * s.rem) > 0.5) fail(`${group}: avatar ${p.alt} is ${p.width} px, not 4 rem (${4 * s.rem})`);
	}
	if (ps.some((p) => /transition/.test(p.src ?? ''))) transitions++;
	const key = `${ps.filter((p) => p.side === 'left').length}+${ps.filter((p) => p.side === 'right').length}`;
	if (ps.length && !byCount[key]) byCount[key] = { box: s.box, stage: ps.map(({ alt, side, depth, left, right, top, bottom, width, active, z }) => ({ alt, side, depth, left, right, top, bottom, width, active, z })) };
}

const shots = opt.shots ? String(opt.shots) : null;
if (shots) mkdirSync(shots, { recursive: true });
const report = [];

/**
 * Waits until the stage has settled: three equal measurements 150 ms apart (the stage animates for 0.3 s), at most `ms`.
 * Fixed sleeps made the check flaky on a busy machine (a measurement in the middle of a move, or before the next message).
 */
async function settled(ms = 6000) {
	const sig = (s) => JSON.stringify([s.href, s.text, s.waiting, s.box?.visible, s.stage.map((p) => [p.key, p.left, p.width, p.bottom, p.active, p.src])]);
	let prev = await measure(), same = 0;
	for (const until = Date.now() + ms; Date.now() < until && same < 2; ) {
		await sleep(150);
		const next = await measure();
		same = sig(next) === sig(prev) ? same + 1 : 0;
		prev = next;
	}
	return prev;
}

/**
 * Opens the group's page and waits until its first message waits for a click. Retries when the page doesn't load (the
 * dev server restarting or answering an error) – that is the infrastructure, not the VN. Returns the start URL or null.
 */
async function open(url) {
	for (let attempt = 1; attempt <= 3; attempt++) {
		const r = await send('Page.navigate', { url });
		if (!r.result?.errorText) {
			// the group starts once the chapter and its scene files are loaded (the first load in dev compiles modules)
			for (const until = Date.now() + 30000; Date.now() < until; await sleep(200)) {
				if (await evaluate(`!!document.querySelector('.continue-overlay')`)) return evaluate('location.href');
			}
		}
		console.log(`  retry: ${url} ${r.result?.errorText ? `did not load (${r.result.errorText})` : 'showed no message within 30 s'}`);
		await sleep(1000);
	}
	return null;
}

/** Plays one group: one click per waited message, checks every step (failures go to the current `sink`). */
async function play(g, name) {
	const url = `${base}${prefix}/${toon}/chapter/${g.chapter}${g.label ? `?label=${encodeURIComponent(g.label)}` : ''}`;
	const start = await open(url);
	const loaded = documents;
	if (!start) {
		fail(`${name}: the group did not start (no message after 3 loads)`);
		return { clicks: 0, left: false, reloaded: false };
	}
	const lang = await evaluate('document.documentElement.lang');
	if (lang !== locale) fail(`${name}: page language ${lang}, expected ${locale}`);
	let clicks = 0, left = false;
	for (let i = 0; i <= g.waited + 3; i++) {
		let s = await settled();
		if (s.href !== start) { left = true; break; }
		if (!s.waiting && clicks < g.waited) {
			// a message is still due (the script has more waited messages): wait for it instead of giving up early
			for (const until = Date.now() + 15000; !s.waiting && s.href === start && Date.now() < until; ) { await sleep(200); s = await measure(); }
			if (s.waiting) s = await settled();
			if (s.href !== start) { left = true; break; }
		}
		if (documents !== loaded) return { clicks, left, reloaded: true };
		checkStep(s, name, g.expected[i]);
		if (shots && s.stage.length >= 2) {
			const r = await send('Page.captureScreenshot', { format: 'png' });
			writeFileSync(join(shots, `${locale}-${width}-c${g.chapter}-${g.label}-${String(i).padStart(3, '0')}.png`), Buffer.from(r.result.data, 'base64'));
		}
		if (!s.waiting) break;
		await evaluate(`document.querySelector('.continue-overlay')?.click()`);
		clicks++;
	}
	// the last click jumps (client-side navigation) or closes the box
	for (const until = Date.now() + 5000; !left && Date.now() < until; await sleep(200)) left = (await evaluate('location.href')) !== start;
	if (documents !== loaded) return { clicks, left, reloaded: true };
	const endsWithJump = g.events.at(-1)?.type === 'jumpTo';
	const boxOpen = left ? null : (await settled(2000)).box?.visible;
	if (!(clicks === g.waited && (endsWithJump ? left : !boxOpen))) fail(`${name}: ${clicks} clicks for ${g.waited} waited messages (${endsWithJump ? (left ? 'jumped' : 'did not jump') : boxOpen ? 'box still open' : 'box closed'})`);
	return { clicks, left, reloaded: false };
}

let replays = 0;
for (const g of groups) {
	const name = `chapter ${g.chapter}${g.label ? ` · ${g.label}` : ''}`;
	const counters = { steps, leftSteps, transitions, overlaps: overlaps.length, byCount: Object.keys(byCount) };
	let result, attempts;
	for (let attempt = 1; attempt <= 3; attempt++) {
		sink = [];
		result = await play(g, name);
		if (!result.reloaded) break;
		// the dev server reloaded or hot-updated the page in the middle of the group (a file changed – another agent's or
		// app's work, a dependency re-optimized): the group restarted from scratch, its clicks don't add up – play it again
		replays++;
		const why = viteLog.filter((l) => !/connect(ing|ed)/.test(l)).slice(-2).join(' / ');
		console.log(`  replay: the dev server reloaded / updated the page during ${name}${why ? ` (${why})` : ''}`);
		({ steps, leftSteps, transitions } = counters);
		overlaps.length = counters.overlaps;
		for (const k of Object.keys(byCount)) if (!counters.byCount.includes(k)) delete byCount[k];
	}
	attempts = sink;
	sink = null;
	attempts.forEach(fail);
	const ok = !attempts.length;
	report.push({ group: name, waited: g.waited, clicks: result.clicks, jumped: result.left });
	const endsWithJump = g.events.at(-1)?.type === 'jumpTo';
	console.log(`${ok ? 'ok  ' : 'FAIL'} ${name.padEnd(40)} ${result.clicks}/${g.waited} clicks${endsWithJump ? (result.left ? ', jumped' : ', no jump') : ''}`);
}

const pct = (x) => `${(x * 100).toFixed(1)} %`;
console.log(`\n${locale} ${width}×${height}: ${groups.length} groups, ${steps} steps, ${leftSteps} with the left group, ${transitions} with a transition image, ${fullSteps} with the centre group, console errors: ${consoleErrors.length}${replays ? `, ${replays} replay(s) after a reload` : ''}`);
for (const side of ['left', 'right']) {
	const all = overlaps.filter((o) => o.side === side);
	const free = all.filter((o) => !o.crowded).map((o) => o.r), crowded = all.filter((o) => o.crowded).map((o) => o.r);
	if (free.length) console.log(`  ${side} overlap with room: ${free.length} pairs, ${pct(Math.min(...free))} – ${pct(Math.max(...free))}`);
	if (crowded.length) console.log(`  ${side} overlap crowded: ${crowded.length} pairs, ${pct(Math.min(...crowded))} – ${pct(Math.max(...crowded))}`);
}
for (const [n, m] of Object.entries(byCount)) {
	console.log(`  left+right ${n} (box x ${m.box?.left}, top ${m.box?.top}, z ${m.box?.z}): ` + m.stage.sort((a, b) => (a.side === b.side ? a.depth - b.depth : a.side === 'left' ? -1 : 1)).map((p) => `${p.side[0]}${p.depth} ${p.alt} x ${p.left}–${p.right} (w ${p.width}) bottom ${p.bottom} z ${p.z}${p.active ? ' *' : ''}`).join(' | '));
}
if (fullExample) console.log(`  centre (full) example: ` + fullExample.map((p) => `${p.depth} ${p.alt} centre ${(p.left + p.width / 2).toFixed(1)} w ${p.width} h ${p.height} feet ${p.feet} z ${p.z}${p.active ? ' *' : ''}`).join(' | '));
for (const e of consoleErrors.slice(0, 10)) console.log('  console: ' + e);
if (opt.json) writeFileSync(String(opt.json), JSON.stringify({ locale, width, height, report, byCount, overlaps, steps, leftSteps, transitions, consoleErrors, failures }, null, '\t'));
ws.close();
await closeChrome();
if (failures.length) { console.log(`${failures.length} failure(s)`); process.exit(1); }
