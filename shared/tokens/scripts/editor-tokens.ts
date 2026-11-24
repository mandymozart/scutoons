// editor.css and kit.css from the kit's tokens (kit/tokens.toml): the web tools take the kit's look and its components'
// states, so their values are read from that file, never copied by hand.
//   node --experimental-transform-types --no-warnings scripts/editor-tokens.ts           writes editor.css and kit.css
//   node --experimental-transform-types --no-warnings scripts/editor-tokens.ts --check   fails when either is stale or edited
import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

type Value = string | number | boolean | string[];
export type Table = Record<string, Value>;

export interface Look {
	/** the colours by deck-kit name, "#rrggbb" or "#rrggbbaa" (lists such as axis and strips stay in the deck) */
	colors: Record<string, string>;
	panelOpacity: number;
	/** a menu's, a popover's, a dialog box's strength over what lies under it: above a panel's, still see-through */
	popoverOpacity: number;
	inkRing: boolean;
	signals: Record<string, string>;
	outlets: Record<string, string>;
	colorblind: Record<string, string>;
}

/** a component's state: its parts (text, ground, border, ring, knob, mark) naming a token, or its opacity or blur (rem) */
export type State = Record<string, { token: string; alpha?: number } | number>;

export interface KitTokens {
	day: Look;
	night: Look;
	/** the sizes by deck-kit name: base and edge in px, spacing in em, the rest in rem */
	size: Record<string, number>;
	groups: string[];
	/** [component.<name>.<state>] by name, then state, in the file's order */
	components: Record<string, Record<string, State>>;
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
/** the grounds a thing stands on: never an opaque grey, the stage shows through (the surface is one only at panel_opacity) */
const GROUNDS = ['surface_head', 'control', 'hover', 'selected', 'within', 'selection'];

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
	const { panel_opacity, popover_opacity, ink_ring, ...merged } = { ...f.color, ...f[`${name}.color`] };
	if (typeof panel_opacity !== 'number' || typeof popover_opacity !== 'number' || typeof ink_ring !== 'boolean') throw new Error(`[${name}.color] needs panel_opacity, popover_opacity and ink_ring`);
	for (const [k, v] of [['panel_opacity', panel_opacity], ['popover_opacity', popover_opacity]] as const) {
		if (!(v > 0 && v < 1)) throw new Error(`${k} ${v}: the stage shows through (0 < it < 1)`);
	}
	const colors = colours(merged, `${name}.color`);
	for (const k of ['surface', 'text', 'pressed', 'pressed_text']) if (!colors[k]) throw new Error(`[${name}.color] has no ${k}`);
	if (colors.surface.length !== 7) throw new Error(`[${name}.color] surface is opaque (#rrggbb): panel_opacity makes it see-through`);
	for (const k of GROUNDS) {
		if (colors[k] && (colors[k].length !== 9 || colors[k].endsWith('ff'))) throw new Error(`[${name}.color] ${k} is an opaque ground: grounds are see-through (#rrggbbaa)`);
	}
	return {
		colors,
		panelOpacity: panel_opacity,
		popoverOpacity: popover_opacity,
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
	return { day, night, size, groups, components: components(f, night.colors) };
}

const PARTS = ['text', 'ground', 'border', 'ring', 'knob', 'mark'];
/** the colours a state may name: those tool-colours.css names --tool-* */
const TOOL_TOKENS = ['text', 'muted', 'line', 'bright', 'control', 'hover', 'pressed', 'pressed_text', 'selected', 'within', 'selection', 'surface_head', 'danger', 'go', 'warn', 'stop'];
/** what a state may name beside the colours: the see-through surface of a panel (panel_opacity) and of a popover (popover_opacity), and nothing */
const EXTRA_TOKENS = ['panel', 'popover', 'none'];
/** never a ground at full strength: they are opaque greys (or the ink itself) */
const NOT_A_GROUND = ['line', 'muted', 'text', 'bright', 'surface', 'stage'];

function components(f: Record<string, Table>, colors: Record<string, string>): Record<string, Record<string, State>> {
	const out: Record<string, Record<string, State>> = {};
	for (const [key, table] of Object.entries(f)) {
		const m = /^component\.([a-z_][a-z0-9_]*)\.([a-z_][a-z0-9_]*)$/.exec(key);
		if (!m) {
			if (key.startsWith('component')) throw new Error(`[${key}]: a state is [component.<name>.<state>]`);
			continue;
		}
		const state: State = {};
		for (const [part, v] of Object.entries(table)) {
			const where = `[${key}] ${part}`;
			if (part === 'opacity') {
				if (typeof v !== 'number' || !(v > 0 && v <= 1)) throw new Error(`${where}: a number above 0, at most 1`);
				state.opacity = v;
				continue;
			}
			// the overlay level's blur (RULES H15): the only component that may blur what lies under it
			if (part === 'blur') {
				if (m[1] !== 'overlay') throw new Error(`${where}: only the overlay blurs (RULES H15)`);
				if (typeof v !== 'number' || !(v > 0 && v <= 4)) throw new Error(`${where}: rem, above 0, at most 4`);
				state.blur = v;
				continue;
			}
			if (!PARTS.includes(part)) throw new Error(`${where}: not a part (${PARTS.join(' ')} opacity blur)`);
			const r = typeof v === 'string' ? /^([a-z_]+)(?: (\d{1,2})%)?$/.exec(v) : null;
			if (!r) throw new Error(`${where}: ${JSON.stringify(v)} is not "<token>" or "<token> <n>%"`);
			const [, token, pct] = r;
			if (!(token in colors && TOOL_TOKENS.includes(token)) && !EXTRA_TOKENS.includes(token)) throw new Error(`${where}: no token "${token}" the tools name (${TOOL_TOKENS.join(' ')} ${EXTRA_TOKENS.join(' ')})`);
			if (part === 'ground' && !pct && NOT_A_GROUND.includes(token)) throw new Error(`${where}: "${token}" is an opaque ground – grounds are see-through ("${token} <n>%")`);
			state[part] = pct ? { token, alpha: Number(pct) } : { token };
		}
		(out[m[1]] ??= {})[m[2]] = state;
	}
	return out;
}

/** the --tool-* name of a kit token (tool-colours.css): the tools read every colour through those */
const TOOL_NAME: Record<string, string> = { surface_head: 'head', pressed_text: 'pressed-text' };

function stateValue(v: { token: string; alpha?: number }): string {
	if (v.token === 'none') return 'transparent';
	const name = `var(--tool-${TOOL_NAME[v.token] ?? kebab(v.token)})`;
	return v.alpha === undefined ? name : `color-mix(in srgb, ${name} ${v.alpha}%, transparent)`;
}

export function renderKit(t: KitTokens): string {
	const lines: string[] = [];
	for (const [name, states] of Object.entries(t.components)) {
		for (const [state, parts] of Object.entries(states)) {
			for (const [part, v] of Object.entries(parts)) {
				lines.push(`\t--kit-${kebab(name)}-${kebab(state)}-${part}: ${typeof v === 'number' ? `${num(v)}${part === 'blur' ? 'rem' : ''}` : stateValue(v)};`);
			}
		}
	}
	return [
		'/*',
		' * @scutoons/tokens: the kit components\' states – GENERATED from kit/tokens.toml ([component.<name>.<state>]) by',
		' * shared/tokens/scripts/editor-tokens.ts, do not edit (npm run editor-tokens -w @scutoons/tokens). Each names a',
		' * --tool-* colour (tool-colours.css); set on every theme root so a part in the other theme resolves its own.',
		' */',
		':root,\n[data-theme],\n[data-colorblind] {',
		...lines,
		'}',
		'',
	].join('\n');
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
		`\t/* a menu's, a popover's, a dialog box's see-through ground: over a panel, stronger than it */`,
		`\t--editor-popover-opacity: ${num(l.popoverOpacity)};`,
		`\t--editor-popover-ground: rgb(${rgb(surface)} / ${num(l.popoverOpacity)});`,
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
export const TARGET_KIT = join(here, '../kit.css');

if (process.argv[1] && fileURLToPath(import.meta.url) === process.argv[1]) {
	const tokens = parse(readFileSync(SOURCE, 'utf8'));
	const out: [string, string, string][] = [
		[TARGET, render(tokens), 'editor.css'],
		[TARGET_KIT, renderKit(tokens), 'kit.css'],
	];
	if (process.argv.includes('--check')) {
		const stale = out.filter(([file, css]) => !existsSync(file) || readFileSync(file, 'utf8').replace(/\r\n/g, '\n') !== css).map(([, , name]) => name);
		if (stale.length) {
			console.error(`shared/tokens/${stale.join(' and ')} differ from what kit/tokens.toml gives (stale or edited by hand): npm run editor-tokens -w @scutoons/tokens`);
			process.exit(1);
		}
		console.log('@scutoons/tokens: editor.css and kit.css match kit/tokens.toml – ok');
	} else {
		for (const [file, css, name] of out) writeFileSync(file, css);
		console.log('wrote shared/tokens/editor.css and kit.css');
	}
}
