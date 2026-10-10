// The full-width strip's scroll (components/toon/renderer/strip.svelte + strip-scroll.ts):
//   npm test -w viewer
// Regression (owner 2026-10-10, Firefox Android, catpope ch2): scrolling back up jumped – the strips' height followed
// the visible viewport (dvh), so each strip above grew / shrank by ≈ (1 + ratios) × the toolbar's height whenever the
// mobile toolbar slid in or out (≈ 1600 px for ch2's four strips).
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { ratioSum, stripOffset, stripScroll } from '../src/lib/components/toon/renderer/strip-scroll.ts';

test('ratioSum: the frames side by side, in screen heights', () => {
	assert.ok(Math.abs(ratioSum(Array.from({ length: 7 }, () => ({ width: 390, height: 693 }))) - (7 * 390) / 693) < 1e-9);
	assert.equal(ratioSum([{ width: 0, height: 693 }]), 0);
});

test('the scroll range maps onto the travel: start, middle, end, clamped', () => {
	assert.equal(stripOffset(0, 1000, 900), 0);
	assert.equal(stripOffset(500, 1000, 900), 450);
	assert.equal(stripOffset(1000, 1000, 900), 900);
	assert.equal(stripOffset(1500, 1000, 900), 900);
	assert.equal(stripOffset(-20, 1000, 900), 0);
	assert.equal(stripOffset(300, 0, 900), 0);
	assert.equal(stripScroll(450, 1000, 900), 500);
	assert.ok(Math.abs(stripScroll(stripOffset(321, 1000, 900), 1000, 900) - 321) < 1e-9);
	assert.equal(stripScroll(100, 1000, 0), 0);
});

test('the strip section is not sized from the dynamic viewport (dvh / svh) nor from a measured travel', () => {
	const source = readFileSync(new URL('../src/lib/components/toon/renderer/strip.svelte', import.meta.url), 'utf8');
	const rule = /\n\t\.strip \{([^}]*)\}/.exec(source)?.[1];
	assert.ok(rule, '.strip rule found');
	const heights = rule.split('\n').filter((line) => /^\s*height:/.test(line));
	assert.ok(heights.length > 0, '.strip has a height');
	for (const line of heights) {
		assert.doesNotMatch(line, /dvh|svh|--travel/, line.trim());
		assert.match(line, /--ratios/, line.trim());
	}
	assert.match(heights.at(-1)!, /100lvh/);
});
