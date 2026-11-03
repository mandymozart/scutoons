// scene.ts (#197, @scutoons/live-deck since #198; #320): the phone deck's own scene when its character has no rig – a
// trimmed copy of the cathedral deck (deck/native/scenes/cathedral.toml): the choir's voice on a generated melody (a walk),
// a drone, the incense (pink noise) its cutoff swayed by an LFO, the mixer, the FFT and the space node whose hue / glow
// colour the being. One description, two views of it:
// - `FIXED_SCENE`: the scene live-core's engine plays whole (the same engine as a native deck – #320: the page plays
//   nothing itself any more);
// - `deckGraph`: the being on the wire (`deck.graph`), so the conductor builds the same body as a native deck's.
// The sliders' ranges and labels (`PARAMS`, `MELODY`) are the page's UI; their keys are the scene's paths.
import type { DeckGraph } from '@scutoons/live';
import type { PathKind } from './client.ts';

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
	// the base under the LFO's sway (#188: the knob turns the base, the sway rides on top)
	'incense.cutoff': { min: 150, max: 8000, value: 1800, log: true, label: 'cutoff' },
	// silent until the slider is raised: the X1's cathedral already hisses its own incense (owner 2026-10-31: two noises)
	'incense.gain': { min: 0, max: 0.15, value: 0, label: 'incense' },
	'sway.rate': { min: 0.02, max: 3, value: 0.07, log: true, label: 'LFO rate' },
	'sway.depth': { min: 0, max: 1, value: 0.6, label: 'LFO depth' },
	'space.hue': { min: 0, max: 1, value: 0.52, label: 'hue' },
	'space.glow': { min: 0, max: 1, value: 0.6, label: 'glow' }
};

/**
 * The voice's melody (owner 2026-10-31: "random, one parameter turns it into a drone and back to more range … stay out
 * of the 12 tone system … sliding scale"): the scene's `walk` – a seeded random walk on a continuous scale around 147 Hz
 * on the tempo clock (#236), the same melody on every deck. `spread` 0 = a held drone, 1 = ±2 octaves; `rate` = steps a
 * beat; `glide` = how much of a step the slide takes. Page-only controls (not on the wire).
 */
export const MELODY: Record<string, Spec> = {
	'melody.spread': { min: 0, max: 1, value: 0, label: 'spread' },
	'melody.rate': { min: 0.25, max: 8, value: 2, log: true, label: 'steps / beat' },
	'melody.glide': { min: 0, max: 1, value: 0.5, label: 'glide' }
};

/** a node of the scene: its id, kind and params (numbers, strings) */
type Node = { id: string; kind: string } & Record<string, number | string>;
type Link = { from: string; to: string; scale?: number; offset?: number };

const value = (path: string) => (PARAMS[path] ?? MELODY[path]).value;

/** The fixed scene. The sway is unipolar (the engine's LFO: 0..depth – #320 resolved the page's old ±depth sway): its
 *  link lifts the cutoff above the slider's base by up to `depth` × 2500 Hz. */
const NODES: Node[] = [
	{ id: 'melody', kind: 'walk', seed: 84, base: 147, octaves: 2, spread: value('melody.spread'), rate: value('melody.rate'), glide: value('melody.glide') },
	{ id: 'voice', kind: 'fm', freq: 147, ratio: value('voice.ratio'), index: value('voice.index'), gain: value('voice.gain') },
	{ id: 'drone', kind: 'osc', freq: value('drone.freq'), gain: value('drone.gain') },
	{ id: 'incense', kind: 'noise', color: 'pink', cutoff: value('incense.cutoff'), gain: value('incense.gain') },
	{ id: 'sway', kind: 'lfo', rate: value('sway.rate'), depth: value('sway.depth') },
	{ id: 'mix', kind: 'mixer' },
	{ id: 'fft', kind: 'fft' },
	{ id: 'space', kind: 'space', hue: value('space.hue'), glow: value('space.glow') },
	{ id: 'out', kind: 'audio_out' }
];
const LINKS: Link[] = [
	{ from: 'melody.cv', to: 'voice.cv' },
	{ from: 'voice.out', to: 'mix.in1' },
	{ from: 'drone.out', to: 'mix.in2' },
	{ from: 'incense.out', to: 'mix.in3' },
	{ from: 'sway.out', to: 'incense.cutoff', scale: 2500, offset: value('incense.cutoff') },
	{ from: 'mix.out', to: 'fft.in' },
	{ from: 'mix.out', to: 'out.in' },
	{ from: 'fft.level', to: 'space.pulse', scale: 0.6 }
];

/** a TOML value: a number, or a string */
const tomlValue = (v: number | string) => (typeof v === 'number' ? (Number.isInteger(v) ? `${v}.0` : `${v}`) : JSON.stringify(v));

/** the scene live-core plays (the deck's TOML) */
export const FIXED_SCENE = [
	...NODES.map((n) => ['[[node]]', ...Object.entries(n).map(([k, v]) => `${k} = ${k === 'seed' ? v : tomlValue(v)}`)].join('\n')),
	...LINKS.map((l) => ['[[link]]', ...Object.entries(l).map(([k, v]) => `${k} = ${tomlValue(v)}`)].join('\n'))
].join('\n') + '\n';

/** the outputs a hose shows (stream.rs: the hoses' sources) – audio outputs stream their block peak */
export const OUTPUTS: Record<string, PathKind> = {
	'melody.cv': 'audio',
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

/** the being on the wire: the fixed scene's nodes and links */
export function deckGraph(character: string, moods: string[]): DeckGraph {
	return {
		nodes: NODES.map((n) => ({ id: n.id, kind: n.kind })),
		links: LINKS.map(({ from, to, scale }) => ({ from, to, ...(scale !== undefined && scale !== 1 ? { scale } : {}) })),
		character,
		size: 0.5,
		moods,
		expose: Object.keys(PARAMS)
	};
}

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
