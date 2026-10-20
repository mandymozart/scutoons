// The slice numbers (tools/storyboard-render/spec.js, one source – lib.mjs re-exports them) and the storyboard
// validator's whole-slices rule (scripts/storyboard/storyboard.mjs) – #111, agents/research/SLICE-GRID.md §1, §3.1.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import * as spec from '../../../tools/storyboard-render/spec.js';
import * as lib from '../../../scripts/storyboard/lib.mjs';
import { validate } from '../../../scripts/storyboard/storyboard.mjs';

test('one source: lib.mjs re-exports spec.js', () => {
	for (const k of ['W', 'S', 'UNIT', 'SLICE_ASPECTS', 'slicesOf', 'aspectSlices', 'parseAspect']) assert.equal(lib[k], spec[k], k);
	assert.equal(lib.SLICE_H, spec.UNIT);
	assert.deepEqual([spec.W, spec.S, spec.UNIT], [390, 844, 624]);
});

test('slicesOf: whole slices within 0.5 %, else null', () => {
	assert.equal(spec.slicesOf(624), 1);
	assert.equal(spec.slicesOf(625), 1); // #78's 0.16 %
	assert.equal(spec.slicesOf(1248), 2);
	assert.equal(spec.slicesOf(3120), 5);
	assert.equal(spec.slicesOf(1560), null); // 1:4 = 2.5 slices
	assert.equal(spec.slicesOf(693), null); // 9:16
	assert.equal(spec.slicesOf(681), null); // a joined rest, 8.4 % off
	assert.equal(spec.slicesOf(300), null);
});

test('aspectSlices: the slice aspects are 1 … 5 slices, at their exact heights', () => {
	assert.deepEqual(spec.SLICE_ASPECTS.map(spec.aspectSlices), [1, 2, 3, 4, 5]);
	assert.deepEqual(spec.SLICE_ASPECTS.map((a) => lib.shotHeight(a, 'vertical')), [624, 1248, 1872, 2496, 3120]);
	for (const a of ['9:16', '1:3', '1:4', '1:5', 'x']) assert.equal(spec.aspectSlices(a), null, a);
});

const sizeWarnings = (format, aspect) => {
	const shot = { id: 'sb-1-1', format, aspect, action: { de: 'x' }, source: { want: 'art' }, view: 'real', light: { key: 'left' } };
	const sb = { storyboard: 1, toon: 't', chapter: 1, groups: [{ renderer: { type: 'vertical' }, shots: [shot] }] };
	return validate(sb).warnings.filter((w) => /slice/.test(w));
};

test('validator: viewport / vertical shots warn unless whole slices', () => {
	assert.deepEqual(sizeWarnings('viewport', '1:1.6'), []);
	assert.deepEqual(sizeWarnings('vertical', '1:3.2'), []);
	assert.deepEqual(sizeWarnings('vertical', '1:4.8'), []);
	assert.match(sizeWarnings('viewport', '9:16')[0], /a viewport is one slice, 1:1\.6/);
	assert.match(sizeWarnings('viewport', '1:3.2')[0], /a viewport is one slice/);
	assert.match(sizeWarnings('vertical', '1:3')[0], /under 2 slices: a viewport \(1:1\.6\)\? else 1:3\.2/);
	assert.match(sizeWarnings('vertical', '1:1.6')[0], /under 2 slices/);
	assert.match(sizeWarnings('vertical', '1:4')[0], /2\.5 slices – not whole slices: 1:3\.2 or 1:4\.8/);
	assert.match(sizeWarnings('vertical', '1:9')[0], /not whole slices: 1:8 or 1:9\.6/);
	assert.deepEqual(sizeWarnings('strip', '3:1'), []); // strips and panoramas stay outside (SLICE-GRID.md §2)
});
