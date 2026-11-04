// clock.ts (#321): the core's tempo clock as the page keeps it – the state block's names and decoder are GENERATED from the
// Rust layout (clock-state.ts, #372), re-exported here, and the page's clock. No decision and no measurement here: the
// core decides when to ping, what anchor a leader sends, where the beat is, the picker's choice, the MIDI clock out and
// the strobe's flash (#370), measures the error on the show's line and words it (#387: `LiveCore.clockReport`); the 3D
// draws the beat between two clock frames itself (deck-space-web `set_beat`).
export * from './clock-state.ts';

/** the page's clock: ms since the epoch, the same in the page, its workers and the worklet's stamps */
export const nowMs = (): number => performance.timeOrigin + performance.now();
