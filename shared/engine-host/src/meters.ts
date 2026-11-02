// meters.ts (#282): the per-output peaks of the engine's analysis frames as a meter reads them. Pure – no DOM, no wasm.
// live-core reports each audio output's largest sample since the last frame (≈ 60 a second); the host samples its
// snapshot only every 250 ms, so a bare frame would miss most peaks. `OutputMeters` holds each peak and lets it fall.

/** how fast a held peak falls: it loses 63 % per this many seconds (a meter, not a scope) */
export const FALL_SECONDS = 0.25;

const clamp01 = (v: number): number => (Number.isFinite(v) ? Math.min(1, Math.max(0, v)) : 0);

export class OutputMeters {
	private held: number[] = [];

	/** One analysis frame's peaks (0..1 per output) `dt` seconds after the one before. The number of outputs follows the
	 *  engine's (the stereo master = 2); a held peak only falls until the next one is higher. */
	push(peaks: ArrayLike<number>, dt: number): void {
		const keep = Math.exp(-Math.max(0, dt) / FALL_SECONDS);
		const n = peaks.length;
		if (this.held.length > n) this.held.length = n;
		for (let i = 0; i < n; i++) this.held[i] = Math.max(clamp01(peaks[i]), (this.held[i] ?? 0) * keep);
	}

	/** the readings, 0..1 per output ([] = no sound yet) */
	read(): number[] {
		return this.held.slice();
	}

	reset(): void {
		this.held = [];
	}
}
