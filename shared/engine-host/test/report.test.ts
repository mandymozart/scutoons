// #356: the copyable sound report – the device, the block times against the budget, the first seconds as a table.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import type { Second } from '@scutoons/live-core';
import { deviceInfo, engineReport, playoutOf, seriesTable } from '../src/report.ts';
import { emptySnapshot } from '../src/status.ts';

const second = (t: number, over: Partial<Second> = {}): Second => ({
	t,
	blocks: 375,
	records: 0,
	renderUs: 120,
	renderP50Us: 100,
	renderP99Us: 400,
	renderPeakUs: 9_800,
	analysisPeakUs: 350,
	budgetUs: 2_666.7,
	warmMs: 180,
	lead: 3_072,
	gapMs: 4.2,
	limitPeakDb: 7.25,
	limitMeanDb: 2.5,
	limitingPct: 40,
	underruns: 0,
	...over
});

test('the table: a row a second, the block times as % of the budget, the lead in ms', () => {
	const rows = seriesTable([second(1), second(2, { underruns: 3 })], 48_000);
	assert.equal(rows.length, 3);
	assert.match(rows[0], /p50%\s+p99%\s+max%/);
	assert.deepEqual(rows[1].trim().split(/\s+/), ['1.0', '4', '15', '367', '350', '4.2', '64', '0', '7.3', '2.5', '40']);
	assert.equal(rows[2].trim().split(/\s+/)[7], '3');
});

test('the report: device, context, playout when the browser has it, warm-up, the table and the log', () => {
	const s = { ...emptySnapshot(), phase: 'running' as const, transport: 'shared' as const, isolated: true, sampleRate: 48_000, loadP50: 4, loadP99: 15, load: 368, warmMs: 180, lead: 64, dropouts: 0, limitPeakDb: 7.25, limitMeanDb: 2.5, limitingPct: 40 };
	const base = { at: '2026-11-03T12:00:00.000Z', page: '/live/deck', userAgent: 'SM-S901B Chrome/141', cores: 8, memory: 8, simd: true, snapshot: s, channels: 2, baseLatency: 0.0107, outputLatency: 0.04, series: [second(1)], log: ['12:00:00.000 [sound] running'] };
	const text = engineReport({ ...base, playout: { glitches: 2, glitchMs: 21.3, totalS: 60, averageLatencyMs: 41, maximumLatencyMs: 60 } });
	assert.match(text, /SM-S901B/);
	assert.match(text, /wasm simd yes/);
	assert.match(text, /48000 Hz · 2 ch · base latency 10\.7 ms · output latency 40\.0 ms/);
	assert.match(text, /device glitches 2 \(21\.3 ms in 60 s\) · latency avg 41 \/ max 60 ms/);
	assert.match(text, /warm-up 180 ms/);
	assert.match(text, /limiter now 7\.3 dB deepest · 2\.5 dB mean · holding the sound down 40 % of the second/);
	assert.match(text, /p50 4% · p99 15% · max 368%/);
	assert.match(text, /\[sound\] running\n$/);
	assert.match(engineReport({ ...base, playout: null, series: [], log: [] }), /device glitches not reported by this browser/);
});

test('deviceInfo outside a browser: unknowns, never a throw', () => {
	const d = deviceInfo();
	assert.equal(d.playout, null);
	assert.equal(d.baseLatency, 0);
	assert.equal(typeof d.simd, 'boolean');
});

test("the browser's glitch count: Chrome's playbackStats in seconds, the spec's playoutStats in ms – the same numbers", () => {
	const chrome = playoutOf({ underrunEvents: 3, underrunDuration: 0.0213, totalDuration: 60, averageLatency: 0.041, maximumLatency: 0.06 });
	const spec = playoutOf(undefined, { fallbackFramesEvents: 3, fallbackFramesDuration: 21.3, totalFramesDuration: 60_000, averageLatency: 41, maximumLatency: 60 });
	assert.deepEqual(chrome, { glitches: 3, glitchMs: 21.3, totalS: 60, averageLatencyMs: 41, maximumLatencyMs: 60 });
	assert.deepEqual(spec, chrome);
	assert.equal(playoutOf(), null);
});
