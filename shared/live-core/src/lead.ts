// The adaptive lead (#208): how far the worker renders ahead of the worklet. A dropout grows it a step (up to a cap) –
// the next stall of the same length then fits; a calm minute shrinks it a step (down to the start), so one hiccup
// doesn't cost latency for the rest of the show. deck-stage's render thread runs the same rule
// (deck/stage/src/audio/lead.rs).

/** a lead shrinks a step after this long without a dropout (ms) */
export const CALM_MS = 60_000;

export class AdaptiveLead {
	readonly min: number;
	readonly cap: number;
	readonly step: number;
	private current: number;
	private calmSince: number | null = null;
	private seen = 0;

	/** starts at `min` (frames); grows and shrinks by `step` within `min … cap` */
	constructor(min: number, cap: number, step: number) {
		this.min = min;
		this.cap = Math.max(cap, min);
		this.step = Math.max(1, step);
		this.current = min;
	}

	get value(): number {
		return this.current;
	}

	/**
	 * The dropout count `dropouts` (a running total) at `now` (ms): new dropouts since the last call grow the lead one
	 * step, a calm minute shrinks it one step (and the next minute starts). Returns the lead.
	 */
	update(now: number, dropouts: number): number {
		if (dropouts !== this.seen) {
			this.seen = dropouts;
			this.current = Math.min(this.current + this.step, this.cap);
			this.calmSince = now;
			return this.current;
		}
		if (this.calmSince === null) this.calmSince = now;
		if (now - this.calmSince >= CALM_MS) {
			this.current = Math.max(this.current - this.step, this.min);
			this.calmSince = now;
		}
		return this.current;
	}
}
