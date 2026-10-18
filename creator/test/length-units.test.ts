// length-units.ts (owner 2026-10-17 / -18): stored lengths shown in %, px, pt, rem (or em) – the stored number never changes
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { fontLength, fontPx, sliceLength } from '../src/lib/editor/length-units.ts';

const unit = (u: { options: { id: string }[] }, id: string) => u.options.find((o) => o.id === id) as any;
const round = (v: number, d: number) => Math.round(v * 10 ** d) / 10 ** d;

test('% of the slice width: the default text size 4 % of a 390 px slice', () => {
	const size = sliceLength('text-size', 390, 'rem');
	assert.deepEqual(size.options.map((o) => o.id), ['%', 'px', 'pt', 'rem']);
	assert.equal(round(unit(size, 'px').toShown(4), 1), 15.6);
	assert.equal(round(unit(size, 'rem').toShown(4), 3), 0.975);
	assert.equal(round(unit(size, 'pt').toShown(4), 2), 11.7);
	assert.equal(unit(size, '%').toShown(4), 4);
	// a typed value survives the round trip; steps per unit
	for (const [v, id] of [[1, 'rem'], [12, 'pt'], [18, 'px'], [0.875, 'rem']] as const) assert.equal(round(unit(size, id).toShown(unit(size, id).toStored(v)), 3), v);
	assert.deepEqual(size.options.map((o) => o.step), [0.1, 1, 0.5, 0.05]);
});

test('% of the slice height: y 10 % of a 625 px tall slice = 62.5 px', () => {
	const y = sliceLength('position', 625);
	assert.equal(unit(y, 'px').toShown(10), 62.5);
	assert.equal(unit(y, 'px').toStored(125), 20);
	assert.equal(y.fallback, '%');
});

test('em of the font size: a 0.14 em outline at 4 % of 390 px (15.6 px) ≈ 2.2 px; line height as ×', () => {
	const stroke = fontLength('stroke', fontPx(4, 390), 'px');
	assert.equal(round(unit(stroke, 'px').toShown(0.14), 1), 2.2);
	assert.equal(round(unit(stroke, 'px').toStored(3.12), 3), 0.2);
	const lh = fontLength('line-height', fontPx(undefined, 390), 'em', '×', 0.05);
	assert.equal(unit(lh, 'em').label, '×');
	assert.equal(round(unit(lh, 'px').toShown(1.35), 2), 21.06);
});
