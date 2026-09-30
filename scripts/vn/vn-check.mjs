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
//   - no mood text
// usage: node scripts/vn/vn-check.mjs [--base=http://localhost:8000] [--api=http://localhost:9000/api] [--toon=sandbox] [--locale=de] [--width=375]
//        [--height=812] [--from=1] [--to=<last chapter>] [--shots=<dir>] [--json=<file>]
// Needs the dev viewer (and API / CDN) running. Exit code 1 on any failed check.
import { spawn } from 'node:child_process';
import { existsSync, mkdirSync, mkdtempSync, writeFileSync } from 'node:fs';
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
const chrome = spawn(CHROME, ['--headless=new', `--remote-debugging-port=${port}`, `--user-data-dir=${mkdtempSync(join(tmpdir(), 'vn-check-'))}`, '--no-first-run', '--no-default-browser-check', '--disable-extensions', '--mute-audio', `--window-size=${width},${height}`, 'about:blank'], { stdio: 'ignore' });
let ws;
for (let i = 0; i < 50 && !ws; i++) {
	try { const u = (await (await fetch(`http://127.0.0.1:${port}/json/list`)).json()).find((t) => t.type === 'page')?.webSocketDebuggerUrl; if (u) ws = new WebSocket(u); } catch {}
	if (!ws) await sleep(200);
}
await new Promise((r) => ws.addEventListener('open', r));
let seq = 0; const pending = new Map(); const consoleErrors = [];
ws.addEventListener('message', (e) => {
	const m = JSON.parse(e.data);
	if (m.id && pending.has(m.id)) { pending.get(m.id)(m); pending.delete(m.id); }
	else if (m.method === 'Runtime.consoleAPICalled' && m.params.type === 'error') consoleErrors.push(m.params.args.map((a) => a.value ?? a.description).join(' '));
	else if (m.method === 'Runtime.exceptionThrown') consoleErrors.push(m.params.exceptionDetails.exception?.description ?? 'exception');
});
const send = (method, params = {}) => new Promise((r) => { const i = ++seq; pending.set(i, r); ws.send(JSON.stringify({ id: i, method, params })); });
const evaluate = async (expression) => (await send('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true })).result.result?.value;
await send('Page.enable'); await send('Runtime.enable');
await send('Emulation.setDeviceMetricsOverride', { width, height, deviceScaleFactor: 1, mobile: width < 768 });

// the state of the stage after each click
const MEASURE = `(() => {
	const q = (x) => Math.round(x * 10) / 10; const r = (el) => { const b = el.getBoundingClientRect(); return { left: q(b.left), right: q(b.right), top: q(b.top), bottom: q(b.bottom), width: q(b.width) }; };
	// x from the layout (computed left / width): the speaker's wiggle (translateX) must not count
	const lx = (el) => { const cs = getComputedStyle(el); const left = q(parseFloat(cs.left)), w = q(parseFloat(cs.width)); return { left, right: q(left + w), width: w }; };
	const stage = [...document.querySelectorAll('.character-portrait')].map((el) => ({ ...r(el), ...lx(el), key: el.dataset.key, side: el.dataset.side, depth: Number(el.dataset.depth), z: Number(getComputedStyle(el).zIndex), active: el.classList.contains('active'), glow: /drop-shadow/.test(el.style.filter), dim: /brightness/.test(el.style.filter), alt: el.querySelector('img')?.alt, src: el.querySelector('img')?.getAttribute('src') }));
	const box = document.querySelector('.vn-message');
	return {
		href: location.href,
		speaker: box?.querySelector('.character-name')?.textContent?.trim() ?? null,
		box: box ? { ...r(box), z: Number(getComputedStyle(box).zIndex), visible: box.classList.contains('visible') } : null,
		stage,
		rem: parseFloat(getComputedStyle(document.documentElement).fontSize),
		moodBadges: document.querySelectorAll('.mood-badge').length,
		waiting: !!document.querySelector('.continue-overlay')
	};
})()`;

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
			out.push({ scene: new Map(scene), speaker, order: { left: order('avatar'), right: order('portrait') } });
		}
	}
	return out;
}

const failures = [];
const fail = (msg) => { failures.push(msg); console.log('  FAIL ' + msg); };
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
		if (expected.speaker && expected.scene.has(expected.speaker) && expected.scene.get(expected.speaker) !== 'full' && !ps.some((p) => p.key === expected.speaker && p.active)) fail(`${group}: the speaker ${expected.speaker} is not shown as speaking`);
	}
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
for (const g of groups) {
	const name = `chapter ${g.chapter}${g.label ? ` · ${g.label}` : ''}`;
	const url = `${base}${prefix}/${toon}/chapter/${g.chapter}${g.label ? `?label=${encodeURIComponent(g.label)}` : ''}`;
	await send('Page.navigate', { url });
	// the group starts once the chapter and its scene files are loaded (the first load in dev compiles modules)
	for (let t = 0; t < 40 && !(await evaluate(`!!document.querySelector('.continue-overlay')`)); t++) await sleep(250);
	const start = await evaluate('location.href');
	const lang = await evaluate('document.documentElement.lang');
	if (lang !== locale) fail(`${name}: page language ${lang}, expected ${locale}`);
	let clicks = 0, left = false;
	for (let i = 0; i <= g.waited + 3; i++) {
		await sleep(420); // the stage moves in 0.3 s
		const s = await evaluate(MEASURE);
		if (s.href !== start) { left = true; break; }
		checkStep(s, name, g.expected[i]);
		if (shots && s.stage.length >= 2) {
			const r = await send('Page.captureScreenshot', { format: 'png' });
			writeFileSync(join(shots, `${locale}-${width}-c${g.chapter}-${g.label}-${String(i).padStart(3, '0')}.png`), Buffer.from(r.result.data, 'base64'));
		}
		if (!s.waiting) {
			// the next message may take a moment (a busy dev server): up to 3 s before the group counts as done
			let waiting = false;
			for (let t = 0; t < 12 && !waiting; t++) { await sleep(250); waiting = (await evaluate(MEASURE)).waiting; }
			if (!waiting) break;
		}
		await evaluate(`document.querySelector('.continue-overlay')?.click()`);
		clicks++;
	}
	await sleep(600);
	if (!left) left = (await evaluate('location.href')) !== start;
	const endsWithJump = g.events.at(-1)?.type === 'jumpTo';
	const boxOpen = left ? null : await evaluate(`!!document.querySelector('.vn-message.visible')`);
	const ok = clicks === g.waited && (endsWithJump ? left : !boxOpen);
	if (!ok) fail(`${name}: ${clicks} clicks for ${g.waited} waited messages (${endsWithJump ? (left ? 'jumped' : 'did not jump') : boxOpen ? 'box still open' : 'box closed'})`);
	report.push({ group: name, waited: g.waited, clicks, jumped: left });
	console.log(`${ok ? 'ok  ' : 'FAIL'} ${name.padEnd(40)} ${clicks}/${g.waited} clicks${endsWithJump ? (left ? ', jumped' : ', no jump') : ''}`);
}

const pct = (x) => `${(x * 100).toFixed(1)} %`;
console.log(`\n${locale} ${width}×${height}: ${groups.length} groups, ${steps} steps, ${leftSteps} with the left group, ${transitions} with a transition image, console errors: ${consoleErrors.length}`);
for (const side of ['left', 'right']) {
	const all = overlaps.filter((o) => o.side === side);
	const free = all.filter((o) => !o.crowded).map((o) => o.r), crowded = all.filter((o) => o.crowded).map((o) => o.r);
	if (free.length) console.log(`  ${side} overlap with room: ${free.length} pairs, ${pct(Math.min(...free))} – ${pct(Math.max(...free))}`);
	if (crowded.length) console.log(`  ${side} overlap crowded: ${crowded.length} pairs, ${pct(Math.min(...crowded))} – ${pct(Math.max(...crowded))}`);
}
for (const [n, m] of Object.entries(byCount)) {
	console.log(`  left+right ${n} (box x ${m.box?.left}, top ${m.box?.top}, z ${m.box?.z}): ` + m.stage.sort((a, b) => (a.side === b.side ? a.depth - b.depth : a.side === 'left' ? -1 : 1)).map((p) => `${p.side[0]}${p.depth} ${p.alt} x ${p.left}–${p.right} (w ${p.width}) bottom ${p.bottom} z ${p.z}${p.active ? ' *' : ''}`).join(' | '));
}
for (const e of consoleErrors.slice(0, 10)) console.log('  console: ' + e);
if (opt.json) writeFileSync(String(opt.json), JSON.stringify({ locale, width, height, report, byCount, overlaps, steps, leftSteps, transitions, consoleErrors, failures }, null, '\t'));
ws.close();
chrome.kill();
if (failures.length) { console.log(`${failures.length} failure(s)`); process.exit(1); }
