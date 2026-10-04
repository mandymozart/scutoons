// npm run test -w creator (node's own runner, like @scutoons/content)
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { deltas, dialogScale, gridSteps, rotationAngle, scaleFactor, scaledRect, transformedDeltas } from '../src/lib/editor/canvas-geometry.ts';

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
