// The wire records against the fixture both sides read: deck/live-core/fixtures/wire.json (the Rust round trip is
// deck/live-core/src/wire.rs) – TS encodes every record to the same bytes and decodes the bytes to the same record.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { analysisOf, encodeSend, FRAME, HEADER, LIMITER_BAR, MAX_OUTPUTS, METER_MARKS, METER_ZONES, OUTPUT_COUNT, PEAKS, POSITIONS, REDUCTION, VALUE_COUNT, VALUES_AT, ZONES, readRecords, recordSize, writeSend, WireError, type SendRecord } from '../src/wire.ts';

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

test('the frame carries the channel count and the peaks (#282), then the values (#320); an older frame has none', () => {
	assert.equal(FRAME, 4 + 512 + 1 + 1 + 3 * MAX_OUTPUTS + 1 + 1);
	assert.deepEqual([REDUCTION, OUTPUT_COUNT, PEAKS, POSITIONS, ZONES, LIMITER_BAR, VALUE_COUNT, VALUES_AT], [516, 517, 518, 550, 582, 614, 615, 616]);
	const f = new Float32Array(FRAME + 3);
	f[VALUE_COUNT] = 2;
	f.set([7, 8, 9], VALUES_AT);
	assert.deepEqual([...analysisOf(f).values], [7, 8], 'as many values as the count says');
	f[REDUCTION] = 6;
	f[OUTPUT_COUNT] = 2;
	f.set([0.5, 0.25, 0.9], PEAKS);
	f.set([0.8, 0.6], POSITIONS);
	f.set([1, 0], ZONES);
	f[LIMITER_BAR] = 0.5;
	const a = analysisOf(f);
	assert.equal(a.reduction, 6);
	assert.deepEqual([...a.outputs], [0.5, 0.25], 'as many as the count says, a view on the frame');
	assert.deepEqual([...a.positions], [Math.fround(0.8), Math.fround(0.6)]);
	assert.deepEqual([...a.zones].map((z) => METER_ZONES[z]), ['hot', 'ok']);
	assert.equal(a.limiterBar, 0.5);
	f[PEAKS] = 0.75;
	assert.equal(a.outputs[0], 0.75);
	f[OUTPUT_COUNT] = 99;
	assert.equal(analysisOf(f).outputs.length, MAX_OUTPUTS, 'a count past the slots is cut');
	// a frame from an engine that predates the slots: the old layout, nothing past the reduction
	const old = analysisOf(new Float32Array(4 + 512 + 1));
	assert.equal(old.outputs.length, 0);
	assert.equal(old.values.length, 0);
	assert.equal(old.reduction, 0);
	assert.equal(old.limiterBar, 0);
});

test('the meter scale arrives as generated constants of the core (deck-core meters)', () => {
	const { zero, ok, hot } = METER_MARKS;
	assert.ok(ok < hot && hot < zero && zero < 1, 'the marks climb, with headroom above 0 dB');
	assert.deepEqual(METER_ZONES, ['ok', 'hot', 'clip']);
});
