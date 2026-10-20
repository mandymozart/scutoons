// Compare – the pure part (#125; the port of #62's compare.js – agents/research/VARIANTS-CANVAS.md §3.4). Tested in
// test/storyboard-compare.test.ts. The SIDES are the board's selection (A, B, C, D in the order picked; Ctrl + click and
// More ▾ make it): A is the reference. Opening (C, "Compare · n"): 2–4 selected = those; fewer = the current shot's
// chosen (of the selected card's kind) as A in front – with nothing selected, the current card beside it; a click on a
// card while Compare is open REPLACES THE LAST side (never A: with fewer than two it fills the empty one); a rejected
// card never enters (§3.5). A side taken out while Compare is open leaves its place EMPTY (null): the other sides keep
// their letters, the next click fills the first empty one; closed, the selection closes up. The sides stand at ONE
// height, each in its shot's aspect, as large as the pane allows (fitSides – nothing scrolls sideways). The videos run
// on ONE CLOCK (Frame.io): as long as the longest; a shorter one holds its last frame (sideAt); images just stand.
import type { BoardRecord } from './storyboard-board-layout.ts';

/** at most this many sides (A–D) */
export const MAX_SIDES = 4;
export const LETTERS = 'ABCD';
/** a side's parts (rem): the line over the picture, the lines under it, the gap between them */
export const SIDE_HEAD = 1.5;
export const SIDE_INFO = 8;
export const SIDE_GAP = 0.25;
/** the gap between the sides, the pane's padding, a side's column at least (its words), its picture at least (rem) */
export const GAP = 1;
export const PAD = 1;
export const MIN_COL = 9;
export const MIN_H = 4;

export type Side = { shot: string; key: string };

/** the shot's chosen card – the chosen of `kind`, else its chosen video, else any – as a card key, null = none */
export function chosenKey(records: readonly Pick<BoardRecord, 'id' | 'kind' | 'approved' | 'status'>[] | null | undefined, kind: string | null): string | null {
	const chosen = (records ?? []).filter((r) => r?.approved && r.status === 'succeeded');
	const r = chosen.find((x) => x.kind === kind) ?? chosen.find((x) => x.kind === 'video') ?? chosen[0];
	return r ? String(r.id) : null;
}

/** the selection as sides: a card still on the board stays, an empty place or a card gone is a hole (null) */
export const resolveSides = (selected: readonly (Side | null)[] | null | undefined, exists: (s: Side) => boolean = () => true) =>
	(selected ?? []).map((s) => (s && exists(s) ? s : null));
/** the cards only, in their order (the selection closed up) */
export const filled = (sides: readonly (Side | null)[] | null | undefined): Side[] => (sides ?? []).filter((s): s is Side => !!s);
/** the letters by key: a side's place (a hole keeps its letter free) */
export const lettersOf = (sides: readonly (Side | null)[] | null | undefined) => new Map((sides ?? []).flatMap((s, i) => (s ? [[s.key, LETTERS[i]] as [string, string]] : [])));

export interface OpenAt {
	selected?: readonly (Side | null)[];
	shot: string | null;
	records?: readonly Pick<BoardRecord, 'id' | 'kind' | 'approved' | 'status'>[];
	current?: (Side & { kind?: string | null; rejected?: boolean }) | null;
	kindOf?: (s: Side) => string | null;
}

/** the sides Compare opens with → { sides (1–4 – one side leaves B empty), why: '' | 'select' (nothing to compare) } */
export function sidesOnOpen({ selected = [], shot, records = [], current = null, kindOf = () => null }: OpenAt): { sides: Side[]; why: '' | 'select' } {
	const sel = filled(selected);
	if (sel.length >= 2) return { sides: sel.slice(0, MAX_SIDES), why: '' };
	const base = sel[0] ?? (current && !current.rejected ? { shot: current.shot, key: current.key } : null);
	const chosen = shot ? chosenKey(records, base ? (sel[0] ? kindOf(base) : (current?.kind ?? null)) : null) : null;
	const sides: Side[] = [];
	for (const s of [chosen && shot ? { shot, key: chosen } : null, base]) if (s && !sides.some((x) => x.key === s.key)) sides.push(s);
	return sides.length ? { sides, why: '' } : { sides: [], why: 'select' };
}

/** a click on a card while Compare is open: the first EMPTY side – none empty, the LAST side (A stays); already a side:
 *  no change; fewer than two sides: the next; rejected: it stays out */
export function clickSide(sides: (Side | null)[], card: Side, rejected = false): { selected: (Side | null)[]; why: '' | 'rejected' } {
	if (rejected) return { selected: sides, why: 'rejected' };
	if (sides.some((s) => s?.key === card.key)) return { selected: sides, why: '' };
	const hole = sides.indexOf(null);
	if (hole >= 0) return { selected: sides.map((s, i) => (i === hole ? card : s)), why: '' };
	if (sides.length < 2) return { selected: [...sides, card], why: '' };
	return { selected: [...sides.slice(0, -1), card], why: '' };
}

/** Ctrl + click / More ▾ "Add to Compare": into the first empty side, else at the end – a fifth replaces the last (D) */
export function addSide(sides: (Side | null)[], card: Side): (Side | null)[] {
	const hole = sides.indexOf(null);
	if (hole >= 0) return sides.map((s, i) => (i === hole ? card : s));
	return [...sides.slice(0, MAX_SIDES - 1), card];
}

/** cards out of the selection: Compare open – their sides empty, the others keep their letters; closed – close up */
export function takeOut(sides: readonly (Side | null)[], keys: readonly string[], open: boolean): (Side | null)[] {
	const out = (s: Side | null) => !s || keys.includes(s.key);
	return open ? sides.map((s) => (out(s) ? null : s)) : sides.filter((s) => !out(s));
}

/** the sides at ONE height, each as wide as its aspect (w / h) – but a column at least `minCol` wide – side by side with
 *  `gap` between, inside `width` × `height` (rem) → { h, widths }: the largest h that fits both ways */
export function fitSides({ ratios, width, height, gap = 0, minCol = 0 }: { ratios: number[]; width: number; height: number; gap?: number; minCol?: number }): { h: number; widths: number[] } {
	const n = ratios.length;
	if (!n || !(width > 0) || !(height > 0)) return { h: 0, widths: ratios.map(() => 0) };
	const avail = width - gap * (n - 1);
	const m = Math.max(0, Math.min(minCol, avail / n));
	const need = (h: number) => ratios.reduce((s, r) => s + Math.max(h * r, m), 0);
	let [lo, hi] = [0, height];
	if (need(hi) > avail) {
		for (let i = 0; i < 40; i++) {
			const mid = (lo + hi) / 2;
			if (need(mid) <= avail) lo = mid;
			else hi = mid;
		}
		hi = lo;
	}
	const h = Math.floor(hi * 100) / 100;
	return { h, widths: ratios.map((r) => Math.max(h * r, m)) };
}

// --- the clock ----------------------------------------------------------------------------------------------------------
/** a video held at its end stands this far before it (ms) – its last frame, not past it */
export const HOLD_MS = 1;
/** a playing video further than this from the clock is put back (ms) */
export const DRIFT_MS = 200;

/** the clock's length: the longest video (ms; 0 = no video) */
export const clockLength = (lengths: readonly (number | null | undefined)[] | null | undefined) =>
	Math.max(0, ...(lengths ?? []).filter((l): l is number => typeof l === 'number' && Number.isFinite(l) && l > 0));

/** a video of `len` ms at the clock's time `t`: where it stands (ms) and whether it holds its last frame */
export function sideAt(t: number, len: number | null | undefined): { ms: number; held: boolean } {
	if (!(len && len > 0)) return { ms: 0, held: false };
	const at = Math.max(0, t);
	return at >= len ? { ms: Math.max(0, len - HOLD_MS), held: true } : { ms: at, held: false };
}

/** the clock after `dt` ms of play: { t, ended } – it stops at its length */
export function advance(t: number, dt: number, length: number): { t: number; ended: boolean } {
	const next = Math.max(0, t) + Math.max(0, dt || 0);
	return next >= length ? { t: length, ended: true } : { t: next, ended: false };
}

/** a playing video drifted from the clock (both ms) */
export const drifted = (actual: number, target: number, tol = DRIFT_MS) => Math.abs(actual - target) > tol;

/** ms → "0:02.0" (m:ss.d) */
export function clockText(ms: number): string {
	const ds = Math.round(Math.max(0, ms || 0) / 100);
	const s = Math.floor(ds / 10);
	return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, '0')}.${ds % 10}`;
}

// --- what a side says ----------------------------------------------------------------------------------------------------
/** how its main input went in, as a word key: a still is a start frame for image → video, else the still; a control
 *  the video (the UI words: storyboard.compare_role_*) */
export function roleWord(input: { role?: string } | null | undefined, mode: string | null | undefined): '' | 'start' | 'still' | 'video' | 'other' {
	if (!input?.role) return '';
	if (input.role === 'still') return mode === 'image_to_video' ? 'start' : 'still';
	return input.role === 'control' ? 'video' : 'other';
}
