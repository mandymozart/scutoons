import { describe, it } from 'node:test';
import { expect } from './expect.ts';
import {
	EMPTY_STAGE,
	addTo,
	cellOf,
	cellRect,
	keepOn,
	offStage,
	placeOn,
	readStageState,
	slotRects,
	viewsOn,
	zoneAt,
	zoneRects,
	type StageState
} from '../src/editor/stage.ts';

const one = (v: string): StageState => ({ layout: 'one', slots: [v] });

describe('the stage: four layouts, a view per cell', () => {
	it('lays the cells out over the whole window', () => {
		const area = { x: 0, y: 0, w: 1000, h: 800 };
		expect(cellRect('whole', area)).toEqual(area);
		expect(cellRect('left', area)).toEqual({ x: 0, y: 0, w: 500, h: 800 });
		expect(cellRect('bottom-right', area)).toEqual({ x: 500, y: 400, w: 500, h: 400 });
		expect(slotRects({ layout: 'half-right', slots: ['a', null, 'b'] }, area).map((s) => [s.cell, s.view])).toEqual([
			['top-left', 'a'],
			['bottom-left', null],
			['right', 'b']
		]);
	});

	it('the top edge: the view takes the whole stage, the others go back to their panels', () => {
		expect(placeOn(EMPTY_STAGE, 'flow', 'whole')).toEqual(one('flow'));
		expect(placeOn({ layout: 'quarters', slots: ['a', 'b', 'c', 'd'] }, 'flow', 'whole')).toEqual(one('flow'));
	});

	it('an edge: that half, the view there keeps the other side', () => {
		expect(placeOn(EMPTY_STAGE, 'flow', 'left')).toEqual({ layout: 'half-left', slots: ['flow', null, null] });
		expect(placeOn(one('a'), 'flow', 'left')).toEqual({ layout: 'half-left', slots: ['flow', 'a', null] });
		expect(placeOn(one('a'), 'flow', 'right')).toEqual({ layout: 'half-right', slots: ['a', null, 'flow'] });
		// four quarters to a left half: the right column stays, the left one leaves
		expect(placeOn({ layout: 'quarters', slots: ['a', 'b', 'c', 'd'] }, 'flow', 'left')).toEqual({ layout: 'half-left', slots: ['flow', 'b', 'd'] });
	});

	it('a corner: a quarter – the half beside it stays, a half on its side splits in two', () => {
		expect(placeOn(one('a'), 'flow', 'top-right')).toEqual({ layout: 'half-left', slots: ['a', 'flow', null] });
		expect(placeOn(one('a'), 'flow', 'bottom-left')).toEqual({ layout: 'half-right', slots: [null, 'flow', 'a'] });
		expect(placeOn({ layout: 'half-left', slots: ['a', 'b', 'c'] }, 'flow', 'bottom-right')).toEqual({ layout: 'half-left', slots: ['a', 'b', 'flow'] });
		expect(placeOn({ layout: 'half-left', slots: ['a', 'b', 'c'] }, 'flow', 'top-left')).toEqual({ layout: 'quarters', slots: ['flow', 'b', 'a', 'c'] });
		expect(placeOn({ layout: 'quarters', slots: ['a', 'b', 'c', 'd'] }, 'flow', 'top-right')).toEqual({ layout: 'quarters', slots: ['a', 'flow', 'c', 'd'] });
	});

	it('a view moved on the stage leaves its old cell', () => {
		const s = placeOn({ layout: 'half-left', slots: ['a', 'b', 'c'] }, 'b', 'bottom-right');
		expect(s).toEqual({ layout: 'half-left', slots: ['a', 'c', 'b'] });
		expect(viewsOn(s)).toEqual(['a', 'c', 'b']);
		expect(cellOf(s, 'b')).toBe('bottom-right');
	});

	it('Views puts a view on the stage: the whole, a free cell, a grown layout', () => {
		expect(addTo(EMPTY_STAGE, 'a')).toEqual(one('a'));
		expect(addTo(one('a'), 'a')).toEqual(one('a'));
		expect(addTo(one('a'), 'b')).toEqual({ layout: 'half-left', slots: ['a', 'b', null] });
		expect(addTo({ layout: 'half-left', slots: ['a', 'b', null] }, 'c')).toEqual({ layout: 'half-left', slots: ['a', 'b', 'c'] });
		expect(addTo({ layout: 'half-left', slots: ['a', 'b', 'c'] }, 'd')).toEqual({ layout: 'quarters', slots: ['d', 'b', 'a', 'c'] });
		expect(addTo({ layout: 'quarters', slots: ['a', 'b', 'c', 'd'] }, 'e')).toEqual({ layout: 'quarters', slots: ['a', 'b', 'c', 'e'] });
	});

	it('a view off the stage frees its cell; the last one empties the stage', () => {
		expect(offStage({ layout: 'half-left', slots: ['a', 'b', null] }, 'a')).toEqual({ layout: 'half-left', slots: [null, 'b', null] });
		expect(offStage(one('a'), 'a')).toEqual(EMPTY_STAGE);
		expect(offStage(one('a'), 'x')).toEqual(one('a'));
		expect(keepOn({ layout: 'half-left', slots: ['a', 'b', 'c'] }, (v) => v !== 'b')).toEqual({ layout: 'half-left', slots: ['a', null, 'c'] });
	});

	it('reads a stored stage, every part checked', () => {
		expect(readStageState({ layout: 'half-left', slots: ['a', 'a', 3] })).toEqual({ layout: 'half-left', slots: ['a', null, null] });
		expect(readStageState({ layout: 'one', slots: ['a', 'b'] })).toEqual(one('a'));
		expect(readStageState({ layout: 'thirds', slots: ['a'] })).toEqual(EMPTY_STAGE);
		expect(readStageState({ layout: 'quarters', slots: [null, null] })).toEqual(EMPTY_STAGE);
		expect(readStageState(undefined)).toEqual(EMPTY_STAGE);
	});
});

describe('the drop zones: to the screen’s very edges', () => {
	const win = { w: 1440, h: 900 };
	const z = { band: 32, corner: 96 };

	it('finds the corner along either edge, the top edge, the sides; nothing in the middle or along the bottom', () => {
		expect(zoneAt(0, 0, win, z)).toBe('top-left');
		expect(zoneAt(80, 10, win, z)).toBe('top-left');
		expect(zoneAt(10, 80, win, z)).toBe('top-left');
		expect(zoneAt(1439, 50, win, z)).toBe('top-right');
		expect(zoneAt(5, 899, win, z)).toBe('bottom-left');
		expect(zoneAt(1430, 850, win, z)).toBe('bottom-right');
		expect(zoneAt(700, 0, win, z)).toBe('whole');
		expect(zoneAt(700, -20, win, z)).toBe('whole');
		expect(zoneAt(0, 450, win, z)).toBe('left');
		expect(zoneAt(1440, 450, win, z)).toBe('right');
		expect(zoneAt(700, 450, win, z)).toBeNull();
		expect(zoneAt(700, 899, win, z)).toBeNull();
		// a panel's head under the bar (y ≈ 64) is no corner
		expect(zoneAt(60, 64, win, z)).toBeNull();
	});

	it('draws what each zone covers', () => {
		expect(zoneRects('whole', win, z)).toEqual([{ x: 96, y: 0, w: 1248, h: 32 }]);
		expect(zoneRects('top-left', win, z)).toEqual([
			{ x: 0, y: 0, w: 96, h: 32 },
			{ x: 0, y: 32, w: 32, h: 64 }
		]);
		for (const r of zoneRects('bottom-right', win, z)) expect(zoneAt(r.x + r.w / 2, r.y + r.h / 2, win, z)).toBe('bottom-right');
	});
});
