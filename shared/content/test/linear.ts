// A complexity check instead of a wall-clock limit (a busy machine made the old "< 3000 ms" assertions flaky): the
// work for n and for 2n is timed (best of a few runs, after a warm-up) – linear work takes about twice as long for 2n,
// quadratic work four times. The bound is generous on both sides: at most 3× (plus a few ms of timer noise).
import assert from 'node:assert/strict';
import { performance } from 'node:perf_hooks';

const best = (fn: () => unknown, runs: number): number => {
	let min = Infinity;
	for (let i = 0; i < runs; i++) {
		const t = performance.now();
		fn();
		min = Math.min(min, performance.now() - t);
	}
	return min;
};

export function assertLinear(work: (n: number) => unknown, n: number, runs = 3): void {
	work(n); // warm-up (JIT)
	const one = best(() => work(n), runs);
	const two = best(() => work(2 * n), runs);
	assert.ok(two < 3 * one + 30, `not linear: n = ${n} took ${one.toFixed(1)} ms, 2n took ${two.toFixed(1)} ms`);
}
