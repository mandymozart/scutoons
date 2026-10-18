// The score stage's edits (agents/research/COLLAGE.md §4.4): a score item's collage – the pieces on the part readers
// open (`readerPart`: the choir when there is one, else the full score), bound to that part's file. Pure functions on
// the stored item – the stage runs each inside one `doc.edit` (one undo step); where a piece sits comes from the page
// geometry of the score view (@scutoons/reader/score-anchors), never from the DOM.
import { readerPart, validateContainers, type CollagePiece, type CollageSheet, type Path, type Problem, type ScoreAt } from '@scutoons/content';
import { anchorScorePiece, findBar, placeScorePiece, type BarRef, type PageGeometry } from '@scutoons/reader/score-anchors';
import { pickedToPiece, type PickedPiece } from './collage-pick.ts';

/** a new piece's width in staff spaces (it scales with the music, like the notes) */
export const NEW_SCORE_PIECE_WIDTH = 12;
/** the smallest width a corner drag leaves (staff spaces / % of the page) */
const MIN_WIDTH = 0.5;

type Obj = Record<string, any>;
const isObj = (v: unknown): v is Obj => typeof v === 'object' && v !== null && !Array.isArray(v);
const round = (n: number, digits: number) => Math.round(n * 10 ** digits) / 10 ** digits || 0;

/** the part the stage edits: the one readers open (null = the full score) */
export const stagePart = (item: unknown): string | null => readerPart(isObj(item) && isObj(item.score) ? item.score : null);

/** where the part's collage lives in the item document */
export const collagePath = (part: string | null): Path => (part ? ['score', 'parts', part, 'collage'] : ['score', 'collage']);

/** the part's own object (`score` or `score.parts.<part>`) – it holds the file and the collage */
function hostOf(item: unknown, part: string | null): Obj | null {
	const score = isObj(item) ? item.score : null;
	if (!isObj(score)) return null;
	const host = part ? (isObj(score.parts) ? score.parts[part] : null) : score;
	return isObj(host) ? host : null;
}

/** the file the part's pieces are placed on (its MusicXML path), null when the item has none */
export function partFile(item: unknown, part: string | null): string | null {
	const file = hostOf(item, part)?.musicxml;
	return typeof file === 'string' && file ? file : null;
}

/** the part's collage as stored (null = none yet) */
export function sheetOf(item: unknown, part: string | null): CollageSheet | null {
	const sheet = hostOf(item, part)?.collage;
	return isObj(sheet) ? (sheet as CollageSheet) : null;
}

/** the part's pieces (back → front within each side); none = [] */
export function sheetPieces(item: unknown, part: string | null): CollagePiece[] {
	const pieces = sheetOf(item, part)?.pieces;
	return Array.isArray(pieces) ? pieces : [];
}

/**
 * The part's collage, made when it has none: `{ file: <the part's MusicXML>, pieces: [] }` – the first piece binds the
 * collage to the file it is placed on. Null when the item has no such part / file (nothing is written then)
 */
export function ensureSheet(item: unknown, part: string | null): CollageSheet | null {
	const host = hostOf(item, part);
	const file = partFile(item, part);
	if (!host || !file) return null;
	if (!isObj(host.collage)) host.collage = { file, pieces: [] };
	if (!Array.isArray(host.collage.pieces)) host.collage.pieces = [];
	return host.collage as CollageSheet;
}

/** a piece's place as the stage writes it: its bar (+ staff / page) and the offsets – anchorScorePiece's answer */
export interface ScorePlaceAt {
	at: ScoreAt;
	x: number;
	y: number;
}

/**
 * The new piece of a pick: the file from the CDN root (a score is a catalog row – no toon path form), a fresh id, its
 * centre (anchor 'c') at `place`, `width` staff spaces wide (% of the page on a page piece)
 */
export function newScorePiece(picked: PickedPiece, place: ScorePlaceAt, width: number, taken: ReadonlySet<string> = new Set()): CollagePiece {
	const piece = pickedToPiece(picked, '', { rect: { x: place.x, y: place.y, width }, anchor: 'c' }, taken);
	return { ...piece, at: { ...place.at } };
}

/** adds a piece to the part's collage (made when it has none); its index, null when the item has no file to place on */
export function addScorePiece(item: unknown, part: string | null, piece: CollagePiece): number | null {
	const sheet = ensureSheet(item, part);
	if (!sheet) return null;
	sheet.pieces.push(piece);
	return sheet.pieces.length - 1;
}

/** where a new piece lands: the middle of the page the stage shows (the bar nearest to it) */
export const centrePlace = (geo: PageGeometry): ScorePlaceAt | null => anchorScorePiece({ x: 50, y: 50 }, geo);

/** a drag moved the piece: its new bar (+ staff, page flag) and offsets – `at` is replaced whole (anchorScorePiece's) */
export function movePiece(piece: CollagePiece, place: ScorePlaceAt): void {
	piece.at = { ...place.at };
	piece.rect = { ...piece.rect, x: place.x, y: place.y };
}

/**
 * A piece's width from its corner handle: the pointer's distance from the piece's centre (both in % of the page) is the
 * half diagonal of the box (whatever its rotation) – in staff spaces on the music, % of the page on a page piece
 */
export function cornerWidth(piece: Pick<CollagePiece, 'ratio' | 'at'>, centre: { x: number; y: number }, pointer: { x: number; y: number }, geo: PageGeometry): number {
	const dx = ((pointer.x - centre.x) / 100) * geo.width;
	const dy = ((pointer.y - centre.y) / 100) * geo.height;
	const ratio = piece.ratio > 0 ? piece.ratio : 1;
	const width = (2 * Math.hypot(dx, dy)) / Math.sqrt(1 + 1 / (ratio * ratio));
	return Math.max(MIN_WIDTH, round(piece.at?.page ? (width / geo.width) * 100 : width / geo.sp, 2));
}

/** removes the piece `id`; the last one takes the collage away (no empty `collage` stays). False = not there */
export function removeScorePiece(item: unknown, part: string | null, id: string): boolean {
	const host = hostOf(item, part);
	const pieces = sheetPieces(item, part);
	const index = pieces.findIndex((p) => isObj(p) && p.id === id);
	if (!host || index < 0) return false;
	pieces.splice(index, 1);
	if (!pieces.length) delete host.collage;
	return true;
}

/**
 * The Bar / Page switch, keeping the piece where it is on `geo` (the page that holds its bar): to the page – the place
 * and width in % of the page (its staff goes); to the music – the bar and staff nearest its centre, the width in staff
 * spaces. Without the page at hand only the flag changes.
 */
export function setPiecePage(piece: CollagePiece, page: boolean, geo: PageGeometry | null): void {
	if (!!piece.at?.page === page) return;
	const place = geo ? placeScorePiece(piece, geo) : null;
	if (!geo || !place || !piece.at) {
		piece.at = { ...(piece.at ?? { bar: '' }), page: page || undefined };
		if (!page) delete piece.at.page;
		return;
	}
	if (page) {
		const { bar, nth } = piece.at;
		piece.at = { bar, ...(nth && nth > 1 ? { nth } : {}), page: true };
		piece.rect = { x: round(place.left, 2), y: round(place.top, 2), width: round(place.width, 2) };
		return;
	}
	const anchored = anchorScorePiece({ x: place.left, y: place.top }, geo);
	if (!anchored) return;
	piece.at = anchored.at;
	piece.rect = { x: anchored.x, y: anchored.y, width: round(((place.width / 100) * geo.width) / geo.sp, 2) };
}

// ── the collage list (COLLAGE.md §4.4): over the notes front first, a rule, behind them front first ───────────────

export interface CollageRow {
	/** the piece's index in `pieces` */
	index: number;
	behind: boolean;
}

/** the list's rows: the pieces over the notes (the front one first), then the pieces behind them (the front one first) */
export function collageRows(pieces: readonly unknown[]): CollageRow[] {
	const rows = pieces.map((p, index) => ({ index, behind: isObj(p) && p.behind === true }));
	return [...rows.filter((r) => !r.behind).reverse(), ...rows.filter((r) => r.behind).reverse()];
}

/** a row moves only within its side (over / behind: the side is the inspector's switch) */
export function canMoveCollageRow(rows: readonly CollageRow[], from: number, to: number): boolean {
	return from !== to && !!rows[from] && !!rows[to] && rows[from].behind === rows[to].behind;
}

/**
 * The list's drag (`collageRows` order): the side's pieces in their new order, back → front, in the places that side
 * had in `pieces` (the other side stays where it is). A new list, null = refused (across the sides)
 */
export function moveCollageRows<T>(pieces: readonly T[], from: number, to: number): T[] | null {
	const rows = collageRows(pieces);
	if (!canMoveCollageRow(rows, from, to)) return null;
	const side = rows.filter((r) => r.behind === rows[from].behind);
	const offset = rows.indexOf(side[0]);
	const order = side.map((r) => r.index);
	const [moved] = order.splice(from - offset, 1);
	order.splice(to - offset, 0, moved);
	// front first → back first, into the side's slots (ascending)
	const slots = side.map((r) => r.index).sort((a, b) => a - b);
	const next = [...pieces];
	order.reverse().forEach((old, i) => (next[slots[i]] = pieces[old]));
	return next;
}

// ── problems ──────────────────────────────────────────────────────────────────────────────────────────────────

/** the ids of the pieces whose bar the file lacks (`bars` = the layout's; none read yet = none missing) */
export function missingBars(pieces: readonly unknown[], bars: readonly BarRef[] | null): Set<string> {
	const out = new Set<string>();
	if (!bars?.length) return out;
	for (const p of pieces) if (isObj(p) && typeof p.id === 'string' && !findBar(bars, p.at)) out.add(p.id);
	return out;
}

/** the collage was placed on another file than the part has now (a replaced file: "check the bars") */
export function fileChanged(item: unknown, part: string | null): boolean {
	const sheet = sheetOf(item, part);
	const file = partFile(item, part);
	return !!sheet && !!file && typeof sheet.file === 'string' && sheet.file !== file;
}

/**
 * The stage's problems in the item document `entity`: the collage's own (validateContainers on this item – a broken
 * piece, the limits, the file it was placed on) and the pieces whose bar the file lacks (readers skip them)
 */
export function stageProblems(item: unknown, part: string | null, bars: readonly BarRef[] | null, entity: string, missing: (bar: string) => string): Problem[] {
	const out: Problem[] = [];
	if (!isObj(item)) return out;
	const where = collagePath(part);
	const inside = (path: (string | number)[]) => where.every((key, i) => path[i] === key);
	validateContainers({ items: [item] }, (level, path, message) => {
		const local = path.slice(2);
		if (inside(local)) out.push({ level, file: entity, locale: null, path: local, message });
	});
	const pieces = sheetPieces(item, part);
	const gone = missingBars(pieces, bars);
	pieces.forEach((p, i) => {
		if (gone.has(p.id)) out.push({ level: 'warning', file: entity, locale: null, path: [...where, 'pieces', i, 'at', 'bar'], message: missing(String(p.at?.bar ?? '')) });
	});
	return out;
}
