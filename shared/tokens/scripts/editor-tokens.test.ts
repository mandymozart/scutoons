import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { test } from 'node:test';
import { parse, render, SOURCE, TARGET } from './editor-tokens.ts';

const rs = readFileSync(SOURCE, 'utf8');

test('reads both looks with the same roles, the sizes and the base from tokens.rs', () => {
	const t = parse(rs);
	assert.deepEqual(Object.keys(t.day.colors), Object.keys(t.night.colors));
	assert.deepEqual(t.night.colors.primary, [0xb4, 0xff, 0x32, 1]);
	assert.deepEqual(t.night.colors.accent, [0x00, 0xd1, 0xff, 1]);
	assert.equal(t.night.colors.selection[3], 0.6);
	assert.equal(t.night.panelOpacity, 0.5);
	assert.equal(t.night.inkRing, true);
	assert.equal(t.day.inkRing, false);
	assert.equal(t.base, 14);
	assert.equal(t.rems.row, 2.15);
	assert.equal(t.spacing, 0.15);
	assert.equal(t.edge, 1);
	assert.ok(!('axis' in t.night.colors) && !('strips' in t.night.colors), 'nested lists stay in the deck');
});

test('night is the default, day the variant; sizes in rem', () => {
	const css = render(parse(rs));
	const night = css.slice(css.indexOf(":root,\n[data-theme='night']"), css.indexOf("[data-theme='day']"));
	assert.match(night, /--editor-surface: #1a1c18;/);
	assert.match(night, /--editor-panel-ground: rgb\(26 28 24 \/ 0\.5\);/);
	assert.match(css, /\[data-theme='day'\] \{[^}]*--editor-surface: #f3f3f0;[^}]*--editor-halo: none;/);
	assert.match(css, /--editor-row: 2\.15rem;/);
	assert.doesNotMatch(css, /--editor-(?!base|edge|halo)[a-z-]+: [\d.]+px/, 'px only for the base, the edge and the halo');
});

test('a changed value in tokens.rs changes the output', () => {
	const changed = rs.replace('accent: rgb(0x00d1ff)', 'accent: rgb(0x123456)');
	assert.match(render(parse(changed)), /--editor-accent: #123456;/);
});

test('editor.css is up to date', () => {
	assert.equal(readFileSync(TARGET, 'utf8').replace(/\r\n/g, '\n'), render(parse(rs)));
});
