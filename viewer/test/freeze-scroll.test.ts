// The page's scroll stop while the story waits (src/lib/visual-novel/freeze-scroll.ts, vn-page.svelte):
//   npm test -w viewer
// Regression (owner 2026-10-10, phone, catpope ch2 scene-14): a VN message over a pinned full-width strip turned the page
// black – `overflow: hidden` on <html> made <body> (base.css: overflow-x hidden) a scroll container of its own, the
// strip's sticky pin stuck to it and left the screen. The freeze must not touch the root's overflow.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { freezeScroll, scrollKey } from '../src/lib/visual-novel/freeze-scroll.ts';

const el = (tagName: string, inside = '') => ({ tagName, isContentEditable: false, closest: (s: string) => (inside && s.includes(inside) ? {} : null) });

test('scroll keys are blocked on the page, not in a text field or on a button', () => {
	for (const key of ['ArrowDown', 'ArrowUp', 'PageDown', 'PageUp', 'Home', 'End', ' ', 'ArrowRight']) assert.equal(scrollKey(key, el('DIV')), true, key);
	assert.equal(scrollKey('ArrowDown', null), true);
	assert.equal(scrollKey('Enter', el('DIV')), false);
	assert.equal(scrollKey('a', el('DIV')), false);
	// the askText input: typing (space, arrows, Home / End) stays the field's
	for (const key of [' ', 'ArrowLeft', 'Home', 'End']) assert.equal(scrollKey(key, el('INPUT')), false, key);
	assert.equal(scrollKey('ArrowDown', { tagName: 'DIV', isContentEditable: true }), false);
	// Space presses a focused choice button
	assert.equal(scrollKey(' ', el('SPAN', 'button')), false);
	assert.equal(scrollKey('PageDown', el('BUTTON', 'button')), true);
});

test('the freeze never changes the root\'s overflow (a pinned strip would leave the screen)', () => {
	const sources = ['../src/lib/visual-novel/freeze-scroll.ts', '../src/lib/components/toon/interactions/vn-page.svelte'].map((p) =>
		readFileSync(new URL(p, import.meta.url), 'utf8').replace(/\/\*[\s\S]*?\*\/|\/\/.*$/gm, '')
	);
	for (const src of sources) {
		assert.doesNotMatch(src, /\.style\.overflow\s*=/);
		assert.doesNotMatch(src, /style\.(position|top|height)\s*=/);
	}
	assert.match(sources[1], /freezeScroll\(\)/);
});

test('freezes nest: the story\'s and an open pull panel\'s – the root is let go by the last one only', () => {
	const html = { style: { overscrollBehavior: 'auto' }, dataset: {} as Record<string, string> };
	const g = globalThis as Record<string, unknown>;
	const before = { document: g.document, window: g.window };
	g.document = { documentElement: html };
	g.window = { scrollX: 0, scrollY: 0, addEventListener() {}, removeEventListener() {} };
	try {
		const story = freezeScroll();
		const panel = freezeScroll();
		assert.equal(html.style.overscrollBehavior, 'none');
		story();
		story(); // undoing twice changes nothing
		assert.equal(html.style.overscrollBehavior, 'none');
		assert.ok('vnBlocking' in html.dataset);
		panel();
		assert.equal(html.style.overscrollBehavior, 'auto');
		assert.ok(!('vnBlocking' in html.dataset));
	} finally {
		g.document = before.document;
		g.window = before.window;
	}
});
