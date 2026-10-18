// @scutoons/reader/score: the score view as its own entry, so an app loads it only when a score opens (a dynamic
// import – research/TEXT-ACROSS-SLICES.md §9). Kept out of the package index: its importers would carry it eagerly.
export { default as ScoreView, DEFAULT_SCORE_LABELS, type ScoreLabels } from './score-view.svelte';
export { default as ScoreIllumination } from './score-illumination.svelte';
export { default as ScoreCollage } from './score-collage.svelte';
// collage pieces on scores (agents/research/COLLAGE.md §4): where they sit, the layout they sit in – the creator's stage
export { default as ScorePieces } from './score-pieces.svelte';
export { anchorScorePiece, pageFrame, pageGeometry, placeScorePiece, type BarBox, type PageGeometry, type ScorePlace, type StaffBox } from './score-anchors.ts';
export { barIndex, barTime, barX, findBar, type BarRef, type Beat } from './score-bars.ts';
export { DEFAULT_ZOOM, PHONE_ZOOM, ZOOMS, scoreOptions, stepZoom } from './score-layout.ts';
