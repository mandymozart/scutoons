// @scutoons/reader/score-anchors: the pure half of the score collage (agents/research/COLLAGE.md §4.2) – where pieces
// sit on a page, the file's bars, the page layout – with no Svelte and no Verovio, for code and node tests outside the
// score view (the creator's score stage, lib/editor/score-collage.ts). The score view's entry (./index.ts) re-exports it.
export { anchorScorePiece, pageFrame, pageGeometry, placeScorePiece, type BarBox, type PageGeometry, type ScorePlace, type StaffBox } from './score-anchors.ts';
export { barIndex, barTime, barX, findBar, type BarRef, type Beat } from './score-bars.ts';
export { DEFAULT_ZOOM, PHONE_ZOOM, ZOOMS, scoreOptions, stepZoom } from './score-layout.ts';
