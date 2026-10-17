// The 9-point anchor of text boxes / placed items (anchor.ts, owner 2026-10-17).   npm test -w @scutoons/content
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { ANCHORS, anchorAt, anchorFactors, anchorPoint, anchorTranslate, anchoredRect, reanchor, suggestedAnchor, validateFile } from '../src/index.ts';

test('factors: none / tl = top-left; the nine points', () => {
	assert.deepEqual(anchorFactors(undefined), { ax: 0, ay: 0 });
	assert.deepEqual(anchorFactors('nope'), { ax: 0, ay: 0 });
	assert.deepEqual(anchorFactors('tl'), { ax: 0, ay: 0 });
	assert.deepEqual(anchorFactors('t'), { ax: 0.5, ay: 0 });
	assert.deepEqual(anchorFactors('tr'), { ax: 1, ay: 0 });
	assert.deepEqual(anchorFactors('l'), { ax: 0, ay: 0.5 });
	assert.deepEqual(anchorFactors('c'), { ax: 0.5, ay: 0.5 });
	assert.deepEqual(anchorFactors('r'), { ax: 1, ay: 0.5 });
	assert.deepEqual(anchorFactors('bl'), { ax: 0, ay: 1 });
	assert.deepEqual(anchorFactors('b'), { ax: 0.5, ay: 1 });
	assert.deepEqual(anchorFactors('br'), { ax: 1, ay: 1 });
	for (const a of ANCHORS) {
		const f = anchorFactors(a);
		assert.equal(anchorAt(f.ax, f.ay), a);
	}
});

test('anchoredRect: tl is the old top-left; others pull the box back by its size', () => {
	const rect = { x: 90, y: 50, width: 40, height: 20 };
	assert.deepEqual(anchoredRect(rect, undefined), { left: 90, top: 50, width: 40, height: 20 });
	assert.deepEqual(anchoredRect(rect, 'tr'), { left: 50, top: 50, width: 40, height: 20 });
	assert.deepEqual(anchoredRect(rect, 'c'), { left: 70, top: 40, width: 40, height: 20 });
	assert.deepEqual(anchoredRect(rect, 'br'), { left: 50, top: 30, width: 40, height: 20 });
	// no stored height: the drawn one counts (an auto height, an icon's ratio)
	assert.deepEqual(anchoredRect({ x: 50, y: 80, width: 20 }, 'b', 10), { left: 40, top: 70, width: 20, height: 10 });
	assert.deepEqual(anchoredRect({ x: 50, y: 80, width: 20 }, 'b'), { left: 40, top: 80, width: 20, height: 0 });
	const p = anchorPoint(50, 30, 40, 20, 'br');
	assert.deepEqual(p, { x: 90, y: 50 });
});

test('reanchor keeps the box where it is', () => {
	const rect = { x: 10, y: 20, width: 30, height: 10 };
	for (const from of ANCHORS) {
		for (const to of ANCHORS) {
			const start = { ...rect, ...reanchor(rect, 'tl', from) };
			const next = { ...start, ...reanchor(start, from, to) };
			const a = anchoredRect(start, from);
			const b = anchoredRect(next, to);
			assert.ok(Math.abs(a.left - b.left) < 0.011 && Math.abs(a.top - b.top) < 0.011, `${from} → ${to}`);
		}
	}
	assert.deepEqual(reanchor(rect, 'tl', 'br'), { x: 40, y: 30 });
	assert.deepEqual(reanchor({ x: 40, y: 30, width: 30 }, 'br', 'tl', 10), { x: 10, y: 20 });
});

test('anchorTranslate: CSS for the reader (none for tl)', () => {
	assert.equal(anchorTranslate(undefined), null);
	assert.equal(anchorTranslate('tl'), null);
	assert.equal(anchorTranslate('tr'), '-100% 0%');
	assert.equal(anchorTranslate('c'), '-50% -50%');
	assert.equal(anchorTranslate('b'), '-50% -100%');
});

test('suggestedAnchor: near an edge / corner, a hint only', () => {
	assert.equal(suggestedAnchor({ left: 40, top: 40, width: 20, height: 10 }, undefined), null);
	assert.equal(suggestedAnchor({ left: 58, top: 2, width: 40, height: 10 }, undefined), 'tr');
	// only the side it is near changes: from the top-left → top-right, from the centre → right
	assert.equal(suggestedAnchor({ left: 58, top: 40, width: 40, height: 10 }, undefined), 'tr');
	assert.equal(suggestedAnchor({ left: 58, top: 40, width: 40, height: 10 }, 'c'), 'r');
	assert.equal(suggestedAnchor({ left: 30, top: 89, width: 40, height: 10 }, 'tl'), 'bl');
	assert.equal(suggestedAnchor({ left: 58, top: 2, width: 40, height: 10 }, 'tr'), null);
	// full width: the side stays as it is
	assert.equal(suggestedAnchor({ left: 0, top: 88, width: 100, height: 10 }, 't'), 'b');
});

test('validation: the anchor enum; edge warnings use the drawn box', () => {
	const chapter = (slice: object) => ({
		title: 'P',
		number: 2,
		groups: [{ label: 'g', renderer: { type: 'vertical' }, slices: [{ image: 's1/c2/a.webp', width: 390, height: 625, ...slice }] }]
	});
	const problems = (slice: object) =>
		validateFile('chapter', chapter(slice), 'de', { file: 'x' } as never)
			.map((p) => `${p.level} ${p.path.join('.')}`)
			.filter((p) => p.includes('slices.0.'));
	// a box at the right edge anchored there: inside the slice
	assert.deepEqual(problems({ dialog: [{ rect: { x: 98, y: 10, width: 40 }, text: 'Hi', anchor: 'tr' }] }), []);
	// the same numbers top-left: past the side
	assert.deepEqual(problems({ dialog: [{ rect: { x: 98, y: 10, width: 40 }, text: 'Hi' }] }), ['warning groups.0.slices.0.dialog.0.rect']);
	assert.deepEqual(problems({ dialog: [{ rect: { x: 10, y: 10, width: 40 }, text: 'Hi', anchor: 'middle' }] }), ['error groups.0.slices.0.dialog.0.anchor']);
	assert.deepEqual(problems({ items: [{ id: 'a-1', item: 'x', rect: { x: 50, y: 95, width: 20 }, anchor: 'b' }] }), []);
	assert.deepEqual(problems({ items: [{ id: 'a-1', item: 'x', rect: { x: 50, y: 95, width: 20 }, anchor: 'down' }] }), ['error groups.0.slices.0.items.0.anchor']);
	// x past 100 is fine when the anchor is on the right
	assert.deepEqual(problems({ items: [{ id: 'a-1', item: 'x', rect: { x: 110, y: 10, width: 20 }, anchor: 'tr' }] }), []);
	assert.deepEqual(problems({ items: [{ id: 'a-1', item: 'x', rect: { x: 110, y: 10, width: 20 } }] }), ['warning groups.0.slices.0.items.0.rect.x']);
});
