import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import { parse, parseToml, render, renderKit, SOURCE, TARGET, TARGET_KIT } from './editor-tokens.ts';

const toml = readFileSync(SOURCE, 'utf8').replace(/\r\n/g, '\n');

test('reads both looks with the same roles, the tables and the sizes from kit/tokens.toml', () => {
	const t = parse(toml);
	assert.deepEqual(Object.keys(t.day.colors).sort(), Object.keys(t.night.colors).sort());
	assert.equal(t.night.colors.stage, '#000000');
	assert.equal(t.night.colors.control, '#f3f3f00f', 'a control stands on the text colour at 6 %');
	assert.equal(t.day.colors.selected, '#17191629', 'a selected item: text at 16 %');
	assert.equal(t.night.panelOpacity, 0.85);
	assert.equal(t.day.panelOpacity, 0.85, 'one strength for both looks');
	assert.equal(t.night.inkRing, true);
	assert.equal(t.day.inkRing, false);
	assert.deepEqual(Object.keys(t.night.signals), ['event', 'control', 'clock', 'audio', 'picture', 'address', 'vote', 'scene', 'far']);
	assert.deepEqual(Object.keys(t.day.outlets), ['page', 'stage', 'target', 'device']);
	assert.deepEqual(t.groups, ['#5fb3ff', '#7fd68a', '#f2b84b', '#e07ad6', '#ff7a6b', '#9a8cff']);
	assert.deepEqual(Object.keys(t.night.colorblind), ['go', 'warn', 'stop']);
	assert.equal(t.size.base, 14);
	assert.equal(t.size.row, 2.15);
	assert.equal(t.size.spacing, 0.15);
	assert.equal(t.size.edge, 1);
});

test('palette D: no accent, no lime primary, no signal hue among the UI colours', () => {
	const t = parse(toml);
	for (const l of [t.day, t.night]) for (const k of ['accent', 'primary', 'primary_line', 'primary_ink']) assert.ok(!(k in l.colors), k);
	assert.doesNotMatch(render(t), /--editor-(accent|primary)/);
});

test('night is the default, day the variant; sizes in rem, base and edge in px', () => {
	const css = render(parse(toml));
	const night = css.slice(css.indexOf(":root,\n[data-theme='night']"), css.indexOf("[data-theme='day']"));
	assert.match(night, /--editor-surface: #000000;/);
	assert.match(night, /--editor-panel-ground: rgb\(0 0 0 \/ 0\.85\);/);
	assert.match(night, /--editor-halo: 0 0 2px #000000,/);
	assert.match(night, /--signal-audio: #19b8d8;/);
	assert.match(night, /--outlet-device: #f2df6f;/);
	assert.match(night, /--editor-colorblind-go: #56b4e9;/);
	assert.match(css, /\[data-theme='day'\] \{[^}]*--editor-panel-ground: rgb\(255 255 255 \/ 0\.85\);[^}]*--editor-halo: none;/);
	assert.match(css, /--editor-row: 2\.15rem;/);
	assert.match(css, /--editor-base: 14px;/);
	assert.match(css, /--editor-label-spacing: 0\.15em;/);
	assert.match(css, /--group-6: #9a8cff;/);
	assert.doesNotMatch(css, /--editor-(?!base|edge|halo)[a-z-]+: [\d.]+px/, 'px only for the base, the edge and the halo');
});

test('a changed value in the file changes the output', () => {
	const changed = toml.replace('audio = "#19b8d8"', 'audio = "#123456"');
	assert.match(render(parse(changed)), /--signal-audio: #123456;/);
});

test('the reader refuses what the kit file must not hold', () => {
	assert.throws(() => parse(toml.replace('text = "#f3f3f0"', 'text = [0.95, 0.95, 0.94, 1.0]')), /cannot read the value|not #rrggbb/);
	assert.throws(() => parse(toml.replace('go = "#56b4e9"', 'go = "blue"')), /not #rrggbb/);
	assert.throws(() => parse(toml.replace('surface = "#000000"', 'surface = "#000000d9"')), /surface is opaque/);
	assert.throws(() => parse(toml.replace('hover = "#1719161a"', 'hover = "#e4e8dcff"')), /hover is an opaque ground/);
	assert.throws(() => parse(toml.replace('[day.signals]\nevent = "#22271e"\n', '[day.signals]\n')), /day and night differ in their signals/);
	assert.throws(() => parseToml('[a]\nx = 1\nx = 2\n'), /x twice/);
	assert.deepEqual(parseToml('[a.b] # c\nx = ["#000000", # one\n  "#ffffff"]\n')['a.b'], { x: ['#000000', '#ffffff'] });
});

test('editor.css is up to date and untouched by hand', () => {
	assert.equal(readFileSync(TARGET, 'utf8').replace(/\r\n/g, '\n'), render(parse(toml)));
});

test("the components' states: tokens by name, see-through grounds, no hue for on, focus and selected", () => {
	const t = parse(toml);
	assert.deepEqual(t.components.tab.hover, { text: { token: 'text' }, border: { token: 'line' } });
	assert.deepEqual(t.components.chip.on.ground, { token: 'text', alpha: 12 });
	const css = renderKit(t);
	assert.match(css, /--kit-tab-rest-border: transparent;/);
	assert.match(css, /--kit-toggle-on-knob: var\(--tool-pressed-text\);/);
	assert.match(css, /--kit-button-primary-hover-ground: color-mix\(in srgb, var\(--tool-pressed\) 85%, transparent\);/);
	assert.match(css, /--kit-section-on-ground: var\(--tool-head\);/);
	assert.match(css, /--kit-tab-disabled-opacity: 0\.4;/);
	for (const name of ['toggle', 'focus', 'list_row', 'chip', 'tile', 'icon_button', 'button_primary']) {
		for (const [state, parts] of Object.entries(t.components[name])) {
			if (/ok|warning|error|destroy|dot/.test(state)) continue;
			for (const [part, v] of Object.entries(parts)) if (typeof v !== 'number') assert.doesNotMatch(v.token, /^(go|warn|stop|danger)$/, `${name}.${state}.${part}`);
		}
	}
});

test('a state refuses an opaque grey ground, an unknown token or part', () => {
	const hover = '[component.tile.hover]\nground = "hover"';
	assert.ok(toml.includes(hover));
	const with_ = (s: string) => toml.replace(hover, `[component.tile.hover]\n${s}`);
	assert.throws(() => parse(with_('ground = "line"')), /opaque ground/);
	assert.throws(() => parse(with_('ground = "accent"')), /no token "accent"/);
	assert.throws(() => parse(with_('shade = "hover"')), /not a part/);
	assert.doesNotThrow(() => parse(with_('ground = "muted 20%"')));
});

test('kit.css is up to date and untouched by hand', () => {
	assert.equal(readFileSync(TARGET_KIT, 'utf8').replace(/\r\n/g, '\n'), renderKit(parse(toml)));
});
