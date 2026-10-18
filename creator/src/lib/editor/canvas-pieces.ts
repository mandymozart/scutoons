// The slice canvas' collage pieces (agents/research/COLLAGE.md §3 "Editing"): a piece as a canvas box – its rect (x / y
// = the anchor point, the width; the height always from the file's ratio, never stretched), rotation and anchor – and
// where a new piece goes (the piece tool's click / drag, "+ piece"). Pure (no Svelte, no DOM): creator/test/canvas-pieces.test.ts.
import { anchorFactors, pieceHeight, pieceName, pieceStyle, type Anchor, type CollagePiece, type Path, type PiecePoint } from '@scutoons/content';
import type { Factors, Rect } from './canvas-geometry.ts';

/** a new piece's width in % of the slice (a click places it centred there) */
export const NEW_PIECE_WIDTH = 30;

/** a piece as the canvas draws and drags it (the canvas' own Item shape) */
export interface PieceItem {
	kind: 'piece';
	index: number;
	/** the stored rect (`[...slice, 'pieces', i, 'rect']`) – what a drag writes */
	path: Path;
	/** x / y / width as stored, the height as drawn (from the ratio – never written) */
	rect: Required<Rect>;
	label: string;
	rotation: number;
	scale: number;
	f: Factors;
	/** a progressive blur's two points (% of the piece's own box): the canvas' dots */
	ramp?: { from: PiecePoint; to: PiecePoint };
}

const num = (v: unknown): v is number => typeof v === 'number' && Number.isFinite(v);
const r2 = (v: number) => Math.round(v * 100) / 100;

/**
 * The slice's pieces as canvas boxes (array order = back → front; a piece the renderer can't lay out – no rect, no
 * ratio – has no box either). `sliceRatio` = the slice's height ÷ width.
 */
export function pieceItems(pieces: unknown, slicePath: Path, sliceRatio: number, name: (piece: CollagePiece) => string = pieceName): PieceItem[] {
	if (!Array.isArray(pieces)) return [];
	const out: PieceItem[] = [];
	pieces.forEach((p: CollagePiece | null, index) => {
		const rect = p?.rect;
		if (!p || typeof p !== 'object' || !num(p.ratio) || p.ratio <= 0 || !rect || !num(rect.x) || !num(rect.y) || !num(rect.width) || rect.width <= 0) return;
		const blur = pieceStyle(p).blur;
		out.push({
			kind: 'piece',
			index,
			path: [...slicePath, 'pieces', index, 'rect'],
			rect: { x: rect.x, y: rect.y, width: rect.width, height: r2(pieceHeight(rect.width, p.ratio, sliceRatio > 0 ? sliceRatio : 1)) },
			label: `▲ ${name(p)}`,
			rotation: num(p.rotation) ? p.rotation : 0,
			scale: 1,
			f: anchorFactors(p.anchor),
			...(blur?.from && blur?.to ? { ramp: { from: blur.from, to: blur.to } } : {})
		});
	});
	return out;
}

/**
 * Whether a piece's canvas box takes the pointer. Pieces are art under the stack, often as large as the slice: under
 * another tool their boxes would swallow every drag meant to draw (the areas' rule – owner 2026-10-03: "full-slice
 * events covered everything"). So only the piece tool or the piece's own selection grabs it; else the tool draws.
 */
export const pieceTakesPointer = (tool: string, selected: boolean) => tool === 'piece' || selected;

/** where a new piece goes, as `pickedToPiece` takes it */
export interface PiecePlace {
	rect: CollagePiece['rect'];
	anchor?: Anchor;
}

/**
 * The piece tool's spot → the new piece's place: a click (no width) = centred there at `NEW_PIECE_WIDTH`; a drawn rect =
 * its top-left corner and width (the height follows the file); none ("+ piece") = the middle of the slice
 */
export function piecePlace(spot: { x: number; y: number; width?: number } | null): PiecePlace {
	const clamp = (v: number) => r2(Math.min(100, Math.max(0, v)));
	if (!spot) return { rect: { x: 50, y: 50, width: NEW_PIECE_WIDTH }, anchor: 'c' };
	if (num(spot.width) && spot.width > 0) return { rect: { x: clamp(spot.x), y: clamp(spot.y), width: r2(spot.width) } };
	return { rect: { x: clamp(spot.x), y: clamp(spot.y), width: NEW_PIECE_WIDTH }, anchor: 'c' };
}
