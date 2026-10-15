/**
 * Score looks (owner 2026-10-15: "my own fancy fonts … much more interesting notation fonts … colors"): a look is a
 * set of Verovio options (the music font Gootville – "old school" – and the engraving's line weights) plus the CSS in
 * score-looks.css, keyed by `data-look` on the sheet. `boil` = the hand-drawn lines wobble (cartoon "boiling lines",
 * score-boil.svelte), off under reduced motion. The plain view has no look (Verovio's defaults, Leipzig).
 * Draft in the lab (/account/lab/score) – not yet chosen for readers.
 */
export interface ScoreLook {
	id: string;
	/** Verovio options on top of the view's own (page size, margins …) */
	options: Record<string, unknown>;
	boil?: boolean;
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
	/** sepia ink on old paper, lettering for the title and the tempo */
	{ id: 'old-school', options: PEN, boil: true },
	/** the same pen, every notehead in its pitch's colour (C red … B pink – children's colour notation) */
	{ id: 'old-school-colour', options: { ...PEN, svgAdditionalAttribute: ['note@pname'] }, boil: true }
];
