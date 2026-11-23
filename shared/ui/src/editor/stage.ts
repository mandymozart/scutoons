/**
 * Taking the stage – the pure part. A view (a panel tab) can lie on the stage, which is the whole window under the
 * floating panels, in one of four simple layouts: one view · a half and two quarters · two quarters and a half · four
 * quarters. A tab or a panel dragged to a screen edge or corner lands there (`zoneAt`, `placeOn`); the panels themselves
 * never dock. An empty slot shows what lies under the stage (the open item's own view).
 */
import type { Rect, Size } from './floats.ts';

export type StageLayout = 'one' | 'half-left' | 'half-right' | 'quarters';
export type Cell = 'whole' | 'left' | 'right' | 'top-left' | 'top-right' | 'bottom-left' | 'bottom-right';

export const LAYOUTS: readonly StageLayout[] = ['one', 'half-left', 'half-right', 'quarters'];
export const CELLS: Readonly<Record<StageLayout, readonly Cell[]>> = {
	one: ['whole'],
	'half-left': ['left', 'top-right', 'bottom-right'],
	'half-right': ['top-left', 'bottom-left', 'right'],
	quarters: ['top-left', 'top-right', 'bottom-left', 'bottom-right']
};

/** The layout and the view in each of its cells (null: free), in the order of `CELLS[layout]` */
export interface StageState {
	layout: StageLayout;
	slots: (string | null)[];
}

export const EMPTY_STAGE: StageState = { layout: 'one', slots: [null] };

/** The four states of a view */
export type ViewState = 'hidden' | 'panel' | 'stage' | 'full';
export const VIEW_STATES: readonly ViewState[] = ['hidden', 'panel', 'stage', 'full'];

const UNIT: Readonly<Record<Cell, Rect>> = {
	whole: { x: 0, y: 0, w: 1, h: 1 },
	left: { x: 0, y: 0, w: 0.5, h: 1 },
	right: { x: 0.5, y: 0, w: 0.5, h: 1 },
	'top-left': { x: 0, y: 0, w: 0.5, h: 0.5 },
	'top-right': { x: 0.5, y: 0, w: 0.5, h: 0.5 },
	'bottom-left': { x: 0, y: 0.5, w: 0.5, h: 0.5 },
	'bottom-right': { x: 0.5, y: 0.5, w: 0.5, h: 0.5 }
};

const contains = (a: Rect, b: Rect) => a.x <= b.x && a.y <= b.y && a.x + a.w >= b.x + b.w && a.y + a.h >= b.y + b.h;

/** A cell's rectangle in `area` */
export const cellRect = (cell: Cell, area: Rect): Rect => {
	const u = UNIT[cell];
	return { x: area.x + u.x * area.w, y: area.y + u.y * area.h, w: u.w * area.w, h: u.h * area.h };
};

/** The views on the stage, in cell order */
export const viewsOn = (s: StageState): string[] => s.slots.filter((v): v is string => !!v);

export const isOn = (s: StageState, view: string): boolean => s.slots.includes(view);

export const cellOf = (s: StageState, view: string): Cell | undefined => {
	const i = s.slots.indexOf(view);
	return i < 0 ? undefined : CELLS[s.layout][i];
};

/** Every cell of the layout with its view and rectangle in `area` */
export const slotRects = (s: StageState, area: Rect): Array<{ cell: Cell; view: string | null; rect: Rect }> =>
	CELLS[s.layout].map((cell, i) => ({ cell, view: s.slots[i] ?? null, rect: cellRect(cell, area) }));

/** A view off the stage: its cell is free; with no view left the stage is empty */
export const offStage = (s: StageState, view: string): StageState => {
	if (!isOn(s, view)) return s;
	const slots = s.slots.map((v) => (v === view ? null : v));
	return slots.some(Boolean) ? { layout: s.layout, slots } : EMPTY_STAGE;
};

const isRightCell = (c: Cell) => c === 'right' || c === 'top-right' || c === 'bottom-right';

/** The layout a view dropped on `target` makes of the layout there is */
const layoutFor = (s: StageState, target: Cell): StageLayout => {
	if (target === 'whole') return 'one';
	if (target === 'left') return 'half-left';
	if (target === 'right') return 'half-right';
	if (s.layout === 'quarters') return 'quarters';
	if (s.layout === 'half-left') return isRightCell(target) ? 'half-left' : 'quarters';
	if (s.layout === 'half-right') return isRightCell(target) ? 'quarters' : 'half-right';
	// one view or none: the half on the other side keeps it
	return isRightCell(target) ? 'half-left' : 'half-right';
};

/**
 * A view dropped on a cell of the stage: the layout follows (a half, a quarter, the whole), the view takes that cell and
 * the others keep theirs where they still exist – else the cell their old one covers, else a free one; a view without a
 * cell left leaves the stage (it is back in its panel)
 */
export const placeOn = (s: StageState, view: string, target: Cell): StageState => {
	const layout = layoutFor(s, target);
	const cells = CELLS[layout];
	const slots: (string | null)[] = cells.map((c) => (c === target ? view : null));
	const rest = CELLS[s.layout].flatMap((c, i) => (s.slots[i] && s.slots[i] !== view ? [{ view: s.slots[i] as string, from: c }] : []));
	const left = new Set(rest.map((r) => r.view));
	// the larger cells first: a view that had the whole stage keeps a half rather than a quarter
	const area = (c: Cell) => UNIT[c].w * UNIT[c].h;
	const order = cells.map((cell, i) => ({ cell, i })).sort((a, b) => area(b.cell) - area(a.cell));
	const fill = (fits: (from: Cell, cell: Cell) => boolean) =>
		order.forEach(({ cell, i }) => {
			if (slots[i]) return;
			const r = rest.find((o) => left.has(o.view) && fits(o.from, cell));
			if (!r) return;
			slots[i] = r.view;
			left.delete(r.view);
		});
	fill((from, cell) => from === cell);
	fill((from, cell) => contains(UNIT[from], UNIT[cell]));
	if (layout !== 'one') fill(() => true);
	return { layout, slots };
};

/**
 * A view put on the stage without a place (Views): the stage empty, it takes the whole; else the first free cell; else
 * the layout grows (one → a half and two quarters → four quarters); four full, it takes the last quarter
 */
export const addTo = (s: StageState, view: string): StageState => {
	if (isOn(s, view)) return s;
	if (!viewsOn(s).length) return { layout: 'one', slots: [view] };
	const free = s.slots.indexOf(null);
	if (free >= 0) return { layout: s.layout, slots: s.slots.map((v, i) => (i === free ? view : v)) };
	const next: Record<StageLayout, Cell> = { one: 'top-right', 'half-left': 'top-left', 'half-right': 'top-right', quarters: 'bottom-right' };
	return placeOn(s, view, next[s.layout]);
};

/** A stored stage, every part checked: a known layout, its number of slots, each view once */
export const readStageState = (raw: unknown): StageState => {
	if (typeof raw !== 'object' || raw === null) return EMPTY_STAGE;
	const r = raw as Record<string, unknown>;
	const layout = LAYOUTS.find((l) => l === r.layout);
	if (!layout || !Array.isArray(r.slots)) return EMPTY_STAGE;
	const seen = new Set<string>();
	const slots = CELLS[layout].map((_, i) => {
		const v = (r.slots as unknown[])[i];
		if (typeof v !== 'string' || !v || seen.has(v)) return null;
		seen.add(v);
		return v;
	});
	return slots.some(Boolean) ? { layout, slots } : EMPTY_STAGE;
};

/** A stage with only the views `keep` allows (a view without content now is left out) */
export const keepOn = (s: StageState, keep: (view: string) => boolean): StageState => viewsOn(s).filter((v) => !keep(v)).reduce(offStage, s);

/** How far the drop zones reach in from the screen's edges (px): the bands along the edges, the corners along each edge */
export interface ZoneSize {
	band: number;
	corner: number;
}

/**
 * The drop zone under the pointer, over the whole window – the bar and the status bar too: a corner (along either edge
 * near it), the top edge (the whole stage), the left or right edge (that half); none elsewhere, nor along the bottom
 */
export const zoneAt = (x: number, y: number, win: Size, { band, corner }: ZoneSize): Cell | null => {
	const L = x <= band;
	const R = x >= win.w - band;
	const T = y <= band;
	const B = y >= win.h - band;
	if ((T && x <= corner) || (L && y <= corner)) return 'top-left';
	if ((T && x >= win.w - corner) || (R && y <= corner)) return 'top-right';
	if ((B && x <= corner) || (L && y >= win.h - corner)) return 'bottom-left';
	if ((B && x >= win.w - corner) || (R && y >= win.h - corner)) return 'bottom-right';
	if (T) return 'whole';
	if (L) return 'left';
	if (R) return 'right';
	return null;
};

/** What a zone covers on the screen (a corner: its two arms along the edges) – drawn while dragging */
export const zoneRects = (cell: Cell, win: Size, { band, corner }: ZoneSize): Rect[] => {
	const { w, h } = win;
	const mid = { w: Math.max(0, w - 2 * corner), h: Math.max(0, h - 2 * corner) };
	switch (cell) {
		case 'whole':
			return [{ x: corner, y: 0, w: mid.w, h: band }];
		case 'left':
			return [{ x: 0, y: corner, w: band, h: mid.h }];
		case 'right':
			return [{ x: w - band, y: corner, w: band, h: mid.h }];
		case 'top-left':
			return [{ x: 0, y: 0, w: corner, h: band }, { x: 0, y: band, w: band, h: corner - band }];
		case 'top-right':
			return [{ x: w - corner, y: 0, w: corner, h: band }, { x: w - band, y: band, w: band, h: corner - band }];
		case 'bottom-left':
			return [{ x: 0, y: h - band, w: corner, h: band }, { x: 0, y: h - corner, w: band, h: corner - band }];
		case 'bottom-right':
			return [{ x: w - corner, y: h - band, w: corner, h: band }, { x: w - band, y: h - corner, w: band, h: corner - band }];
	}
};

export const ZONE_CELLS: readonly Cell[] = ['whole', 'left', 'right', 'top-left', 'top-right', 'bottom-left', 'bottom-right'];

/** Past the top edge (px from it): held there ≈ 0.5 s, the whole-stage drop turns into full screen */
export const PAST_TOP = 2;
export const HOLD_MS = 500;
export const isPastTop = (y: number): boolean => y <= PAST_TOP;
