// length-units.ts: what a stored length is a part of is all the creator says – the conversions are the core's
import { test } from 'node:test';
import assert from 'node:assert/strict';
import type { UnitConversions } from '@scutoons/ui/core-units';
import { fontLength, fontPx, sliceLength } from '../src/lib/editor/length-units.ts';

// a stub core that records what it is asked: no arithmetic of its own
let refsSeen: number[] = [];
const core: UnitConversions = {
	offered: (_d, refs) => ((refsSeen = Array.from(refs).map((v) => (Number.isNaN(v) ? -1 : v))), ['percent', 'px']),
	convert: (v) => v,
	pick: (_c, fallback) => fallback,
	word: (u) => u,
	decimals: () => 1,
	step: () => 1
};

test('no core yet: no units, the field shows its stored unit', () => {
	assert.equal(sliceLength(null, 'position', 390), undefined);
	assert.equal(fontLength(null, 'stroke', 15.6), undefined);
});

test('a slice length is % of the slice axis at 1x, on the web reference', () => {
	const y = sliceLength(core, 'position', 625)!;
	assert.deepEqual(refsSeen, [-1, -1, -1, 625, -1, 1, -1]);
	assert.equal(y.kind, 'position');
	assert.equal(y.fallback, 'percent');
	assert.equal(sliceLength(core, 'text-size', 390, 'rem')!.fallback, 'rem');
});

test('a font length is em of the font size, the line height shown as ×', () => {
	const lh = fontLength(core, 'line-height', fontPx(undefined, 390), 'em', '×')!;
	assert.deepEqual(refsSeen, [-1, -1, -1, 15.6, -1, 1, 1]);
	assert.equal(lh.fallback, 'em');
	assert.equal(fontPx(4, 390), 15.6);
});
