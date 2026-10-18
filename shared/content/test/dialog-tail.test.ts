// Free speech-bubble tails (dialog-tail.ts, owner 2026-10-18).   npm test -w @scutoons/content
import { test } from 'node:test';
import assert from 'node:assert/strict';
import {
	defaultTail,
	dialogTail,
	freeTailFrom,
	isFreeTail,
	nearestBase,
	speechTail,
	tailBase,
	tailHalfWidth,
	tailKindOf,
	thoughtTail,
	tipOutside,
	toLocal,
	toSlice,
	type Dialog
} from '../src/index.ts';

const bubble = (extra: Partial<Dialog> = {}): Dialog => ({ rect: { x: 10, y: 10, width: 40 }, text: 'Hi', variant: 'bubble', frame: true, ...extra });

test('which tail a box draws: free, old fixed (defaults), none', () => {
	const free = { kind: 'thought', tip: { x: 1, y: 2 }, base: { side: 'top', at: 0.5 } } as const;
	assert.equal(isFreeTail(free), true);
	assert.equal(isFreeTail({ kind: 'speech', tip: { x: 1 }, base: { side: 'top', at: 0 } }), false);
	assert.equal(isFreeTail('bottom'), false);
	assert.deepEqual(dialogTail(bubble({ tail: free })), free);
	assert.equal(dialogTail(bubble()), 'bottom-left');
	assert.equal(dialogTail(bubble({ shape: 'wobbly' })), null);
	assert.equal(dialogTail(bubble({ tail: 'none' })), null);
	assert.equal(dialogTail(bubble({ tail: 'top' })), 'top');
	// not a drawn bubble: no tail at all
	assert.equal(dialogTail({ ...bubble({ tail: free }), frame: false }), null);
	assert.equal(dialogTail({ ...bubble(), variant: 'caption' }), null);
	assert.equal(tailKindOf(bubble({ tail: free })), 'thought');
	assert.equal(tailKindOf(bubble({ tail: 'bottom-right' })), 'speech');
	assert.equal(tailKindOf(bubble({ tail: 'none' })), 'none');
});

test('new and converted tails', () => {
	const box = { left: 10, top: 10, width: 40, height: 12 };
	const d = defaultTail('speech', box, 625 / 390);
	assert.deepEqual(d.base, { side: 'bottom', at: 0.3 });
	// a bit below-left of the base (22 / 22)
	assert.ok(d.tip.x < 22 && d.tip.y > 22, JSON.stringify(d.tip));
	const top = freeTailFrom('top-right', box, 4, 625 / 390);
	assert.equal(top.base.side, 'top');
	assert.equal(top.base.at, 0.75);
	assert.ok(top.tip.y < 10);
	assert.equal(freeTailFrom('bottom', box).tip.x, 30);
	assert.equal(freeTailFrom('bottom-left', box, 4, 1, 'thought').kind, 'thought');
});

test('box frame ↔ slice: turned and scaled round trip', () => {
	const f = { left: 100, top: 50, width: 200, height: 80, rotation: 30, scale: 1.5 };
	for (const q of [{ x: 0, y: 0 }, { x: 200, y: 80 }, { x: 37, y: -500 }]) {
		const back = toLocal(f, toSlice(f, q));
		assert.ok(Math.abs(back.x - q.x) < 1e-9 && Math.abs(back.y - q.y) < 1e-9);
	}
	// upright, no scale: a plain offset
	assert.deepEqual(toLocal({ ...f, rotation: 0, scale: 1 }, { x: 110, y: 60 }), { x: 10, y: 10 });
});

test('the base keeps the corner inset; the nearest side jumps at the corners', () => {
	// 1rem = 16 px from the corners
	assert.deepEqual(tailBase('bottom', 0, 200, 80, 16).point, { x: 16, y: 80 });
	assert.deepEqual(tailBase('bottom', 1, 200, 80, 16).point, { x: 184, y: 80 });
	assert.deepEqual(tailBase('right', 0.5, 200, 80, 16).point, { x: 200, y: 40 });
	assert.deepEqual(tailBase('left', 0, 200, 80, 16).normal, { x: -1, y: 0 });
	// a side shorter than two insets: its middle
	assert.deepEqual(tailBase('left', 0.9, 200, 20, 16).point, { x: 0, y: 10 });
	assert.deepEqual(nearestBase({ x: 100, y: 120 }, 200, 80, 16), { side: 'bottom', at: 0.5 });
	assert.deepEqual(nearestBase({ x: -30, y: 40 }, 200, 80, 16), { side: 'left', at: 0.5 });
	// into the top-right corner: the top side, held at the inset
	assert.deepEqual(nearestBase({ x: 199, y: -40 }, 200, 80, 16), { side: 'top', at: 0.92 });
	assert.deepEqual(nearestBase({ x: 260, y: 1 }, 200, 80, 16), { side: 'right', at: 0.2 });
	// the base width never crosses a corner
	assert.equal(tailHalfWidth(40, 16, 200), 14.4);
	assert.equal(tailHalfWidth(10, 16, 200), 7.5);
});

test('speech wedge and thought circles', () => {
	const base = tailBase('bottom', 0.5, 200, 80, 16);
	const tip = { x: 60, y: 200 };
	assert.equal(tipOutside(base, tip), true);
	assert.equal(tipOutside(base, { x: 60, y: 40 }), false);
	const w = speechTail(base, tip, 10, 3);
	assert.equal(w.stroke, 'M90 80 L60 200 L110 80');
	assert.equal(w.fill, 'M90 77 L90 80 L60 200 L110 80 L110 77 Z');
	const c = thoughtTail(base, { x: 100, y: 300 }, 20);
	assert.equal(c.length, 3);
	assert.ok(c[0].r > c[1].r && c[1].r > c[2].r);
	assert.deepEqual([c[2].cx, c[2].cy], [100, 300]);
	// the largest is off the bubble
	assert.ok(c[0].cy - c[0].r > 80);
	assert.ok(c[0].cy < c[1].cy && c[1].cy < c[2].cy);
});
