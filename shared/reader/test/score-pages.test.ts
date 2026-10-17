import assert from 'node:assert/strict';
import { test } from 'node:test';
import { pageAt, reported, restsOn, strayed } from '../src/score/score-pages.ts';

test('the page a scroll position shows: the nearest snap point, clamped to the pages', () => {
	assert.equal(pageAt(0, 400, 5), 1);
	assert.equal(pageAt(199, 400, 5), 1);
	assert.equal(pageAt(201, 400, 5), 2);
	assert.equal(pageAt(1600, 400, 5), 5);
	assert.equal(pageAt(9999, 400, 5), 5);
	assert.equal(pageAt(-30, 400, 5), 1);
	// no size yet / no pages: page 1
	assert.equal(pageAt(500, 0, 5), 1);
	assert.equal(pageAt(500, 400, 0), 1);
});

test('resting on a page: within a hundredth of a page (fractional widths, device pixels)', () => {
	assert.ok(restsOn(1201.5, 400.5, 4));
	assert.ok(restsOn(1199, 400, 4));
	assert.ok(!restsOn(1180, 400, 4));
	assert.ok(!restsOn(1200, 0, 4));
});

test('a smooth turn 1 → 5 reports no page in between, then page 5 once it rests there', () => {
	let target: number | null = 5;
	for (const left of [100, 500, 900, 1300, 1590]) {
		const r = reported(left, 400, 6, target);
		assert.equal(r.page, null);
		target = r.target;
	}
	assert.deepEqual(reported(1600, 400, 6, target), { page: 5, target: null });
});

test('without a pending turn every scroll reports the page it shows (a swipe, a wheel)', () => {
	assert.deepEqual(reported(390, 400, 6, null), { page: 2, target: null });
	assert.deepEqual(reported(130, 400, 6, null), { page: 1, target: null });
});

test('a new target replaces the old one: the pages on the way to the first are ignored as well', () => {
	// Play jumped to 3 while a turn to 2 was on its way: resting on 2 must not report 2
	assert.deepEqual(reported(400, 400, 6, 3), { page: null, target: 3 });
	assert.deepEqual(reported(800, 400, 6, 3), { page: 3, target: null });
});

test('a scroll the reader left between pages belongs to the nearest page; one resting on a page stays', () => {
	assert.equal(strayed(1200, 400, 6), null);
	assert.equal(strayed(1201.5, 400.5, 6), null);
	assert.equal(strayed(1130, 400, 6), 4);
	assert.equal(strayed(1290, 400, 6), 4);
	assert.equal(strayed(1450, 400, 6), 5);
	// past the last page (a fractional max scroll): the last page
	assert.equal(strayed(2100, 400, 6), 6);
	// no size yet: nothing to do
	assert.equal(strayed(130, 0, 6), null);
});
