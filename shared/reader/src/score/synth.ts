/**
 * The score view's player (owner 2026-10-14: "can it play the notes with a synthesizer in the browser … you can touch the
 * screen, and when you slide, it changes some delay and adds the mix to it"): the shown part's notes from Verovio's
 * timemap on a small Web Audio synth – no samples – through a delay the reader plays with a finger (scoreSynth.pad).
 * The pure parts (the notes from a timemap, the pad's mapping) are tested in test/score-synth.test.ts.
 */

/** one entry of Verovio's renderToTimemap(): at `tstamp` ms these note ids start / stop sounding */
export interface TimemapEntry {
	tstamp: number;
	on?: string[];
	off?: string[];
}

/** a note to play: start and length in ms from the score's beginning, MIDI pitch */
export interface ScoreNote {
	id: string;
	at: number;
	dur: number;
	pitch: number;
}

/**
 * the notes of a timemap: each `on` id paired with its next `off`; `pitchOf` = the MIDI pitch of a note id (Verovio's
 * getMIDIValuesForElement) – a note without a pitch (a rest, an unpitched mark) is left out
 */
export function notesFromTimemap(timemap: TimemapEntry[], pitchOf: (id: string) => number | null): ScoreNote[] {
	const started = new Map<string, number>();
	const notes: ScoreNote[] = [];
	for (const entry of timemap) {
		for (const id of entry.off ?? []) {
			const at = started.get(id);
			if (at === undefined) continue;
			started.delete(id);
			const pitch = pitchOf(id);
			if (pitch !== null && pitch > 0) notes.push({ id, at, dur: Math.max(30, entry.tstamp - at), pitch });
		}
		for (const id of entry.on ?? []) started.set(id, entry.tstamp);
	}
	return notes.sort((a, b) => a.at - b.at);
}

/** what a finger on the pad sets: x (0 left … 1 right) = the echo's time and how long it rings, y (0 top … 1 bottom)
 *  = how much of it is mixed in (the top all echo, the bottom dry) */
export interface PadEffect {
	/** seconds between the echoes */
	time: number;
	/** 0 … 1: how much of each echo comes back */
	feedback: number;
	/** 0 … 1: the echo's share in the mix */
	wet: number;
}

const clamp01 = (v: number) => Math.min(1, Math.max(0, v));

export function padEffect(x: number, y: number): PadEffect {
	const cx = clamp01(x);
	return {
		// exponential: the short slap-backs get as much room as the long echoes
		time: 0.06 * Math.pow(0.9 / 0.06, cx),
		feedback: 0.2 + 0.55 * cx,
		wet: 1 - clamp01(y)
	};
}

/** the note's pan: low voices a little left, high ones a little right (a choir spread out) */
export const panOf = (pitch: number) => Math.min(0.5, Math.max(-0.5, (pitch - 62) / 36));

const hz = (pitch: number) => 440 * Math.pow(2, (pitch - 69) / 12);

/** how far ahead notes are scheduled (s) and how often the scheduler looks (ms) */
const LOOKAHEAD = 0.25;
const TICK = 40;

export class ScoreSynth {
	private ctx: AudioContext;
	private out: GainNode;
	private delay: DelayNode;
	private feedback: GainNode;
	private wet: GainNode;
	private timer: ReturnType<typeof setInterval> | null = null;
	private notes: ScoreNote[] = [];
	private next = 0;
	/** the context time of the score's 0 ms */
	private origin = 0;
	private voices = new Set<OscillatorNode>();

	/** create it inside a click: browsers only let a gesture start audio */
	constructor() {
		this.ctx = new AudioContext();
		const ctx = this.ctx;
		this.out = ctx.createGain();
		this.out.gain.value = 0.7;
		const comp = ctx.createDynamicsCompressor();
		this.out.connect(comp).connect(ctx.destination);
		// the echo: a delay whose repeats come back darker (a low-pass in the loop), mixed in by `wet`
		this.delay = ctx.createDelay(1.5);
		this.feedback = ctx.createGain();
		const tone = ctx.createBiquadFilter();
		tone.type = 'lowpass';
		tone.frequency.value = 2600;
		this.wet = ctx.createGain();
		this.wet.gain.value = 0;
		this.delay.connect(tone).connect(this.feedback).connect(this.delay);
		tone.connect(this.wet).connect(this.out);
		this.pad(0.5, 1);
	}

	/** play `notes` from `fromMs` on; `onend` once the last note has sounded */
	play(notes: ScoreNote[], fromMs: number, onend: () => void) {
		this.stop();
		void this.ctx.resume();
		this.notes = notes;
		this.next = notes.findIndex((n) => n.at >= fromMs);
		if (this.next < 0) return onend();
		this.origin = this.ctx.currentTime + 0.12 - fromMs / 1000;
		const last = notes.reduce((end, n) => Math.max(end, n.at + n.dur), 0);
		this.timer = setInterval(() => {
			const horizon = (this.ctx.currentTime + LOOKAHEAD - this.origin) * 1000;
			while (this.next < this.notes.length && this.notes[this.next].at <= horizon) this.voice(this.notes[this.next++]);
			if (this.next >= this.notes.length && this.now() > last + 400) {
				this.stop();
				onend();
			}
		}, TICK);
	}

	/** where the playback is (ms in the score) */
	now(): number {
		return (this.ctx.currentTime - this.origin) * 1000;
	}

	stop() {
		if (this.timer) clearInterval(this.timer);
		this.timer = null;
		const t = this.ctx.currentTime;
		for (const osc of this.voices) {
			try {
				osc.stop(t + 0.05);
			} catch {
				/* already stopped */
			}
		}
		this.voices.clear();
	}

	/** a finger on the pad (x, y in 0 … 1 of the sheet) – glides there */
	pad(x: number, y: number) {
		const fx = padEffect(x, y);
		const t = this.ctx.currentTime;
		this.delay.delayTime.setTargetAtTime(fx.time, t, 0.08);
		this.feedback.gain.setTargetAtTime(fx.feedback, t, 0.05);
		this.wet.gain.setTargetAtTime(fx.wet * 0.9, t, 0.05);
	}

	/** the finger lifted: the mix glides back to dry, the echoes still in the air ring out */
	release() {
		this.wet.gain.setTargetAtTime(0, this.ctx.currentTime, 0.4);
	}

	close() {
		this.stop();
		void this.ctx.close();
	}

	/** one note: a soft triangle with a quiet sine an octave up, a short swell in and a gentle fade out */
	private voice(note: ScoreNote) {
		const ctx = this.ctx;
		const start = Math.max(ctx.currentTime, this.origin + note.at / 1000);
		const end = start + note.dur / 1000;
		const env = ctx.createGain();
		env.gain.setValueAtTime(0, start);
		env.gain.linearRampToValueAtTime(0.16, start + 0.02);
		env.gain.setTargetAtTime(0.1, start + 0.02, 0.15);
		env.gain.setTargetAtTime(0, end, 0.08);
		const pan = ctx.createStereoPanner();
		pan.pan.value = panOf(note.pitch);
		env.connect(pan);
		pan.connect(this.out);
		pan.connect(this.delay);
		for (const [type, mult, level] of [['triangle', 1, 1], ['sine', 2, 0.25]] as const) {
			const osc = ctx.createOscillator();
			osc.type = type;
			osc.frequency.value = hz(note.pitch) * mult;
			const g = ctx.createGain();
			g.gain.value = level;
			osc.connect(g).connect(env);
			osc.start(start);
			osc.stop(end + 0.5);
			this.voices.add(osc);
			osc.onended = () => this.voices.delete(osc);
		}
	}
}
