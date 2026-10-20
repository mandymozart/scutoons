// The variants board's zoom (src/lib/board-zoom.js, agents/research/VARIANTS-CANVAS.md §3.2): the range, a key's step
// (landing on 100 %), the wheel, the % label, 0 = fit the widest band (bisection against bandLayout's width – the
// captions' jump at 60 % included), zooming about the pointer – over a picture, and over the background beside the
// nearest one (the band's body as the anchor drifted rows away: the gaps are fixed rem).
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { clampZoom, fitZoom, keepUnder, nearestBox, percent, wheelZoom, ZOOM_MAX, ZOOM_MIN, ZOOM_STEP, zoomStep } from '../src/lib/board-zoom.js';
import { bandLayout, CAPTIONS_MIN } from '../src/lib/board-layout.js';

test('the range, a key\'s step, the wheel, the label', () => {
	assert.equal(clampZoom(5), ZOOM_MAX);
	assert.equal(clampZoom(0.01), ZOOM_MIN);
	assert.equal(clampZoom(NaN), 1);
	assert.equal(zoomStep(1, 1), ZOOM_STEP);
	assert.equal(zoomStep(1, -1), 1 / ZOOM_STEP);
	assert.equal(zoomStep(0.9, 1), 1, 'a step across 100 % stops there');
	assert.equal(zoomStep(1.1, -1), 1);
	assert.equal(zoomStep(ZOOM_MAX, 1), ZOOM_MAX);
	assert.equal(zoomStep(ZOOM_MIN, -1), ZOOM_MIN);
	assert.ok(wheelZoom(1, -100) > 1 && wheelZoom(1, 100) < 1, 'the wheel up zooms in');
	assert.equal(wheelZoom(1, -1e6), ZOOM_MAX);
	assert.equal(percent(0.8), '80 %');
	assert.equal(percent(1.234), '123 %');
});

test('0 fits the widest band: the largest whole percent whose width fits', () => {
	const linear = (z) => 10 + 40 * z;
	assert.equal(fitZoom(linear, 50), 1);
	assert.equal(fitZoom(linear, 30), 0.5);
	assert.equal(fitZoom(linear, 1000), ZOOM_MAX);
	assert.equal(fitZoom(linear, 5), ZOOM_MIN, 'nothing fits: the smallest');
	assert.equal(fitZoom(linear, 0), 1, 'no room measured: 100 %');
	// against the real layout: 12 results of a 9:16 shot
	const rec = (n) => ({ id: `s--g${n}`, file: `s--g${n}.png`, inputs: [{ role: 'still', key: 'out:c/clean/s.png' }], parents: [] });
	const widthAt = (z) => bandLayout({ shot: 's', records: Array.from({ length: 12 }, (_, i) => rec(i + 1)), aspect: '9:16', zoom: z }).width;
	for (const avail of [40, 60, 80, 120]) {
		const z = fitZoom(widthAt, avail);
		assert.ok(widthAt(z) <= avail, `${avail}: fits at ${z}`);
		assert.ok(z === ZOOM_MAX || widthAt(z + 0.01) > avail, `${avail}: ${z} is the largest`);
	}
	// the captions' column makes the width jump at 60 %: just below it fits, at it not
	const avail = (widthAt(CAPTIONS_MIN - 0.005) + widthAt(CAPTIONS_MIN)) / 2;
	assert.ok(widthAt(CAPTIONS_MIN) > widthAt(CAPTIONS_MIN - 0.005));
	assert.equal(fitZoom(widthAt, avail), CAPTIONS_MIN - 0.01);
});

test('zooming about the pointer: the same spot of the box stays under it', () => {
	const before = { left: 100, top: 200, width: 50, height: 80 };
	// the box doubled and moved: the point 40 % across / 25 % down must come back under (120, 220)
	const after = { left: 130, top: 260, width: 100, height: 160 };
	assert.deepEqual(keepUnder(before, after, 120, 220), { dx: 130 + 40 - 120, dy: 260 + 40 - 220 });
	assert.deepEqual(keepUnder(before, before, 120, 220), { dx: 0, dy: 0 }, 'nothing moved: no scroll');
	assert.deepEqual(keepUnder({ left: 0, top: 0, width: 0, height: 0 }, { left: 5, top: 7, width: 0, height: 0 }, 0, 0), { dx: 5, dy: 7 });
});

test('over the background: the nearest picture is the anchor – a point beside it keeps its distance in its scale', () => {
	const boxes = [{ left: 0, top: 0, width: 50, height: 90 }, { left: 70, top: 0, width: 50, height: 90 }, { left: 70, top: 130, width: 50, height: 90 }];
	assert.equal(nearestBox(boxes, 20, 40), 0, 'inside the first');
	assert.equal(nearestBox(boxes, 64, 40), 1, 'in the gap, nearer the second');
	assert.equal(nearestBox(boxes, 95, 118), 2, 'between the rows, nearer the lower');
	assert.equal(nearestBox(boxes, 500, -300), 1, 'far off: the nearest corner');
	assert.equal(nearestBox([], 0, 0), -1);
	// 10px right of a 50 × 90 picture at 100 %; zoomed 2× the picture is 100 × 180 – the point stays 20px right of it
	const before = boxes[1], after = { left: 40, top: -45, width: 100, height: 180 };
	const { dx, dy } = keepUnder(before, after, 130, 45);
	assert.deepEqual({ dx, dy }, { dx: 40 + 120 - 130, dy: -45 + 90 - 45 });
});
