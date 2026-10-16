// The endless band's drag hint – when it plays (components/toon/renderer/strip-nudge.ts; owner 2026-10-16 Q10,
// TEXT-ACROSS-SLICES.md §8):   npm test -w viewer
// Timers are a hand-driven clock; the cue is a stand-in that records its plays and cancels.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { NUDGE_DWELL, nudgeController, nudgeFrame, shownPart, type NudgeEnv } from '../src/lib/components/toon/renderer/strip-nudge.ts';

function setup(over: Partial<{ hints: boolean; blocked: boolean; busy: boolean; movable: boolean }> = {}) {
	const env = { hints: true, blocked: false, busy: false, movable: true, ...over };
	let now = 0;
	const timers = new Map<number, { at: number; fn: () => void }>();
	let next = 1;
	const log = { plays: 0, cancels: 0, end: null as null | (() => void) };
	const nudge = nudgeController({
		hints: () => env.hints,
		blocked: () => env.blocked,
		busy: () => env.busy,
		movable: () => env.movable,
		play(end) {
			log.plays++;
			log.end = end;
			return () => void log.cancels++;
		},
		setTimer(fn, ms) {
			timers.set(next, { at: now + ms, fn });
			return next++;
		},
		clearTimer: (handle) => void timers.delete(handle as number)
	} satisfies NudgeEnv);
	/** the clock runs `ms` on, firing what is due in order */
	function advance(ms: number) {
		const until = now + ms;
		for (;;) {
			const due = [...timers].filter(([, t]) => t.at <= until).sort((a, b) => a[1].at - b[1].at)[0];
			if (!due) break;
			timers.delete(due[0]);
			now = due[1].at;
			due[1].fn();
		}
		now = until;
	}
	return { env, nudge, log, advance, timers };
}

test('in view with hints on: plays once after the dwell, never again in this page view', () => {
	const { nudge, log, advance } = setup();
	nudge.view(true);
	advance(NUDGE_DWELL - 1);
	assert.equal(log.plays, 0);
	advance(1);
	assert.equal(log.plays, 1);
	assert.equal(nudge.state, 'playing');
	log.end!();
	assert.equal(nudge.state, 'done');
	nudge.view(false);
	nudge.view(true);
	advance(NUDGE_DWELL * 5);
	assert.equal(log.plays, 1);
});

test('scrolled past within the dwell: nothing; back in view: the dwell starts anew', () => {
	const { nudge, log, advance } = setup();
	nudge.view(true);
	advance(NUDGE_DWELL / 2);
	nudge.view(false);
	advance(NUDGE_DWELL * 2);
	assert.equal(log.plays, 0);
	assert.equal(nudge.state, 'idle');
	nudge.view(true);
	advance(NUDGE_DWELL);
	assert.equal(log.plays, 1);
});

test('hints off: never – switched on later while in view (the member answer arrived), it plays', () => {
	const { env, nudge, log, advance } = setup({ hints: false });
	nudge.view(true);
	advance(NUDGE_DWELL * 3);
	assert.equal(log.plays, 0);
	env.hints = true;
	nudge.changed();
	advance(NUDGE_DWELL);
	assert.equal(log.plays, 1);
});

test('hints switched off during the dwell or while it plays: nothing / cancelled', () => {
	const a = setup();
	a.nudge.view(true);
	a.advance(NUDGE_DWELL / 2);
	a.env.hints = false;
	a.nudge.changed();
	a.advance(NUDGE_DWELL * 2);
	assert.equal(a.log.plays, 0);
	const b = setup();
	b.nudge.view(true);
	b.advance(NUDGE_DWELL);
	b.env.hints = false;
	b.nudge.changed();
	assert.equal(b.log.cancels, 1);
	assert.equal(b.nudge.state, 'done');
});

test('the story blocking the page: not while it blocks; it plays once the story lets go', () => {
	const { env, nudge, log, advance } = setup({ blocked: true });
	nudge.view(true);
	advance(NUDGE_DWELL * 3);
	assert.equal(log.plays, 0);
	env.blocked = false;
	nudge.changed();
	advance(NUDGE_DWELL);
	assert.equal(log.plays, 1);
	env.blocked = true;
	nudge.changed();
	assert.equal(log.cancels, 1);
});

test('a band that is whole and still: never', () => {
	const { nudge, log, advance } = setup({ movable: false });
	nudge.view(true);
	advance(NUDGE_DWELL * 3);
	assert.equal(log.plays, 0);
});

test('a drag / glide running at the dwell end: it waits until the band rests, a whole dwell', () => {
	const { env, nudge, log, advance } = setup({ busy: true });
	nudge.view(true);
	advance(NUDGE_DWELL * 2);
	assert.equal(log.plays, 0);
	assert.equal(nudge.state, 'armed');
	env.busy = false;
	advance(NUDGE_DWELL);
	assert.equal(log.plays, 1);
});

test('the reader moving the band spends a waiting nudge and cancels a running one', () => {
	const a = setup();
	a.nudge.view(true);
	a.advance(NUDGE_DWELL / 2);
	a.nudge.touch();
	a.advance(NUDGE_DWELL * 3);
	assert.equal(a.log.plays, 0);
	assert.equal(a.nudge.state, 'done');
	const b = setup();
	b.nudge.view(true);
	b.advance(NUDGE_DWELL);
	b.nudge.touch();
	assert.equal(b.log.cancels, 1);
	b.nudge.view(false);
	b.nudge.view(true);
	b.advance(NUDGE_DWELL * 3);
	assert.equal(b.log.plays, 1);
});

test('a pointer going down: a waiting nudge waits a whole dwell again, a running one is cancelled', () => {
	const { nudge, log, advance } = setup();
	nudge.view(true);
	advance(NUDGE_DWELL - 100);
	nudge.poke();
	advance(NUDGE_DWELL - 1);
	assert.equal(log.plays, 0);
	advance(1);
	assert.equal(log.plays, 1);
	nudge.poke();
	assert.equal(log.cancels, 1);
	assert.equal(nudge.state, 'done');
});

test('leaving the view while it plays cancels it; destroy clears a waiting timer', () => {
	const a = setup();
	a.nudge.view(true);
	a.advance(NUDGE_DWELL);
	a.nudge.view(false);
	assert.equal(a.log.cancels, 1);
	const b = setup();
	b.nudge.view(true);
	b.nudge.destroy();
	assert.equal(b.timers.size, 0);
	b.advance(NUDGE_DWELL * 2);
	assert.equal(b.log.plays, 0);
});

test('the swept frame: the first slice when half of what can show of it is in the window, else the most visible', () => {
	// 390 phone, frames 390 wide: the band at its start
	assert.equal(nudgeFrame([{ l: 0, w: 390 }, { l: 390, w: 390 }], 390), 0);
	// frame 0 mostly out to the left: frame 1 shows more
	assert.equal(nudgeFrame([{ l: -300, w: 390 }, { l: 90, w: 390 }], 390), 1);
	// frame 0 just half in
	assert.equal(nudgeFrame([{ l: -195, w: 390 }, { l: 195, w: 390 }], 390), 0);
	// a wide window (1440): frame 0 whole at the left
	assert.equal(nudgeFrame([{ l: 0, w: 386 }, { l: 386, w: 386 }, { l: 772, w: 386 }, { l: 1158, w: 386 }], 1440), 0);
	// the page arrived at slice 3: frame 0 wrapped to the far right, off screen
	assert.equal(nudgeFrame([{ l: 1544, w: 386 }, { l: -772, w: 386 }, { l: -386, w: 386 }, { l: 0, w: 386 }], 1440), 3);
	assert.equal(nudgeFrame([], 390), -1);
});

test('the line spans the part of the frame on screen: a 443 wide first slice in a 390 window, one scrolled in from the left', () => {
	assert.deepEqual(shownPart({ l: 0, w: 443 }, 390), [0, 390]);
	assert.deepEqual(shownPart({ l: -100, w: 390 }, 390), [100, 390]);
	assert.deepEqual(shownPart({ l: 0, w: 386 }, 1440), [0, 386]);
});
