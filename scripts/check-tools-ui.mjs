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
//                                                 CloseIcon / CloseButton of @scutoons/ui (`close-ok` exempts a line);
//                                                 field grid (owner 2026-10-03): no local `.g2/.g3/.g4` copy (selector
//                                                 or `class="grid gN"`) and no local gap on `.sc-fields` (tools.css)
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
import { existsSync, mkdtempSync, readFileSync, readdirSync, rmSync, statSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, relative, resolve, sep } from 'node:path';
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
// side bars (owner 2026-11-21, RULES H10: "i hate those left border indicators. as mentioned many time."): no thick
// left / start rule as a state or accent mark – a state is the frame's colour, a dot or coloured words. A border-left or
// border-inline-start of 3px / 0.2em / 0.2rem or more is a side bar (a line with `sidebar-ok` + the reason is exempt)
const sideBars = [];
const SIDE_BAR = /border-(left|inline-start)(-width)?\s*:\s*(?:[^;]*\s)?((?:[3-9]|\d{2,})px|(?:0?\.[2-9]\d*|[1-9][\d.]*)r?em)\b/;
// a left / start edge coloured by state (error, warning, a level) is the same side bar, whatever its width
const SIDE_STATE = /border-(left|inline-start)(-color)?\s*:[^;]*(--state-|--error\b|--warn\b|--border-width-focus)/;
for (const file of [...new Set([...CONTROL_DIRS, 'shared/catalog/src', 'shared/rig-designer/src'].flatMap(walk))]) {
	const name = relative(ROOT, file).replace(/\\/g, '/');
	const text = readFileSync(file, 'utf8');
	const css = file.endsWith('.css') ? text : text.replace(/<style[^>]*>([\s\S]*?)<\/style>|[^\n]/g, (m, inner) => (inner !== undefined ? m : ' '));
	css.split('\n').forEach((line, i) => {
		if ((SIDE_BAR.test(line) || SIDE_STATE.test(line)) && !/sidebar-ok/.test(line)) sideBars.push(`${name}:${i + 1}: ${line.trim().slice(0, 120)}`);
	});
}
// blur (owner 2026-11-24, RULES H15 [editor-style]): blur exists on the overlay level only – the kit's Overlay
// (shared/ui/src/editor/overlay.svelte); no panel, menu, popover or page of the tools blurs what lies behind it
const blurs = [];
const OVERLAY = 'shared/ui/src/editor/overlay.svelte';
for (const file of DIRS.flatMap(walk)) {
	const name = relative(ROOT, file).split(sep).join('/');
	if (name === OVERLAY) continue;
	readFileSync(file, 'utf8').split('\n').forEach((line, i) => {
		if (/backdrop-filter\s*:\s*(?!none\b)/.test(line)) blurs.push(`${name}:${i + 1}: ${line.trim().slice(0, 120)}`);
	});
}
// field grid (owner 2026-10-03, COMPONENTS.md 4.4.2): one tools.css `.sc-fields[data-cols][data-density]` – no local
// `.g2/.g3/.g4` copies (selector or `class="grid gN"` markup), no local field-grid class (`.grid`, `.two`, `.pair`,
// `.form-grid`, `.meta-grid` with display: grid; a line with `grid-ok` + the reason is exempt, e.g. a thumbnail grid)
// and no local gap on `.sc-fields` (the density sets it)
const fieldGrids = [];
const FIELD_GRID_CLASS = /\.(grid|two|pair|form-grid|meta-grid)(?![\w-])[^\s>+~,]*\s*\)?\s*$/;
for (const file of DIRS.flatMap(walk).filter((f) => !f.endsWith('tools.css'))) {
	const text = readFileSync(file, 'utf8');
	const name = relative(ROOT, file).replace(/\\/g, '/');
	const lineAt = (i) => text.slice(0, i).split('\n').length;
	const styles = file.endsWith('.css') ? [{ css: text, at: 0 }] : [...text.matchAll(/(<style[^>]*>)([\s\S]*?)<\/style>/g)].map((m) => ({ css: m[2], at: m.index + m[1].length }));
	for (const { css, at } of styles) {
		const plain = css.replace(/\/\*[\s\S]*?\*\//g, (c) => c.replace(/[^\n]/g, ' '));
		for (const m of plain.matchAll(/([^{}]+)\{([^{}]*)\}/g)) {
			const sel = m[1].trim();
			const line = lineAt(at + m.index + m[1].search(/\S/));
			// `grid-ok` anywhere in the rule (the comments are blanked in `plain`, same offsets in `css`)
			if (/grid-ok/.test(css.slice(m.index, m.index + m[0].length))) continue;
			// a local status chip / badge filled from --badge-* (owner 2026-10-03, COMPONENTS.md step 3): tools.css .sc-chip
			// (.is-*), .sc-chip-control, .sc-mark – @scutoons/ui's own Badge is the one shared component
			if (/background(-color)?:\s*var\(--badge-/.test(m[2]) && !/\.sc-/.test(sel) && !name.endsWith('shared/ui/src/badge.svelte'))
				fieldGrids.push(`${name}:${line}: ${sel.replace(/\s+/g, ' ').slice(0, 60)} (a local status chip: tools.css .sc-chip.is-* / .sc-chip-control / .sc-mark)`);
			// a restyled fold row (owner 2026-10-03, step 4: tools.css styles every summary; `fold-ok` + reason for a fold that is a control, e.g. a dropdown)
			if (sel.split(/,(?![^(]*\))/).some((x) => /(^|[\s>])summary(\[|:(?!hover)|$)/.test(x.trim()) || /\]\s*>?\s*summary$/.test(x.trim())) && /(^|[;\s])(font-size|text-transform|letter-spacing|cursor|margin(-bottom)?)\s*:/.test(m[2]) && !/fold-ok/.test(css.slice(m.index, m.index + m[0].length)))
				fieldGrids.push(`${name}:${line}: ${sel.replace(/\s+/g, ' ').slice(0, 60)} (a local fold style: tools.css styles summary, or \`fold-ok\` + the reason)`);
			// a local segmented control (owner 2026-10-03, step 6): @scutoons/ui Segmented / tools.css .sc-seg
			if (sel.split(/,(?![^(]*\))/).some((x) => /\.(seg|segments|segmented)(?![\w-])[^\s>+~]*$/.test(x.trim())) && /display\s*:/.test(m[2]))
				fieldGrids.push(`${name}:${line}: ${sel.replace(/\s+/g, ' ').slice(0, 60)} (a local segmented control: @scutoons/ui Segmented / tools.css .sc-seg)`);
			if (/\.g\d\b/.test(sel)) fieldGrids.push(`${name}:${line}: ${sel.replace(/\s+/g, ' ').slice(0, 60)} (a local field-grid copy)`);
			else if (/display:\s*grid/.test(m[2]) && sel.split(/,(?![^(]*\))/).some((x) => FIELD_GRID_CLASS.test(x.trim())))
				fieldGrids.push(`${name}:${line}: ${sel.replace(/\s+/g, ' ').slice(0, 60)} (a local field grid: tools.css .sc-fields, or \`grid-ok\` + the reason)`);
			else if (/\.sc-fields\b/.test(sel) && /(^|[;\s])(row-|column-)?gap\s*:/.test(m[2])) fieldGrids.push(`${name}:${line}: ${sel.replace(/\s+/g, ' ').slice(0, 60)} { gap } (the density sets the gaps)`);
		}
	}
	if (file.endsWith('.svelte'))
		text.replace(/<style[^>]*>[\s\S]*?<\/style>/g, (s) => s.replace(/[^\n]/g, ' ')).split('\n').forEach((l, i) => {
			if (/class="[^"]*\bg\d\b/.test(l)) fieldGrids.push(`${name}:${i + 1}: ${l.trim().slice(0, 100)}`);
			// a fold row with its own type class (tools.css styles every summary – COMPONENTS.md step 4)
			if (/<summary\b[^>]*\bclass="[^"]*\bsc-(label|caption)\b/.test(l)) fieldGrids.push(`${name}:${i + 1}: ${l.trim().slice(0, 100)} (a restyled fold row: tools.css styles summary)`);
			// an app component imported by a relative path (COMPONENTS.md 4.2.6: `$lib/...`; the shared packages keep theirs)
			if (/^(creator|admin|embassy)\/src\/lib\//.test(name) && /from '\.\.?\/[^']+\.svelte'/.test(l)) fieldGrids.push(`${name}:${i + 1}: ${l.trim().slice(0, 100)} (import components via $lib/…)`);
			// hand-built tabs (COMPONENTS.md step 14b): @scutoons/ui Tabs (role="tab", arrow keys); a floating panel's tabs are the editor kit's
			if ((/role="tablist"/.test(l) || /class="tabs?[\s"]/.test(l)) && !/shared\/ui\/src\/(tabs|editor\/floating-panel)\.svelte$/.test(name))
				fieldGrids.push(`${name}:${i + 1}: ${l.trim().slice(0, 100)} (hand-built tabs: @scutoons/ui Tabs)`);
		});
}
// colours (owner 2026-11-24, RULES H11 [editor-style]: "stick to the color schema of the deck editor … there is no grey
// backgrounds in our concept"): the tools' CSS (creator, the editor kit, tools.css) takes every colour from the semantic
// --tool-* tokens (shared/tokens/tool-colours.css, on editor.css generated from kit/tokens.toml) –
//   - no literal colour (hex, rgb()/hsl()/…, a named colour) in a declaration; the token files are the only place for them
//   - no raw editor colour (--editor-surface, --editor-text …) and no reader ground (--paper, --ink, --surface-panel):
//     the grey surface is a ground only through --tool-panel / --tool-popover / --tool-head, which say what the box is
//   - a see-through ground (a panel, a menu, a popover, a tooltip, an overlay box) carries the halo: text-shadow: var(--tool-halo)
// COLOUR_ALLOW: file + text of the declaration + why, for what truly must stay (keep it short)
const COLOUR_DIRS = ['creator/src', 'shared/ui/src/editor', 'shared/rig-designer/src', 'shared/tokens/tools.css'];
const COLOUR_ALLOW = [
	// [file, a substring of the declaration, the reason]
	// empty after #831's sweep; url() contents (the select chevron's data URI) are not read as colours
];
const COLOUR_PROPS = /^(color|background(-color|-image)?|border(-(top|right|bottom|left|block|inline)(-(start|end))?)?(-color)?|outline(-color)?|box-shadow|text-shadow|fill|stroke|caret-color|accent-color|text-decoration(-color)?|column-rule(-color)?|scrollbar-color|filter|stop-color|flood-color|lighting-color|--[a-z0-9-]+)$/;
const LITERAL = /#[0-9a-fA-F]{3,8}\b|\b(rgba?|hsla?|hwb|lab|lch|oklab|oklch)\(|(^|[\s,(])(white|black|gr[ae]y|silver|red|green|blue|yellow|orange|purple|pink|magenta|cyan|lime|navy|teal|maroon|olive|aqua|fuchsia)(?=$|[\s,)])/;
const RAW_COLOUR = /var\(--(editor-(stage|surface|surface-head|line|text|muted|control|hover|pressed|pressed-text|selected|within|accent|primary|primary-line|primary-ink|bright|selection|danger|go|warn|stop|panel-ground|halo)|paper|ink|surface-panel)\b/;
// no ground is grey (owner 2026-11-24, kit/README.md "Always"): a ground is see-through. Opaque greys as a fill: the line,
// muted, bright or text colour at full strength (a dot or a 1–2px line is a mark, not a ground), a colour mixed into
// another opaque one (color-mix without `transparent`), a glow that fades into the stage (a gradient to --tool-ground)
const GROUND_PROP = /^background(-color|-image)?$/;
const OPAQUE_GREY = /var\(--tool-(line|muted|bright|text)\)/;
const MARK_RULE = /border-radius\s*:\s*50%|(^|;|\s)(width|height)\s*:\s*(1px|2px|var\(--editor-edge\)|0\.125rem|calc\(2 \* var\(--editor-edge\)\))\s*(;|$)/;
function greyGround(value, body) {
	if (/-gradient\((?:[^()]|\([^()]*\))*var\(--tool-ground\)/.test(value)) return 'a glow into the stage is a grey ground';
	if (/color-mix\(/.test(value) && !/transparent/.test(value)) return 'an opaque colour-mix – mix into transparent';
	const solid = value.replace(/color-mix\((?:[^()]|\([^()]*\))*\)/g, '');
	if (OPAQUE_GREY.test(solid) && !/gradient\(/.test(solid) && !MARK_RULE.test(body)) return 'an opaque grey fill – a see-through ground (--tool-control / hover / selected / panel / popover, a kit state)';
	return null;
}
// every --tool-* and kit state a tool reads is defined (tool-colours.css, tools.css, kit.css): a deleted alias such as
// --tool-accent or a mistyped state would silently paint nothing
const DEFINED = new Set(['shared/tokens/tool-colours.css', 'shared/tokens/tools.css', 'shared/tokens/kit.css'].flatMap((f) => [...readFileSync(join(ROOT, f), 'utf8').matchAll(/(--(?:tool|kit)-[a-z0-9-]+)\s*:/g)].map((m) => m[1])));
const STATE_VAR = /var\((--kit-[a-z0-9-]+-(text|ground|border|ring|knob|mark|opacity))\b|var\((--tool-[a-z0-9-]+)\b/g;
// literal colours: also in the shared media and sign-in parts the tools use, and in inline styles of the tools' markup
// (shared/ui outside its editor kit also serves the reader apps, which keep their own scrims and the colour picker its hues)
const LITERAL_ONLY_DIRS = ['shared/media/src', 'shared/auth/src'];
const INLINE_LITERAL = /\bstyle(:[a-z-]+)?=(["'{])[^"'}]*(#[0-9a-fA-F]{3,8}\b|\b(rgba?|hsla?)\()/;
const colours = [];
for (const file of [...new Set([...COLOUR_DIRS, ...LITERAL_ONLY_DIRS].flatMap(walk))]) {
	const name = relative(ROOT, file).replace(/\\/g, '/');
	const text = readFileSync(file, 'utf8');
	const onlyLiterals = !COLOUR_DIRS.some((d) => name === d || name.startsWith(d + '/'));
	if (file.endsWith('.svelte')) {
		text.replace(/<style[^>]*>[\s\S]*?<\/style>/g, (m) => m.replace(/[^\n]/g, ' ')).split('\n').forEach((l, i) => {
			if (INLINE_LITERAL.test(l)) colours.push(`${name}:${i + 1}: ${l.trim().slice(0, 100)} (a literal colour in an inline style: a --tool-* token)`);
		});
	}
	if (onlyLiterals) {
		const styles = file.endsWith('.css') ? [text] : [...text.matchAll(/<style[^>]*>([\s\S]*?)<\/style>/g)].map((m) => m[1]);
		for (const css of styles) {
			const plain = css.replace(/\/\*[\s\S]*?\*\//g, '').replace(/url\((['"]?)[^)]*\1\)/g, 'url()');
			for (const m of plain.matchAll(/([a-z-]+|--[a-z0-9-]+)\s*:\s*([^;{}]+)/g)) {
				if (COLOUR_PROPS.test(m[1]) && /#[0-9a-fA-F]{3,8}\b|\b(rgba?|hsla?|hwb|oklch)\(/.test(m[2])) colours.push(`${name}: ${m[1]}: ${m[2].trim().slice(0, 60)} (a literal colour: a token)`);
			}
		}
		continue;
	}
	for (const m of text.matchAll(STATE_VAR)) {
		const v = m[1] ?? m[3];
		if (!DEFINED.has(v) && !/^--tool-(bar|sidebar|panel)-(height|width)$/.test(v)) colours.push(`${name}:${text.slice(0, m.index).split('\n').length}: ${v} (not a token: tool-colours.css, tools.css and kit.css define none of that name)`);
	}
}
for (const file of [...new Set(COLOUR_DIRS.flatMap(walk))]) {
	const text = readFileSync(file, 'utf8');
	const name = relative(ROOT, file).replace(/\\/g, '/');
	const styles = file.endsWith('.css') ? [{ css: text, at: 0 }] : [...text.matchAll(/(<style[^>]*>)([\s\S]*?)<\/style>/g)].map((m) => ({ css: m[2], at: m.index + m[1].length }));
	for (const { css, at } of styles) {
		const plain = css.replace(/\/\*[\s\S]*?\*\//g, (c) => c.replace(/[^\n]/g, ' '));
		for (const rule of plain.matchAll(/([^{}]+)\{([^{}]*)\}/g)) {
			const sel = rule[1].trim().replace(/\s+/g, ' ').slice(0, 60);
			const body = rule[2];
			let offset = at + rule.index + rule[1].length + 1;
			for (const decl of body.split(';')) {
				const m = decl.match(/^\s*([a-z-]+|--[a-z0-9-]+)\s*:\s*([\s\S]+)$/);
				const where = offset + decl.search(/\S/);
				offset += decl.length + 1;
				if (!m || !COLOUR_PROPS.test(m[1])) continue;
				const value = m[2].replace(/url\((['"]?)[^)]*\1\)/g, 'url()').trim();
				const line = text.slice(0, where).split('\n').length;
				if (COLOUR_ALLOW.some(([f, s]) => f === name && decl.includes(s))) continue;
				const say = `${name}:${line}: ${sel} { ${m[1]}: ${m[2].trim().replace(/\s+/g, ' ').slice(0, 70)} }`;
				if (LITERAL.test(value)) colours.push(`${say} (a literal colour: a --tool-* token)`);
				else if (RAW_COLOUR.test(value)) colours.push(`${say} (a raw editor / reader colour: the --tool-* token – a grey ground only as --tool-panel / --tool-popover / --tool-head)`);
				else if (GROUND_PROP.test(m[1]) && greyGround(value, body)) colours.push(`${say} (${greyGround(value, body)})`);
			}
			if (/background(-color)?\s*:[^;]*var\(--(tool-panel|tool-popover|kit-menu-rest-ground|kit-overlay-box-ground)\)/.test(body) && !/text-shadow\s*:\s*var\(--tool-halo\)/.test(body)) {
				const line = text.slice(0, at + rule.index + rule[1].search(/\S/)).split('\n').length;
				colours.push(`${name}:${line}: ${sel} (a see-through panel / popover ground without its halo: text-shadow: var(--tool-halo), as the deck's ink_ring)`);
			}
		}
	}
}
// one UI kit (owner 2026-11-24, RULES H12 [editor-style]: "Deck editor is the source of truth and we need a unique ui kit
// with recurring components"): a button, pill, tab, menu line or chip of the creator is a @scutoons/ui/editor component
// (Button, IconButton, Pill, PillTab(s), Menu, MenuItem …) – no border-radius / padding / height of its own on a
// button-like selector in creator/src. The shell (the bar, menus, panels, status bar, Home) and the kit page are clean;
// the area pages not on the kit yet are counted per file in KIT_BASELINE, which only shrinks (growth needs
// `Owner-approved-baseline:`, RULES A7). A line carrying `kit-ok` + the reason is exempt.
// `--kit-baseline` rewrites the baseline from what is there now (only after files got cleaner).
const KIT_BASELINE_FILE = join(ROOT, 'scripts/tools-kit-baseline.json');
const KIT_CLEAN = [/^creator\/src\/lib\/components\/shell\//, /^creator\/src\/lib\/components\/page\/ui-kit/, /^creator\/src\/routes\/\(app\)\/guide\//];
// the last compound of a selector names a button-like thing: a button, a role that is one, or a class saying so
const KIT_SEL = /(^|[\s>+~])(button|[\w-]*\[role=['"]?(button|tab|menuitem\w*|switch)['"]?\]|[\w-]*\.(btn|button|icon-button|round|pill|pills|tab|tabs|tab-button|chip|chips|seg|segment|menu-item|menuitem)(?![\w-]))[^\s>+~]*$/;
const KIT_PROP = /^(border(-(top|bottom)-(left|right)|-(start|end)-(start|end))?-radius|padding(-(top|right|bottom|left|block|inline)(-(start|end))?)?|height|min-height)$/;
const kitCounts = {};
const kitLines = {};
for (const file of walk('creator/src')) {
	const text = readFileSync(file, 'utf8');
	const name = relative(ROOT, file).replace(/\\/g, '/');
	const styles = file.endsWith('.css') ? [{ css: text, at: 0 }] : [...text.matchAll(/(<style[^>]*>)([\s\S]*?)<\/style>/g)].map((m) => ({ css: m[2], at: m.index + m[1].length }));
	for (const { css, at } of styles) {
		const plain = css.replace(/\/\*[\s\S]*?\*\//g, (c) => c.replace(/[^\n]/g, ' '));
		for (const rule of plain.matchAll(/([^{}]+)\{([^{}]*)\}/g)) {
			const selectors = rule[1].trim().replace(/:global\(([^()]*(?:\([^()]*\))*[^()]*)\)/g, '$1').replace(/:(where|is|not|has)\([^()]*\)/g, '').split(/,(?![^(]*\))/).map((x) => x.trim());
			if (rule[1].trim().startsWith('@') || !selectors.some((x) => KIT_SEL.test(x))) continue;
			let offset = at + rule.index + rule[1].length + 1;
			for (const decl of rule[2].split(';')) {
				const m = decl.match(/^\s*([a-z-]+)\s*:\s*([\s\S]+)$/);
				const where = offset + decl.search(/\S/);
				offset += decl.length + 1;
				if (!m || !KIT_PROP.test(m[1])) continue;
				const lineNo = text.slice(0, where).split('\n').length;
				if (/kit-ok/.test(text.split('\n')[lineNo - 1] ?? '')) continue;
				kitCounts[name] = (kitCounts[name] ?? 0) + 1;
				(kitLines[name] ??= []).push(`${name}:${lineNo}: ${rule[1].trim().replace(/\s+/g, ' ').slice(0, 60)} { ${m[1]}: ${m[2].trim().slice(0, 40)} }`);
			}
		}
	}
}
let kitBaseline = existsSync(KIT_BASELINE_FILE) ? JSON.parse(readFileSync(KIT_BASELINE_FILE, 'utf8')) : {};
if (flag('--kit-baseline')) {
	const next = Object.fromEntries(Object.entries(kitCounts).filter(([f]) => !KIT_CLEAN.some((r) => r.test(f))).sort(([a], [b]) => a.localeCompare(b)));
	const grown = Object.entries(next).filter(([f, n]) => n > (kitBaseline[f] ?? 0));
	if (grown.length && Object.keys(kitBaseline).length) console.warn(`kit baseline grows for ${grown.map(([f]) => f).join(', ')} – needs Owner-approved-baseline: (RULES A7)`);
	writeFileSync(KIT_BASELINE_FILE, JSON.stringify(next, null, '\t') + '\n');
	kitBaseline = next;
	console.log(`kit baseline: ${Object.keys(next).length} files, ${Object.values(next).reduce((a, b) => a + b, 0)} local button / pill / tab rules written`);
}
const kitLocal = [];
for (const [file, n] of Object.entries(kitCounts)) {
	if (KIT_CLEAN.some((r) => r.test(file))) kitLocal.push(...kitLines[file]);
	else if (n > (kitBaseline[file] ?? 0)) kitLocal.push(`${file}: ${n} local button / pill / tab rules, the baseline allows ${kitBaseline[file] ?? 0}:\n    ${kitLines[file].join('\n    ')}`);
}
const kitShrunk = Object.entries(kitBaseline).filter(([f, n]) => (kitCounts[f] ?? 0) < n).map(([f]) => f);
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
if (fieldGrids.length) console.error(`field grids / chips: ${fieldGrids.length} local copies (owner 2026-10-03: one tools.css .sc-fields with data-cols="1|2|3|4|fill|fit" and data-density="page|panel|dense", gaps by the hierarchy level; a special layout sets only its grid-template-columns; agents/COMPONENTS.md 4.4.2):\n  ${fieldGrids.join('\n  ')}`);
if (blurs.length) console.error(`blur: ${blurs.length} outside the overlay level (owner 2026-11-24, RULES H15: blur exists on the overlay level only – the switcher, Search, a panel taken full, the dialogs go through @scutoons/ui/editor Overlay):\n  ${blurs.join('\n  ')}`);
if (sideBars.length) console.error(`side bars: ${sideBars.length} thick left / start rules (owner 2026-11-21, RULES H10: a state is the frame's colour, a dot or coloured words – never a side bar; a line with \`sidebar-ok\` + the reason is exempt):\n  ${sideBars.join('\n  ')}`);
if (colours.length) console.error(`colours: ${colours.length} outside the deck editor's scheme (owner 2026-11-24, RULES H11: every colour through the --tool-* tokens of shared/tokens/tool-colours.css; no literal colours, no grey ground behind content on the stage; an exception goes into COLOUR_ALLOW with its reason):\n  ${colours.join('\n  ')}`);
if (kitLocal.length) console.error(`ui kit: ${kitLocal.length} local button / pill / tab styling outside @scutoons/ui (owner 2026-11-24, RULES H12: one UI kit, the deck editor its source of truth – use the kit's Button / IconButton / Pill / PillTab / Menu / MenuItem / ListRow / Tile; the shell and the kit page are clean, the other files may not grow past scripts/tools-kit-baseline.json; \`kit-ok\` + the reason exempts a line):\n  ${kitLocal.join('\n  ')}`);
if (kitShrunk.length && !flag('--kit-baseline')) console.log(`ui kit: ${kitShrunk.length} files got cleaner than the baseline – shrink it: node scripts/check-tools-ui.mjs --static --kit-baseline`);
if ((problems.length || bolds.length || heights.length || crosses.length || fieldGrids.length || sideBars.length || blurs.length || colours.length || kitLocal.length) && !flag('--browser-only')) process.exit(1);
console.log('tools: no px font sizes, nothing under caption, no bold beyond the page title / sc-emphasis – ok');
console.log('controls: no height / vertical padding bypassing --control-height – ok');
console.log('close crosses: the one drawn CloseIcon, no glyphs – ok');
console.log('field grids / chips: tools.css .sc-fields and .sc-chip, no local field grids, gaps or status chips – ok');
console.log('side bars: no thick left / start rules – ok');
console.log('blur: on the overlay level only – ok');
console.log('colours: the --tool-* tokens only, no literals, no grey grounds – ok');
console.log(`ui kit: the shell on the kit, ${Object.values(kitCounts).reduce((a, b) => a + b, 0)} local button / pill / tab rules left in ${Object.keys(kitCounts).length} area files (the baseline) – ok`);
// the kit's tokens (kit/tokens.toml → shared/tokens/editor.css): a stale or hand-edited editor.css fails here too, so
// check:tools-css covers it without a second script in package.json
{
	const { spawnSync } = await import('node:child_process');
	const kit = spawnSync(process.execPath, ['--experimental-transform-types', '--no-warnings', join(ROOT, 'shared/tokens/scripts/editor-tokens.ts'), '--check'], { stdio: 'inherit' });
	if (kit.status !== 0) process.exit(kit.status ?? 1);
}
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
	['creator-chapter-slice', `${CREATOR}/catpope/chapter/1?group=1&slice=2`], // a slice with text boxes in both shapes (#145: group 1 = keyvisual, 2 slices)
	['creator-cast', `${CREATOR}/catpope/cast`],
	['creator-credits', `${CREATOR}/catpope/credits`],
	['creator-script', `${CREATOR}/catpope/script`],
	['creator-toon-settings', `${CREATOR}/catpope/toon`],
	['creator-characters', `${CREATOR}/characters`],
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
	// a group page (concept B): the radio row, switches and buttons of "Lesen"
	['viewer-account-reading', `${VIEWER}/account/reading`, 'viewer'],
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
