// The variants board's layout (#125; the port of #62's board-layout.js – agents/research/VARIANTS-CANVAS.md §3.2 +
// §3.5). Pure: no Svelte, no DOM (test/storyboard-board-layout.test.ts). One BAND per picked shot; inside a band
// everything is laid out by the data – nobody drags or wires (owner 2026-10-20: rows of results, as recommended):
//   - column 0 = the SOURCES, a row each: the render (the shot as the creator draws it), one card per take of the shot,
//     the dropped files (a record `generator: 'upload'` without parents – a drop is a source);
//   - a parent's results form ONE row, oldest left (by id – the server's order); a generation's results open a new row
//     under it, one column right of it; a result hangs under its MAIN input = its first parent on the board (the job
//     sends its generated references as `parents`, the main input first – storyboard-generate.ts jobBody), else its
//     take's card, else the render (a job made from the shot itself: its prompt, the characters);
//   - child rows follow right under their parent's row, the rightmost parent's first – no line crosses;
//   - a RUNNING job (queued / running) is a placeholder where its result will land: at its main input's row's end.
// A loop in the parents (a broken record) never reaches a source: it lands in the row "earlier" – nothing is lost.
// THE FOLD (§3.5, owner 2026-10-20: "wenn rejected the need to fold somehow so we can make space"): a rejected card is a
// STRIP in its place (the picture's height, SW wide – drawn like the Rail); with nothing open under it its branch folds
// into that strip ("g3 +2"), folded neighbours share one strip; the row closes up, the line to it stays. A row of
// nothing but strips LIES DOWN as one line. `showRejected` folds nothing, `open` = the keys a look unfolded.
// Lines (2px ink) as segments [x1, y1, x2, y2]: a comb above each row with a tick into every result, a drop from the
// parent into it – from a source out of its picture's right side. Sizes in rem: one picture height for all bands
// (PIC_H × zoom), a card as wide as its shot's aspect but at least a caption wide; below CAPTIONS_MIN the captions hide.
import type { GenerationMark } from '@scutoons/content/storyboard';
import type { Generation } from './storyboard-generations.ts';
import { toEuro } from './storyboard-money.ts';
import type { EuroRate } from './storyboard-generations.ts';

/** one picture height in all bands (rem at 100 %) */
export const PIC_H = 9;
/** a column is at least a caption wide */
export const CAPTION_W = 5;
/** the caption under a picture (id + model, two lines) */
export const META = 2.5;
/** above a row's pictures: its comb (at COMB) and the ticks into them; a source's label */
export const TOP = 1.25;
export const COMB = 0.5;
export const GAP_X = 1.25;
/** between rows – the current card's actions (a control high) hang into it, above the row below */
export const GAP_Y = 1.5;
export const PAD = 1;
export const PAD_T = 0.5;
/** under the last row: room for the current card's actions */
export const PAD_B = 3.5;
/** a strip: SW wide standing (the Rail's width), SW high lying down – at least LIE_W wide then */
export const SW = 1.5;
export const LIE_W = 5;
/** below this zoom the captions hide (§3.2) – the columns are then the pictures' width */
export const CAPTIONS_MIN = 0.6;
/** two marks set by one X are this close (ms): the server stamps each PUT itself (`marked_at`), one X sends them at once */
export const STAMP_MS = 10_000;

/** what the board reads of a generation */
export type BoardRecord = Pick<Generation, 'id' | 'shot' | 'kind' | 'take' | 'parents' | 'generator' | 'status' | 'approved' | 'mark'> &
	Partial<Pick<Generation, 'file' | 'model' | 'cost' | 'currency' | 'marked_at' | 'progress' | 'estimate'>>;
export type CardStatus = 'chosen' | 'review' | 'rejected' | null;
export type RowLabel = 'source' | 'earlier' | null;

export interface BoardNode {
	key: string;
	kind: 'source' | 'result' | 'running' | 'strip';
	source: 'render' | 'take' | 'upload' | null;
	take: string | null;
	/** a result's / a dropped file's record (null: the render, a take, a running job) */
	record: BoardRecord | null;
	/** a running job's queued / running row */
	job: BoardRecord | null;
	parent: string | null;
	kids: string[];
	x: number;
	y: number;
	w: number;
	pw: number;
	h: number;
	row: number;
	/** a strip: every card folded into it (the first first); whole = its branch folded in; lie = a row lying down */
	keys?: string[];
	whole?: boolean;
	lie?: boolean;
}
export interface BandRow {
	y: number;
	x: number;
	keys: string[];
	label: RowLabel;
	lie: boolean;
}
export interface CardSize {
	h: number;
	pw: number;
	w: number;
	meta: number;
}
export interface BandLayout {
	nodes: BoardNode[];
	rows: BandRow[];
	lines: [number, number, number, number][];
	width: number;
	height: number;
	card: CardSize;
	/** every card, folded or not */
	tree: Map<string, BoardNode>;
}

/** a shot's aspect "9:16" → width / height (9 / 16 when unreadable) */
export function ratioOf(aspect: string | null | undefined): number {
	const m = /^(\d+(?:\.\d+)?):(\d+(?:\.\d+)?)$/.exec(String(aspect ?? '').trim());
	return m && +m[1] > 0 && +m[2] > 0 ? +m[1] / +m[2] : 9 / 16;
}
/** a band's card at `zoom`: the picture h × pw, the column w, the caption's height (rem; 0 = captions hidden) */
export function cardSize(aspect: string | null | undefined, zoom = 1): CardSize {
	const h = PIC_H * zoom;
	const pw = h * ratioOf(aspect);
	const captions = zoom >= CAPTIONS_MIN;
	return { h, pw, w: captions ? Math.max(pw, CAPTION_W * zoom) : pw, meta: captions ? META : 0 };
}

/** a generation's card key */
export const keyOf = (id: number) => String(id);
export const renderKey = (shot: string) => `render:${shot}`;
export const takeKey = (shot: string, take: string) => `take:${shot}:${take}`;
/** a key that is a generation's (not the render, a take) → its id, else null */
export const idOfKey = (key: string | null | undefined): number | null => (key && /^\d+$/.test(key) ? Number(key) : null);

/** a generation's short name: "g7" from its file `<shot>--g7.png`, else "#<id>" */
export function shortName(r: Pick<BoardRecord, 'id'> & { file?: string | null }): string {
	const g = /--(g\d+)\.[a-z0-9]+$/i.exec(r.file ?? '')?.[1];
	return g ?? `#${r.id}`;
}

/** a card's status (§3.5): chosen (approved) · review · rejected · null (none) */
export function statusOf(r: Pick<BoardRecord, 'approved' | 'mark'> | null | undefined): CardStatus {
	if (!r) return null;
	if (r.approved) return 'chosen';
	return r.mark === 'review' || r.mark === 'rejected' ? r.mark : null;
}
const isOpenJob = (r: BoardRecord) => r.status === 'queued' || r.status === 'running';
/** a dropped file: a source card (§3.2) */
export const isDrop = (r: Pick<BoardRecord, 'generator' | 'parents'>) => r.generator === 'upload' && !r.parents?.length;

function emptyNode(key: string, f: Partial<BoardNode>): BoardNode {
	return { key, kind: 'result', source: null, take: null, record: null, job: null, parent: null, kids: [], x: 0, y: 0, w: 0, pw: 0, h: 0, row: 0, ...f };
}

/** a card and what hangs under it (keys, the card first, depth first) in a tree */
function branch(tree: Map<string, BoardNode>, key: string): string[] {
	const out: string[] = [];
	const walk = (k: string) => {
		const n = tree.get(k);
		if (!n || out.includes(k)) return;
		out.push(k);
		n.kids.forEach(walk);
	};
	walk(key);
	return out;
}

/** where a record hangs: its first parent that is a card on the board, else its take's card, else the render */
function parentOf(r: BoardRecord, tree: Map<string, BoardNode>, shot: string): string {
	const p = (r.parents ?? []).map(keyOf).find((k) => k !== keyOf(r.id) && tree.get(k)?.record);
	if (p) return p;
	if (r.take && tree.has(takeKey(shot, r.take))) return takeKey(shot, r.take);
	return renderKey(shot);
}

export interface BandInput {
	shot: string;
	/** the shot's takes (ids, in order) */
	takes?: readonly string[];
	/** the shot's generations: the succeeded ones are cards, the queued / running ones placeholders, the rest nothing */
	records?: readonly BoardRecord[];
	aspect?: string | null;
	zoom?: number;
	showRejected?: boolean;
	/** the keys a look unfolded */
	open?: readonly string[];
}

/** A band's layout – rem inside the band's body (see the head of this file). */
export function bandLayout({ shot, takes = [], records = [], aspect, zoom = 1, showRejected = false, open = [] }: BandInput): BandLayout {
	const card = cardSize(aspect, zoom);
	const tree = new Map<string, BoardNode>();
	const add = (key: string, f: Partial<BoardNode>) => tree.set(key, emptyNode(key, f));
	add(renderKey(shot), { kind: 'source', source: 'render' });
	for (const t of takes) if (t && !tree.has(takeKey(shot, t))) add(takeKey(shot, t), { kind: 'source', source: 'take', take: t });
	const sources = [...tree.keys()];
	// oldest first (the server's ids)
	const mine = [...records].filter((r) => r && r.shot === shot).sort((a, b) => a.id - b.id);
	for (const r of mine) {
		if (r.status !== 'succeeded' || tree.has(keyOf(r.id))) continue;
		const drop = isDrop(r);
		add(keyOf(r.id), { kind: drop ? 'source' : 'result', source: drop ? 'upload' : null, take: r.take ?? null, record: r });
		if (drop) sources.push(keyOf(r.id));
	}
	// a running job: after the results (a new run lands at its row's end)
	for (const r of mine) if (isOpenJob(r) && !tree.has(keyOf(r.id))) add(keyOf(r.id), { kind: 'running', job: r });
	const roots: string[] = [];
	for (const n of tree.values()) {
		if (n.kind === 'source') continue;
		const parent = parentOf((n.record ?? n.job)!, tree, shot);
		n.parent = parent;
		tree.get(parent)!.kids.push(n.key);
	}
	// a loop of parents (a broken record) never reaches a source: it goes to the earlier row
	const reached = new Set<string>();
	const walk = (k: string) => {
		if (reached.has(k)) return;
		reached.add(k);
		tree.get(k)!.kids.forEach(walk);
	};
	sources.forEach(walk);
	for (const n of tree.values()) {
		if (reached.has(n.key)) continue;
		if (n.parent) tree.get(n.parent)!.kids = tree.get(n.parent)!.kids.filter((k) => k !== n.key);
		n.parent = null;
		roots.push(n.key);
		walk(n.key);
	}

	// the fold: a rejected result folds unless shown; its branch folds in WHOLE when nothing under it is open
	const opened = new Set(open);
	const folded = (n: BoardNode) => !showRejected && n.kind === 'result' && statusOf(n.record) === 'rejected' && !opened.has(n.key);
	const whole = new Map<string, boolean>();
	const isWhole = (n: BoardNode): boolean => {
		if (!whole.has(n.key)) whole.set(n.key, folded(n) && n.kids.every((k) => isWhole(tree.get(k)!)));
		return whole.get(n.key)!;
	};

	const rows: BandRow[] = [];
	const lines: [number, number, number, number][] = [];
	const nodes: BoardNode[] = [];
	const placed = new Set<string>();
	let y = PAD_T;
	const centre = (d: BoardNode) => d.x + d.pw / 2;
	const bottom = (d: BoardNode) => d.y + TOP + d.h + (d.kind === 'strip' ? 0 : card.meta);
	type Piece = { n: BoardNode; strip: boolean; whole: boolean; keys: string[] };
	/** a row: its lead (a source) at column 0, then `items` from x0; then the child rows of its items, rightmost first */
	function row(lead: BoardNode | null, items: string[], x0: number, parent: BoardNode | null, label: RowLabel) {
		const at: BandRow = { y, x: lead ? PAD : x0, keys: [], label, lie: false };
		rows.push(at);
		const pieces: Piece[] = [];
		for (const k of items) {
			if (placed.has(k)) continue;
			const n = tree.get(k)!;
			if (isWhole(n)) {
				const keys = branch(tree, k);
				keys.forEach((x) => placed.add(x));
				const last = pieces.at(-1);
				if (last?.whole) last.keys.push(...keys);
				else pieces.push({ n, strip: true, whole: true, keys });
			} else {
				placed.add(k);
				pieces.push({ n, strip: folded(n), whole: false, keys: [k] });
			}
		}
		at.lie = !lead && pieces.length > 0 && pieces.every((p) => p.strip);
		let x = at.x;
		const put = (d: BoardNode) => {
			Object.assign(d, { x, y, row: rows.length - 1 });
			at.keys.push(d.key);
			nodes.push(d);
			x += d.w + GAP_X;
			return d;
		};
		let head: BoardNode | null = null;
		if (lead) {
			placed.add(lead.key);
			head = put({ ...lead, w: card.w, pw: card.pw, h: card.h });
		}
		const these = pieces.map((p) =>
			put(
				!p.strip
					? { ...p.n, w: card.w, pw: card.pw, h: card.h }
					: at.lie
						? { ...p.n, kind: 'strip', keys: p.keys, whole: p.whole, lie: true, w: Math.max(card.w, LIE_W), pw: Math.max(card.w, LIE_W), h: SW }
						: { ...p.n, kind: 'strip', keys: p.keys, whole: p.whole, lie: false, w: SW, pw: SW, h: card.h }
			)
		);
		y += TOP + (at.lie ? SW : card.h + card.meta) + GAP_Y;
		if (these.length && (head || parent)) {
			const comb = at.y + COMB;
			const last = centre(these.at(-1)!);
			if (head) {
				const mid = at.y + TOP + card.h / 2;
				const xc = head.x + head.w + GAP_X / 2;
				lines.push([head.x + head.pw, mid, xc, mid], [xc, mid, xc, comb], [xc, comb, last, comb]);
			} else if (parent) {
				const px = centre(parent);
				lines.push([px, bottom(parent), px, comb], [px, comb, last, comb]);
			}
			for (const d of these) lines.push([centre(d), comb, centre(d), at.y + TOP]);
		}
		for (const d of [...these].reverse()) if (d.kids.some((k) => !placed.has(k))) row(null, d.kids, d.x + d.w + GAP_X, d, null);
	}
	for (const s of sources) row(tree.get(s)!, tree.get(s)!.kids, 0, null, 'source');
	if (roots.some((k) => !placed.has(k))) row(null, roots, PAD, null, 'earlier');

	const width = Math.max(...nodes.map((n) => n.x + n.w)) + PAD;
	return { nodes, rows, lines, width, height: y - GAP_Y + PAD_B, card, tree };
}

/** the drawn box that shows `key`: its card, or the strip it is folded into (null when it isn't in the band) */
export const nodeOf = (layout: BandLayout | null | undefined, key: string): BoardNode | null => layout?.nodes.find((n) => n.key === key || n.keys?.includes(key)) ?? null;
/** a strip's label: its first card's name + the cards folded in with it ("g3 +2") */
export const stripLabel = (strip: BoardNode) => `${strip.record ? shortName(strip.record) : strip.key}${(strip.keys?.length ?? 1) > 1 ? ` +${strip.keys!.length - 1}` : ''}`;

export interface BandAt {
	shot: string;
	layout: BandLayout;
	folded?: boolean;
}
export type CardAt = { shot: string; key: string };
export type Dir = 'left' | 'right' | 'up' | 'down';

/** the card a key moves to – strips, running placeholders and folded bands are skipped (§3.5, §3.2): left / right in
 *  its row; up / down the row above / below at the nearest column – over a band's edge the previous band's last row /
 *  the next band's first. → { shot, key } | null (the edge of the board) */
export function neighbour(bands: readonly BandAt[], at: CardAt | null, dir: Dir): CardAt | null {
	let b = bands.findIndex((x) => x.shot === at?.shot);
	const node = b >= 0 && at ? nodeOf(bands[b].layout, at.key) : null;
	if (!node || !at) return null;
	const cardsOf = (band: BandAt, r: number) =>
		band.folded ? [] : band.layout.rows[r].keys.map((k) => band.layout.nodes.find((n) => n.key === k)!).filter((n) => n.kind !== 'strip' && n.kind !== 'running');
	if (dir === 'left' || dir === 'right') {
		if (bands[b].folded) return null;
		const own = cardsOf(bands[b], node.row);
		const to = dir === 'left' ? own.filter((n) => n.x < node.x).at(-1) : own.find((n) => n.x > node.x);
		return to ? { shot: at.shot, key: to.key } : null;
	}
	const step = dir === 'up' ? -1 : 1;
	const cx = node.x + node.pw / 2;
	let r = bands[b].folded ? -2 : node.row + step;
	while (b >= 0 && b < bands.length) {
		if (r >= 0 && r < bands[b].layout.rows.length) {
			const cards = cardsOf(bands[b], r);
			if (cards.length) return { shot: bands[b].shot, key: cards.reduce((m, n) => (Math.abs(n.x + n.pw / 2 - cx) < Math.abs(m.x + m.pw / 2 - cx) ? n : m)).key };
			r += step;
			continue;
		}
		b += step;
		r = b >= 0 && b < bands.length ? (step < 0 ? bands[b].layout.rows.length - 1 : 0) : -1;
	}
	return null;
}

/** a card and what hangs under it (keys, the card first, depth first) – folded or not */
export const branchOf = (layout: BandLayout, key: string) => branch(layout.tree, key);
/** a click on a strip opens a LOOK (§3.5): the keys it unfolds – every card in the strip and what hangs under each */
export const lookKeys = (layout: BandLayout, key: string) => [...new Set((nodeOf(layout, key)?.keys ?? [key]).flatMap((k) => branchOf(layout, k)))];

/** X on a card (§3.5): the ids one reject marks – the card and what hangs under it with a record, without the chosen and
 *  what is rejected already; [] when the card itself can't be rejected (a source, the chosen, rejected already) */
export function rejectIds(layout: BandLayout, key: string): number[] {
	const own = layout.tree.get(key);
	if (!own?.record || own.kind === 'source' || ['chosen', 'rejected'].includes(statusOf(own.record) ?? '')) return [];
	return branchOf(layout, key)
		.map((k) => layout.tree.get(k)!.record)
		.filter((r): r is BoardRecord => !!r && !['chosen', 'rejected'].includes(statusOf(r) ?? ''))
		.map((r) => r.id);
}

/** two marks of one X (the server stamps each PUT: `marked_at`) */
export function sameStamp(a: string | null | undefined, b: string | null | undefined, ms = STAMP_MS): boolean {
	if (!a || !b) return false;
	const [x, y] = [Date.parse(a), Date.parse(b)];
	return Number.isFinite(x) && Number.isFinite(y) && Math.abs(x - y) <= ms;
}

/** X on a rejected card: what comes back, each to its mark before (`before(id)` – what this session knows; else none):
 *  the card is the root of its X (its parent wasn't rejected by the same X) → everything that X folded in its branch;
 *  else only the card. → [{ mark, ids }] ([] when the card isn't rejected) */
export function unrejectIds(layout: BandLayout, key: string, before: (id: number) => GenerationMark | null = () => null): { mark: GenerationMark | null; ids: number[] }[] {
	const n = layout.tree.get(key);
	const r = n?.record;
	if (!n || !r || statusOf(r) !== 'rejected') return [];
	const up = n.parent ? layout.tree.get(n.parent)?.record : null;
	const root = !(statusOf(up) === 'rejected' && sameStamp(up?.marked_at, r.marked_at));
	const back = root && r.marked_at
		? branchOf(layout, key).map((k) => layout.tree.get(k)!.record).filter((x): x is BoardRecord => !!x && statusOf(x) === 'rejected' && sameStamp(x.marked_at, r.marked_at))
		: [r];
	const groups = new Map<string, { mark: GenerationMark | null; ids: number[] }>();
	for (const x of back) {
		const was = before(x.id);
		const mark = was === 'rejected' ? null : was;
		const g = groups.get(String(mark)) ?? groups.set(String(mark), { mark, ids: [] }).get(String(mark))!;
		g.ids.push(x.id);
	}
	return [...groups.values()];
}

/** a band head's figures: the results made (a dropped one too), how many of them are rejected, the euro spent on the
 *  shot's generations (a failed job's cost too; a currency without a rate is left out) */
export function bandFacts(records: readonly BoardRecord[] = [], rate: EuroRate | null = null): { done: number; rejected: number; eur: number } {
	const done = records.filter((r) => r?.status === 'succeeded');
	const eur = records.reduce((s, r) => s + (r?.cost ? (toEuro(r.cost, r.currency ?? null, rate) ?? 0) : 0), 0);
	return { done: done.length, rejected: done.filter((r) => statusOf(r) === 'rejected').length, eur };
}
