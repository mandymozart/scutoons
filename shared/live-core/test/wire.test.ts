// The wire records against the fixture both sides read: deck/live-core/fixtures/wire.json (the Rust round trip is
// deck/live-core/src/wire.rs) – TS encodes every record to the same bytes and decodes the bytes to the same record.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { analysisOf, encodeSend, FRAME, HEADER, readRecords, recordSize, writeSend, WireError, type SendRecord } from '../src/wire.ts';

type Row = { name: string; record: SendRecord & { v: number[] }; hex: string };
const fixture = fileURLToPath(new URL('../../../deck/live-core/fixtures/wire.json', import.meta.url));
const rows = JSON.parse(readFileSync(fixture, 'utf8')) as Row[];

const hex = (b: Uint8Array) => Buffer.from(b.buffer, b.byteOffset, b.byteLength).toString('hex');
const bytes = (h: string) => new Uint8Array(Buffer.from(h, 'hex'));

test('fixture: every record encodes to its bytes', () => {
	assert.ok(rows.length >= 4);
	for (const r of rows) assert.equal(hex(encodeSend(r.record)), r.hex, r.name);
});

test('fixture: the bytes decode to the record (values rounded to f32 like Rust)', () => {
	for (const r of rows) {
		const [got, ...rest] = [...readRecords(bytes(r.hex))];
		assert.equal(rest.length, 0);
		assert.equal(got.mode, r.record.mode, r.name);
		assert.equal(got.node, r.record.node);
		assert.equal(got.port, r.record.port);
		assert.equal(got.amount, Math.fround(r.record.amount));
		assert.deepEqual([...got.v], r.record.v.map(Math.fround));
	}
});

test('records back to back, written in place at any offset (aligned or not)', () => {
	const all = rows.map((r) => r.hex).join('');
	assert.deepEqual([...readRecords(bytes(all))].map((r) => r.node), rows.map((r) => r.record.node));
	for (const offset of [0, 4, 3]) {
		const buf = new Uint8Array(offset + 64);
		const end = writeSend(buf, offset, rows[1].record);
		assert.equal(end, offset + recordSize(4));
		assert.equal(hex(buf.subarray(offset, end)), rows[1].hex, `offset ${offset}`);
	}
});

test('malformed bytes throw with the offset', () => {
	const good = bytes(rows[0].hex);
	const fails = (b: Uint8Array, what: string, at = 0) =>
		assert.throws(() => [...readRecords(b)], (e: unknown) => e instanceof WireError && e.what === what && e.at === at);
	fails(good.subarray(0, HEADER), 'truncated values');
	fails(good.subarray(0, 7), 'a truncated header');
	const op = good.slice();
	op[0] = 9;
	fails(op, 'an unknown op');
	const mode = good.slice();
	mode[1] = 7;
	fails(mode, 'an unknown mode');
	const two = new Uint8Array([...good, 1, 0, 0]);
	fails(two, 'a truncated header', good.length);
	assert.throws(() => writeSend(new Uint8Array(8), 0, rows[0].record), RangeError);
});

test('the analysis frame names its values and views the spectrum', () => {
	const f = new Float32Array(FRAME).map((_, i) => i);
	const a = analysisOf(f);
	assert.deepEqual([a.level, a.bass, a.mid, a.high, a.spectrum.length, a.spectrum[0]], [0, 1, 2, 3, 512, 4]);
	f[4] = 99;
	assert.equal(a.spectrum[0], 99, 'a view, not a copy');
});
