// The few messages between the page, the worker and the worklet (#184). None of them is on the hot path in the shared
// transport: records, audio and analysis go through the rings; these set things up, carry the load (the edge: scene
// text in, the table out) and once a second the stats. In the transfer transport (no SharedArrayBuffer) records,
// audio chunks and analysis frames travel as transferred ArrayBuffers – the same bytes as in the rings.
import type { Table } from './engine.ts';
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
	| { type: 'init'; wasm: string; rate: number; lead: number; shared: SharedSetup | null; audioPort: MessagePort | null }
	| { type: 'load'; id: number; scene: string }
	| { type: 'records'; bytes: ArrayBuffer }
	| { type: 'stop' };

export interface WorkerStats {
	blocks: number;
	records: number;
	/** µs per 128-frame block: a control step + the render + the copy out */
	renderUs: number;
	/** #208: the longest block of the last second (µs) and a block's budget – its duration (2667 µs at 48 kHz) */
	renderPeakUs: number;
	budgetUs: number;
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
	| { type: 'error'; error: string };

/** worker ↔ worklet over a MessagePort, transfer transport only */
export type AudioFeed = { chunk: Float32Array } | { consumed: number; underruns: number };

/** worklet → page */
export type FromWorklet = { underruns: number; played: number };

/** The worklet's processorOptions. */
export interface WorkletOptions {
	audio: RingBuffers | null;
	/** the local ring's size in frames (transfer transport) */
	capacity: number;
	/** #208: SharedSetup.status (shared transport) */
	status: SharedArrayBuffer | null;
}
