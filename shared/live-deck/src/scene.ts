// scene.ts (#197, @scutoons/live-deck since #198): the page's side of the deck's scene. The scene itself is the core's
// (#360): without a rig the deck plays the X1's cathedral – deck-space plays it (space-web `deck_scene`, a `RigPlayed`)
// and live-core loads its part; no scene, patch, range or label is written here. The mood's energy ladder is the core's
// too (#367: deck-space mood.rs, space-web `DeckState`). What is left: the slider's `Spec` (the knobs' shape) and the
// slider position ↔ value (#369 moves it into the core).

/** an exposed parameter: its range, default, curve, and the slider's label */
export interface Spec {
	min: number;
	max: number;
	value: number;
	log?: boolean;
	label: string;
}

/** a slider's position 0..1 ↔ the parameter's value (log ranges for pitches, cutoffs, rates) */
export const toValue = (s: Spec, t: number) => (s.log ? s.min * (s.max / s.min) ** t : s.min + (s.max - s.min) * t);
export const toPos = (s: Spec, v: number) => (s.log ? Math.log(v / s.min) / Math.log(s.max / s.min) : (v - s.min) / (s.max - s.min));
