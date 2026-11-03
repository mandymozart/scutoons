// meters.ts (#282): the per-output peaks of the engine's analysis frames as a meter reads them. Pure – no DOM, no wasm.
// live-core reports each audio output's largest sample since the last frame (≈ 60 a second); the host samples its
// snapshot only every 250 ms, so a bare frame would miss most peaks. `OutputMeters` holds each peak and lets it fall.

/** how fast a held peak falls: it loses 63 % per this many seconds (a meter, not a scope) */
export const FALL_SECONDS = 0.25;

// core-only-ok: #373 a peak made safe to draw (moves with the meter ballistics)
const clamp01 = (v: number): number => (Number.isFinite(v) ? Math.min(1, Math.max(0, v)) : 0);

/** the most outputs the engine reports – live-core's MAX_OUTPUTS */
export const MAX_OUTPUTS = 8;

export class OutputMeters {
	private readonly held = new Float32Array(MAX_OUTPUTS);
	private count = 0;
	private shown = this.held.subarray(0, 0);

	/** One analysis frame's peaks (0..1 per output) `dt` seconds after the one before. The number of outputs follows the
	 *  engine's (the stereo master = 2); a held peak only falls until the next one is higher. No allocation per frame
	 *  (a new view only when the count changes). */
	push(peaks: ArrayLike<number>, dt: number): void {
		const keep = Math.exp(-Math.max(0, dt) / FALL_SECONDS);
		const n = Math.min(peaks.length, MAX_OUTPUTS);
		if (n !== this.count) {
			for (let i = n; i < this.count; i++) this.held[i] = 0;
			this.count = n;
			this.shown = this.held.subarray(0, n);
		}
		for (let i = 0; i < n; i++) this.held[i] = Math.max(clamp01(peaks[i]), this.held[i] * keep);
	}

	/** The readings, 0..1 per output ([] = no sound yet): one array updated in place by `push` – the same object every
	 *  time while the count holds, so a requestAnimationFrame loop reads it without going through the snapshot. */
	get peaks(): Float32Array {
		return this.shown;
	}

	/** a copy as a plain array (the snapshot's) */
	read(): number[] {
		return Array.from(this.shown);
	}

	reset(): void {
		this.held.fill(0);
		this.count = 0;
		this.shown = this.held.subarray(0, 0);
	}
}
