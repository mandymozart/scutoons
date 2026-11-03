// The few messages between the page, the worker and the worklet (#184). None of them is on the hot path in the shared
// transport: records, audio and analysis go through the rings; these set things up, carry the load (the edge: scene
// text in, the table out) and once a second the stats. In the transfer transport (no SharedArrayBuffer) records,
// audio chunks and analysis frames travel as transferred ArrayBuffers – the same bytes as in the rings.
import type { BlockStats, Table } from './engine.ts';
import type { RingBuffers } from './ring.ts';

export type Transport = 'shared' | 'transfer';

export interface SharedSetup {
	control: RingBuffers;
	audio: RingBuffers;
	analysis: RingBuffers;
	/** #208: Int32 [the worklet's dropouts] – the worker grows its lead on them */
	status: SharedArrayBuffer;
}

/** page → worker */
export type ToWorker =
	/** `channels`: the device channels (the destination's – 2 on a phone); the audio ring and the feed carry them interleaved */
	| { type: 'init'; wasm: string; rate: number; channels: number; lead: number; shared: SharedSetup | null; audioPort: MessagePort | null }
	| { type: 'load'; id: number; scene: string }
	| { type: 'records'; bytes: ArrayBuffer }
	/** #321: something for the core's tempo clock (clock.ts) */
	| { type: 'clock'; op: ClockOp }
	| { type: 'stop' };

/** once a second: the last second's blocks (engine.ts `BlockStats`) and the pump's state */
export interface WorkerStats extends BlockStats {
	/** #356: how long the warm-up before the first sound took (ms; 0 = none yet) */
	warmMs: number;
	/** #208: the adaptive lead now (frames) and the longest pause between the worker's pumps in the last second (ms) –
	 * a pause past the lead's time is a dropout the main thread or a GC caused */
	lead: number;
	gapMs: number;
}

/** worker → page */
export type FromWorker =
	| { type: 'ready' }
	| { type: 'loaded'; id: number; table: Table }
	| { type: 'failed'; id: number; error: string }
	| { type: 'analysis'; frame: Float32Array }
	/** #282: signal records (signals.ts) of the scene's `[outputs]`, a few per second at most */
	| { type: 'signals'; bytes: ArrayBuffer }
	| { type: 'stats'; stats: WorkerStats }
	/** #321: the clock's state block (`clock.ts` STATE) after each clock frame */
	| { type: 'clock'; state: Float64Array }
	| { type: 'error'; error: string };

/** #321: what the page hands the core's clock – every time on the page's ms (`performance.timeOrigin + now()`) */
export type ClockOp =
	/** `getOutputTimestamp`: context second `context` heard at `heard` (the worker adds the worklet's skew) */
	| { op: 'stamp'; context: number; heard: number }
	/** a pong: the ping's `t0` (µs of the page's clock), the relay's `t1` (ms), back at `t2` */
	| { op: 'pong'; t0: number; t1: number; t2: number }
	/** the relay's `tempo` (relay ms `at`), arrived at `arrived` */
	| { op: 'tempo'; bpm: number; perBar: number; beat: number; at: number; running: boolean; arrived: number }
	/** a Web MIDI message (its first three bytes), stamped by the browser */
	| { op: 'midi'; bytes: [number, number, number]; at: number }
	| { op: 'joined'; now: number }
	/** the picker: `source` = deck-core's `Source::ALL` index (internal, midi, link, osc, relay) */
	| { op: 'choose'; source: number; lead: boolean }
	/** 0 tap, 1 faster, 2 slower, 3 start / stop, 4 the click */
	| { op: 'key'; key: number; at: number };

/** worker ↔ worklet over a MessagePort, transfer transport only: chunks of interleaved frames, frames consumed (and,
 * #321, the skew: the worklet's context frame minus the frames it played) */
export type AudioFeed = { chunk: Float32Array } | { consumed: number; underruns: number; skew: number };

/** worklet → page */
export type FromWorklet = { underruns: number; played: number };

/** The worklet's processorOptions. */
export interface WorkletOptions {
	audio: RingBuffers | null;
	/** the local ring's size in samples (transfer transport) */
	capacity: number;
	/** the device channels, interleaved in the ring – the node's `outputChannelCount` */
	channels: number;
	/** #208: SharedSetup.status (shared transport): [dropouts, #321 the skew] */
	status: SharedArrayBuffer | null;
}
