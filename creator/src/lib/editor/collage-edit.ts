// Edits of a collage piece (agents/research/COLLAGE.md §7) and of a slice's layer list with pieces (§3). Pure
// functions on the stored piece / slice – the inspector runs them inside one `doc.edit` (one undo step).
//
// A piece names a look (`preset`: a code look, collage.ts COLLAGE_LOOKS, or a custom preset of `colorations` – §12) and
// may replace single keys of it in `style`: a key replaces the look's whole (a whole shadow), `null` = off, no key = the
// look's value. So an effect edit is copy-on-write: the first change of a look's shadow copies the look's shadow into
// `style` and changes that copy; clearing an effect the look has writes `null`, one it lacks just drops the key; an
// emptied `style` goes. `presets` = the stored custom presets (presets-doc.ts `ownPresets`; none = code looks only) –
// $state proxies of another document, so every value taken from them is a JSON copy.
import {
	collageLook,
	getAt,
	pieceStyle,
	setSliceLayers,
	sliceLayers,
	type CollagePiece,
	type PieceBlur,
	type PieceFigure,
	type PieceGradient,
	type PieceOutline,
	type PieceShadow,
	type PieceStyle,
	type Path,
	type PresetList,
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
/** a fill's kind: none (as drawn), one colour, a gradient */
export type FillKind = 'none' | 'color' | 'gradient';
/** a blur's kind: even, or a ramp between two points */
export type BlurKind = 'even' | 'ramp';
/** the colour a new fill starts with when there is none to start from */
const FILL_INK = '#111111';

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
export function setEffect<K extends EffectKey>(piece: CollagePiece, key: K, value: Exclude<PieceStyle[K], null> | undefined, presets?: PresetList): void {
	const look = collageLook(piece.preset, presets);
	if (value !== undefined) ownStyle(piece)[key] = copy(value) as PieceStyle[K];
	else if (look?.[key] !== undefined && look[key] !== null) ownStyle(piece)[key] = null as PieceStyle[K];
	else if (piece.style) delete piece.style[key];
	tidy(piece);
}

/** the look's own value of a style key (none / off = undefined) */
const lookValue = <K extends EffectKey>(piece: CollagePiece, key: K, presets?: PresetList): Exclude<PieceStyle[K], null> | undefined =>
	(collageLook(piece.preset, presets)?.[key] ?? undefined) as Exclude<PieceStyle[K], null> | undefined;

type EditableEffect = 'outline' | 'shadow' | 'blur';

/**
 * Changes some fields of an effect (a shadow's x, an outline's colour): the effect as drawn – the piece's own, else the
 * look's (copied) – with the change over it becomes the piece's own. An effect that is off starts from `base` (what a
 * field just cleared), else the look's (switched off over the look), else `EFFECT_DEFAULTS`
 */
export function editEffect<K extends EditableEffect>(
	piece: CollagePiece,
	key: K,
	change: Partial<(typeof EFFECT_DEFAULTS)[K]>,
	base?: (typeof EFFECT_DEFAULTS)[K],
	presets?: PresetList
): void {
	const drawn = pieceStyle(piece, presets)[key] as (typeof EFFECT_DEFAULTS)[K] | undefined;
	const start = drawn ?? base ?? (lookValue(piece, key, presets) as (typeof EFFECT_DEFAULTS)[K] | undefined) ?? EFFECT_DEFAULTS[key];
	const next = { ...copy(start), ...change } as Record<string, unknown>;
	for (const k of Object.keys(next)) if (next[k] === undefined) delete next[k];
	setEffect(piece, key, next as unknown as Exclude<PieceStyle[K], null>, presets);
}

/** what an effect's colour field cleared, per piece and key (`setEffectColor`) – with the look it was cleared under */
export type ClearedEffects = Map<string, { preset?: string; value: unknown }>;

/**
 * An outline's / shadow's colour field: empty turns the effect off – ColorField sends that on every keystroke, so a
 * colour selected and retyped passes through it – and `cleared` keeps the effect as it was drawn; a colour brings the
 * effect back with that colour – as it was (its width, offsets, blur), when the look is still the one it was cleared
 * under, else the look's, else the defaults
 */
export function setEffectColor(piece: CollagePiece, key: 'outline' | 'shadow', color: string | undefined, cleared: ClearedEffects, presets?: PresetList): void {
	const at = `${piece.id}.${key}`;
	if (color) {
		const was = cleared.get(at);
		const base = was && was.preset === piece.preset ? (copy(was.value) as (typeof EFFECT_DEFAULTS)[typeof key]) : undefined;
		return editEffect(piece, key, { color }, base, presets);
	}
	const drawn = pieceStyle(piece, presets)[key];
	if (drawn) cleared.set(at, { preset: piece.preset, value: copy(drawn) });
	setEffect(piece, key, undefined, presets);
}

export const fillKind = (fill: PieceStyle['fill']): FillKind => (fill === undefined || fill === null ? 'none' : typeof fill === 'string' ? 'color' : 'gradient');
export const blurKind = (blur: PieceStyle['blur']): BlurKind => (blur?.from && blur?.to ? 'ramp' : 'even');

/** a #rgb / #rrggbb colour's relative luminance (WCAG: 0 black … 1 white); 0 for anything else */
function luminance(color: string): number {
	const hex = /^#([0-9a-f]{3}|[0-9a-f]{6})$/i.exec(color)?.[1];
	if (!hex) return 0;
	const full = hex.length === 3 ? [...hex].map((c) => c + c).join('') : hex;
	const [r, g, b] = [0, 2, 4].map((i) => {
		const c = parseInt(full.slice(i, i + 2), 16) / 255;
		return c <= 0.04045 ? c / 12.92 : ((c + 0.055) / 1.055) ** 2.4;
	});
	return 0.2126 * r + 0.7152 * g + 0.0722 * b;
}

/**
 * The fill switched to another kind: a colour starts from the fill's (first) colour, a gradient runs from it to white –
 * to ink (FILL_INK) when it starts light (relative luminance > 0.5: white, light tints, cyan), so Negative's white
 * doesn't run into white –, none clears it. A fill switched off over the look comes back as the look's when that is the
 * kind asked for, else starts from the look's colour. The kind it already has changes nothing (Segmented sends the
 * chosen option again – a re-click must not rebuild a gradient or make the look's fill the piece's own); false =
 * nothing changed
 */
export function setFillKind(piece: CollagePiece, kind: FillKind, presets?: PresetList): boolean {
	const fill = pieceStyle(piece, presets).fill;
	if (kind === fillKind(fill)) return false;
	const look = lookValue(piece, 'fill', presets);
	if (kind === 'none') setEffect(piece, 'fill', undefined, presets);
	else if (!fill && look && fillKind(look) === kind) {
		// off over the look (own null): dropping the own key brings the look's fill back
		delete piece.style?.fill;
		tidy(piece);
	} else {
		const source = fill ?? look;
		const from = typeof source === 'string' ? source : (source?.stops?.[0]?.color ?? FILL_INK);
		const to = luminance(from) > 0.5 ? FILL_INK : '#ffffff';
		const next: string | PieceGradient = kind === 'color' ? from : { stops: [{ at: 0, color: from }, { at: 100, color: to }] };
		setEffect(piece, 'fill', next, presets);
	}
	return true;
}

/**
 * An even blur (no points) or a ramp (sharp at `from`, blurred from `to`) – the amount kept. The kind the blur already
 * has changes nothing (a re-click keeps the ramp's points); false = nothing changed
 */
export function setBlurRamp(piece: CollagePiece, ramp: boolean, presets?: PresetList): boolean {
	const blur = pieceStyle(piece, presets).blur;
	if (blur && blurKind(blur) === (ramp ? 'ramp' : 'even')) return false;
	const amount = blur?.amount ?? EFFECT_DEFAULTS.blur.amount;
	setEffect(piece, 'blur', ramp ? { amount, ...copy(RAMP_POINTS) } : { amount }, presets);
	return true;
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
 * renumbers the stack (`setSliceLayers`). Answers where each piece went – `moved[old] = new`, the same index when the
 * stack moved – so the selected piece stays selected (`followPieces`); null = a move across, refused, nothing changed
 */
export function moveLayerRows(slice: any, from: number, to: number): number[] | null {
	const rows = layerRows(slice);
	if (!slice || !canMoveLayerRow(rows, from, to)) return null;
	const [row] = rows.splice(from, 1);
	rows.splice(to, 0, row);
	const pieces: unknown[] = [...(slice.pieces ?? [])];
	if (row.kind !== 'piece') {
		setSliceLayers(slice, rows.filter((r) => r.kind !== 'piece').reverse());
		return pieces.map((_, i) => i);
	}
	// back → front: the piece that is now at `next` was at `old`
	const order = rows.filter((r) => r.kind === 'piece').reverse().map((r) => r.index);
	slice.pieces = order.map((old) => pieces[old]);
	const moved: number[] = [];
	order.forEach((old, next) => (moved[old] = next));
	return moved;
}

/**
 * Where each piece of `before` is in `after` by its id (-1 = gone; a piece without an id keeps its index while there is
 * one) – an undo / redo may put the pieces back in another order (a reorder) or take one away (an add)
 */
export function piecesMoved(before: unknown, after: unknown): number[] {
	const ids = (list: unknown) => (Array.isArray(list) ? list.map((p) => (p && typeof p === 'object' ? (p as CollagePiece).id : undefined)) : []);
	const next = ids(after);
	return ids(before).map((id, i) => (typeof id === 'string' && id ? next.indexOf(id) : i < next.length ? i : -1));
}

/**
 * The canvas selection after the pieces moved (`moveLayerRows`, `piecesMoved`): a selected piece goes where its piece
 * went, a gone one is let go (null); another kind – or a piece that stayed – is the same selection
 */
export function followPieces<S extends { kind: string; index: number }>(selection: S | null, moved: readonly number[]): S | null {
	if (selection?.kind !== 'piece') return selection;
	const next = moved[selection.index];
	if (next === undefined || next === selection.index) return selection;
	return next < 0 ? null : { ...selection, index: next };
}

/** the piece at `path` of a document (`[...slicePath, 'pieces', i]`) – for edits in `doc.edit` */
export function pieceAt(file: unknown, path: (string | number)[]): CollagePiece | undefined {
	const piece = getAt(file, path);
	return piece && typeof piece === 'object' ? (piece as CollagePiece) : undefined;
}

/**
 * Where the piece `id` is now, for an edit that waited (a decode, a dialog, a toast's Undo): the selection may have
 * moved on and a Remove or an undo may have renumbered the list, so it is looked up by id in the list `path` pointed
 * into – never the piece that is at `path` now. Undefined when it is gone (the edit is dropped)
 */
export function piecePath(file: unknown, path: Path, id: string): Path | undefined {
	const list = getAt(file, path.slice(0, -1));
	if (!Array.isArray(list)) return undefined;
	const index = list.findIndex((p) => p && typeof p === 'object' && (p as CollagePiece).id === id);
	return index < 0 ? undefined : [...path.slice(0, -1), index];
}

/** `piecePath` of a character picture: undefined too when it is no longer one (a re-pick's edit is dropped) */
export function figurePath(file: unknown, path: Path, id: string): Path | undefined {
	const at = piecePath(file, path, id);
	return at && pieceAt(file, at)?.figure ? at : undefined;
}
