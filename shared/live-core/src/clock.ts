// clock.ts (#321): the core's tempo clock as the page keeps it – the state block's names and decoder are GENERATED from the
// Rust layout (clock-state.ts, #372), re-exported here; what stays is the page's own measurement over the last minute and
// the drawing of the beat between two clock frames. No decision here: the core decides when to ping, what anchor a leader
// sends, where the beat is, the picker's choice, the MIDI clock out and the strobe's flash (#370).
import type { ClockState } from './clock-state.ts';

export * from './clock-state.ts';

/** the error's spread over the window (ms): its size's p50 / p95 / max, its signed mean, the frames counted */
export interface ClockSummary {
	n: number;
	p50: number;
	p95: number;
	max: number;
	mean: number;
	seconds: number;
}

/** past this the core's controller snaps onto the show's line instead of nudging (deck-core tempo::sync) */
const SNAP_MS = 50;

/** #321: how far off the show's line this deck played over the last minute (the HUD's SHOW ±x ms, kept) – the sound
 *  report's measurement without a microphone. Counts the frames that follow an anchor (not a leader's own line). */
export class ClockWindow {
	private readonly at: number[] = [];
	private readonly error: number[] = [];
	private readonly spanMs: number;
	private settled = false;
	constructor(spanMs = 60_000) {
		this.spanMs = spanMs;
	}

	add(ms: number, c: ClockState): void {
		if (!c.locked || !c.anchor || c.leading) {
			this.settled = false;
			return;
		}
		// the join's first frames are the deck before its snap onto the show's line (minutes off) – counted from the line on
		if (!this.settled && Math.abs(c.errorMs) > SNAP_MS) return;
		this.settled = true;
		this.at.push(ms);
		this.error.push(c.errorMs);
		while (this.at.length && this.at[0] < ms - this.spanMs) {
			this.at.shift();
			this.error.shift();
		}
	}

	summary(): ClockSummary | null {
		const n = this.error.length;
		if (!n) return null;
		const size = this.error.map(Math.abs).sort((a, b) => a - b);
		const at = (q: number) => size[Math.min(n - 1, Math.floor(q * n))];
		const mean = this.error.reduce((s, e) => s + e, 0) / n;
		return { n, p50: at(0.5), p95: at(0.95), max: size[n - 1], mean, seconds: (this.at[n - 1] - this.at[0]) / 1000 };
	}
}

/** the beat the clock's line says at the page's `ms` (a drawing between two clock frames) */
// core-only-ok: #370 the core's line drawn between two 25 ms clock frames (the wasm runs in the worker)
export const beatAt = (c: ClockState, ms: number): number => (c.running ? c.beat + ((ms - c.heardMs) / 60_000) * c.bpm : c.beat);

/** the page's clock: ms since the epoch, the same in the page, its workers and the worklet's stamps */
export const nowMs = (): number => performance.timeOrigin + performance.now();
