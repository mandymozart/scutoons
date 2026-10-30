// The rings' logic: wrap-around, full / empty, counters past 2^31, records through the byte ring, the analysis slot –
// and one real two-thread run over a SharedArrayBuffer (a Node worker thread writes, the main thread reads).
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { Worker } from 'node:worker_threads';
import { Latest, READ, Ring, ringBuffers, WRITE } from '../src/ring.ts';
import { readRecords, recordSize, writeSend } from '../src/wire.ts';

const seq = (from: number, n: number) => Float32Array.from({ length: n }, (_, i) => from + i);

test('capacity must be a power of two', () => {
	assert.throws(() => ringBuffers(100, 4, false), RangeError);
	assert.equal(Ring.floats(ringBuffers(8, 4, false)).capacity, 8);
});

test('write and read wrap around the end', () => {
	const r = Ring.floats(ringBuffers(8, 4, false));
	assert.equal(r.write(seq(0, 6)), 6);
	const out = new Float32Array(4);
	assert.equal(r.read(out), 4);
	assert.deepEqual([...out], [0, 1, 2, 3]);
	assert.equal(r.write(seq(6, 6)), 6, 'two at the end, four from the start');
	assert.equal(r.available(), 8);
	assert.equal(r.space(), 0);
	const all = new Float32Array(10);
	assert.equal(r.read(all), 8);
	assert.deepEqual([...all.subarray(0, 8)], [4, 5, 6, 7, 8, 9, 10, 11]);
});

test('a full ring takes nothing more, an empty one gives nothing', () => {
	const r = Ring.floats(ringBuffers(4, 4, false));
	assert.equal(r.write(seq(0, 6)), 4, 'only what fits');
	assert.equal(r.write(seq(0, 1)), 0);
	assert.equal(r.writeAll(seq(0, 1)), false);
	const out = new Float32Array(4);
	r.read(out);
	assert.equal(r.read(out), 0);
	assert.equal(r.writeAll(seq(0, 5)), false, 'all or nothing');
	assert.equal(r.available(), 0);
});

test('the counters wrap past 2^31 without losing a sample', () => {
	const r = Ring.floats(ringBuffers(8, 4, false));
	const start = 0x7ffffffc | 0; // four short of int32's end
	Atomics.store(r.state, WRITE, start);
	Atomics.store(r.state, READ, start);
	const out = new Float32Array(3);
	let next = 0;
	for (let k = 0; k < 10; k++) {
		assert.equal(r.write(seq(next, 3)), 3);
		r.read(out);
		assert.deepEqual([...out], [next, next + 1, next + 2]);
		next += 3;
	}
	assert.ok(Atomics.load(r.state, WRITE) < 0, 'the counter went negative (wrapped)');
	assert.equal(r.available(), 0);
});

test('records go through the byte ring whole and come out in order', () => {
	const r = Ring.bytes(ringBuffers(64, 1, false));
	const scratch = new Uint8Array(64);
	const send = (node: number, v: number[]) => r.writeAll(scratch.subarray(0, writeSend(scratch, 0, { mode: 'blend', node, port: 1, amount: 0.5, v })));
	assert.ok(send(1, [1, 2]) && send(2, [3])); // 20 + 16 bytes
	assert.equal(send(3, [4, 5, 6, 7, 8]), false, '32 more bytes do not fit in 64 − 36');
	const inbox = new Uint8Array(64);
	let n = r.read(inbox);
	assert.deepEqual([...readRecords(inbox.subarray(0, n))].map((x) => [x.node, ...x.v]), [[1, 1, 2], [2, 3]]);
	// now one record wraps around the end of the data
	assert.ok(send(3, [4, 5, 6, 7, 8]));
	n = r.read(inbox);
	assert.equal(n, recordSize(5));
	assert.deepEqual([...readRecords(inbox.subarray(0, n))].map((x) => [...x.v]), [[4, 5, 6, 7, 8]]);
});

test('the analysis slot gives the newest frame once and refuses a torn one', () => {
	const l = new Latest(Latest.buffers(4, false));
	const dst = new Float32Array(4);
	assert.equal(l.take(dst, 0), 0, 'nothing published yet');
	l.publish([1, 2, 3, 4]);
	const s = l.take(dst, 0);
	assert.equal(s, 2);
	assert.deepEqual([...dst], [1, 2, 3, 4]);
	assert.equal(l.take(dst, s), s, 'nothing new');
	Atomics.store(l.seq, 0, 3); // a writer in the middle of a copy
	assert.equal(l.take(dst, s), s, 'odd = being written: keep the old one');
});

test('two threads over a SharedArrayBuffer: 200 000 samples in order', async () => {
	const b = ringBuffers(1024, 4, true);
	const total = 200_000;
	const src = `
		const { workerData } = require('node:worker_threads');
		const { state, data, total } = workerData;
		const s = new Int32Array(state), d = new Float32Array(data), mask = d.length - 1;
		let next = 0;
		while (next < total) {
			const w = Atomics.load(s, 0), free = d.length - ((w - Atomics.load(s, 1)) | 0);
			const n = Math.min(free, total - next, 100);
			if (n <= 0) { Atomics.wait(s, 1, Atomics.load(s, 1), 5); continue; }
			for (let i = 0; i < n; i++) d[(w + i) & mask] = next + i;
			Atomics.store(s, 0, (w + n) | 0);
			next += n;
		}`;
	const worker = new Worker(src, { eval: true, workerData: { ...b, total } });
	const r = Ring.floats(b);
	const out = new Float32Array(128);
	let expect = 0;
	const until = Date.now() + 20_000;
	while (expect < total && Date.now() < until) {
		const n = r.read(out, true);
		for (let i = 0; i < n; i++) {
			if (out[i] !== expect) assert.fail(`sample ${expect}: got ${out[i]}`);
			expect++;
		}
		if (n === 0) await new Promise((res) => setImmediate(res));
	}
	await worker.terminate();
	assert.equal(expect, total);
});
