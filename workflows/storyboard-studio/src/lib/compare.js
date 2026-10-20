// Compare – the pure part (agents/research/VARIANTS-CANVAS.md §3.4, issue #62 stage 4; test/compare.test.mjs). The
// SIDES are the board's selection (board.selected – A, B, C, D in the order picked; Ctrl + click and More ▾ "Add to
// Compare" make it): A is the reference. Opening (C, "Compare · n"): 2–4 selected = those; fewer = the current shot's
// chosen (of the selected card's kind) as A in front – with nothing selected, the current card beside it; a click on a
// card while Compare is open REPLACES THE LAST side (never A: with fewer than two it fills the empty one); a rejected
// card never enters (§3.5). A side taken out while Compare is open (X – "X on side B folds it and empties B" –, Ctrl +
// click, More ▾, its card gone from the board) leaves its place EMPTY (a hole, null): the other sides keep their
// letters, the next click – plain or Ctrl – fills the first empty one; closed, the selection closes up. The sides stand
// at ONE height, each in its shot's aspect, as large as the pane allows (fitSides – nothing scrolls sideways). The
// videos run on ONE CLOCK (Frame.io): one time for all, as long as the longest; a shorter one holds its last frame
// (sideAt); images just stand.

/** at most this many sides (A–D) */
export const MAX_SIDES = 4;
export const LETTERS = 'ABCD';
/** a side's parts (rem – CompareSide draws them at these heights, Compare fits the pictures into what is left): the
 *  line over the picture, the lines under it (model · length · seed · cost in up to 3 – a narrow side –, from … in 1,
 *  the prompt in 3), the gap between them */
export const SIDE_HEAD = 1.5;
export const SIDE_INFO = 8;
export const SIDE_GAP = 0.25;
/** the gap between the sides, the pane's padding, a side's column at least (its words), its picture at least (rem) */
export const GAP = 1;
export const PAD = 1;
export const MIN_COL = 9;
export const MIN_H = 4;

/** the shot's chosen card – the chosen of `kind` (an image and a video each have theirs), else its chosen video, else
 *  any – as a record id, null = none (records: GET /api/generated's, `approved` = approve.mjs's `from`) */
export function chosenKey(records, kind) {
	const chosen = (records ?? []).filter((r) => r?.approved && r.file && !r.missing);
	return (chosen.find((r) => r.kind === kind) ?? chosen.find((r) => r.kind === 'video') ?? chosen[0])?.id ?? null;
}

/** the selection as sides: a card still on the board stays, an empty place or a card gone (`exists` false – a layout
 *  variant taken, a record or a take's render gone) is a hole (null) */
export const resolveSides = (selected, exists = () => true) => (selected ?? []).map((s) => (s && exists(s) ? s : null));
/** the cards only, in their order (the selection closed up) */
export const filled = (sides) => (sides ?? []).filter(Boolean);
/** the letters by key: a side's place (a hole keeps its letter free) */
export const lettersOf = (sides) => new Map((sides ?? []).flatMap((s, i) => (s ? [[s.key, LETTERS[i]]] : [])));

/**
 * The sides Compare opens with. `selected` = [{ shot, key }] (the selection; holes ignored), `shot` = the current
 * shot, `records` = its records, `current` = the current card { shot, key, kind, rejected } (or null), `kindOf(card)` =
 * a selected card's kind. → { sides: [{ shot, key }] (1–4 – one side leaves B empty: the next click fills it), why: '' |
 * the reason none can open }
 */
export function sidesOnOpen({ selected = [], shot, records = [], current = null, kindOf = () => null }) {
	selected = filled(selected);
	if (selected.length >= 2) return { sides: selected.slice(0, MAX_SIDES), why: '' };
	const base = selected[0] ?? (current && !current.rejected ? { shot: current.shot, key: current.key } : null);
	const chosen = shot ? chosenKey(records, base ? (selected[0] ? kindOf(base) : current?.kind) : null) : null;
	const sides = [];
	for (const s of [chosen && { shot, key: chosen }, base]) if (s && !sides.some((x) => x.key === s.key)) sides.push(s);
	return sides.length ? { sides, why: '' } : { sides: [], why: 'Select 2–4 cards to compare (Ctrl + click) – then C.' };
}

/**
 * A click on a card while Compare is open: it fills the first EMPTY side – none empty, it comes in as the LAST side (A
 * stays the reference). Already a side: no change; fewer than two sides: it is the next; rejected: it stays out.
 * `sides` may hold holes (null). → { selected, why }
 */
export function clickSide(sides, card, rejected = false) {
	if (rejected) return { selected: sides, why: 'rejected' };
	if (sides.some((s) => s?.key === card.key)) return { selected: sides, why: '' };
	const hole = sides.indexOf(null);
	if (hole >= 0) return { selected: sides.map((s, i) => (i === hole ? card : s)), why: '' };
	if (sides.length < 2) return { selected: [...sides, card], why: '' };
	return { selected: [...sides.slice(0, -1), card], why: '' };
}

/** Ctrl + click / More ▾ "Add to Compare": into the first empty side, else at the end – a fifth replaces the last (D) */
export function addSide(sides, card) {
	const hole = sides.indexOf(null);
	if (hole >= 0) return sides.map((s, i) => (i === hole ? card : s));
	return [...sides.slice(0, MAX_SIDES - 1), card];
}

/** cards out of the selection (`keys`): Compare open (`open`) – their sides empty, the others keep their letters;
 *  closed – the rest close up */
export function takeOut(sides, keys, open) {
	const out = (s) => !s || keys.includes(s.key);
	return open ? sides.map((s) => (out(s) ? null : s)) : sides.filter((s) => !out(s));
}

/**
 * The sides at ONE height, each as wide as its aspect (w / h) at that height – but a column at least `minCol` wide for
 * its words (a narrow picture leaves room beside it) – side by side with `gap` between, inside `width` × `height` (rem).
 * → { h, widths } (rem): the largest h that fits both ways (0 when there is no room).
 */
export function fitSides({ ratios, width, height, gap = 0, minCol = 0 }) {
	const n = ratios.length;
	if (!n || !(width > 0) || !(height > 0)) return { h: 0, widths: ratios.map(() => 0) };
	const avail = width - gap * (n - 1);
	const m = Math.max(0, Math.min(minCol, avail / n));
	const need = (h) => ratios.reduce((s, r) => s + Math.max(h * r, m), 0);
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
/** a playing video further than this from the clock is put back (ms) – less would seek on every decode hiccup */
export const DRIFT_MS = 200;

/** the clock's length: the longest video (ms; 0 = no video) */
export const clockLength = (lengths) => Math.max(0, ...(lengths ?? []).filter((l) => Number.isFinite(l) && l > 0));

/** a video of `len` ms at the clock's time `t`: where it stands (ms) and whether it holds its last frame (it ended) */
export function sideAt(t, len) {
	if (!(len > 0)) return { ms: 0, held: false };
	const at = Math.max(0, t);
	return at >= len ? { ms: Math.max(0, len - HOLD_MS), held: true } : { ms: at, held: false };
}

/** the clock after `dt` ms of play: { t, ended } – it stops at its length */
export function advance(t, dt, length) {
	const next = Math.max(0, t) + Math.max(0, dt || 0);
	return next >= length ? { t: length, ended: true } : { t: next, ended: false };
}

/** a playing video drifted from the clock (both ms) */
export const drifted = (actual, target, tol = DRIFT_MS) => Math.abs(actual - target) > tol;

/** ms → "0:02.0" (m:ss.d) */
export function clockText(ms) {
	const ds = Math.round(Math.max(0, ms || 0) / 100);
	const s = Math.floor(ds / 10);
	return `${Math.floor(s / 60)}:${String(s % 60).padStart(2, '0')}.${ds % 10}`;
}

// --- what a side says ----------------------------------------------------------------------------------------------------
const ROLE = { control: 'video', keyframe: 'keyframe', reference: 'reference', character: 'character', style: 'style' };
/** how its main input went in: a still is a start frame for image → video, else the picture edited / referenced */
export const roleWord = (input, mode) => (!input?.role ? '' : input.role === 'still' ? (mode === 'image_to_video' ? 'start frame' : 'still') : ROLE[input.role] ?? input.role);

/** "from g5 (start frame)", "from the render + g3 (video)" – `main` = the words for where it hangs (its main input on
 *  the board), `others` = the other parents' words; '' when nothing is known */
export function fromText(record, main, others = []) {
	if (!record || record.earlier) return '';
	const what = [main, ...others].filter(Boolean);
	if (!what.length) return '';
	const role = roleWord(record.inputs?.[0], record.mode);
	return `from ${what.join(' + ')}${role ? ` (${role})` : ''}`;
}
