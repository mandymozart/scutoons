// Edits of a collage piece (agents/research/COLLAGE.md §7) and of a slice's layer list with pieces (§3). Pure
// functions on the stored piece / slice – the inspector runs them inside one `doc.edit` (one undo step).
//
// A piece names a look (`preset`, collage.ts COLLAGE_LOOKS) and may replace single keys of it in `style`: a key replaces
// the look's whole (a whole shadow), `null` = off, no key = the look's value. So an effect edit is copy-on-write: the
// first change of a look's shadow copies the look's shadow into `style` and changes that copy; clearing an effect the
// look has writes `null`, one it lacks just drops the key; an emptied `style` goes.
import {
	collageLook,
	getAt,
	pieceStyle,
	setSliceLayers,
	sliceLayers,
	type CollagePiece,
	type PieceBlur,
	type PieceFigure,
	type PieceOutline,
	type PieceShadow,
	type PieceStyle,
	type SliceLayerRef
} from '@scutoons/content';

/** the style keys that are effects (objects / a fill): cleared to `null` when the look has them */
export type EffectKey = 'fill' | 'outline' | 'shadow' | 'blur';
/** the plain keys: no key = the look's value */
export type PlainKey = 'blend' | 'opacity';

/** what a new effect starts from when neither the piece nor its look has one (sizes in % of the piece's width) */
export const EFFECT_DEFAULTS: { outline: PieceOutline; shadow: PieceShadow; blur: PieceBlur } = {
	outline: { color: '#ffffff', width: 2 },
	shadow: { color: '#111111', x: 2, y: 2, blur: 4 },
	blur: { amount: 4 }
};
/** a progressive blur's start: sharp at the top, blurred from the bottom */
export const RAMP_POINTS = { from: { x: 50, y: 0 }, to: { x: 50, y: 100 } };

const copy = <T>(value: T): T => (value === undefined || value === null ? value : JSON.parse(JSON.stringify(value)));

/** drops the piece's `style` when nothing is left in it */
function tidy(piece: CollagePiece): void {
	const style = piece.style;
	if (!style) return;
	for (const key of Object.keys(style) as (keyof PieceStyle)[]) if (style[key] === undefined) delete style[key];
	if (!Object.keys(style).length) delete piece.style;
}

/** the piece's own style (made when missing; read back through the piece – a $state proxy, GOTCHAS) */
function ownStyle(piece: CollagePiece): PieceStyle {
	if (!piece.style) piece.style = {};
	return piece.style!;
}

/** a look (none = as drawn): writes `preset` and drops the piece's own values */
export function setPreset(piece: CollagePiece, look: string | undefined): void {
	if (look) piece.preset = look;
	else delete piece.preset;
	delete piece.style;
}

/** a plain style key (blend, opacity): `undefined` = the look's value */
export function setPlain<K extends PlainKey>(piece: CollagePiece, key: K, value: PieceStyle[K] | undefined): void {
	if (value === undefined) {
		if (piece.style) delete piece.style[key];
	} else ownStyle(piece)[key] = value;
	tidy(piece);
}

/** an effect as a whole: a value replaces the look's; `undefined` clears it (`null` when the look has it, else no key) */
export function setEffect<K extends EffectKey>(piece: CollagePiece, key: K, value: Exclude<PieceStyle[K], null> | undefined): void {
	const look = collageLook(piece.preset);
	if (value !== undefined) ownStyle(piece)[key] = copy(value) as PieceStyle[K];
	else if (look?.[key] !== undefined && look[key] !== null) ownStyle(piece)[key] = null as PieceStyle[K];
	else if (piece.style) delete piece.style[key];
	tidy(piece);
}

/**
 * Changes some fields of an effect (a shadow's x, an outline's colour): the effect as drawn – the piece's own, else the
 * look's (copied) – else `EFFECT_DEFAULTS`, with the change over it, becomes the piece's own
 */
export function editEffect<K extends 'outline' | 'shadow' | 'blur'>(piece: CollagePiece, key: K, change: Partial<(typeof EFFECT_DEFAULTS)[K]>): void {
	const drawn = pieceStyle(piece)[key] as (typeof EFFECT_DEFAULTS)[K] | undefined;
	const next = { ...copy(drawn ?? EFFECT_DEFAULTS[key]), ...change } as Record<string, unknown>;
	for (const k of Object.keys(next)) if (next[k] === undefined) delete next[k];
	setEffect(piece, key, next as unknown as Exclude<PieceStyle[K], null>);
}

/** an even blur (no points) or a ramp (sharp at `from`, blurred from `to`) – the amount kept */
export function setBlurRamp(piece: CollagePiece, ramp: boolean): void {
	const amount = pieceStyle(piece).blur?.amount ?? EFFECT_DEFAULTS.blur.amount;
	setEffect(piece, 'blur', ramp ? { amount, ...copy(RAMP_POINTS) } : { amount });
}

/** a character picture re-picked (mood, view): the figure, the resolved file (in the host's path form) and its ratio */
export function setFigure(piece: CollagePiece, figure: PieceFigure, src: string, ratio: number | null): void {
	piece.figure = copy(figure);
	piece.src = src;
	if (ratio && ratio > 0) piece.ratio = ratio;
}

// ── the slice's layer list with pieces (COLLAGE.md §3 "Layers list") ──────────────────────────────────────────────

/** the rows of a slice's layer list, front first: the stack (text boxes, items, areas), then the pieces (front piece first) */
export function layerRows(slice: unknown): SliceLayerRef[] {
	return sliceLayers(slice as Parameters<typeof sliceLayers>[0]).reverse();
}

/** a row may move only within its side: pieces among pieces, the stack among the stack */
export function canMoveLayerRow(rows: readonly SliceLayerRef[], from: number, to: number): boolean {
	if (from === to || !rows[from] || !rows[to]) return false;
	return (rows[from].kind === 'piece') === (rows[to].kind === 'piece');
}

/**
 * Moves a row of the layer list (`layerRows` order): piece ↔ piece reorders `pieces[]` (back → front), stack ↔ stack
 * renumbers the stack (`setSliceLayers`); a move across is refused (false, nothing changed)
 */
export function moveLayerRows(slice: any, from: number, to: number): boolean {
	const rows = layerRows(slice);
	if (!slice || !canMoveLayerRow(rows, from, to)) return false;
	const [row] = rows.splice(from, 1);
	rows.splice(to, 0, row);
	if (row.kind === 'piece') {
		const pieces = [...(slice.pieces ?? [])];
		slice.pieces = rows.filter((r) => r.kind === 'piece').reverse().map((r) => pieces[r.index]);
	} else setSliceLayers(slice, rows.filter((r) => r.kind !== 'piece').reverse());
	return true;
}

/** the piece at `path` of a document (`[...slicePath, 'pieces', i]`) – for edits in `doc.edit` */
export function pieceAt(file: unknown, path: (string | number)[]): CollagePiece | undefined {
	const piece = getAt(file, path);
	return piece && typeof piece === 'object' ? (piece as CollagePiece) : undefined;
}
