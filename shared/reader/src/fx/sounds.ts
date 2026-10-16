/**
 * The reader's effect sounds and haptics (agents/FX.md): short sounds synthesized with WebAudio – no files to load,
 * they play at once on a tap. The app routes them (`configureFx`): the viewer into Howler's master gain, so the music's
 * mute applies to them too; without an output an own AudioContext is used (the creator's preview).
 * Volume = the account page's "Sound effects" (this device), haptics = its "Vibration" switch. A context that is
 * still locked (no tap yet – a scroll-fired cue) stays silent: the browser only unlocks audio on a gesture.
 */

import { FX_SOUNDS, type FxSoundChoice, type FxSoundName } from '@scutoons/content';

export { FX_SOUNDS };
export type FxSound = FxSoundName;

export function isFxSound(value: unknown): value is FxSound {
	return typeof value === 'string' && (FX_SOUNDS as readonly string[]).includes(value);
}

export interface FxOutput {
	ctx: AudioContext;
	destination: AudioNode;
}

let output: () => FxOutput | null = () => null;
let volume = 0.8;
let haptics = true;
let own: AudioContext | null = null;
const gains = new WeakMap<AudioContext, GainNode>();

/** the app's routing (output), the reader's volume 0…1 and haptics switch */
export function configureFx(options: { output?: () => FxOutput | null; volume?: number; haptics?: boolean }) {
	if (options.output) output = options.output;
	if (options.volume !== undefined) {
		volume = Math.min(1, Math.max(0, options.volume));
		// gains already made follow at once
		for (const ctx of [own, output()?.ctx]) {
			const g = ctx && gains.get(ctx);
			if (g) g.gain.value = volume;
		}
	}
	if (options.haptics !== undefined) haptics = options.haptics;
}

/** the context + this module's gain (its volume) on the app's output, or an own context */
function route(): { ctx: AudioContext; out: GainNode } | null {
	if (typeof window === 'undefined') return null;
	let target = output();
	if (!target) {
		const AC = window.AudioContext ?? (window as unknown as { webkitAudioContext?: typeof AudioContext }).webkitAudioContext;
		if (!AC) return null;
		own ??= new AC();
		target = { ctx: own, destination: own.destination };
	}
	const { ctx } = target;
	if (ctx.state === 'suspended') void ctx.resume().catch(() => {});
	if (ctx.state !== 'running' && ctx.state !== 'suspended') return null;
	let out = gains.get(ctx);
	if (!out) {
		out = ctx.createGain();
		out.gain.value = volume;
		out.connect(target.destination);
		gains.set(ctx, out);
	}
	return { ctx, out };
}

function tone(r: { ctx: AudioContext; out: GainNode }, f0: number, f1: number, at: number, dur: number, type: OscillatorType, gain: number) {
	const t = r.ctx.currentTime + at;
	const o = r.ctx.createOscillator();
	const g = r.ctx.createGain();
	o.type = type;
	o.frequency.setValueAtTime(f0, t);
	o.frequency.exponentialRampToValueAtTime(Math.max(1, f1), t + dur);
	g.gain.setValueAtTime(0.0001, t);
	g.gain.exponentialRampToValueAtTime(gain, t + 0.008);
	g.gain.exponentialRampToValueAtTime(0.0001, t + dur);
	o.connect(g).connect(r.out);
	o.start(t);
	o.stop(t + dur + 0.02);
}

const noiseBuffers = new WeakMap<AudioContext, AudioBuffer>();
function noise(r: { ctx: AudioContext; out: GainNode }, at: number, dur: number, gain: number, type: BiquadFilterType, f0: number, f1: number) {
	let buffer = noiseBuffers.get(r.ctx);
	if (!buffer) {
		buffer = r.ctx.createBuffer(1, r.ctx.sampleRate, r.ctx.sampleRate);
		const d = buffer.getChannelData(0);
		for (let i = 0; i < d.length; i++) d[i] = Math.random() * 2 - 1;
		noiseBuffers.set(r.ctx, buffer);
	}
	const t = r.ctx.currentTime + at;
	const s = r.ctx.createBufferSource();
	const f = r.ctx.createBiquadFilter();
	const g = r.ctx.createGain();
	s.buffer = buffer;
	f.type = type;
	f.frequency.setValueAtTime(f0, t);
	f.frequency.exponentialRampToValueAtTime(f1, t + dur);
	f.Q.value = 1.2;
	g.gain.setValueAtTime(0.0001, t);
	g.gain.exponentialRampToValueAtTime(gain, t + dur * 0.25);
	g.gain.exponentialRampToValueAtTime(0.0001, t + dur);
	s.connect(f).connect(g).connect(r.out);
	s.start(t);
	s.stop(t + dur + 0.02);
}

const SOUNDS: Record<FxSound, (r: { ctx: AudioContext; out: GainNode }) => void> = {
	pling: (r) => {
		tone(r, 1318, 1318, 0, 0.18, 'sine', 0.35);
		tone(r, 1976, 1976, 0.07, 0.3, 'sine', 0.3);
	},
	whoosh: (r) => noise(r, 0, 0.32, 0.35, 'bandpass', 500, 3500),
	thunk: (r) => {
		tone(r, 170, 60, 0, 0.2, 'sine', 0.7);
		noise(r, 0, 0.06, 0.25, 'lowpass', 900, 300);
	},
	tick: (r) => tone(r, 2600, 2400, 0, 0.035, 'triangle', 0.12),
	pop: (r) => tone(r, 280, 980, 0, 0.09, 'triangle', 0.35),
	thud: (r) => {
		tone(r, 110, 38, 0, 0.4, 'sine', 0.9);
		noise(r, 0, 0.14, 0.45, 'lowpass', 700, 120);
	},
	soft: (r) => noise(r, 0, 0.5, 0.12, 'lowpass', 2000, 300),
	key: (r) => tone(r, 1800 + Math.random() * 400, 1600, 0, 0.025, 'square', 0.05),
	zap: (r) => {
		noise(r, 0, 0.25, 0.4, 'highpass', 6000, 1200);
		tone(r, 1600, 200, 0, 0.25, 'sawtooth', 0.12);
	},
	boom: (r) => {
		tone(r, 80, 30, 0, 0.6, 'sine', 0.9);
		noise(r, 0, 0.4, 0.5, 'lowpass', 1200, 60);
	},
	hum: (r) => {
		tone(r, 55, 50, 0, 1.2, 'sine', 0.5);
		tone(r, 110, 98, 0, 1.2, 'sine', 0.15);
	}
};

/** plays one effect sound (nothing while audio is locked or the volume is 0) */
export function playFxSound(sound: FxSound | null | undefined) {
	if (!sound || volume <= 0) return;
	const r = route();
	if (!r) return;
	try {
		SOUNDS[sound]?.(r);
	} catch {
		/* a closed context: no sound */
	}
}

/** a reveal's sound: the content's choice (`none` = silent), else the effect's own */
export function playRevealSound(choice: FxSoundChoice | undefined, own?: FxSound) {
	if (choice === 'none') return;
	playFxSound(choice ?? own);
}

/** a short vibration (phones; the account's "Vibration" switch) */
export function buzz(pattern: number | number[]) {
	if (!haptics || typeof navigator === 'undefined') return;
	if (!navigator.vibrate) return iosTicks(pattern);
	try {
		navigator.vibrate(pattern);
	} catch {
		/* not allowed here */
	}
}

/**
 * iPhone "polyfill" (owner 2026-10-16: "do the hidden checkbox as a polyfill for now … we can kill it later"): Safari has
 * no `navigator.vibrate`, but toggling an `<input type="checkbox" switch>` (Safari 18+) gives one system haptic tick –
 * only inside a tap (user activation), so a scroll-fired cue stays silent. One tick per "on" step of the pattern. An
 * unofficial trick: switch it off here (IOS_SWITCH_HAPTICS = false) when Apple closes it or a real API comes.
 */
const IOS_SWITCH_HAPTICS = true;
let iosSwitch: HTMLLabelElement | null | undefined;

function iosSwitchLabel(): HTMLLabelElement | null {
	if (iosSwitch !== undefined) return iosSwitch;
	iosSwitch = null;
	if (!IOS_SWITCH_HAPTICS || typeof document === 'undefined') return null;
	// only where the switch exists (Safari 18+): other browsers ignore the attribute
	const probe = document.createElement('input');
	probe.type = 'checkbox';
	probe.setAttribute('switch', '');
	if (!('switch' in probe) && !/iP(hone|ad|od)/.test(navigator.userAgent)) return null;
	const label = document.createElement('label');
	label.setAttribute('aria-hidden', 'true');
	label.style.cssText = 'position:fixed;width:1px;height:1px;overflow:hidden;opacity:0;pointer-events:none;left:-100px;top:0';
	probe.tabIndex = -1;
	label.appendChild(probe);
	document.body.appendChild(label);
	return (iosSwitch = label);
}

function iosTicks(pattern: number | number[]) {
	const label = iosSwitchLabel();
	if (!label) return;
	const steps = Array.isArray(pattern) ? pattern : [pattern];
	// even entries vibrate, odd ones pause: a tick at the start of every vibrate step
	let at = 0;
	steps.forEach((ms, i) => {
		if (i % 2 === 0 && ms > 0) setTimeout(() => label.click(), at);
		at += Math.max(0, ms);
	});
}
