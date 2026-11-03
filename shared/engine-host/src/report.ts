// report.ts (#356): the sound's measurement as one copyable text – what a phone owner pastes into an issue. The device
// and the audio context, the worker's block times (p50 / p99 / the longest against the 128-frame budget), the warm-up
// and the first seconds as a table, the engine log's last lines. Pure, except `deviceInfo` (it reads the browser).
import type { ClockState, ClockSummary, Second } from '@scutoons/live-core';
import type { EngineSnapshot } from './status.ts';

/** the browser's own count of output glitches – the device ran dry, which the worklet's ring cannot see (Chrome's
 *  `AudioContext.playbackStats`, or the later spec's `playoutStats`), when it has one */
export interface Playout {
	glitches: number;
	glitchMs: number;
	/** seconds played */
	totalS: number;
	averageLatencyMs: number;
	maximumLatencyMs: number;
}

/** Chrome's `playbackStats` (seconds) */
interface PlaybackStats {
	underrunEvents: number;
	underrunDuration: number;
	totalDuration: number;
	averageLatency: number;
	maximumLatency: number;
}

/** the spec's `playoutStats` (milliseconds) */
interface PlayoutStats {
	fallbackFramesEvents: number;
	fallbackFramesDuration: number;
	totalFramesDuration: number;
	averageLatency: number;
	maximumLatency: number;
}

/** either stats object as `Playout`; null without one */
export function playoutOf(playback?: PlaybackStats, playout?: PlayoutStats): Playout | null {
	const r = (v: number) => Math.round(v * 10) / 10;
	if (playback) {
		const p = playback;
		return { glitches: p.underrunEvents, glitchMs: r(1000 * p.underrunDuration), totalS: r(p.totalDuration), averageLatencyMs: r(1000 * p.averageLatency), maximumLatencyMs: r(1000 * p.maximumLatency) };
	}
	if (playout) {
		const p = playout;
		return { glitches: p.fallbackFramesEvents, glitchMs: r(p.fallbackFramesDuration), totalS: r(p.totalFramesDuration / 1000), averageLatencyMs: r(p.averageLatency), maximumLatencyMs: r(p.maximumLatency) };
	}
	return null;
}

export interface ReportInput {
	/** when (ISO) and where (the page's path + query) */
	at: string;
	page: string;
	userAgent: string;
	cores: number;
	/** GB (`navigator.deviceMemory`), 0 = unknown */
	memory: number;
	/** WebAssembly SIMD (the engine's module needs it) */
	simd: boolean;
	snapshot: EngineSnapshot;
	channels: number;
	/** the AudioContext's latencies (s), 0 = unknown */
	baseLatency: number;
	outputLatency: number;
	playout: Playout | null;
	/** the first seconds (live-core `series()`) */
	series: readonly Second[];
	/** the engine log's last lines, formatted */
	log: string[];
	/** #321: the tempo clock now and its error over the last minute (null before the sound / while not following) */
	clock?: ClockState | null;
	clockWindow?: ClockSummary | null;
}

/** #321: the show clock's line – where this deck takes its tempo from, the core's line for it (SHOW ±x MS RTT y MS),
 *  how far off the show's line it played over the last minute */
export function clockLine(c: ClockState | null | undefined, w: ClockSummary | null | undefined): string {
	if (!c) return 'clock not running';
	const f = (v: number) => `${v >= 0 ? '+' : ''}${v.toFixed(1)}`;
	// the state in deck-core's words (the native HUD's and the panel's line)
	const now = `clock ${c.source} · ${c.bpm.toFixed(1)} BPM${c.line ? ` · ${c.line}` : ''}`;
	if (!w) return `${now} · no minute of following yet`;
	return `${now} · last ${w.seconds.toFixed(0)} s |show| p50 ${w.p50.toFixed(1)} · p95 ${w.p95.toFixed(1)} · max ${w.max.toFixed(1)} ms (mean ${f(w.mean)}, ${w.n} frames)`;
}

/** µs as a share of the block budget (%) */
const pct = (us: number, budget: number): string => String(Math.round((100 * us) / Math.max(1, budget)));
const ms = (s: number): string => (s > 0 ? `${(1000 * s).toFixed(1)} ms` : '?');

/** the second-by-second table: t · p50 · p99 · max (% of the budget) · analysis (µs) · pump gap (ms) · lead (ms) · dropouts
 *  · the limiter's deepest and mean gain reduction (dB) and its share of the second (%) */
export function seriesTable(series: readonly Second[], rate: number): string[] {
	const rows = ['t_s  p50%  p99%  max%  ana_us  gap_ms  lead_ms  drops  lim_db  avg_db  lim%'];
	for (const s of series) {
		rows.push(
			[
				s.t.toFixed(1).padStart(4),
				pct(s.renderP50Us, s.budgetUs).padStart(5),
				pct(s.renderP99Us, s.budgetUs).padStart(5),
				pct(s.renderPeakUs, s.budgetUs).padStart(5),
				String(Math.round(s.analysisPeakUs)).padStart(7),
				s.gapMs.toFixed(1).padStart(7),
				String(Math.round((1000 * s.lead) / Math.max(1, rate))).padStart(8),
				String(s.underruns).padStart(6),
				s.limitPeakDb.toFixed(1).padStart(7),
				s.limitMeanDb.toFixed(1).padStart(7),
				String(Math.round(s.limitingPct)).padStart(5)
			].join(' ')
		);
	}
	return rows;
}

export function engineReport(r: ReportInput): string {
	const s = r.snapshot;
	const p = r.playout;
	const lines = [
		`SCUTOONS deck sound report (#356) ${r.at}`,
		`page ${r.page}`,
		`ua ${r.userAgent}`,
		`cores ${r.cores || '?'} · memory ${r.memory ? `${r.memory} GB` : '?'} · wasm simd ${r.simd ? 'yes' : 'NO'}`,
		`sound ${s.phase} · ${s.transport ?? '-'} transport · cross-origin isolated ${s.isolated ? 'yes' : 'no'} · rendered in a worker, played by an AudioWorklet`,
		`context ${s.sampleRate} Hz · ${r.channels} ch · base latency ${ms(r.baseLatency)} · output latency ${ms(r.outputLatency)} · state ${s.audio || '-'}`,
		`device glitches ${p ? `${p.glitches} (${p.glitchMs} ms in ${p.totalS} s) · latency avg ${p.averageLatencyMs} / max ${p.maximumLatencyMs} ms` : 'not reported by this browser'}`,
		`warm-up ${s.warmMs ? `${s.warmMs.toFixed(0)} ms` : 'none'} · lead ${s.lead} ms · dropouts ${s.dropouts ?? '-'}/min`,
		`limiter now ${s.limitPeakDb.toFixed(1)} dB deepest · ${s.limitMeanDb.toFixed(1)} dB mean · holding the sound down ${Math.round(s.limitingPct)} % of the second`,
		`now block p50 ${s.loadP50}% · p99 ${s.loadP99}% · max ${s.load}% of the budget · picture ${s.fps.toFixed(0)} fps, ${s.cpu.toFixed(1)}/${s.peak.toFixed(1)} ms · 3D ${s.backend || '-'} ${s.quality}`,
		clockLine(r.clock, r.clockWindow),
		'',
		...seriesTable(r.series, s.sampleRate)
	];
	if (r.log.length) lines.push('', ...r.log);
	return lines.join('\n') + '\n';
}

/** a module that uses one SIMD instruction – valid where the engine's simd128 build runs (wasm-feature-detect's probe) */
const SIMD_PROBE = new Uint8Array([0, 97, 115, 109, 1, 0, 0, 0, 1, 5, 1, 96, 0, 1, 123, 3, 2, 1, 0, 10, 10, 1, 8, 0, 65, 0, 253, 15, 253, 98, 11]);

/** what the browser says about itself and the running audio context (none → unknowns) */
export function deviceInfo(context?: AudioContext): Pick<ReportInput, 'at' | 'page' | 'userAgent' | 'cores' | 'memory' | 'simd' | 'baseLatency' | 'outputLatency' | 'playout'> {
	const ctx = context as (AudioContext & { playbackStats?: PlaybackStats; playoutStats?: PlayoutStats }) | undefined;
	const nav = globalThis.navigator as (Navigator & { deviceMemory?: number }) | undefined;
	let simd = false;
	try {
		simd = WebAssembly.validate(SIMD_PROBE);
	} catch {
		/* no WebAssembly */
	}
	const loc = globalThis.location;
	return {
		at: new Date().toISOString(),
		page: loc ? loc.pathname + loc.search : '',
		userAgent: nav?.userAgent ?? '',
		cores: nav?.hardwareConcurrency ?? 0,
		memory: nav?.deviceMemory ?? 0,
		simd,
		baseLatency: ctx?.baseLatency ?? 0,
		outputLatency: ctx?.outputLatency ?? 0,
		playout: playoutOf(ctx?.playbackStats, ctx?.playoutStats)
	};
}
