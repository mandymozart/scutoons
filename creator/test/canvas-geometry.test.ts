// npm run test -w creator (node's own runner, like @scutoons/content)
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { centreSnap, deltas, dialogScale, gridSteps, movedRect, pointInBox, rotationAngle, scaleFactor, scaledRect, transformedDeltas } from '../src/lib/editor/canvas-geometry.ts';

const near = (a: number, b: number, eps = 1e-6) => assert.ok(Math.abs(a - b) <= eps, `${a} ≉ ${b}`);
const box = { x: 10, y: 20, width: 40, height: 10 };
const free = gridSteps(0, 390, 625 / 390);
const px = { width: 390, height: 625 };

test('gridSteps: 30 px on a 390 px slice = 7.69 % across, the height by the ratio; 0 = no snap', () => {
	const g = gridSteps(30, 390, 625 / 390);
	near(g.stepX, (30 / 390) * 100);
	near(g.stepY, (30 / 625) * 100);
	near(g.snapX(10), (Math.round(10 / g.stepX) * g.stepX));
	assert.equal(free.snapX(12.345), 12.345);
	assert.equal(free.stepX, 0);
});

test('deltas: move follows the pointer, snapped onto the grid', () => {
	assert.deepEqual(deltas('move', 3, -2, box, free), { ddx: 3, ddy: -2, dw: 0, dh: 0 });
	const g = gridSteps(30, 390, 625 / 390);
	const d = deltas('move', 1, 0, box, g);
	near((box.x + d.ddx) / g.stepX, Math.round((box.x + d.ddx) / g.stepX));
});

test('deltas: resize from each edge; never under 1 % wide / high', () => {
	assert.deepEqual(deltas('e', 5, 0, box, free), { ddx: 0, ddy: 0, dw: 5, dh: 0 });
	assert.deepEqual(deltas('w', 5, 0, box, free), { ddx: 5, ddy: 0, dw: -5, dh: 0 });
	assert.deepEqual(deltas('s', 0, 4, box, free), { ddx: 0, ddy: 0, dw: 0, dh: 4 });
	assert.deepEqual(deltas('n', 0, 4, box, free), { ddx: 0, ddy: 4, dw: 0, dh: -4 });
	assert.equal(box.width + deltas('e', -100, 0, box, free).dw, 1);
	assert.equal(box.width + deltas('w', 100, 0, box, free).dw, 1);
});

test('transformedDeltas: without rotation and scale it is the plain resize', () => {
	const t = transformedDeltas('e', 5, 3, box, { rotation: 0, scale: 1 }, px, null);
	near(t.ddx, 0);
	near(t.ddy, 0);
	near(t.dw, 5);
	assert.equal(t.dh, 0);
});

test('transformedDeltas: a box turned 90° grows its width with a pointer moving down', () => {
	// the box's local x axis points down the slice: dy (in px) becomes the width change
	const t = transformedDeltas('e', 0, 2, box, { rotation: 90, scale: 1 }, px, null);
	near(t.dw, (2 * 6.25) / 3.9, 1e-6);
	assert.equal(t.dh, 0);
});

test('transformedDeltas: a box scaled 2× changes half as much', () => {
	const t = transformedDeltas('e', 4, 0, box, { rotation: 0, scale: 2 }, px, null);
	near(t.dw, 2);
});

test('rotationAngle: straight up = 0°, right = 90°, 15° steps on request, −180 … 180', () => {
	const shown = { x: 40, y: 40, width: 20, height: 20 }; // centre 50 / 50
	assert.equal(rotationAngle({ x: 50, y: 10 }, shown, px, false), 0);
	assert.equal(rotationAngle({ x: 90, y: 50 }, shown, px, false), 90);
	assert.equal(rotationAngle({ x: 50, y: 90 }, shown, px, false), -180);
	const a = rotationAngle({ x: 55, y: 10 }, shown, px, true);
	assert.equal(a % 15, 0);
});

test('scaleFactor: the distance from the centre against the start; null at the centre', () => {
	const shown = { x: 40, y: 40, width: 20, height: 20 };
	near(scaleFactor({ x: 60, y: 50 }, { x: 70, y: 50 }, shown, px)!, 2);
	assert.equal(scaleFactor({ x: 50, y: 50 }, { x: 70, y: 50 }, shown, px), null);
});

test('dialogScale: 0.1 … 10, 0.05 steps with snap, 2 places', () => {
	assert.equal(dialogScale(1, 1.237, false), 1.24);
	assert.equal(dialogScale(1, 1.237, true), 1.25);
	assert.equal(dialogScale(1, 100, false), 10);
	assert.equal(dialogScale(1, 0.001, true), 0.1);
});

test('scaledRect: around the centre, keeps the ratio, height only when it had one', () => {
	assert.deepEqual(scaledRect({ x: 10, y: 10, width: 20, height: 10 }, 10, 2, (v) => v), { x: 0, y: 5, width: 40, height: 20 });
	assert.deepEqual(scaledRect({ x: 10, y: 10, width: 20 }, 10, 2, (v) => v), { x: 0, y: 5, width: 40 });
	assert.equal(scaledRect({ x: 10, y: 10, width: 20 }, 10, 0.01, (v) => v).width, 1);
});

test('movedRect: the anchor point follows the drawn box; dragging the edge opposite the anchor keeps it', () => {
	const tl = { ax: 0, ay: 0 };
	const tr = { ax: 1, ay: 0 };
	const b = { ax: 0.5, ay: 1 };
	const start = { x: 90, y: 10, width: 40, height: 10 };
	// a move shifts the point like the box
	assert.deepEqual(movedRect(start, { ddx: -5, ddy: 2, dw: 0, dh: 0 }, 10, false, tr), { x: 85, y: 12, width: 40 });
	// tr: the w handle (left edge) leaves the right edge – the anchor – where it is
	assert.deepEqual(movedRect(start, { ddx: -10, ddy: 0, dw: 10, dh: 0 }, 10, false, tr), { x: 90, y: 10, width: 50 });
	// tr: the e handle moves the anchor with the right edge
	assert.deepEqual(movedRect(start, { ddx: 0, ddy: 0, dw: 5, dh: 0 }, 10, false, tr), { x: 95, y: 10, width: 45 });
	// b: the n handle keeps the bottom; the s handle moves it
	assert.deepEqual(movedRect({ x: 50, y: 90, width: 20 }, { ddx: 0, ddy: -4, dw: 0, dh: 4 }, 10, true, b), { x: 50, y: 90, width: 20, height: 14 });
	assert.deepEqual(movedRect({ x: 50, y: 90, width: 20 }, { ddx: 0, ddy: 0, dw: 0, dh: 3 }, 10, true, b), { x: 50, y: 93, width: 20, height: 13 });
	// no anchor: exactly the old arithmetic
	assert.deepEqual(movedRect({ x: 10, y: 20, width: 40 }, { ddx: 1.234, ddy: 2, dw: -0.5, dh: 0 }, 8, false, tl), { x: 11.23, y: 22, width: 39.5 });
});

test('scaledRect with an anchor: the centre stays, the anchor point moves with the box', () => {
	const r = scaledRect({ x: 60, y: 90, width: 20 }, 10, 2, (v) => v, { ax: 0.5, ay: 1 });
	// centre (60, 85) stays: the new box 40 × 20 → its bottom centre at (60, 95)
	assert.deepEqual(r, { x: 60, y: 95, width: 40 });
	assert.deepEqual(scaledRect({ x: 10, y: 20, width: 20 }, 10, 2, (v) => v), { x: 0, y: 15, width: 40 });
});

test('centreSnap: the box centre, its start or its end edge onto the stage centre within the threshold', () => {
	// a 40 % box at x 10 moved 19 % → centre at 49 % → pulled to 50 %
	assert.deepEqual(centreSnap(10, 40, 19, 1.5), { delta: 20, hit: true });
	// its left edge near the centre (48.8 %) → onto 50 %
	assert.deepEqual(centreSnap(10, 40, 38.8, 1.5), { delta: 40, hit: true });
	// far away: unchanged, no guide
	assert.deepEqual(centreSnap(10, 40, 2, 1.5), { delta: 2, hit: false });
});

test('pointInBox: the pointer in a turned / scaled box’s own frame, % of the box (a blur ramp’s dots)', () => {
	// upright: the box 10 … 50 % across, 20 … 30 % down; its centre is 50 / 50, its corners 0 / 100
	assert.deepEqual(pointInBox({ x: 30, y: 25 }, box, px), { x: 50, y: 50 });
	assert.deepEqual(pointInBox({ x: 10, y: 20 }, box, px), { x: 0, y: 0 });
	assert.deepEqual(pointInBox({ x: 40, y: 30 }, box, px), { x: 75, y: 100 });
	// turned 90° (clockwise): the box's top edge now faces right – a point right of the centre is the top middle
	const sq = { x: 40, y: 40, width: 20, height: 20 };
	const square = { width: 100, height: 100 };
	const top = pointInBox({ x: 60, y: 50 }, sq, square, 90);
	near(top.x, 50);
	near(top.y, 0);
	// … and below the centre is the right edge's middle
	const right = pointInBox({ x: 50, y: 60 }, sq, square, 90);
	near(right.x, 100);
	near(right.y, 50);
	// the same pointer on a box at scale 2: half as far from the centre in the box's own frame
	const scaled = pointInBox({ x: 55, y: 50 }, sq, square, 0, 2);
	near(scaled.x, 62.5);
	near(scaled.y, 50);
	// outside the box: clamped onto its edge
	assert.deepEqual(pointInBox({ x: 95, y: 0 }, box, px), { x: 100, y: 0 });
});
