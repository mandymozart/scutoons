// The home slider's autoplay rules (components/home/slides-autoplay.ts; WCAG 2.2.2, TEXT-ACROSS-SLICES.md §9):
//   npm test -w viewer
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { autoplayAfter, autoplayRuns, autoplayStarts, type AutoplayHolds } from '../src/lib/components/home/slides-autoplay.ts';

const free: AutoplayHolds = { hover: false, focus: false, dragging: false, hidden: false };

test('a page view starts playing unless autoplay is off or the reader prefers reduced motion', () => {
	assert.equal(autoplayStarts(5000, false), true);
	assert.equal(autoplayStarts(5000, true), false);
	assert.equal(autoplayStarts(0, false), false);
});

test('the countdown runs only while playing, with more slides than fit, and nothing holds it', () => {
	assert.equal(autoplayRuns(true, true, free), true);
	assert.equal(autoplayRuns(false, true, free), false);
	// everything fits: nothing to play
	assert.equal(autoplayRuns(true, false, free), false);
	for (const hold of ['hover', 'focus', 'dragging', 'hidden'] as const) assert.equal(autoplayRuns(true, true, { ...free, [hold]: true }), false, hold);
});

test('a hold only pauses: playing stays on, the countdown runs again once it ends', () => {
	const playing = true;
	assert.equal(autoplayRuns(playing, true, { ...free, hover: true }), false);
	assert.equal(autoplayRuns(playing, true, free), true);
});

test('an interaction or reduced motion switched on stops it; the control toggles it, only when it can move', () => {
	assert.equal(autoplayAfter(true, 'interact', true), false);
	assert.equal(autoplayAfter(false, 'interact', true), false);
	assert.equal(autoplayAfter(true, 'reduced', true), false);
	assert.equal(autoplayAfter(true, 'toggle', true), false);
	assert.equal(autoplayAfter(false, 'toggle', true), true);
	assert.equal(autoplayAfter(false, 'toggle', false), false);
});
