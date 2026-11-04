// status.ts (#298): the engine host's phases and how a page words them. Pure – no DOM, no wasm.
import type { DeviceRow, OutputReading, Transport } from '@scutoons/live-core';
import type { MonitorReport } from '@scutoons/live-deck';

/** where the host is: idle (nothing yet) → mounting (wasm, deck, the 3D) → ready (the being is there, no sound) →
 *  starting (the AudioContext, the worker) → running → stopped (the sound ended, the being stays) · error */
export type EnginePhase = 'idle' | 'mounting' | 'ready' | 'starting' | 'running' | 'stopped' | 'error';

/** Start needs a user gesture – the button is live once the deck exists (the sound does not wait for the 3D) and no
 *  sound runs or starts; after an error it tries again */
export const canStart = (p: EnginePhase): boolean => p === 'mounting' || p === 'ready' || p === 'stopped' || p === 'error';
export const canStop = (p: EnginePhase): boolean => p === 'running';

/** what the engine reports about itself (the host's `snapshot()`) */
export interface EngineSnapshot {
	phase: EnginePhase;
	/** the last error, '' = none */
	error: string;
	/** the page is cross-origin isolated (COOP + COEP) – the shared transport needs it */
	isolated: boolean;
	/** the transport live-core runs on, null while no sound runs */
	transport: Transport | null;
	/** the AudioContext's state, '' = none */
	audio: AudioContextState | '';
	/** the AudioContext's clock (s) – advancing means the sound runs */
	clock: number;
	sampleRate: number;
	/** the 3D's backend ('webgpu' | 'webgl2'), '' = none */
	backend: string;
	/** the output levels, 0..1 */
	level: number;
	bass: number;
	mid: number;
	high: number;
	/** the limiter's gain reduction (dB) */
	reduction: number;
	/** #282, #373: one reading per audio output the engine renders (the stereo master = 2) as the core's analysis frame
	 *  carries it – the held peak, its place on the meter's dB scale and its zone (deck-core meters) – [] while no sound runs */
	outputs: OutputReading[];
	/** #282, #373: the scene's `[outputs]` as deck-core's device monitor reads them – a row per protocol that carried a
	 *  message: its last line, the count, the activity 1 → 0 and whether the lamp is lit */
	signals: DeviceRow[];
	/** #364: deck-core's monitor – FPS of drawn frames, CPU, the audio's load, glitches, lead, limiter: the numbers, their
	 *  judgement and the bar's words; null while it is not loaded */
	stats: MonitorReport | null;
	/** the 3D's render quality, e.g. `high 1.00×`, '' = none */
	quality: string;
	/** the 3D's own line (hud) */
	hud: string;
	/** the mood now – the deck's core's ('' before it said) */
	mood: string;
}

export const emptySnapshot = (): EngineSnapshot => ({
	phase: 'idle',
	error: '',
	isolated: false,
	transport: null,
	audio: '',
	clock: 0,
	sampleRate: 0,
	backend: '',
	level: 0,
	bass: 0,
	mid: 0,
	high: 0,
	reduction: 0,
	outputs: [],
	signals: [],
	stats: null,
	quality: '',
	hud: '',
	mood: ''
});

/** The transport in a sentence – why the engine is slower when it is: `shared` rings need the page cross-origin
 *  isolated, otherwise live-core falls back to postMessage (`transfer`). */
export function transportNote(transport: Transport | null, isolated: boolean): string {
	if (!transport) return isolated ? 'not started (the page is cross-origin isolated: shared rings)' : 'not started (the page is not cross-origin isolated: transfer fallback)';
	if (transport === 'shared') return 'shared: SharedArrayBuffer rings (cross-origin isolated)';
	return isolated ? 'transfer: postMessage (isolated, but the transport was asked for)' : 'transfer: postMessage fallback (the page is not cross-origin isolated)';
}
