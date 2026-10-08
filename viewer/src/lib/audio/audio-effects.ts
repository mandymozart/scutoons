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
 * slowly (≈ 160 … 600 Hz – higher took the music's body away, owner 2026-10-08) into a feedback echo (feedback 0.78 –
 * owner: "longer") whose time drifts with two slow LFOs (≈ 120 … 640 ms, speeding up and slowing down unevenly), then
 * panned left and right at a pace that itself speeds up and slows down; mixed with the filtered
 * sound.
 */
export type AudioEffectKind = ReaderEffectKind;

interface Chain {
	input: AudioNode;
	output: GainNode;
	stop(): void;
	/** the reader's scroll pace (1 = still … 4 = fast): psychedelic speeds its echo up by it */
	pace?(factor: number): void;
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
	// the echo's time drifts – slower, faster, in uneven waves (owner 2026-10-08): two slow LFOs of unrelated rates on it
	// (≈ 0.12 … 0.64 s – owner: "a bit more intense"; the warble that comes with it is welcome)
	const drift1 = ctx.createOscillator();
	const drift1Depth = ctx.createGain();
	drift1.frequency.value = 0.07;
	drift1Depth.gain.value = 0.18;
	const drift2 = ctx.createOscillator();
	const drift2Depth = ctx.createGain();
	drift2.type = 'triangle';
	drift2.frequency.value = 0.193;
	drift2Depth.gain.value = 0.08;
	drift1.connect(drift1Depth).connect(delay.delayTime);
	drift2.connect(drift2Depth).connect(delay.delayTime);
	drift1.start();
	drift2.start();
	const feedback = ctx.createGain();
	feedback.gain.value = 0.78;
	const wet = ctx.createGain();
	wet.gain.value = 0;
	wet.gain.setTargetAtTime(0.55, ctx.currentTime, FADE / 3);
	const output = ctx.createGain();
	// the sound wanders left and right, and the wandering itself speeds up and slows down (owner 2026-10-08): a panner
	// swung by an LFO whose rate a second, slower LFO moves (≈ 0.05 … 0.45 swings a second)
	const panner = ctx.createStereoPanner();
	const swing = ctx.createOscillator();
	const swingDepth = ctx.createGain();
	swing.frequency.value = 0.25;
	swingDepth.gain.value = 0.8;
	const pace = ctx.createOscillator();
	const paceDepth = ctx.createGain();
	pace.frequency.value = 0.045;
	paceDepth.gain.value = 0.2;
	pace.connect(paceDepth).connect(swing.frequency);
	swing.connect(swingDepth).connect(panner.pan);
	swing.start();
	pace.start();
	const mix = ctx.createGain();
	let paced = 1;

	input.connect(highpass);
	highpass.connect(dry).connect(mix);
	highpass.connect(delay);
	delay.connect(feedback).connect(delay);
	delay.connect(wet).connect(mix);
	mix.connect(panner).connect(output);
	return {
		input,
		output,
		// scrolling faster: the echo repeats up to 4× faster (its time and its drift divided), back when the reader stops
		// a slow glide either way, so the speeding up is felt (owner: "smoother increase"): ≈ 0.8 s up, ≈ 0.6 s back
		pace(factor) {
			const now = ctx.currentTime;
			const glide = factor > paced ? 0.27 : 0.2;
			paced = factor;
			delay.delayTime.setTargetAtTime(0.38 / factor, now, glide);
			drift1Depth.gain.setTargetAtTime(0.18 / factor, now, glide);
			drift2Depth.gain.setTargetAtTime(0.08 / factor, now, glide);
		},
		stop() {
			for (const o of [lfo, drift1, drift2, swing, pace]) {
				try {
					o.stop();
				} catch {
					/* stopped */
				}
			}
			for (const n of [input, highpass, lfo, depth, drift1, drift1Depth, drift2, drift2Depth, dry, delay, feedback, wet, mix, panner, swing, swingDepth, pace, paceDepth, output]) n.disconnect();
		}
	};
}

/**
 * `haze` ("Dunst", owner 2026-10-08: weed – "not a party drug … in haze"): muffled – a low-pass drifting slowly
 * (≈ 700 … 1300 Hz) – with a short echo that keeps ringing (90 ms, feedback 0.6 – owner: "more feedback").
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
	feedback.gain.value = 0.6;
	const wet = ctx.createGain();
	wet.gain.value = 0;
	wet.gain.setTargetAtTime(0.45, ctx.currentTime, FADE / 3);
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

/**
 * takes the chain out again: the plain line fades back in and stays (the output's route from now on); afterwards only
 * this chain's own connections go – never the output's others (owner 2026-10-08: the sound didn't come back after the
 * account page – the old version disconnected everything 1.2 s later, a chain put in meanwhile included)
 */
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
		try {
			from.disconnect(chain.input);
		} catch {
			/* already gone (a new chain took the output) */
		}
		chain.stop();
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

/**
 * The scroll pace while psychedelic runs (owner 2026-10-08: "if we scroll faster it speeds up the delay time by a
 * multiple, slowing down to a stop it goes back"): the speed of the page's scrolling, smoothed and decaying, as a factor
 * 1 … 4 (≈ 2500 px/s and faster = 4) for the chains' `pace`.
 */
let stopPace: (() => void) | null = null;
let currentPace = 1;
function followScroll() {
	let lastY = window.scrollY;
	let lastT = performance.now();
	let speed = 0;
	let applied = 1;
	const apply = () => {
		const factor = 1 + 3 * Math.min(1, speed / 2500);
		if (Math.abs(factor - applied) < 0.05) return;
		applied = currentPace = factor;
		for (const x of inserts) x.chain.pace?.(factor);
	};
	const onScroll = () => {
		const now = performance.now();
		const v = (Math.abs(window.scrollY - lastY) / Math.max(1, now - lastT)) * 1000;
		lastY = window.scrollY;
		lastT = now;
		// smoothed: the pace builds up with the scrolling rather than jumping
		speed += (Math.min(v, 4000) - speed) * 0.25;
		apply();
	};
	// no scrolling: the pace sinks back to 1 within about half a second
	const decay = setInterval(() => {
		if (performance.now() - lastT > 80) speed *= 0.7;
		if (speed < 20) speed = 0;
		apply();
	}, 100);
	window.addEventListener('scroll', onScroll, { passive: true });
	return () => {
		window.removeEventListener('scroll', onScroll);
		clearInterval(decay);
	};
}

/** an effect over everything the reader hears (null = none) */
export function setAudioEffect(next: AudioEffectKind | null) {
	if (next === kind) return;
	for (const x of inserts.splice(0)) remove(x);
	kind = next;
	if (watch) clearInterval(watch);
	watch = null;
	stopPace?.();
	stopPace = null;
	if (!kind) return;
	attach();
	watch = setInterval(attach, 1000);
	if (kind === 'psychedelic' && typeof window !== 'undefined') stopPace = followScroll();
}

// DEV only: the headless checks look at the graph (which outputs carry the chain, which context each sample plays on)
if (import.meta.env.DEV && typeof window !== 'undefined') {
	(window as unknown as Record<string, unknown>).__scutoonsAudioFx = () => ({
		kind,
		pace: Math.round(currentPace * 100) / 100,
		inserts: inserts.map((x) => ({ state: x.ctx.state, howler: x.ctx === Howler.ctx, master: x.from === Howler.masterGain })),
		howls: ((Howler as unknown as { _howls: { _sounds: { _node?: AudioNode }[]; _src: string | string[] }[] })._howls ?? []).map((h) => ({
			src: String(h._src).split('/').pop(),
			onHowlerCtx: h._sounds.map((s) => s._node?.context === Howler.ctx)
		}))
	});
}

// DEV: a hot module swap (an edit while the page is open) takes this module's chains out first – otherwise the old
// copy keeps its chains in while the new one puts its own in, and the outputs end up tangled (silent music)
if (import.meta.hot) import.meta.hot.dispose(() => setAudioEffect(null));
