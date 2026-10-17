// The home slider's autoplay rules (components/home/slides-autoplay.ts; TEXT-ACROSS-SLICES.md §9):
//   npm test -w viewer
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { autoplayOn, autoplayRuns, type AutoplayHolds } from '../src/lib/components/home/slides-autoplay.ts';

const free: AutoplayHolds = { hover: false, focus: false, touch: false, dragging: false, hidden: false };

test('autoplay is on unless it is off or the reader prefers reduced motion', () => {
	assert.equal(autoplayOn(5000, false), true);
	assert.equal(autoplayOn(5000, true), false);
	assert.equal(autoplayOn(0, false), false);
});

test('the countdown runs only while on, with more slides than fit, and nothing holds it', () => {
	assert.equal(autoplayRuns(true, true, free), true);
	assert.equal(autoplayRuns(false, true, free), false);
	// everything fits: nothing to play
	assert.equal(autoplayRuns(true, false, free), false);
	for (const hold of ['hover', 'focus', 'touch', 'dragging', 'hidden'] as const) assert.equal(autoplayRuns(true, true, { ...free, [hold]: true }), false, hold);
});

test('a hold only pauses: the countdown runs again once it ends (a touch or a drag never stops it for good)', () => {
	assert.equal(autoplayRuns(true, true, { ...free, touch: true, dragging: true }), false);
	assert.equal(autoplayRuns(true, true, free), true);
});
