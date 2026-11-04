// report.ts (#356): the sound's measurement as one copyable text – what a phone owner pastes into an issue. The device
// and the audio context (what the browser says), then deck-core's monitor (#364): its lines (frames, the blocks' load
// against their budget, the lead, the limiter, the glitches), its clock line (#387) and the first seconds as its table;
// the engine log's last lines. Pure, except `deviceInfo` (it reads the browser).
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
	/** the first seconds as the monitor's table (deck-core `Monitor::series_table`) */
	series: readonly string[];
	/** the engine log's last lines, formatted */
	log: string[];
	/** #387: the core's clock line – its source, tempo, status and how far off the show's line it played over the last
	 *  minute (deck-core `Controller::report`); '' before the sound */
	clock?: string;
}

const ms = (s: number): string => (s > 0 ? `${(1000 * s).toFixed(1)} ms` : '?');

export function engineReport(r: ReportInput): string {
	const s = r.snapshot;
	const st = s.stats;
	const p = r.playout;
	const lines = [
		`SCUTOONS deck sound report (#356) ${r.at}`,
		`page ${r.page}`,
		`ua ${r.userAgent}`,
		`cores ${r.cores || '?'} · memory ${r.memory ? `${r.memory} GB` : '?'} · wasm simd ${r.simd ? 'yes' : 'NO'}`,
		`sound ${s.phase} · ${s.transport ?? '-'} transport · cross-origin isolated ${s.isolated ? 'yes' : 'no'} · rendered in a worker, played by an AudioWorklet`,
		`context ${s.sampleRate} Hz · ${r.channels} ch · base latency ${ms(r.baseLatency)} · output latency ${ms(r.outputLatency)} · state ${s.audio || '-'}`,
		`device glitches ${p ? `${p.glitches} (${p.glitchMs} ms in ${p.totalS} s) · latency avg ${p.averageLatencyMs} / max ${p.maximumLatencyMs} ms` : 'not reported by this browser'}`,
		`warm-up ${st?.warm_ms ? `${st.warm_ms.toFixed(0)} ms` : 'none'} · 3D ${s.backend || '-'} ${s.quality}`,
		...(st ? [st.alert_line, ...st.lines].filter(Boolean) : ['no monitor']),
		...(r.clock ? [r.clock] : []),
		'',
		...r.series
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
