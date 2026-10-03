#!/usr/bin/env node
// The brutalist tools' regression check (agents/DESIGN.md "Brutalist tools", owner 2026-10-01).
//
//   node scripts/check-tools-ui.mjs --static      no px font size (and no font size under caption in em) in the tools'
//                                                 CSS: creator, admin, embassy, @scutoons/ui + media (part of `npm run check`);
//                                                 no bold (owner 2026-10-02 "only use bold very rarely"): no bold
//                                                 font-weight in their CSS, no <b>, <strong> only with class sc-emphasis
//                                                 (a line carrying `bold-ok` + the reason is exempt: the page title h1);
//                                                 one control height (owner 2026-10-02): no `height` / `min-height` /
//                                                 vertical `padding` on a control selector (button, .btn, select,
//                                                 input, .sc-button, .sc-icon-button, .sc-control, .sc-chip-control,
//                                                 [role=button]) that bypasses --control-height, in the tools AND the
//                                                 viewer / shop / shared CSS (a line carrying `control-ok` + the reason
//                                                 is exempt; class-named buttons are left to the browser run);
//                                                 close cross (owner 2026-10-02): no × ✕ ✖ glyph as a button's / link's
//                                                 content and no Feather "x" in any app's markup – the one drawn
//                                                 CloseIcon / CloseButton of @scutoons/ui (`close-ok` exempts a line)
//   node scripts/check-tools-ui.mjs               + the pages in headless Chrome (dev servers from `npm run dev`, the
//                                                 test user's session from `npm run dev:test-user`):
//                                                 - 1440 px, fine pointer: no visible text under the caption size
//                                                 - 390 px, touch (coarse pointer): the same + every button / field /
//                                                   nav entry at least 44 px high (the touch minimum)
//                                                 - both widths, tools AND the viewer's main pages: every visible
//                                                   single-line control (button, .btn, select, text-like input,
//                                                   .sc-control, .sc-chip-control) exactly --control-height high (±1 px; DESIGN.md
//                                                   "One control height"). Exempt: textareas, switches, checkboxes /
//                                                   radios, lettering, and an element (or ancestor) with
//                                                   data-control-exempt="<reason>" (cards, canvas handles, big tiles)
//   --only=<name>  --list  --verbose  --browser-only (static findings don't stop the browser run)
//   SCUTOONS_CREATOR / SCUTOONS_ADMIN / SCUTOONS_EMBASSY / SCUTOONS_VIEWER for other origins
//
// Exempt: comic lettering (Bangers, or text inside a slice: container-type inline-size, sized in cqw) and a CSS line
// carrying the comment `px-ok` (with the reason).
import { spawn } from 'node:child_process';
import { existsSync, mkdtempSync, readFileSync, readdirSync, rmSync, statSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { local } from './dev/ports.mjs';

const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const args = process.argv.slice(2);
const flag = (n) => args.includes(n);
const opt = (n) => args.find((a) => a.startsWith(`--${n}=`))?.split('=')[1];
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

// ── static: the CSS of the tools ────────────────────────────────────────────────────────────────────────────────
const DIRS = ['creator/src', 'admin/src', 'embassy/src', 'shared/ui/src', 'shared/auth/src', 'shared/media/src', 'shared/tokens/tools.css'];
function walk(p) {
	const abs = join(ROOT, p);
	if (!existsSync(abs)) return [];
	if (statSync(abs).isFile()) return [abs];
	return readdirSync(abs, { withFileTypes: true }).flatMap((e) =>
		e.isDirectory() ? (e.name === 'paraglide' ? [] : walk(join(p, e.name))) : /\.(svelte|css)$/.test(e.name) ? [join(abs, e.name)] : []
	);
}
const problems = [];
const bolds = [];
const heights = [];
// one control height (owner 2026-10-02): CSS of every app (tools + viewer + shop + shared)
const CONTROL_DIRS = [...DIRS, 'viewer/src', 'viewer/static/css', 'shop/src', 'shared/reader/src', 'shared/tokens/base.css'];
// a selector whose last compound names a single-line control
const CONTROL_SEL = /(^|[\s>+~(,])(button|select|input(?![^\s,]*type=['"]?(checkbox|radio|range|color|file|hidden|image))|[\w-]*\.btn(-[\w-]+)?|[\w-]*\.sc-(button|icon-button|palette-trigger|control|chip-control)|[\w-]*\[role=['"]?button['"]?\])(?=$|[\s.:\[#),>+~])[^\s>+~]*\s*$/;
const isZero = (v) => /^(0|0?\.0+)(px|rem|em)?$/.test(v) || v === 'auto';
function controlBypass(prop, value) {
	value = value.replace(/!important/, '').trim();
	if (/--control-height/.test(value)) return false;
	if (prop === 'height' || prop === 'min-height') return !/^(auto|none|unset|initial|inherit|0)$/.test(value);
	const parts = value.split(/\s+(?![^(]*\))/);
	if (prop === 'padding') return !isZero(parts[0]) || (parts.length >= 3 && !isZero(parts[2]));
	if (/^padding-(top|bottom)$/.test(prop)) return !isZero(value);
	if (prop === 'padding-block') return parts.some((p) => !isZero(p));
	return false;
}
// a small CSS walk: selector stack (nesting, @media), declarations of the innermost rule
function scanControls(css, name, lineOf) {
	const stack = [];
	let buf = '';
	let pos = 0;
	const strip = css.replace(/\/\*[\s\S]*?\*\//g, (m) => m.replace(/[^\n]/g, (c) => (/[{};]/.test(c) ? ' ' : c)));
	for (let i = 0; i < strip.length; i++) {
		const c = strip[i];
		if (c === '{') {
			stack.push(buf.trim());
			buf = '';
			pos = i + 1;
		} else if (c === '}') {
			check(buf, pos);
			stack.pop();
			buf = '';
			pos = i + 1;
		} else if (c === ';') {
			check(buf, pos);
			buf = '';
			pos = i + 1;
		} else buf += c;
	}
	function check(decl, at) {
		const m = decl.match(/^\s*([a-z-]+)\s*:\s*([\s\S]+)$/);
		if (!m || !stack.length) return;
		const sel = stack[stack.length - 1];
		if (sel.startsWith('@')) return;
		const selectors = sel.replace(/:global\(([^()]*(?:\([^()]*\))*[^()]*)\)/g, '$1').replace(/:(where|is|not)\(/g, '(').split(/,(?![^(]*\))/);
		if (!selectors.some((x) => CONTROL_SEL.test(x.trim()))) return;
		if (/textarea/.test(sel) && !selectors.some((x) => CONTROL_SEL.test(x.trim()) && !/textarea/.test(x))) return;
		if (!controlBypass(m[1], m[2])) return;
		const lineNo = lineOf(at + decl.search(/\S/));
		const line = css.split('\n')[lineNo.local - 1] ?? '';
		if (/control-ok/.test(line)) return;
		heights.push(`${name}:${lineNo.abs}: ${sel.replace(/\s+/g, ' ').slice(0, 60)} { ${m[1]}: ${m[2].trim()} }`);
	}
}
for (const file of [...new Set(CONTROL_DIRS.flatMap(walk))]) {
	const text = readFileSync(file, 'utf8');
	const name = relative(ROOT, file).replace(/\\/g, '/');
	if (file.endsWith('.css')) {
		scanControls(text, name, (at) => { const n = text.slice(0, at).split('\n').length; return { local: n, abs: n }; });
		continue;
	}
	for (const m of text.matchAll(/(<style[^>]*>)([\s\S]*?)<\/style>/g)) {
		const start = m.index + m[1].length;
		const css = m[2];
		scanControls(css, name, (at) => { const n = css.slice(0, at).split('\n').length; return { local: n, abs: text.slice(0, start).split('\n').length + n - 1 }; });
	}
}
// close cross (owner 2026-10-02, DESIGN.md "Close cross"): every close / remove cross is the one drawn CloseIcon
// (@scutoons/ui) – no font glyph (× ✕ ✖ &times;) as a control's content, no Feather "x" (a line with `close-ok` is exempt)
const crosses = [];
const GLYPH_CROSS = /<Icon\b[^>]*\bname=["']x["']|<(button|a|Button)\b[^>]*>\s*(×|✕|✖|&times;)|\s(×|✕|✖|&times;)\s*<\/(button|a|Button)>|>(×|✕|✖|&times;)<\/(button|a|Button)>/;
for (const file of [...new Set(CONTROL_DIRS.flatMap(walk))].filter((f) => f.endsWith('.svelte'))) {
	const name = relative(ROOT, file).replace(/\\/g, '/');
	readFileSync(file, 'utf8').replace(/<style[^>]*>[\s\S]*?<\/style>/g, (m) => m.replace(/[^\n]/g, ' ')).split('\n').forEach((line, i) => {
		if (GLYPH_CROSS.test(line) && !/close-ok/.test(line)) crosses.push(`${name}:${i + 1}: ${line.trim().slice(0, 120)}`);
	});
}
const BOLD_CSS = /font-weight:\s*(?:[6-9]00|bold|bolder|var\(--font-weight-bold\))/;
const BOLD_TAG = /<b[\s>]|<strong(?![^>]*\bsc-emphasis\b)[\s>]/;
for (const file of DIRS.flatMap(walk)) {
	const text = readFileSync(file, 'utf8');
	const name = relative(ROOT, file).replace(/\\/g, '/');
	const style = file.endsWith('.css') ? text : (text.match(/<style[^>]*>[\s\S]*?<\/style>/g) ?? []).join('\n');
	const offset = file.endsWith('.css') ? 0 : text.indexOf(style.slice(0, 40));
	style.split('\n').forEach((line, i) => {
		const lineNo = offset >= 0 ? text.slice(0, offset).split('\n').length + i : i + 1;
		if (BOLD_CSS.test(line) && !/bold-ok/.test(line)) bolds.push(`${name}:${lineNo}: ${line.trim()}`);
		if (/px-ok/.test(line)) return;
		const px = line.match(/font-size:\s*([\d.]+)px/);
		const em = line.match(/font-size:\s*(0?\.\d+|0)(em|rem)\b/);
		if (px || (em && Number(em[1]) < 0.75)) problems.push(`${name}:${lineNo}: ${line.trim()}`);
	});
	if (file.endsWith('.svelte')) {
		// markup only (the style block blanked, line numbers kept)
		text.replace(/<style[^>]*>[\s\S]*?<\/style>/g, (m) => m.replace(/[^\n]/g, ' ')).split('\n').forEach((line, i) => {
			if (BOLD_TAG.test(line) && !/bold-ok/.test(line)) bolds.push(`${name}:${i + 1}: ${line.trim().slice(0, 120)}`);
		});
	}
}
if (problems.length) console.error(`tools CSS: ${problems.length} font sizes outside the type scale (use --type-* tokens; DESIGN.md "Brutalist tools"):\n  ${problems.join('\n  ')}`);
if (bolds.length) console.error(`tools: ${bolds.length} bold (owner 2026-10-02 "only use bold very rarely": regular weight; only the page title h1 and <strong class="sc-emphasis"> are bold; DESIGN.md "Brutalist tools"):\n  ${bolds.join('\n  ')}`);
if (heights.length) console.error(`controls: ${heights.length} heights / vertical paddings that bypass --control-height (owner 2026-10-02: every single-line control is exactly --control-height; padding-block 0, the height from the token; DESIGN.md "One control height"; a line with \`control-ok\` + the reason is exempt):\n  ${heights.join('\n  ')}`);
if (crosses.length) console.error(`close crosses: ${crosses.length} font glyphs / Feather "x" (owner 2026-10-02: one drawn cross, @scutoons/ui CloseIcon / CloseButton, as high as the headline, 2px – 1px on thin-line UI; DESIGN.md "Close cross"; a line with \`close-ok\` + the reason is exempt):\n  ${crosses.join('\n  ')}`);
if ((problems.length || bolds.length || heights.length || crosses.length) && !flag('--browser-only')) process.exit(1);
console.log('tools: no px font sizes, nothing under caption, no bold beyond the page title / sc-emphasis – ok');
console.log('controls: no height / vertical padding bypassing --control-height – ok');
console.log('close crosses: the one drawn CloseIcon, no glyphs – ok');
if (flag('--static')) process.exit(0);

// ── browser ─────────────────────────────────────────────────────────────────────────────────────────────────────
const CREATOR = (process.env.SCUTOONS_CREATOR ?? local('creator')).replace(/\/$/, '');
const ADMIN = (process.env.SCUTOONS_ADMIN ?? local('admin')).replace(/\/$/, '');
const EMBASSY = (process.env.SCUTOONS_EMBASSY ?? local('embassy')).replace(/\/$/, '');
const VIEWER = (process.env.SCUTOONS_VIEWER ?? local('viewer')).replace(/\/$/, '');
const PAGES = [
	['creator-home', `${CREATOR}/`],
	['creator-toon', `${CREATOR}/catpope`],
	['creator-season', `${CREATOR}/catpope/season/1`],
	['creator-chapter-meta', `${CREATOR}/catpope/chapter/1`],
	['creator-chapter-audio', `${CREATOR}/catpope/chapter/1?section=audio`],
	['creator-chapter-slice', `${CREATOR}/catpope/chapter/1?group=1&slice=3`],
	['creator-cast', `${CREATOR}/catpope/cast`],
	['creator-credits', `${CREATOR}/catpope/credits`],
	['creator-script', `${CREATOR}/catpope/script`],
	['creator-toon-settings', `${CREATOR}/catpope/toon`],
	['creator-characters', `${CREATOR}/characters`],
	['creator-moods', `${CREATOR}/characters?tab=moods`],
	['creator-wanted', `${CREATOR}/characters?tab=wanted`],
	['creator-guide', `${CREATOR}/guide`],
	['admin-dashboard', `${ADMIN}/`],
	['admin-codes', `${ADMIN}/codes`],
	['admin-code-new', `${ADMIN}/codes/new`],
	['admin-batches', `${ADMIN}/batches`],
	['admin-users', `${ADMIN}/users`],
	['admin-events', `${ADMIN}/events`],
	['admin-content', `${ADMIN}/content`],
	['admin-catalog', `${ADMIN}/catalog`],
	['admin-homepage', `${ADMIN}/homepage`],
	['admin-settings', `${ADMIN}/settings`],
	['embassy-today', `${EMBASSY}/`],
	['embassy-leads', `${EMBASSY}/leads`],
	['embassy-org-new', `${EMBASSY}/orgs/new`],
	// the viewer: only the control-height rule (it keeps the reader's own type, nav-micro included)
	['viewer-home', `${VIEWER}/`, 'viewer'],
	['viewer-toon', `${VIEWER}/catpope`, 'viewer'],
	['viewer-chapter', `${VIEWER}/catpope/chapter/1`, 'viewer'],
	['viewer-account', `${VIEWER}/account`, 'viewer'],
	['viewer-inventory', `${VIEWER}/catpope?inventory=open`, 'viewer'],
	['viewer-events', `${VIEWER}/events`, 'viewer'],
	['viewer-download', `${VIEWER}/catpope/download`, 'viewer']
].filter(([name]) => !opt('only') || name.includes(opt('only')));
if (flag('--list')) {
	for (const [name, url] of PAGES) console.log(name.padEnd(24), url);
	process.exit(0);
}
const sessionFile = join(ROOT, 'logs/dev/test-user.json');
if (!existsSync(sessionFile)) {
	console.error('no test-user session: run `npm run dev:test-user` first');
	process.exit(2);
}
const token = JSON.parse(readFileSync(sessionFile, 'utf8')).token;

const CHROME = process.env.CHROME || ['C:/Program Files/Google/Chrome/Application/chrome.exe', 'C:/Program Files (x86)/Google/Chrome/Application/chrome.exe', 'C:/Program Files (x86)/Microsoft/Edge/Application/msedge.exe', '/Applications/Google Chrome.app/Contents/MacOS/Google Chrome', '/usr/bin/google-chrome'].find((p) => existsSync(p));
if (!CHROME) { console.error('Chrome not found (set CHROME)'); process.exit(2); }
const port = 9800 + Math.floor(Math.random() * 150);
const profile = mkdtempSync(join(tmpdir(), 'check-tools-ui-'));
const chrome = spawn(CHROME, ['--headless=new', `--remote-debugging-port=${port}`, `--user-data-dir=${profile}`, '--no-first-run', '--no-default-browser-check', '--disable-extensions', '--mute-audio', 'about:blank'], { stdio: 'ignore' });
async function done(code) {
	const exited = chrome.exitCode !== null ? Promise.resolve() : new Promise((r) => chrome.once('exit', r));
	chrome.kill();
	await Promise.race([exited, sleep(5000)]);
	try { rmSync(profile, { recursive: true, force: true, maxRetries: 10, retryDelay: 200 }); } catch {}
	process.exit(code);
}
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
const evaluate = async (expression) => { const r = await send('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true }); return r.result?.result?.value; };
await send('Page.enable'); await send('Runtime.enable'); await send('Network.enable');
await send('Page.addScriptToEvaluateOnNewDocument', { source: `try { localStorage.setItem('scutoons_session', ${JSON.stringify(token)}); } catch {}` });
async function idle(quietMs = 700, maxMs = 8000) { const t0 = Date.now(); while (Date.now() - t0 < maxMs) { if (Date.now() - lastNet > quietMs) return; await sleep(100); } }

const INSPECT = `(async () => {
	await document.fonts.ready;
	const root = parseFloat(getComputedStyle(document.documentElement).fontSize);
	const caption = root * 0.75 - 0.05;
	const coarse = matchMedia('(pointer: coarse)').matches;
	const probe = document.createElement('div');
	probe.style.cssText = 'position:absolute;visibility:hidden;box-sizing:border-box;height:var(--control-height)';
	document.body.append(probe);
	const controlHeight = probe.getBoundingClientRect().height;
	probe.remove();
	const lettering = (el) => { for (let n = el; n && n !== document.body; n = n.parentElement) { const cs = getComputedStyle(n); if (/^\\s*["']?Bangers/.test(cs.fontFamily) || /inline-size|size/.test(cs.containerType)) return true; } return false; };
	const describe = (el) => el.tagName.toLowerCase() + ([...el.classList].filter((c) => !c.startsWith('svelte-')).map((c) => '.' + c).join('')) + ' "' + (el.value || el.getAttribute('aria-label') || el.textContent || '').trim().replace(/\\s+/g, ' ').slice(0, 30) + '"';
	// one control height (owner 2026-10-02): the single-line controls
	const LINE_INPUTS = ['', 'text', 'email', 'number', 'date', 'search', 'password', 'url', 'tel', 'time', 'datetime-local', 'month', 'week'];
	const single = (el) => {
		if (el.closest('[data-control-exempt]')) return false;
		if (el.getAttribute('role') === 'switch') return false;
		if (el.tagName === 'BUTTON') return true;
		if (el.tagName === 'SELECT') return !el.multiple && !(el.size > 1);
		if (el.tagName === 'INPUT') return LINE_INPUTS.includes((el.getAttribute('type') || '').toLowerCase());
		if (['sc-control', 'sc-chip-control', 'sc-button', 'btn'].some((c) => el.classList.contains(c))) return true;
		return el.getAttribute('role') === 'button';
	};
	const small = [], tiny = [], off = [];
	for (const el of document.body.querySelectorAll('*')) {
		if (/^(SCRIPT|STYLE|svg|path|g|title)$/.test(el.tagName)) continue;
		if (!el.getClientRects().length) continue;
		const cs = getComputedStyle(el);
		if (cs.visibility === 'hidden' || cs.display === 'none' || Number(cs.opacity) === 0) continue;
		const control = /^(INPUT|SELECT|TEXTAREA|BUTTON)$/.test(el.tagName) || el.classList.contains('sc-nav-item');
		const own = [...el.childNodes].some((n) => n.nodeType === 3 && n.textContent.trim());
		if (CAPTION && (own || control) && parseFloat(cs.fontSize) < caption && !lettering(el)) small.push(cs.fontSize + ' ' + describe(el));
		if (coarse && control && !['checkbox', 'radio', 'range', 'color', 'file', 'hidden'].includes(el.type)) {
			const r = el.getBoundingClientRect();
			if (r.height < 43.5 && r.width > 0 && !lettering(el)) tiny.push(Math.round(r.height) + 'px ' + describe(el));
		}
		if (single(el) && !lettering(el)) {
			const r = el.getBoundingClientRect();
			if (r.width > 0 && Math.abs(r.height - controlHeight) > 1) off.push(Math.round(r.height * 10) / 10 + 'px ' + describe(el));
		}
	}
	return { root, coarse, controlHeight, small, tiny, off };
})()`;

let failed = 0;
// desktop first: once touch emulation was on, Chrome keeps reporting a coarse pointer for the tab
for (const width of [1440, 390]) {
	for (const [name, url, kind] of PAGES) {
		await send('Emulation.setDeviceMetricsOverride', { width, height: width < 768 ? 844 : 900, deviceScaleFactor: 1, mobile: width < 768 });
		await send('Emulation.setTouchEmulationEnabled', { enabled: width < 768, maxTouchPoints: width < 768 ? 5 : 0 });
		await send('Page.navigate', { url });
		await sleep(600); await idle(); await sleep(1200);
		const r = await evaluate(INSPECT.replace('CAPTION &&', kind === 'viewer' ? 'false &&' : 'true &&'));
		if (!r) { console.log(`✗ ${name} @${width}: page did not answer`); failed++; continue; }
		const bad = r.small.length + r.tiny.length + r.off.length;
		if (bad) failed++;
		console.log(`${bad ? '✗' : '✓'} ${name.padEnd(24)} @${width} root ${r.root}px${r.coarse ? ' coarse' : ''}: ${r.small.length} texts < caption, ${r.tiny.length} controls < 44px, ${r.off.length} controls ≠ ${r.controlHeight}px`);
		if (bad && (flag('--verbose') || bad <= 6)) for (const line of [...r.small, ...r.tiny, ...r.off].slice(0, flag('--verbose') ? 200 : 12)) console.log('    ' + line);
	}
}
console.log(failed ? `tools UI: ${failed} page views with problems` : 'tools UI: every page – ok');
await done(failed ? 1 : 0);
