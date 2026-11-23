// editor.css from the kit's tokens (kit/tokens.toml): the web tools take the kit's look, so its values are read from
// that file, never copied by hand.
//   node --experimental-transform-types --no-warnings scripts/editor-tokens.ts           writes editor.css
//   node --experimental-transform-types --no-warnings scripts/editor-tokens.ts --check   fails when editor.css is stale or edited
import { readFileSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

type Value = string | number | boolean | string[];
export type Table = Record<string, Value>;

export interface Look {
	/** the colours by deck-kit name, "#rrggbb" or "#rrggbbaa" (lists such as axis and strips stay in the deck) */
	colors: Record<string, string>;
	panelOpacity: number;
	inkRing: boolean;
	signals: Record<string, string>;
	outlets: Record<string, string>;
	colorblind: Record<string, string>;
}

export interface KitTokens {
	day: Look;
	night: Look;
	/** the sizes by deck-kit name: base and edge in px, spacing in em, the rest in rem */
	size: Record<string, number>;
	groups: string[];
}

/** the part of TOML the kit file uses: tables, comments, strings, numbers, booleans and lists of strings */
export function parseToml(text: string): Record<string, Table> {
	const tables: Record<string, Table> = { '': {} };
	let current = tables[''];
	const lines = text.replace(/\r\n/g, '\n').split('\n');
	for (let i = 0; i < lines.length; i++) {
		let line = strip(lines[i]);
		if (!line) continue;
		const head = /^\[([a-z_][a-z0-9_]*(?:\.[a-z_][a-z0-9_]*)*)\]$/.exec(line);
		if (head) {
			if (tables[head[1]]) throw new Error(`line ${i + 1}: [${head[1]}] twice`);
			current = tables[head[1]] = {};
			continue;
		}
		while (/^[a-z_][a-z0-9_]*\s*=\s*\[/.test(line) && count(line, '[') > count(line, ']') && i + 1 < lines.length) line += ' ' + strip(lines[++i]);
		const pair = /^([a-z_][a-z0-9_]*)\s*=\s*(.+)$/.exec(line);
		if (!pair) throw new Error(`line ${i + 1}: cannot read "${line}"`);
		if (pair[1] in current) throw new Error(`line ${i + 1}: ${pair[1]} twice`);
		current[pair[1]] = value(pair[2].trim(), i + 1);
	}
	return tables;
}

const count = (s: string, c: string) => s.split(c).length - 1;

/** a line without its comment (a `#` outside a string) */
function strip(line: string): string {
	let quoted = false;
	for (let i = 0; i < line.length; i++) {
		if (line[i] === '"') quoted = !quoted;
		else if (line[i] === '#' && !quoted) return line.slice(0, i).trim();
	}
	return line.trim();
}

function value(v: string, at: number): Value {
	if (/^"[^"]*"$/.test(v)) return v.slice(1, -1);
	if (/^-?\d+(\.\d+)?$/.test(v)) return Number(v);
	if (v === 'true' || v === 'false') return v === 'true';
	const list = /^\[(.*)\]$/.exec(v);
	if (list) {
		const items = list[1].split(',').map((s) => s.trim()).filter(Boolean);
		if (items.every((s) => /^"[^"]*"$/.test(s))) return items.map((s) => s.slice(1, -1));
	}
	throw new Error(`line ${at}: cannot read the value ${v}`);
}

const COLOUR = /^#[0-9a-f]{6}([0-9a-f]{2})?$/;
/** the grounds a thing stands on: never an opaque grey, the stage shows through */
const GROUNDS = ['surface', 'surface_head', 'control', 'hover', 'selected', 'within', 'selection'];

function colours(t: Table | undefined, where: string): Record<string, string> {
	const out: Record<string, string> = {};
	for (const [k, v] of Object.entries(t ?? {})) {
		if (Array.isArray(v)) {
			for (const c of v) if (!COLOUR.test(c)) throw new Error(`[${where}] ${k}: "${c}" is not #rrggbb or #rrggbbaa`);
			continue;
		}
		if (typeof v !== 'string' || !COLOUR.test(v)) throw new Error(`[${where}] ${k}: ${JSON.stringify(v)} is not #rrggbb or #rrggbbaa`);
		out[k] = v;
	}
	return out;
}

function look(f: Record<string, Table>, name: 'day' | 'night'): Look {
	const { panel_opacity, ink_ring, ...own } = f[`${name}.color`] ?? {};
	if (typeof panel_opacity !== 'number' || typeof ink_ring !== 'boolean') throw new Error(`[${name}.color] needs panel_opacity and ink_ring`);
	const colors = { ...colours(f.color, 'color'), ...colours(own, `${name}.color`) };
	for (const k of ['surface', 'text', 'pressed', 'pressed_text']) if (!colors[k]) throw new Error(`[${name}.color] has no ${k}`);
	for (const k of GROUNDS) {
		if (colors[k] && (colors[k].length !== 9 || colors[k].endsWith('ff'))) throw new Error(`[${name}.color] ${k} is an opaque ground: grounds are see-through (#rrggbbaa)`);
	}
	return {
		colors,
		panelOpacity: panel_opacity,
		inkRing: ink_ring,
		signals: colours(f[`${name}.signals`], `${name}.signals`),
		outlets: colours(f[`${name}.outlets`], `${name}.outlets`),
		colorblind: colours(f[`${name}.colorblind`], `${name}.colorblind`),
	};
}

export function parse(toml: string): KitTokens {
	const f = parseToml(toml);
	const day = look(f, 'day');
	const night = look(f, 'night');
	for (const part of ['colors', 'signals', 'outlets', 'colorblind'] as const) {
		const [d, n] = [Object.keys(day[part]).sort(), Object.keys(night[part]).sort()];
		if (d.join() !== n.join()) throw new Error(`day and night differ in their ${part}: ${d.join(' ')} | ${n.join(' ')}`);
	}
	for (const k of Object.keys(night.colorblind)) if (!night.colors[k]) throw new Error(`[night.colorblind] ${k} replaces no colour`);
	const size: Record<string, number> = {};
	for (const [k, v] of Object.entries(f.size ?? {})) {
		if (typeof v !== 'number') throw new Error(`[size] ${k} is not a number`);
		size[k] = v;
	}
	for (const k of ['base', 'edge', 'spacing']) if (!size[k]) throw new Error(`[size] has no ${k}`);
	const groups = f.groups?.colors;
	if (!Array.isArray(groups) || groups.length !== 6 || !groups.every((c) => COLOUR.test(c))) throw new Error('[groups] colors: six #rrggbb');
	return { day, night, size, groups };
}

const kebab = (s: string) => s.replace(/_/g, '-');
const num = (n: number) => String(Number(n.toFixed(4)));
const rgb = (hex: string) => [1, 3, 5].map((i) => parseInt(hex.slice(i, i + 2), 16)).join(' ');

function lookCss(l: Look): string[] {
	const surface = l.colors.surface.slice(0, 7);
	const each = (prefix: string, t: Record<string, string>) => Object.entries(t).map(([k, c]) => `\t--${prefix}${kebab(k)}: ${c};`);
	return [
		...each('editor-', l.colors),
		`\t--editor-panel-opacity: ${num(l.panelOpacity)};`,
		`\t/* a floating panel's see-through ground (the surface's colour at the panel opacity, no blur) */`,
		`\t--editor-panel-ground: rgb(${rgb(surface)} / ${num(l.panelOpacity)});`,
		`\t/* the ink ring: text and icons on a panel take its ground's colour as an outline (deck-kit DrawList::halo) */`,
		`\t--editor-halo: ${l.inkRing ? `0 0 2px ${surface}, 0 0 2px ${surface}, 0 0 3px ${surface}` : 'none'};`,
		`\t/* what a port, a pin and a cable carries */`,
		...each('signal-', l.signals),
		`\t/* where a thing plays */`,
		...each('outlet-', l.outlets),
		`\t/* the colour-blind mode's ok, warning and error (tool-colours.css [data-colorblind]) */`,
		...each('editor-colorblind-', l.colorblind),
	];
}

const SIZE_UNIT: Record<string, [string, string]> = { base: ['editor-base', 'px'], edge: ['editor-edge', 'px'], spacing: ['editor-label-spacing', 'em'] };

export function render(t: KitTokens): string {
	return [
		'/*',
		' * @scutoons/tokens: the editor style – GENERATED from kit/tokens.toml by shared/tokens/scripts/editor-tokens.ts, do not',
		' * edit (npm run editor-tokens -w @scutoons/tokens). Night is the default; `data-theme="day"` on <html> or a tool root',
		' * is the day variant (`data-theme="night"` turns it back for a part). Used through tool-colours.css and tools.css.',
		' */',
		":root,\n[data-theme='night'] {",
		...lookCss(t.night),
		'}',
		'',
		"[data-theme='day'] {",
		...lookCss(t.day),
		'}',
		'',
		':root {',
		...Object.entries(t.size).map(([k, v]) => {
			const [name, unit] = SIZE_UNIT[k] ?? [`editor-${kebab(k)}`, 'rem'];
			return `\t--${name}: ${num(v)}${unit};`;
		}),
		`\t/* a group of spores: one of six */`,
		...t.groups.map((c, i) => `\t--group-${i + 1}: ${c};`),
		`\t/* text, labels and titles: ABC Areal where it loads (the CDN), Noto Sans stands in – deck-kit's fonts.rs */`,
		`\t--editor-sans: 'ABC Areal', 'Noto Sans', system-ui, sans-serif;`,
		'}',
		'',
	].join('\n');
}

const here = dirname(fileURLToPath(import.meta.url));
export const SOURCE = join(here, '../../../kit/tokens.toml');
export const TARGET = join(here, '../editor.css');

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
	const css = render(parse(readFileSync(SOURCE, 'utf8')));
	if (process.argv.includes('--check')) {
		if (readFileSync(TARGET, 'utf8').replace(/\r\n/g, '\n') !== css) {
			console.error('shared/tokens/editor.css differs from what kit/tokens.toml gives (stale or edited by hand): npm run editor-tokens -w @scutoons/tokens');
			process.exit(1);
		}
		console.log('@scutoons/tokens: editor.css matches kit/tokens.toml – ok');
	} else {
		writeFileSync(TARGET, css);
		console.log('wrote shared/tokens/editor.css');
	}
}
