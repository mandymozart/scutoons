// Block times (#356): the worker's render time per 128-frame quantum in a fixed histogram – p50, p99 and the longest
// of a window without keeping the samples and without allocating per block (a GC pause in the worker is a dropout).
// Bins are logarithmic, eight to an octave from 4 µs (≈ 9 % wide): a quantile is its bin's upper edge.

/** the first bin's lower edge (µs) */
const FLOOR_US = 4;
/** bins per octave */
const PER_OCTAVE = 8;
/** 4 µs · 2^16 ≈ 262 ms – longer blocks land in the last bin (the longest is kept exactly) */
export const BINS = 16 * PER_OCTAVE;

export class BlockTimes {
	private readonly bins = new Uint32Array(BINS);
	count = 0;
	/** the longest block of the window (µs, exact) */
	max = 0;
	/** the sum of the window's blocks (µs) */
	total = 0;

	push(us: number): void {
		const i = us <= FLOOR_US ? 0 : Math.min(BINS - 1, Math.floor(Math.log2(us / FLOOR_US) * PER_OCTAVE));
		this.bins[i]++;
		this.count++;
		this.total += us;
		if (us > this.max) this.max = us;
	}

	/** the time `q` (0..1) of the blocks took at most (µs; the bin's upper edge, never past the longest); 0 when empty */
	quantile(q: number): number {
		if (!this.count) return 0;
		const want = Math.max(1, Math.ceil(q * this.count));
		let seen = 0;
		for (let i = 0; i < BINS; i++) {
			seen += this.bins[i];
			// the last bin has no upper edge: its blocks are as long as the longest at most
			if (seen >= want) return i === BINS - 1 ? this.max : Math.min(this.max, FLOOR_US * 2 ** ((i + 1) / PER_OCTAVE));
		}
		return this.max;
	}

	/** the mean block (µs) */
	get mean(): number {
		return this.count ? this.total / this.count : 0;
	}

	reset(): void {
		this.bins.fill(0);
		this.count = 0;
		this.max = 0;
		this.total = 0;
	}
}
