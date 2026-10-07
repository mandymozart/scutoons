import { Howler } from 'howler';
import type { ReaderEffectKind } from '@scutoons/content';
import { musicOutput } from './AdaptiveMusicPlayer.ts';

/**
 * Audio effects over everything the reader hears (owner 2026-10-08: "is there a mechanism to inject filters into the
 * audio?"): a chain is put between a master gain and the speakers – the music's own output (AdaptiveMusicPlayer) and
 * Howler's (the samples and the effect sounds) – and taken out again, both with a soft fade (no clicks). Howler
 * replaces its AudioContext now and then (`Howler.unload()` per chapter): a running effect re-attaches itself there.
 * One effect at a time; `setAudioEffect(null)` ends it.
 *
 * One chain per reader effect (@scutoons/content READER_EFFECTS).
 * `psychedelic` ("Farbrausch", owner: "a delay and some high pass filter"): a high-pass that sweeps
 * slowly (≈ 160 … 600 Hz – higher took the music's body away, owner 2026-10-08) into a feedback echo (380 ms, feedback
 * 0.5), mixed with the filtered sound.
 */
export type AudioEffectKind = ReaderEffectKind;

interface Chain {
	input: AudioNode;
	output: GainNode;
	stop(): void;
}

const FADE = 1.2;

function farbrausch(ctx: AudioContext): Chain {
	const input = ctx.createGain();
	const highpass = ctx.createBiquadFilter();
	highpass.type = 'highpass';
	highpass.frequency.value = 20;
	highpass.Q.value = 0.7;
	// the sweep: a slow LFO on the cut-off
	const lfo = ctx.createOscillator();
	const depth = ctx.createGain();
	lfo.frequency.value = 0.08;
	depth.gain.value = 220;
	lfo.connect(depth).connect(highpass.frequency);
	lfo.start();
	highpass.frequency.setTargetAtTime(380, ctx.currentTime, FADE / 3);

	const dry = ctx.createGain();
	dry.gain.value = 0.9;
	const delay = ctx.createDelay(1);
	delay.delayTime.value = 0.38;
	const feedback = ctx.createGain();
	feedback.gain.value = 0.5;
	const wet = ctx.createGain();
	wet.gain.value = 0;
	wet.gain.setTargetAtTime(0.55, ctx.currentTime, FADE / 3);
	const output = ctx.createGain();

	input.connect(highpass);
	highpass.connect(dry).connect(output);
	highpass.connect(delay);
	delay.connect(feedback).connect(delay);
	delay.connect(wet).connect(output);
	return {
		input,
		output,
		stop() {
			try {
				lfo.stop();
			} catch {
				/* stopped */
			}
			for (const n of [input, highpass, lfo, depth, dry, delay, feedback, wet, output]) n.disconnect();
		}
	};
}

/**
 * `haze` ("Dunst", owner 2026-10-08: weed – "not a party drug … in haze"): muffled – a low-pass drifting slowly
 * (≈ 700 … 1300 Hz) – with a soft short echo (90 ms, feedback 0.3) that blurs the edges.
 */
function haze(ctx: AudioContext): Chain {
	const input = ctx.createGain();
	const lowpass = ctx.createBiquadFilter();
	lowpass.type = 'lowpass';
	lowpass.frequency.value = 20000;
	lowpass.Q.value = 0.5;
	const lfo = ctx.createOscillator();
	const depth = ctx.createGain();
	lfo.frequency.value = 0.05;
	depth.gain.value = 300;
	lfo.connect(depth).connect(lowpass.frequency);
	lfo.start();
	lowpass.frequency.setTargetAtTime(1000, ctx.currentTime, FADE / 3);
	const dry = ctx.createGain();
	dry.gain.value = 0.9;
	const delay = ctx.createDelay(1);
	delay.delayTime.value = 0.09;
	const feedback = ctx.createGain();
	feedback.gain.value = 0.3;
	const wet = ctx.createGain();
	wet.gain.value = 0;
	wet.gain.setTargetAtTime(0.35, ctx.currentTime, FADE / 3);
	const output = ctx.createGain();
	input.connect(lowpass);
	lowpass.connect(dry).connect(output);
	lowpass.connect(delay);
	delay.connect(feedback).connect(delay);
	delay.connect(wet).connect(output);
	return {
		input,
		output,
		stop() {
			try {
				lfo.stop();
			} catch {
				/* stopped */
			}
			for (const n of [input, lowpass, lfo, depth, dry, delay, feedback, wet, output]) n.disconnect();
		}
	};
}

const BUILD: Record<AudioEffectKind, (ctx: AudioContext) => Chain> = { psychedelic: farbrausch, haze };

/** one output with the chain in it */
interface Insert {
	ctx: AudioContext;
	from: AudioNode;
	chain: Chain;
}

let kind: AudioEffectKind | null = null;
const inserts: Insert[] = [];
let watch: ReturnType<typeof setInterval> | null = null;

/** puts the chain after `from` (which goes to the speakers); a short crossfade from the plain line */
function insert(ctx: AudioContext, from: AudioNode, effect: AudioEffectKind): Insert {
	const chain = BUILD[effect](ctx);
	const plain = ctx.createGain();
	from.disconnect();
	from.connect(plain).connect(ctx.destination);
	from.connect(chain.input);
	chain.output.gain.value = 0;
	chain.output.connect(ctx.destination);
	const now = ctx.currentTime;
	plain.gain.setValueAtTime(1, now);
	plain.gain.linearRampToValueAtTime(0, now + FADE);
	chain.output.gain.linearRampToValueAtTime(1, now + FADE);
	setTimeout(() => {
		// the plain line faded out: the chain alone from now on
		try {
			from.disconnect(plain);
			plain.disconnect();
		} catch {
			/* taken out meanwhile */
		}
	}, FADE * 1000 + 50);
	return { ctx, from, chain };
}

/** takes the chain out again: the plain line fades back in */
function remove({ ctx, from, chain }: Insert) {
	if (ctx.state === 'closed') return chain.stop();
	const plain = ctx.createGain();
	plain.gain.value = 0;
	from.connect(plain).connect(ctx.destination);
	const now = ctx.currentTime;
	plain.gain.linearRampToValueAtTime(1, now + FADE);
	chain.output.gain.setValueAtTime(chain.output.gain.value, now);
	chain.output.gain.linearRampToValueAtTime(0, now + FADE);
	setTimeout(() => {
		chain.stop();
		try {
			from.disconnect();
			from.connect(ctx.destination);
			plain.disconnect();
		} catch {
			/* the context went meanwhile */
		}
	}, FADE * 1000 + 50);
}

/** the outputs there are now: the music's and Howler's (when it has made its context) */
function outputs(): { ctx: AudioContext; from: AudioNode }[] {
	const out: { ctx: AudioContext; from: AudioNode }[] = [];
	const music = musicOutput();
	if (music) out.push({ ctx: music.ctx, from: music.master });
	if (Howler.ctx && Howler.masterGain) out.push({ ctx: Howler.ctx, from: Howler.masterGain });
	return out;
}

/** every output without the chain gets it (new: Howler made a new context, or made its first one) */
function attach() {
	if (!kind) return;
	for (let i = inserts.length - 1; i >= 0; i--) if (inserts[i].ctx.state === 'closed') inserts.splice(i, 1);
	for (const o of outputs()) if (!inserts.some((x) => x.from === o.from)) inserts.push(insert(o.ctx, o.from, kind));
}

/** an effect over everything the reader hears (null = none) */
export function setAudioEffect(next: AudioEffectKind | null) {
	if (next === kind) return;
	for (const x of inserts.splice(0)) remove(x);
	kind = next;
	if (watch) clearInterval(watch);
	watch = null;
	if (!kind) return;
	attach();
	watch = setInterval(attach, 1000);
}

// DEV only: the headless checks look at the graph (which outputs carry the chain, which context each sample plays on)
if (import.meta.env.DEV && typeof window !== 'undefined') {
	(window as unknown as Record<string, unknown>).__scutoonsAudioFx = () => ({
		kind,
		inserts: inserts.map((x) => ({ state: x.ctx.state, howler: x.ctx === Howler.ctx, master: x.from === Howler.masterGain })),
		howls: ((Howler as unknown as { _howls: { _sounds: { _node?: AudioNode }[]; _src: string | string[] }[] })._howls ?? []).map((h) => ({
			src: String(h._src).split('/').pop(),
			onHowlerCtx: h._sounds.map((s) => s._node?.context === Howler.ctx)
		}))
	});
}
