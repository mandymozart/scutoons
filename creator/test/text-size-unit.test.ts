// text-size-unit.ts (owner 2026-10-17): the text box's size (% of the slice width) in rem / pt / px of the 1x slice
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { sizeToUnit, unitToSize } from '../src/lib/editor/text-size-unit.ts';

test('the default 4 % of a 390 px slice', () => {
	assert.equal(sizeToUnit(4, 'px', 390), 15.6);
	assert.equal(sizeToUnit(4, 'rem', 390), 0.975);
	assert.equal(sizeToUnit(4, 'pt', 390), 11.7);
});

test('a typed value survives the round trip', () => {
	for (const [v, unit] of [[1, 'rem'], [12, 'pt'], [18, 'px'], [0.875, 'rem']] as const)
		assert.equal(sizeToUnit(unitToSize(v, unit, 390), unit, 390), v);
	assert.equal(unitToSize(16, 'px', 400), 4);
});
