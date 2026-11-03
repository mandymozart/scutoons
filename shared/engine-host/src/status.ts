// status.ts (#298): the engine host's phases and how a page words them. Pure – no DOM, no wasm.
import type { Transport } from '@scutoons/live-core';
import { emptySignals, type SignalsView } from './monitor.ts';

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
	fps: number;
	/** the work per frame: average and worst of the last second (ms) */
	cpu: number;
	peak: number;
	/** the output levels, 0..1 */
	level: number;
	bass: number;
	mid: number;
	high: number;
	/** the limiter's gain reduction (dB) */
	reduction: number;
	/** #282: one peak per audio output the engine renders (the stereo master = 2), 0..1, held with a short fall
	 *  (meters.ts) – [] while no sound runs */
	outputs: number[];
	/** #282: the scene's `[outputs]` as a monitor reads them – per kind (midi, osc, dmx) the last message, the count
	 *  and an activity 1 → 0 (monitor.ts) */
	signals: SignalsView;
	/** worklet dropouts over the last minute, null = no sound yet */
	dropouts: number | null;
	/** the worker's peak render load, % of its budget */
	load: number;
	/** #356: the median and the 99th percentile block of the last second, % of the budget */
	loadP50: number;
	loadP99: number;
	/** #356: the warm-up before the first sound (ms), 0 = none */
	warmMs: number;
	/** the worker's lead (ms) */
	lead: number;
	/** a dropout in the last seconds */
	glitching: boolean;
	/** the 3D's render quality, e.g. `high 1.00×`, '' = none */
	quality: string;
	/** the 3D's own line (hud) */
	hud: string;
	/** the mood now */
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
	fps: 0,
	cpu: 0,
	peak: 0,
	level: 0,
	bass: 0,
	mid: 0,
	high: 0,
	reduction: 0,
	outputs: [],
	signals: emptySignals(),
	dropouts: null,
	load: 0,
	loadP50: 0,
	loadP99: 0,
	warmMs: 0,
	lead: 0,
	glitching: false,
	quality: '',
	hud: '',
	mood: 'neutral'
});

/** The transport in a sentence – why the engine is slower when it is: `shared` rings need the page cross-origin
 *  isolated, otherwise live-core falls back to postMessage (`transfer`). */
export function transportNote(transport: Transport | null, isolated: boolean): string {
	if (!transport) return isolated ? 'not started (the page is cross-origin isolated: shared rings)' : 'not started (the page is not cross-origin isolated: transfer fallback)';
	if (transport === 'shared') return 'shared: SharedArrayBuffer rings (cross-origin isolated)';
	return isolated ? 'transfer: postMessage (isolated, but the transport was asked for)' : 'transfer: postMessage fallback (the page is not cross-origin isolated)';
}

/** One line for a status bar: `running · shared · 3D webgpu · 60 fps` */
export function statusLine(s: EngineSnapshot): string {
	const parts: string[] = [s.phase === 'error' && s.error ? `error: ${s.error}` : s.phase];
	if (s.transport) parts.push(s.transport);
	if (s.backend) parts.push(`3D ${s.backend}`);
	if (s.fps > 0) parts.push(`${Math.round(s.fps)} fps`);
	return parts.join(' · ');
}

/** a level 0..1 as a percentage for a meter (clamped) */
export const meterPercent = (v: number): number => Math.round(100 * Math.min(1, Math.max(0, Number.isFinite(v) ? v : 0)));

/** the worker's peak render time as a share of its block budget (%) */
export const loadPercent = (renderPeakUs: number, budgetUs: number): number => Math.round((100 * renderPeakUs) / Math.max(1, budgetUs));

/** the worker's lead in ms */
export const leadMs = (lead: number, sampleRate: number): number => Math.round((1000 * lead) / Math.max(1, sampleRate));
