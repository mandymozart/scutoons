// scene.ts (#197, @scutoons/live-deck since #198): the page's side of the deck's scene. The scene itself is the core's
// (#425): without a rig the deck plays the silent scene – the being, no sound; deck-space plays it (a `RigPlayed`) and
// live-core loads its part; the Cathedral is a test scene (`--scene cathedral`), no fallback. No scene, patch, range or
// label is written here. The mood's energy ladder is the core's
// too (#367: deck-space mood.rs, space-web `DeckState`), and so is a knob's value ↔ 0..1 (#369: `DeckState.norm` /
// `value_at`, `DeckControls`). What is left: the slider's `Spec` as the core answered it.

/** an exposed parameter: its range, default, curve, and the slider's label (deck-space `deck::Slider`) */
export interface Spec {
	min: number;
	max: number;
	value: number;
	log?: boolean;
	label: string;
}
