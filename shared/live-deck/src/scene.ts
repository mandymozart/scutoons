// scene.ts (#197, @scutoons/live-deck since #198): the page's side of the deck's scene. The scene itself is the core's
// (#360): without a rig the deck plays the X1's cathedral – deck-space plays it (space-web `deck_scene`, a `RigPlayed`)
// and live-core loads its part; no scene, patch, range or label is written here. What is left: the slider's `Spec`
// (the knobs' shape), the slider position ↔ value (#369 moves it into the core) and the energy's ladder (#367).

/** an exposed parameter: its range, default, curve, and the slider's label */
export interface Spec {
	min: number;
	max: number;
	value: number;
	log?: boolean;
	label: string;
}

/** the energy's ladder, calm → wild (cathedral.toml `[being] moods`) */
export const LADDER = ['neutral', 'thinking', 'happy', 'laughing', 'surprised'];

/** a character's moods for the ladder: the ladder's moods it has (in order), else all it has, else neutral */
export function ladderFor(available: string[]): string[] {
	const has = LADDER.filter((m) => available.includes(m));
	return has.length >= 2 ? has : available.length ? available : ['neutral'];
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
		/** the clock's tempo (#321: the core's, set by the deck each clock frame; 120 = deck-core's default) */
		public bpm = 120
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
