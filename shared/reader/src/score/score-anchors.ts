/**
 * Where a score's collage pieces sit (agents/research/COLLAGE.md §4.2): a piece sticks to a bar of the file – never a
 * page, never a Verovio id – so it survives the page size, the zoom and every new line / page break. Music pieces:
 * x = % of the bar's width from its start (bar widths vary 2–3× between layouts – 80 % of a bar stays 80 %), y = staff
 * spaces below the staff's top line, width = staff spaces (it scales with the music). Page pieces (`at.page`): % of the
 * page that holds the bar (a background).
 *
 * Pure (node-tested against a real Verovio 6.3.0 page, test/fixtures/score-page.svg): the bars come from the MEI the
 * toolkit holds (`barIndex(tk.getMEI())`: ids → the bar's number and which one of that number), a page's geometry
 * from its SVG string (`pageGeometry`) – no DOM measuring, no px, no rem: the pieces' layer lies exactly on the
 * page's music (`pageFrame`, in the page's leaf), so % of the page is all a piece needs. Verovio's structure
 * (score-layout.ts asks for measure@n + staff@n): `svg.definition-scale viewBox="0 0 W H"` >
 * `g.page-margin transform="translate(mx, my)"` > `g.system` > `g.measure[data-n]` > `g.staff[data-n]`, whose first
 * `<path d="M x1 y L x2 y">` children are its lines (180 units apart at Verovio's default size). A Verovio upgrade
 * re-runs the fixture test.
 */
import type { CollagePiece, ScoreAt } from '@scutoons/content';

/** a bar of the file: its id in the toolkit's MEI / SVG, its number as the file writes it, the n-th bar of that number */
export interface BarRef {
	id: string;
	n: string;
	nth: number;
}

/** a staff of a bar on the page: its number (staff@n, from the top) and its top / bottom line */
export interface StaffBox {
	n: number;
	top: number;
	bottom: number;
}

export interface BarBox extends BarRef {
	/** where the bar's staff lines start / end */
	left: number;
	right: number;
	/** the staves drawn in this bar, top to bottom (a condensed system leaves hidden ones out) */
	staves: StaffBox[];
}

/** one rendered page, in its viewBox units (the page margin included) */
export interface PageGeometry {
	width: number;
	height: number;
	/** a staff space: the distance of two staff lines */
	sp: number;
	bars: BarBox[];
	/**
	 * where the viewBox lies in the SVG element, in % of the element's width: Verovio writes the element's size in whole
	 * pixels (its height rounded up) and fits the viewBox into it centred – up to half a pixel off the element's corner.
	 * The pieces' layer takes this box (`pageFrame`), so % of the page are % of the music exactly.
	 */
	frame: { x: number; y: number; width: number };
}

/** a piece's place on its page in % of the page: the anchor point and the width (CollagePieceView's left / top / width) */
export interface ScorePlace {
	left: number;
	top: number;
	width: number;
}

/** Verovio's staff space at its default size (unit 9 → 18 units, ×10 in the viewBox) – when a page draws no staff */
const DEFAULT_SP = 180;

const ATTRS = new Map<string, RegExp>();
const attr = (attrs: string, name: string): string | null => {
	let re = ATTRS.get(name);
	if (!re) ATTRS.set(name, (re = new RegExp(`(?:^|\\s)${name}="([^"]*)"`)));
	const m = re.exec(attrs);
	return m ? m[1] : null;
};

/** the bars of the file in its order (`tk.getMEI()`): every measure's id, number and which one of that number */
export function barIndex(mei: string): BarRef[] {
	const bars: BarRef[] = [];
	const seen = new Map<string, number>();
	for (const m of mei.matchAll(/<measure\b([^>]*)>/g)) {
		const id = attr(m[1], 'xml:id');
		if (!id) continue;
		const n = attr(m[1], 'n') ?? '';
		const nth = (seen.get(n) ?? 0) + 1;
		seen.set(n, nth);
		bars.push({ id, n, nth });
	}
	return bars;
}

/** the file's bar a piece names (`at.bar`, the `nth` of that number – default 1); none = the file has no such bar */
export function findBar<T extends BarRef>(bars: readonly T[], at: Pick<ScoreAt, 'bar' | 'nth'> | null | undefined): T | undefined {
	if (!at || typeof at.bar !== 'string') return undefined;
	const n = at.bar.trim();
	const nth = at.nth ?? 1;
	return bars.find((bar) => bar.n === n && bar.nth === nth);
}

const LINE = /^\s*M\s*(-?[\d.]+)[\s,]+(-?[\d.]+)\s*L\s*(-?[\d.]+)[\s,]+(-?[\d.]+)\s*$/;
const TAG = /<(\/?)([A-Za-z][\w:.-]*)((?:[^>"']|"[^"]*"|'[^']*')*?)(\/?)>/g;

interface Open {
	name: string;
	/** a measure / a staff being read (their lines, their staves) */
	bar?: BarBox;
	staff?: { n: number; lines: number[]; xs: number[]; done: boolean };
}

/**
 * A rendered page's bars and staves (`tk.renderToSVG(page)`), in the page's viewBox units with the margin's translate
 * added; `bars` = `barIndex` of the same load (a bar it doesn't know counts by its data-n, as the first of its number).
 * Null when the SVG is not a Verovio page.
 */
export function pageGeometry(svg: string, bars: readonly BarRef[]): PageGeometry | null {
	const known = new Map(bars.map((bar) => [bar.id, bar]));
	let box: [number, number] | null = null;
	/** the SVG element's own size (px) */
	let element: [number, number] | null = null;
	let shift: [number, number] = [0, 0];
	const out: BarBox[] = [];
	const spaces: number[] = [];
	const stack: Open[] = [];
	for (const m of svg.matchAll(TAG)) {
		const [, closing, name, attrs, selfClosing] = m;
		if (closing) {
			// close up to the matching element (Verovio writes well-formed XML; a stray tag can't derail the rest)
			while (stack.length) {
				const top = stack.pop()!;
				if (top.staff && top.staff.lines.length) finishStaff(top.staff, stack, spaces);
				if (top.bar && top.bar.staves.length) out.push(top.bar);
				if (top.name === name) break;
			}
			continue;
		}
		const parent = stack[stack.length - 1];
		// a staff's lines are its first <path> children – anything else ends them (ledger lines sit in a <g>)
		if (parent?.staff && !parent.staff.done) {
			const line = name === 'path' ? LINE.exec(attr(attrs, 'd') ?? '') : null;
			if (line && line[2] === line[4]) {
				parent.staff.lines.push(Number(line[2]) + shift[1]);
				parent.staff.xs.push(Number(line[1]) + shift[0], Number(line[3]) + shift[0]);
			} else parent.staff.done = true;
		}
		const open: Open = { name };
		const cls = name === 'g' || name === 'svg' ? (attr(attrs, 'class') ?? '').split(/\s+/) : [];
		if (name === 'svg' && !element && !cls.includes('definition-scale')) {
			const [w, h] = [parseFloat(attr(attrs, 'width') ?? ''), parseFloat(attr(attrs, 'height') ?? '')];
			if (w > 0 && h > 0) element = [w, h];
		} else if (name === 'svg' && cls.includes('definition-scale')) {
			const v = (attr(attrs, 'viewBox') ?? '').trim().split(/[\s,]+/).map(Number);
			if (v.length === 4 && v[2] > 0 && v[3] > 0) box = [v[2], v[3]];
		} else if (name === 'g' && cls.includes('page-margin')) {
			const t = /translate\(\s*(-?[\d.]+)(?:[\s,]+(-?[\d.]+))?\s*\)/.exec(attr(attrs, 'transform') ?? '');
			if (t) shift = [Number(t[1]), Number(t[2] ?? 0)];
		} else if (name === 'g' && cls.includes('measure')) {
			const id = attr(attrs, 'id') ?? '';
			const ref = known.get(id) ?? { id, n: attr(attrs, 'data-n') ?? '', nth: 1 };
			open.bar = { id, n: ref.n, nth: ref.nth, left: Infinity, right: -Infinity, staves: [] };
		} else if (name === 'g' && cls.includes('staff') && stack.some((o) => o.bar)) {
			open.staff = { n: Number(attr(attrs, 'data-n')) || 1, lines: [], xs: [], done: false };
		}
		if (selfClosing) {
			if (open.bar?.staves.length) out.push(open.bar);
			continue;
		}
		stack.push(open);
	}
	if (!box) return null;
	const sp = median(spaces) ?? DEFAULT_SP;
	return { width: box[0], height: box[1], sp, bars: out.map((bar) => ({ ...bar, staves: bar.staves.sort((a, b) => a.top - b.top) })), frame: fitted(box, element) };
}

/** the viewBox fitted into the element (SVG's default `xMidYMid meet`), in % of the element's width */
function fitted([w, h]: [number, number], element: [number, number] | null): PageGeometry['frame'] {
	if (!element) return { x: 0, y: 0, width: 100 };
	const [ew, eh] = element;
	const scale = Math.min(ew / w, eh / h);
	return { x: ((ew - w * scale) / 2 / ew) * 100, y: ((eh - h * scale) / 2 / ew) * 100, width: ((w * scale) / ew) * 100 };
}

/**
 * The CSS of a layer that covers exactly the page's music (the pieces, the stage's boxes) inside the page's leaf, whose
 * top-left corner is the SVG element's: the frame's width, the viewBox's ratio, margins in % of the width (no
 * transform – the layer must stay no group)
 */
export function pageFrame(geo: PageGeometry): Record<string, string> {
	const n = (v: number) => String(Math.round(v * 1e4) / 1e4);
	return { width: `${n(geo.frame.width)}%`, 'aspect-ratio': `${geo.width} / ${geo.height}`, 'margin-left': `${n(geo.frame.x)}%`, 'margin-top': `${n(geo.frame.y)}%` };
}

function finishStaff(staff: NonNullable<Open['staff']>, stack: Open[], spaces: number[]) {
	const bar = [...stack].reverse().find((o) => o.bar)?.bar;
	if (!bar) return;
	const lines = [...staff.lines].sort((a, b) => a - b);
	const top = lines[0];
	const bottom = lines[lines.length - 1];
	if (lines.length > 1) spaces.push((bottom - top) / (lines.length - 1));
	bar.staves.push({ n: staff.n, top, bottom });
	bar.left = Math.min(bar.left, ...staff.xs);
	bar.right = Math.max(bar.right, ...staff.xs);
}

function median(values: number[]): number | null {
	if (!values.length) return null;
	const sorted = [...values].sort((a, b) => a - b);
	const mid = sorted.length >> 1;
	return sorted.length % 2 ? sorted[mid] : (sorted[mid - 1] + sorted[mid]) / 2;
}

/** the staff a piece names in its bar: `at.staff` (default 1), else – hidden in a condensed system – the bar's top one */
function staffOf(bar: BarBox, staff: number | undefined): StaffBox {
	return bar.staves.find((s) => s.n === (staff ?? 1)) ?? bar.staves[0];
}

/** where a score piece sits on this page (% of the page), or null – its bar is on another page or not in the file */
export function placeScorePiece(piece: Pick<CollagePiece, 'rect' | 'at'>, geo: PageGeometry): ScorePlace | null {
	const bar = findBar(geo.bars, piece.at);
	if (!bar || !piece.rect) return null;
	const { x, y, width } = piece.rect;
	if (piece.at!.page) return { left: x, top: y, width };
	const staff = staffOf(bar, piece.at!.staff);
	const left = bar.left + (x / 100) * (bar.right - bar.left);
	const top = staff.top + y * geo.sp;
	return { left: (left / geo.width) * 100, top: (top / geo.height) * 100, width: ((width * geo.sp) / geo.width) * 100 };
}

const round = (n: number, digits: number) => Math.round(n * 10 ** digits) / 10 ** digits || 0;
/** how far a point lies outside [a, b] (0 inside) */
const outside = (v: number, a: number, b: number) => Math.max(0, a - v, v - b);

/**
 * The bar, staff and offsets for a point on this page (% of the page – the stage's pointer): the bar whose system band
 * holds the point (else the nearest system), the nearest bar in it, the nearest staff of that bar. `page` = a page
 * piece: the bar only names the page, x / y stay % of it. Null when the page shows no bar.
 */
export function anchorScorePiece(point: { x: number; y: number }, geo: PageGeometry, page = false): { at: ScoreAt; x: number; y: number } | null {
	const px = (point.x / 100) * geo.width;
	const py = (point.y / 100) * geo.height;
	let best: BarBox | null = null;
	let bestDy = Infinity;
	let bestDx = Infinity;
	for (const bar of geo.bars) {
		const dy = outside(py, bar.staves[0].top, bar.staves[bar.staves.length - 1].bottom);
		const dx = outside(px, bar.left, bar.right);
		if (dy < bestDy - 1e-9 || (Math.abs(dy - bestDy) <= 1e-9 && dx < bestDx)) [best, bestDy, bestDx] = [bar, dy, dx];
	}
	if (!best) return null;
	const at: ScoreAt = { bar: best.n, ...(best.nth > 1 ? { nth: best.nth } : {}) };
	if (page) return { at: { ...at, page: true }, x: round(point.x, 2), y: round(point.y, 2) };
	let staff = best.staves[0];
	for (const s of best.staves) if (outside(py, s.top, s.bottom) < outside(py, staff.top, staff.bottom)) staff = s;
	if (staff.n !== 1) at.staff = staff.n;
	const span = best.right - best.left;
	return { at, x: round(span > 0 ? ((px - best.left) / span) * 100 : 0, 2), y: round((py - staff.top) / geo.sp, 3) };
}
