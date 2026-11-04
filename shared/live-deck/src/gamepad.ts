// gamepad.ts (#228, #369): a Gamepad API pad on the deck's two corner sticks and on the selection. The rules – a stick
// goes down past its press point and up inside its release point, its deflection (y up), A / Y cycling the selection –
// are deck-space's (`DeckControls.pad`, the native stage's pad hands its state to the same). Here only the poll: the first
// connected pad's axes and buttons in, the core's events out to the same calls the touch sticks make.
import type { DeckControlsView } from './controls.ts';

/** what the pad drives: the touch sticks' calls on Hands, and the selection the core moved on */
export interface PadSink {
	stickDown(side: number): void;
	stickMove(side: number, dx: number, dy: number): void;
	stickUp(side: number): void;
	/** a selection button: the core has moved the selection on (`DeckControls.selection`) */
	selected(): void;
}

/** the parts of a Gamepad the poll reads (a stub in tests) */
export interface PadState {
	readonly axes: readonly number[];
	readonly buttons: readonly { readonly pressed: boolean }[];
}

/** the event kinds in `DeckControls.pad`'s answer, four numbers each: [kind, side, dx, dy] */
const DOWN = 0;
const MOVE = 1;
const UP = 2;
const CYCLE = 3;

/** One frame: the first connected pad (none: an empty state – the sticks that were down go up) to the controls, their events to `sink`. */
export function pollPad(c: Pick<DeckControlsView, 'pad'>, pads: readonly (PadState | null | undefined)[], sink: PadSink) {
	const pad = pads.find((p) => !!p);
	const ev = c.pad(pad ? [...pad.axes] : [], pad ? pad.buttons.map((b) => (b.pressed ? 1 : 0)) : []);
	for (let i = 0; i + 3 < ev.length; i += 4) {
		const [kind, side] = [ev[i], ev[i + 1]];
		if (kind === DOWN) sink.stickDown(side);
		else if (kind === MOVE) sink.stickMove(side, ev[i + 2], ev[i + 3]);
		else if (kind === UP) sink.stickUp(side);
		else if (kind === CYCLE) sink.selected();
	}
}
