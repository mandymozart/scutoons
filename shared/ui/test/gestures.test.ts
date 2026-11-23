import { describe, it } from 'node:test';
import { expect } from './expect.ts';
import { FLING, addSample, atBottomEdge, flingEdge, isSwipeUp, throwOffset, velocity, type Sample } from '../src/editor/gestures.ts';

const win = { w: 1400, h: 900 };
/** a pointer moving from (x, y) by (vx, vy) px per ms, a sample every 16 ms for `n` frames */
const path = (x: number, y: number, vx: number, vy: number, n = 6, t0 = 1000): Sample[] =>
	Array.from({ length: n }, (_, i) => ({ x: x + vx * 16 * i, y: y + vy * 16 * i, t: t0 + 16 * i }));
const end = (s: Sample[]) => s.at(-1)!.t;

describe('a fling: fast toward an edge and past it closes', () => {
	it('measures the speed over the last samples only', () => {
		const slowThenFast = [...path(100, 400, 0.1, 0, 5, 0), ...path(200, 400, 2, 0, 6, 200)];
		expect(Math.round(velocity(slowThenFast).vx)).toBe(2);
		expect(velocity([])).toEqual({ vx: 0, vy: 0 });
		expect(velocity([{ x: 1, y: 1, t: 5 }])).toEqual({ vx: 0, vy: 0 });
	});

	it('keeps only the recent samples', () => {
		let s: Sample[] = [];
		for (let t = 0; t <= 400; t += 16) s = addSample(s, { x: t, y: 0, t });
		expect(s.every((o) => 400 - o.t <= 120)).toBe(true);
		expect(s.at(-1)!.t).toBe(400);
	});

	it('flings toward each edge when the pointer would pass it', () => {
		const right = path(1100, 400, 2.5, 0.3);
		expect(flingEdge(right, end(right), win)).toBe('right');
		const left = path(300, 400, -2.5, 0);
		expect(flingEdge(left, end(left), win)).toBe('left');
		const up = path(700, 200, 0.2, -2);
		expect(flingEdge(up, end(up), win)).toBe('top');
		const down = path(700, 700, 0, 2);
		expect(flingEdge(down, end(down), win)).toBe('bottom');
	});

	it('a slow drag to the edge is no fling: the drop zones take it', () => {
		const slow = path(1350, 400, 0.4, 0);
		expect(flingEdge(slow, end(slow), win)).toBeNull();
	});

	it('fast but far from the edge is no fling', () => {
		const middle = path(500, 400, 1.5, 0);
		expect(flingEdge(middle, end(middle), win)).toBeNull();
	});

	it('the pointer rested before the release: no fling', () => {
		const right = path(1100, 400, 2.5, 0);
		expect(flingEdge(right, end(right) + FLING.rest + 1, win)).toBeNull();
	});

	it('takes the main direction: mostly sideways toward the top is sideways', () => {
		const diagonal = path(1200, 100, 2, -1.2);
		expect(flingEdge(diagonal, end(diagonal), win)).toBe('right');
	});

	it('throws a rectangle wholly out of the window, drifting along', () => {
		const rect = { x: 1000, y: 300, w: 300, h: 200 };
		expect(throwOffset(rect, 'right', { vx: 2, vy: 0 }, win)).toEqual({ x: 400, y: 0 });
		expect(throwOffset(rect, 'right', { vx: 2, vy: 1 }, win)).toEqual({ x: 400, y: 200 });
		expect(throwOffset(rect, 'left', { vx: -2, vy: 0 }, win)).toEqual({ x: -1300, y: 0 });
		expect(throwOffset(rect, 'top', { vx: 0, vy: -2 }, win)).toEqual({ x: 0, y: -500 });
		expect(throwOffset(rect, 'bottom', { vx: 0, vy: 2 }, win)).toEqual({ x: 0, y: 600 });
	});
});

describe('a swipe up from the bottom', () => {
	it('starts only near the bottom edge', () => {
		expect(atBottomEdge(890, 900, 16)).toBe(true);
		expect(atBottomEdge(850, 900, 16)).toBe(false);
	});

	it('needs the distance, upward, more up than sideways', () => {
		expect(isSwipeUp(10, -80, 64)).toBe(true);
		expect(isSwipeUp(0, -40, 64)).toBe(false);
		expect(isSwipeUp(0, 80, 64)).toBe(false);
		expect(isSwipeUp(100, -80, 64)).toBe(false);
	});
});
