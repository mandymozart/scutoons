// gamepad.ts (#228): a Gamepad API pad on the deck's two corner sticks and on the selection. The pad's sticks are the
// touch sticks' deflection (stick radii, y up – `deflection`, the same function the corner zones use); the stick's own
// logic (dead zone, gain, relative moves from where it landed) stays deck-space's (Hands → DeckControls, Rust). Here:
// when a stick is pushed past `press` it goes down, follows its deflection while it is out, and goes up once it comes
// back inside `release` (a little lower, so a stick resting near the edge doesn't chatter). A (button 0) cycles the
// selection on its press edge. Standard mapping: axes 0/1 the left stick, 2/3 the right; button 0 A.
import { deflection } from './controls.ts';

/** what the pad drives: the same calls the touch sticks make on Hands */
export interface PadSink {
	stickDown(side: number): void;
	stickMove(side: number, dx: number, dy: number): void;
	stickUp(side: number): void;
}

/** the parts of a Gamepad the poll reads (a stub in tests) */
export interface PadState {
	readonly axes: readonly number[];
	readonly buttons: readonly { readonly pressed: boolean }[];
}

export const PAD_SELECT = 0;
/** where each side's [x, y] axes sit in the standard mapping */
export const PAD_STICK_AXES: readonly [number, number][] = [
	[0, 1],
	[2, 3]
];

/** the next selection after `current` in `names` (wraps; an unknown current starts at the first) */
export function nextSelection(current: string, names: readonly string[]): string {
	if (!names.length) return current;
	return names[(names.indexOf(current) + 1) % names.length];
}

/** The selections A cycles through, as the controls name them: master, the being, then every knob. */
export function selectionNames(knobs: readonly string[]): string[] {
	return ['', 'being', ...knobs.map((p) => `knob:${p}`)];
}

export class PadSticks {
	private readonly down = [false, false];
	private selectPressed = false;

	constructor(
		private readonly sink: PadSink,
		/** A's press: the selection moves on (the page's choose) */
		private readonly onSelect: () => void,
		private readonly press = 0.3,
		private readonly release = 0.15
	) {}

	/** One frame: the first connected pad (null = none). Sticks that were down and no pad is left go up. */
	poll(pads: readonly (PadState | null | undefined)[]) {
		const pad = pads.find((p) => !!p);
		if (!pad) {
			this.reset();
			return;
		}
		PAD_STICK_AXES.forEach(([ax, ay], side) => {
			const x = pad.axes[ax] ?? 0;
			const y = pad.axes[ay] ?? 0;
			const [dx, dy] = deflection([0, 0], [x, y], 1);
			const reach = Math.hypot(x, y);
			if (!this.down[side]) {
				if (reach < this.press) return;
				this.down[side] = true;
				this.sink.stickDown(side);
			} else if (reach <= this.release) {
				this.down[side] = false;
				this.sink.stickUp(side);
				return;
			}
			this.sink.stickMove(side, dx, dy);
		});
		const select = !!pad.buttons[PAD_SELECT]?.pressed;
		if (select && !this.selectPressed) this.onSelect();
		this.selectPressed = select;
	}

	/** the sticks that are down go up (the pad went away, or the page is closing) */
	reset() {
		this.down.forEach((d, side) => {
			if (!d) return;
			this.down[side] = false;
			this.sink.stickUp(side);
		});
		this.selectPressed = false;
	}
}
