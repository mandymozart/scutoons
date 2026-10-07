// A complexity check instead of a wall-clock limit (a busy machine made the old "< 3000 ms" assertions flaky): the
// work for n and for 2n is timed (best of a few runs, after a warm-up) – linear work takes about twice as long for 2n,
// quadratic work four times. The bound is generous on both sides: at most 3× (plus a few ms of timer noise).
// The n and 2n runs are interleaved, so a load spike hits both, and the comparison is repeated up to `attempts` times:
// a real complexity regression fails every attempt, one stall of the machine (parallel builds, GC – chapterFlow's
// "lone divert" test failed once that way on 2026-10-07 at n = 400, ~5 ms vs a 30 ms+ stall) does not.
import assert from 'node:assert/strict';
import { performance } from 'node:perf_hooks';

const time = (fn: () => unknown): number => {
	const t = performance.now();
	fn();
	return performance.now() - t;
};

export function assertLinear(work: (n: number) => unknown, n: number, runs = 3, attempts = 4): void {
	work(n); // warm-up (JIT)
	work(2 * n);
	const seen: string[] = [];
	for (let a = 0; a < attempts; a++) {
		let one = Infinity;
		let two = Infinity;
		for (let i = 0; i < runs; i++) {
			one = Math.min(one, time(() => work(n)));
			two = Math.min(two, time(() => work(2 * n)));
		}
		if (two < 3 * one + 30) return;
		seen.push(`${one.toFixed(1)} ms → ${two.toFixed(1)} ms`);
	}
	assert.fail(`not linear: n = ${n} vs 2n, every attempt: ${seen.join(', ')}`);
}
