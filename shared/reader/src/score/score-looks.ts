/**
 * Score looks (owner 2026-10-15: "my own fancy fonts … much more interesting notation fonts … colors"): a look is a
 * set of Verovio options (the music font Gootville – "old school" – and the engraving's line weights) plus the CSS in
 * score-looks.css, keyed by `data-look` on the sheet. `boil` = the hand-drawn lines wobble (cartoon "boiling lines",
 * score-boil.svelte), off under reduced motion. The plain view has no look (Verovio's defaults, Leipzig).
 * `medieval` (owner 2026-10-17: "looks like a medieval score … the portrait avatar and full figures to illustrate the
 * frames"): parchment, red staff lines, blue rubrics, a blackletter title; the frame and figures are the page's `decor`.
 * Draft in the lab (/account/lab/score) – not yet chosen for readers.
 */
export interface ScoreLook {
	id: string;
	/** Verovio options on top of the view's own (page size, margins …) */
	options: Record<string, unknown>;
	boil?: boolean;
	/** page margins in rem (room for a frame and figures – score-illumination.svelte); default Verovio's own */
	margin?: { top: number; side: number; bottom: number };
}

/** a slightly heavier pen than Verovio's defaults (staff 0.15, stem 0.2, bar 0.3, ledger 0.25, slur 0.6, tie 0.5) */
const PEN = {
	font: 'Gootville',
	staffLineWidth: 0.18,
	stemWidth: 0.26,
	barLineWidth: 0.36,
	ledgerLineThickness: 0.3,
	slurMidpointThickness: 0.75,
	tieMidpointThickness: 0.6,
	lyricSize: 4.8
};

export const SCORE_LOOKS: ScoreLook[] = [
	/** sepia ink on old paper, lettering for the title */
	{ id: 'old-school', options: PEN, boil: true },
	/** the same pen, every notehead in its pitch's colour (C red … B pink – children's colour notation) */
	{ id: 'old-school-colour', options: { ...PEN, svgAdditionalAttribute: ['note@pname'] }, boil: true },
	/** an illuminated manuscript: room around the music for the frame, the initial and the figures in the margins */
	{ id: 'medieval', options: PEN, margin: { top: 3.75, side: 2.25, bottom: 6.25 } },
	/** a 90s collage (owner 2026-10-17: "go real wild … huge and overlay … outlines or gradients … ditch the brown
	 *  background … make it messy"): white paper, the library's shapes huge behind and over the music, big and small
	 *  figures – score-collage.svelte is the page's `decor` */
	{ id: 'wild', options: PEN, margin: { top: 3, side: 1.25, bottom: 1.5 } }
];
