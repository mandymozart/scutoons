// GENERATED from deck/live-core/src/clock.rs and deck-core's tempo by `npm run generate` (cargo run -p live-core --features codegen --bin codecs-ts) – do not edit.
// The core's tempo clock as the page reads it (#321, #370, #372): the state block a clock frame leaves (clock.rs `Slot`s,
// f64 each) read into names – the slots' indices, the flag bits, the clock's sources, and the MIDI messages that reach the
// clock. No decision here: the core decides when to ping, what anchor a leader sends, where the beat is, the picker's
// choice, the MIDI clock out and the strobe's flash. The tables are the Rust side's (Slot, FLAGS, READS, GROUPS).

/** deck-core's `Source::ALL`, in order (the picker's index) */
export const SOURCES = @@SOURCES@@ as const;
export type ClockSource = (typeof SOURCES)[number];

/** the MIDI messages that reach the clock (deck-core's `CLOCK_STATUS`: tick, start, continue, stop, song position) – the port's
 *  filter; notes, controllers and active sensing never reach the core */
export const CLOCK_MESSAGES: ReadonlySet<number> = new Set(@@MESSAGES@@);

/** the state block's length */
export const STATE_LENGTH = @@STATE@@;
/** the state block's slots */
export const SLOT = {
@@SLOTS@@
} as const;
/** the flags slot's bits */
export const FLAG = {
@@FLAGS@@
} as const;

@@TYPES@@

/** the state block → names */
export function clockOf(s: ArrayLike<number>): ClockState {
	return {
@@CLOCK_OF@@
		line: ''
	};
}
