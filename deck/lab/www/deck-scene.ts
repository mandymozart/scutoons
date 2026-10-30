// deck-scene.ts (#197): the phone deck's scene – a trimmed copy of the cathedral deck (deck/stage/scenes/cathedral.toml):
// the choir's line (seq → fm), a drone, the incense (pink noise) swaying with an LFO, the mixer, the FFT and the space
// node whose hue / glow colour the being. Two views of it:
// - `deckGraph`: the being on the wire (`deck.graph`), in deck-stage's kinds and ports, so the conductor builds the
//   same body as a native deck's (link.rs `from_wire`);
// - `ENGINE_TOML`: what live-core plays (its batch kinds are lfo smooth mul add osc fm only): the voice and the drone
//   in wasm; the page runs the sequencer, the LFO and the noise itself (deck.ts) and streams their values.
import type { DeckGraph } from '../../../shared/live/src/protocol.ts';
import type { PathKind } from './deck-client.ts';

/** an exposed parameter: its range, default, curve, and the slider's label */
export interface Spec {
	min: number;
	max: number;
	value: number;
	log?: boolean;
	label: string;
}

/** the parameters the page shows (= the being's `expose`, its 3D buttons on the conductor) */
export const PARAMS: Record<string, Spec> = {
	'voice.ratio': { min: 0.5, max: 4, value: 1.5, label: 'FM ratio' },
	'voice.index': { min: 0, max: 8, value: 2, label: 'FM index' },
	'voice.gain': { min: 0, max: 0.3, value: 0.16, label: 'voice' },
	'drone.freq': { min: 30, max: 220, value: 55, log: true, label: 'drone pitch' },
	'drone.gain': { min: 0, max: 0.3, value: 0.1, label: 'drone' },
	'incense.cutoff': { min: 150, max: 8000, value: 1800, log: true, label: 'cutoff' },
	// silent until the slider is raised: the X1's cathedral already hisses its own incense (owner 2026-10-31: two noises)
	'incense.gain': { min: 0, max: 0.15, value: 0, label: 'incense' },
	'sway.rate': { min: 0.02, max: 3, value: 0.07, log: true, label: 'LFO rate' },
	'sway.depth': { min: 0, max: 1, value: 0.6, label: 'LFO depth' },
	'space.hue': { min: 0, max: 1, value: 0.52, label: 'hue' },
	'space.glow': { min: 0, max: 1, value: 0.6, label: 'glow' }
};

/** the choir's line (cathedral.toml `seq`): MIDI notes, 0 = rest; 84 bpm, two steps a beat, gate 0.6 */
/**
 * The voice's melody (owner 2026-10-31: "random, one parameter turns it into a drone and back to more range … stay out
 * of the 12 tone system … sliding scale"): random pitches on a continuous scale around BASE_HZ, gliding into each other.
 * `spread` 0 = a held drone, 1 = ±2 octaves; `rate` = steps per second; `glide` = how much of a step the slide takes.
 * Page-only controls for now – not on the wire (a native random source + its buttons on the being come later).
 */
export const MELODY: Record<string, Spec> = {
	'melody.spread': { min: 0, max: 1, value: 0, label: 'spread' },
	'melody.rate': { min: 0.3, max: 8, value: 2, log: true, label: 'steps / s' },
	'melody.glide': { min: 0, max: 1, value: 0.5, label: 'glide' }
};
/** the drone's pitch when the spread is 0 (≈ D3, but a frequency, not a note) */
export const BASE_HZ = 147;
/** octaves either side at spread 1 */
export const SPREAD_OCTAVES = 2;

export const midiHz = (n: number) => 440 * 2 ** ((n - 69) / 12);

/** the outputs a hose shows (stream.rs: the hoses' sources) – audio outputs stream their block peak */
export const OUTPUTS: Record<string, PathKind> = {
	'seq.cv': 'audio',
	'voice.out': 'audio',
	'drone.out': 'audio',
	'incense.out': 'audio',
	'mix.out': 'audio',
	'sway.out': 'control',
	'fft.level': 'control',
	'fft.bass': 'control'
};

/** every path the deck streams: the exposed inputs + the hoses' sources */
export const STREAMED: Record<string, PathKind> = {
	...Object.fromEntries(Object.entries(PARAMS).map(([p, s]) => [p, { input: [s.min, s.max] as [number, number] }])),
	...OUTPUTS
};

/** the energy's ladder, calm → wild (cathedral.toml `[being] moods`) */
export const LADDER = ['neutral', 'thinking', 'happy', 'laughing', 'surprised'];

/** a character's moods for the ladder: the ladder's moods it has (in order), else all it has, else neutral */
export function ladderFor(available: string[]): string[] {
	const has = LADDER.filter((m) => available.includes(m));
	return has.length >= 2 ? has : available.length ? available : ['neutral'];
}

/** the being on the wire */
export function deckGraph(character: string, moods: string[]): DeckGraph {
	const node = (id: string, kind: string) => ({ id, kind });
	return {
		nodes: [
			node('seq', 'seq'),
			node('voice', 'fm'),
			node('drone', 'osc'),
			node('incense', 'noise'),
			node('sway', 'lfo'),
			node('mix', 'mixer'),
			node('fft', 'fft'),
			node('space', 'space'),
			node('out', 'audio_out')
		],
		links: [
			{ from: 'seq.cv', to: 'voice.cv' },
			{ from: 'voice.out', to: 'mix.in1' },
			{ from: 'drone.out', to: 'mix.in2' },
			{ from: 'incense.out', to: 'mix.in3' },
			{ from: 'sway.out', to: 'incense.cutoff' },
			{ from: 'mix.out', to: 'fft.in' },
			{ from: 'mix.out', to: 'out.in' },
			{ from: 'fft.level', to: 'space.pulse', scale: 0.6 }
		],
		character,
		size: 0.5,
		moods,
		expose: Object.keys(PARAMS)
	};
}

/** live-core's part: the voice (its gain = an envelope × the voice slider) and the drone */
export const ENGINE_TOML = `
[[node]]
id = "env"
kind = "smooth"
time = 0.04
[[node]]
id = "vgain"
kind = "mul"
b = ${PARAMS['voice.gain'].value}
[[link]]
from = "env"
to = "vgain.a"
[[node]]
id = "voice"
kind = "fm"
freq = ${midiHz(50).toFixed(2)}
ratio = ${PARAMS['voice.ratio'].value}
index = ${PARAMS['voice.index'].value}
[[link]]
from = "vgain"
to = "voice.gain"
[[node]]
id = "drone"
kind = "osc"
freq = ${PARAMS['drone.freq'].value}
gain = ${PARAMS['drone.gain'].value}
[[node]]
id = "fifth"
kind = "osc"
freq = ${PARAMS['drone.freq'].value * 1.5}
gain = ${PARAMS['drone.gain'].value * 0.3}
`;

/** a slider's position 0..1 ↔ the parameter's value (log ranges for pitches, cutoffs, rates) */
export const toValue = (s: Spec, t: number) => (s.log ? s.min * (s.max / s.min) ** t : s.min + (s.max - s.min) * t);
export const toPos = (s: Spec, v: number) => (s.log ? Math.log(v / s.min) / Math.log(s.max / s.min) : (v - s.min) / (s.max - s.min));

/** The energy ladder as a native deck walks it (being.rs `step_head`): the FFT level and the tempo, quick up, slow
 *  down; a finger on a pad lifts it a little; a new rung once the last held `hold` s. */
export class Moods {
	energy = 0;
	private rung = 0;
	private since = -Infinity;

	constructor(
		public moods: string[],
		private readonly hold = 1.5,
		private readonly bpm = 84
	) {}

	/** `level` 0..1 (the FFT's), `touch` 0..1 at `time` s → the mood */
	update(time: number, dt: number, level: number, touch: number): string {
		const tempo = Math.min(1, Math.max(0, (this.bpm - 60) / 100));
		const target = Math.min(1, 0.8 * Math.min(1, level * 1.3) + 0.2 * tempo + 0.25 * touch);
		this.energy += (target - this.energy) * Math.min(1, dt / (target > this.energy ? 0.3 : 1.5));
		const n = this.moods.length;
		const want = Math.min(n - 1, Math.round(this.energy * (n - 1)));
		if (want !== this.rung && time - this.since >= this.hold) {
			this.rung = want;
			this.since = time;
		}
		this.rung = Math.min(this.rung, n - 1);
		return this.moods[this.rung];
	}
}
