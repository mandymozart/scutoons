// #356: the copyable sound report – the device, the block times against the budget, the first seconds as a table.
import { test } from 'node:test';
import assert from 'node:assert/strict';
import { deviceInfo, engineReport, playoutOf } from '../src/report.ts';
import { emptySnapshot } from '../src/status.ts';

/** a monitor report as deck-core gives it (only the fields the report reads) */
const stats = {
	fps: 60,
	sound: true,
	glitches: 0,
	alert: '',
	alert_line: '',
	warm_ms: 180,
	bar: [],
	lines: ['FPS 60.0 FRAME 16.7/18.0 CPU 3.0/4.1', 'DSP 4/15/368%  LEAD 64MS  LIM 7.3', 'GLITCH 0/10M LATE 0 LONG 0 UND 0']
};
const table = ['t_s  p50%  p99%  max%  ana_us  gap_ms  lead_ms  drops  lim_db  avg_db  lim%', ' 1.0     4    15   367     350     4.2       64      0     7.3     2.5    40'];

test('the report: device, context, playout when the browser has it, warm-up, the monitor’s lines and table, the log', () => {
	const s = { ...emptySnapshot(), phase: 'running' as const, transport: 'shared' as const, isolated: true, sampleRate: 48_000, stats };
	const base = { at: '2026-11-03T12:00:00.000Z', page: '/live/deck', userAgent: 'SM-S901B Chrome/141', cores: 8, memory: 8, simd: true, snapshot: s, channels: 2, baseLatency: 0.0107, outputLatency: 0.04, series: table, log: ['12:00:00.000 [sound] running'] };
	const text = engineReport({ ...base, playout: { glitches: 2, glitchMs: 21.3, totalS: 60, averageLatencyMs: 41, maximumLatencyMs: 60 } });
	assert.match(text, /SM-S901B/);
	assert.match(text, /wasm simd yes/);
	assert.match(text, /48000 Hz · 2 ch · base latency 10\.7 ms · output latency 40\.0 ms/);
	assert.match(text, /device glitches 2 \(21\.3 ms in 60 s\) · latency avg 41 \/ max 60 ms/);
	assert.match(text, /warm-up 180 ms/);
	assert.match(text, /DSP 4\/15\/368%  LEAD 64MS  LIM 7\.3\n/, "the monitor's lines, as the core worded them");
	assert.match(text, /lead_ms  drops[\s\S]* 1\.0     4    15   367/, "the core's table");
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

test("#387: the clock line is the core's, printed as it came (none before the sound)", () => {
	const s = { ...emptySnapshot(), phase: 'running' as const, stats };
	const base = { at: '', page: '', userAgent: '', cores: 0, memory: 0, simd: true, snapshot: s, channels: 2, baseLatency: 0, outputLatency: 0, playout: null, series: [], log: [] };
	const line = 'clock relay · 84.0 BPM · SHOW -0.4MS RTT 3.2MS · last 60 s |show| p50 0.3 · p95 1.2 · max 4.8 ms (mean -0.1, 2400 frames)';
	assert.ok(engineReport({ ...base, clock: line }).includes(`GLITCH 0/10M LATE 0 LONG 0 UND 0\n${line}\n`), 'after the monitor, as it came');
	assert.ok(engineReport(base).includes('UND 0\n\n'), 'no line before the sound');
});
