// `npm test -w shared/live-deck` (#228, #369): a Gamepad API pad on the deck's sticks and selection, without a browser,
// against the core's answers (test/fixtures/controls.json – deck-space's controls ran the same script the native stage's
// pad test runs): the poll hands in the first connected pad's state (none: an empty one), the core's events become the
// touch sticks' calls and the selection the core moved on.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { pollPad, type PadSink, type PadState } from '../src/gamepad.ts';
import { CONTROLS, controlsOf } from './controls-core.ts';

function recorder(selection: () => string) {
	const calls: string[] = [];
	const sink: PadSink = {
		stickDown: (s) => calls.push(`down ${s}`),
		stickMove: (s, dx, dy) => calls.push(`move ${s} ${dx.toFixed(3)} ${dy.toFixed(3)}`),
		stickUp: (s) => calls.push(`up ${s}`),
		selected: () => calls.push(`select ${selection()}`)
	};
	return { calls, sink };
}

/** the script's frame as a Gamepad (an all-rest frame with nothing pressed: no pad at all, the way a pad goes away) */
const padOf = (f: (typeof CONTROLS.pad)[number]): PadState | null =>
	f.axes.some((a) => a !== 0) || f.buttons.some(Boolean) ? { axes: f.axes, buttons: f.buttons.map((pressed) => ({ pressed })) } : null;

test("the pad script: the core's events become the sticks' calls and the selection, in order", () => {
	const c = controlsOf();
	const { calls, sink } = recorder(() => c.selection());
	const want: string[] = [];
	for (const f of CONTROLS.pad) {
		const pad = padOf(f);
		pollPad(c, [null, pad], sink);
		const handed = c.calls.at(-1) as [string, number[], number[]];
		assert.deepEqual(handed, ['pad', pad ? f.axes : [], pad ? f.buttons.map(Number) : []], 'the first connected pad, as numbers; none: empty');
		for (const [kind, side, dx, dy] of f.events)
			want.push([`down ${side}`, `move ${side} ${dx.toFixed(3)} ${dy.toFixed(3)}`, `up ${side}`, `select ${f.selection}`][kind]);
	}
	assert.deepEqual(calls, want);
	assert.ok(calls.includes('down 0') && calls.includes('up 1') && calls.includes('select being'), 'the script goes down, up and cycles');
});
