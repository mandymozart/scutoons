// #321: the clock measurement on synthetic recordings – two decks' clicks in their bands, known offsets, room noise.
import assert from 'node:assert/strict';
import { test } from 'node:test';
import { BROWSER, NATIVE, bandpass, click, measure, onsets, parseWav, report } from '../src/measure.ts';

const RATE = 48_000;

/** a mono WAV, 16-bit PCM */
function wav(x: Float32Array, rate = RATE): Uint8Array {
	const b = new Uint8Array(44 + x.length * 2);
	const v = new DataView(b.buffer);
	const put = (o: number, s: string) => [...s].forEach((c, i) => (b[o + i] = c.charCodeAt(0)));
	put(0, 'RIFF');
	v.setUint32(4, 36 + x.length * 2, true);
	put(8, 'WAVE');
	put(12, 'fmt ');
	v.setUint32(16, 16, true);
	v.setUint16(20, 1, true);
	v.setUint16(22, 1, true);
	v.setUint32(24, rate, true);
	v.setUint32(28, rate * 2, true);
	v.setUint16(32, 2, true);
	v.setUint16(34, 16, true);
	put(36, 'data');
	v.setUint32(40, x.length * 2, true);
	x.forEach((s, i) => v.setInt16(44 + i * 2, Math.max(-1, Math.min(1, s)) * 32767, true));
	return b;
}

const noise = (i: number) => (((i * 2654435761) >>> 0) / 2 ** 32) * 2 - 1;

/** 30 beats at 120 BPM: deck A's click at 1 kHz (1.5 on every fourth), B's at 2.5 kHz (3.5) `off(k)` s later, quiet room noise */
function recording(off: (k: number) => number): Float32Array {
	const x = new Float32Array(16 * RATE);
	for (let k = 0; k < 30; k++) {
		const t = 0.5 + k * 0.5;
		const a = click(RATE, k % 4 ? NATIVE.freq : 1500, t, 16, 0.4);
		const b = click(RATE, k % 4 ? BROWSER.freq : 3500, t + off(k), 16, 0.25);
		for (let i = Math.round(t * RATE) - 2000; i < Math.min(x.length, Math.round(t * RATE) + 9000); i++) if (i >= 0) x[i] += a[i] + b[i];
	}
	for (let i = 0; i < x.length; i++) x[i] += 0.003 * noise(i);
	return x;
}

test('the WAV is read and a band finds its clicks only', () => {
	const x = recording(() => 0.004);
	const a = parseWav(wav(x));
	assert.equal(a.rate, RATE);
	assert.equal(a.samples.length, x.length);
	assert.equal(onsets(bandpass(a.samples, RATE, NATIVE), RATE).length, 30);
	assert.equal(onsets(bandpass(a.samples, RATE, BROWSER), RATE).length, 30);
});

test('a steady 3 ms: the offsets say so to a quarter of a millisecond', () => {
	const r = measure(wav(recording(() => -0.003)));
	assert.equal(r.pairs, 30);
	assert.ok(Math.abs(r.median - 3) < 0.25, `median ${r.median}`);
	assert.ok(r.within);
});

test('a jittery follower: p95 and max of the offsets', () => {
	// B jitters ±6 ms around A (a deterministic spread)
	const off = (k: number) => 0.006 * Math.sin(k * 1.7);
	const r = measure(wav(recording(off)));
	assert.equal(r.pairs, 30);
	const want = Array.from({ length: 30 }, (_, k) => Math.abs(off(k)) * 1e3).sort((p, q) => p - q);
	assert.ok(Math.abs(r.max - want[29]) < 0.2, `max ${r.max} vs ${want[29]}`);
	assert.ok(!r.within, 'past ±5 ms');
});

test('report pairs the nearest, skips what has no partner', () => {
	const r = report([1, 1.5, 2, 9], [1.002, 1.499, 2.01], 0.1, 5);
	assert.deepEqual(r.offsets.map((x) => Math.round(x)), [-2, 1, -10]);
	assert.equal(r.pairs, 3);
	assert.equal(Math.round(r.max), 10);
});
