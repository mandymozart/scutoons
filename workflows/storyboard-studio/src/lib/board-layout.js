// The variants board's layout (agents/research/VARIANTS-CANVAS.md §3.2 + §3.5, issue #62) – pure: no Svelte, no DOM
// (test/board-layout.test.mjs). One BAND per picked shot; inside a band everything is laid out by the data – nobody
// drags or wires (owner 2026-10-20: rows of results, as recommended):
//   - column 0 = the SOURCES, a row each: the render without texts (the saved frame is the same card), the kept layout
//     variants (v1, v2 … – Q5: A, owner 2026-10-20), one card per take with a handheld render, the dropped files
//     (`generator: 'upload'` without parents – a drop is a source);
//   - a parent's results form ONE row, oldest left (by g<n>); a generation's results open a new row under it, one column
//     right of it; a result hangs under its MAIN input (inputs[0]): a take's video / a variant's frame / the render →
//     that source card (Aleph from take 3 with g5 as the keyframe sits in take 3's row), a generated file → that parent
//     (parents[0] – the first parent on the board); the others are named in its tooltip;
//   - a record without a parent on the board hangs under the source any input names (a take's render → that take's
//     card, the render / the saved frame / the approved copy → the render), else in the row "Earlier – no record" under
//     the sources – a file without a v2 record too (`earlier`): nothing is guessed;
//   - child rows follow right under their parent's row, the rightmost parent's first – no line crosses;
//   - a RUNNING job (§3.3) is a placeholder where its result will land: at the end of its main input's row, else the
//     render's – not a card (no status, the keys skip it); its record replaces it when the job ends.
// THE FOLD (§3.5, owner 2026-10-20: "wenn rejected the need to fold somehow so we can make space"): a rejected card is a
// STRIP in its place (the picture's height, SW wide – drawn like the Rail); with nothing open under it its branch folds
// into that strip ("g3 +2"), and folded neighbours in a row share one strip; the row closes up, the line to it stays. A
// row of nothing but strips LIES DOWN as one line (SW high – the height comes back). `showRejected` folds nothing, `open`
// = the keys a look unfolded (lookKeys). The tree keeps every card (`tree`) – X's ids and the look read it.
// Lines (2px ink, BoardLines.svelte) as segments [x1, y1, x2, y2]: a comb above each row with a tick into every result,
// a drop from the parent's caption into it – from a source out of its picture's right side. Sizes in rem: one picture
// height for all bands (PIC_H × zoom), a card as wide as its shot's aspect but at least a caption wide (a narrow picture
// leaves room; another ratio is letterboxed in it); below CAPTIONS_MIN the captions hide. Keys: neighbour().

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
/** a strip: SW wide standing (the Rail's width), SW high lying down – at least LIE_W wide then (✕ + "g12 +1" at the caption size) */
export const SW = 1.5;
export const LIE_W = 5;
/** below this zoom the captions hide (§3.2) – the columns are then the pictures' width */
export const CAPTIONS_MIN = 0.6;
export const LABELS = { source: 'Source', variant: 'Variant', earlier: 'Earlier – no record' };

/** a shot's aspect "9:16" → width / height (9 / 16 when unreadable) */
export function ratioOf(aspect) {
	const m = /^(\d+(?:\.\d+)?):(\d+(?:\.\d+)?)$/.exec(String(aspect ?? '').trim());
	return m && +m[1] > 0 && +m[2] > 0 ? +m[1] / +m[2] : 9 / 16;
}
/** a band's card at `zoom`: the picture h × pw, the column w, the caption's height (rem; 0 = captions hidden) */
export function cardSize(aspect, zoom = 1) {
	const h = PIC_H * zoom;
	const pw = h * ratioOf(aspect);
	const captions = zoom >= CAPTIONS_MIN;
	return { h, pw, w: captions ? Math.max(pw, CAPTION_W * zoom) : pw, meta: captions ? META : 0 };
}
/** the n of "<shot>--g<n>" (0 for anything else) – the order a shot's results were made in */
export const nOf = (id) => Number(/--g(\d+)$/.exec(String(id ?? ''))?.[1] ?? 0);
/** "<shot>--g7" → "g7" */
export const shortId = (id) => String(id ?? '').replace(/^[\w-]+?--/, '');

/** a card's status (§3.5): chosen (approve.mjs `from` – no mark) · review · rejected · null (none) */
export function statusOf(r) {
	if (!r) return null;
	if (r.approved) return 'chosen';
	return r.mark === 'review' || r.mark === 'rejected' ? r.mark : null;
}

const esc = (s) => String(s).replace(/[^\w-]/g, '\\$&');
const bare = (key) => String(key ?? '').replace(/^frame:/, '').replace(/#.*$/, '');
const PASS = '(?:grey|beauty|depth|normals)';
/** the source card an input key names in `shot`'s band (`tree` – only one that is on the board) – a take's (its handheld
 *  render or frame, generated-log.mjs TAKE_VIDEO / TAKE_FRAME), a layout variant's (its saved frame), the render (clean,
 *  the saved frame; the approved copy only with `chosen` – it counts as its `from`, a parent, when that is known) – or
 *  null */
function sourceOf(key, shot, tree, chosen = true) {
	const k = bare(key);
	const own = (x) => (tree.has(x) ? x : null);
	const take = new RegExp(`^out:[a-z0-9-]+/handheld/${esc(shot)}--([\\w-]+?)(?:--${PASS}\\.\\w+|/${PASS}/f\\d+\\.png)$`).exec(k)?.[1];
	if (take) return own(`take:${shot}:${take}`);
	const variant = new RegExp(`^out:[a-z0-9-]+/variants/${esc(shot)}--([\\w-]+)\\.png$`).exec(k)?.[1];
	if (variant) return own(`variant:${shot}:${variant}`);
	if (new RegExp(`^out:[a-z0-9-]+/(?:clean|frames)/${esc(shot)}\\.png$`).test(k)) return `render:${shot}`;
	return chosen && new RegExp(`^boards:dataset/files/[a-z0-9-]+/${esc(shot)}/chosen\\.\\w+$`).test(k) ? `render:${shot}` : null;
}
/** where a record hangs (§3.2 "under its MAIN input"): every writer puts the main input first (jobs.mjs inputsOf,
 *  convert-generated-log.mjs) – a take's video or the render there → that source (Aleph: take 3's video + g5 as the
 *  keyframe hangs in take 3's row, the mockup's g10); else its first parent on the board (parents[0] = the main input
 *  when that is a generated file); else a source any input names; else null (the earlier row) */
function parentOf(r, tree, shot) {
	if (r.earlier) return null;
	return sourceOf(r.inputs?.[0]?.key, shot, tree, false)
		?? (r.parents ?? []).find((p) => p !== r.id && tree.get(p)?.record)
		?? (r.inputs ?? []).map((i) => sourceOf(i?.key, shot, tree)).find(Boolean)
		?? null;
}
/** a card and what hangs under it (keys, the card first, depth first) in a tree */
function branch(tree, key) {
	const out = [];
	const walk = (k) => { const n = tree.get(k); if (!n || out.includes(k)) return; out.push(k); n.kids.forEach(walk); };
	walk(key);
	return out;
}

/**
 * A band's layout. `shot` = the shot id; `variants` = its kept layouts (GET /api/variants: [{ id, note, created, shot }]);
 * `takes` = its takes with a handheld render (ids, in order); `records` = GET /api/generated's records of the shot – only
 * those with a file are cards (a failed / cancelled job has none: the board shows results, the Generate panel the jobs);
 * `jobs` = its running jobs (GET /api/generate/jobs: [{ id, inputs, parents, … }]); `aspect` = the shot's ("9:16");
 * `zoom` (1 = 100 %); `showRejected` = no fold; `open` = the keys a look unfolded.
 * → { nodes: the drawn boxes, row by row – cards [{ key, kind: 'source' | 'result' | 'running', source: 'render' |
 *     'variant' | 'take' | 'upload' | null, variant, take, record, job, parent (a key | null), kids (keys), x, y, w, pw, h,
 *     row }] and strips [{ …its first card, kind: 'strip', keys (every card folded into it, the first first), whole (its
 *     branch folded in), lie }], rows: [{ y, x, keys, label, lie }], lines: [[x1, y1, x2, y2]], width, height, card: { w,
 *     pw, h, meta }, tree: Map key → card (all of them, folded or not) } – rem inside the band's body
 */
export function bandLayout({ shot, variants = [], takes = [], records = [], jobs = [], aspect, zoom = 1, showRejected = false, open = [] }) {
	const card = cardSize(aspect, zoom);
	const tree = new Map();
	const add = (key, f) => tree.set(key, { key, kind: 'result', source: null, variant: null, take: null, record: null, job: null, parent: null, kids: [], ...f });
	// the sources: the render, the layout variants after it (Q5: A), the takes
	add(`render:${shot}`, { kind: 'source', source: 'render' });
	for (const v of variants) if (v?.id) add(`variant:${shot}:${v.id}`, { kind: 'source', source: 'variant', variant: v });
	for (const t of takes) add(`take:${shot}:${t}`, { kind: 'source', source: 'take', take: t });
	const sources = [...tree.keys()];
	// oldest first: by g<n>, else the log's order (a stable sort)
	const results = records.filter((r) => r?.id && r.file && !tree.has(r.id)).map((r, i) => ({ r, i })).sort((a, b) => nOf(a.r.id) - nOf(b.r.id) || a.i - b.i).map(({ r }) => r);
	for (const r of results) {
		const upload = r.generator === 'upload' && !r.parents?.length && !r.earlier;
		add(r.id, { kind: upload ? 'source' : 'result', source: upload ? 'upload' : null, take: r.take ?? null, record: r });
		if (upload) sources.push(r.id);
	}
	// a running job: after the results (a new run lands at its row's end)
	for (const j of jobs) if (j?.id && !tree.has(`job:${j.id}`)) add(`job:${j.id}`, { kind: 'running', job: j });
	// each result under its main input on the board, else its source, else the earlier row (roots)
	const roots = [];
	for (const n of tree.values()) {
		if (n.kind === 'source') continue;
		const parent = n.job ? parentOf({ inputs: n.job.inputs, parents: n.job.parents }, tree, shot) ?? sources[0] : parentOf(n.record, tree, shot);
		if (parent) { n.parent = parent; tree.get(parent).kids.push(n.key); } else roots.push(n.key);
	}
	// a loop of parents (a broken log) never reaches a source: it goes to the earlier row too
	const reached = new Set();
	const walk = (k) => { if (reached.has(k)) return; reached.add(k); tree.get(k).kids.forEach(walk); };
	[...sources, ...roots].forEach(walk);
	for (const n of tree.values()) if (!reached.has(n.key)) { if (n.parent) tree.get(n.parent).kids = tree.get(n.parent).kids.filter((k) => k !== n.key); n.parent = null; roots.push(n.key); walk(n.key); }

	// the fold: a rejected result folds unless shown; its branch folds in WHOLE when nothing under it is open
	const opened = new Set(open);
	const folded = (n) => !showRejected && n.kind === 'result' && statusOf(n.record) === 'rejected' && !opened.has(n.key);
	const whole = new Map();
	const isWhole = (n) => (whole.has(n.key) ? whole.get(n.key) : whole.set(n.key, folded(n) && n.kids.every((k) => isWhole(tree.get(k)))).get(n.key));

	const rows = [], lines = [], nodes = [], placed = new Set();
	let y = PAD_T;
	const centre = (d) => d.x + d.pw / 2;
	const bottom = (d) => d.y + TOP + d.h + (d.kind === 'strip' ? 0 : card.meta);
	/** a row: its lead (a source) at column 0, then `items` from x0; then the child rows of its items, rightmost first */
	function row(lead, items, x0, parent, label) {
		const at = { y, x: lead ? PAD : x0, keys: [], label, lie: false };
		rows.push(at);
		// the pieces: a card; a folded card = a strip (a whole branch merges with a whole neighbour's)
		const pieces = [];
		for (const k of items) {
			if (placed.has(k)) continue;
			const n = tree.get(k);
			if (isWhole(n)) {
				const keys = branch(tree, k);
				keys.forEach((x) => placed.add(x));
				if (pieces.at(-1)?.whole) pieces.at(-1).keys.push(...keys);
				else pieces.push({ n, strip: true, whole: true, keys });
			} else {
				placed.add(k);
				pieces.push({ n, strip: folded(n), whole: false, keys: [k] });
			}
		}
		at.lie = !lead && pieces.length > 0 && pieces.every((p) => p.strip);
		let x = at.x;
		const put = (d) => { Object.assign(d, { x, y, row: rows.length - 1 }); at.keys.push(d.key); nodes.push(d); x += d.w + GAP_X; return d; };
		const head = lead && (placed.add(lead.key), put({ ...lead, w: card.w, pw: card.pw, h: card.h }));
		const these = pieces.map((p) => put(!p.strip ? { ...p.n, w: card.w, pw: card.pw, h: card.h }
			: at.lie ? { ...p.n, kind: 'strip', keys: p.keys, whole: p.whole, lie: true, w: Math.max(card.w, LIE_W), pw: Math.max(card.w, LIE_W), h: SW }
				: { ...p.n, kind: 'strip', keys: p.keys, whole: p.whole, lie: false, w: SW, pw: SW, h: card.h }));
		y += TOP + (at.lie ? SW : card.h + card.meta) + GAP_Y;
		if (these.length && (head || parent)) {
			const comb = at.y + COMB;
			const last = centre(these.at(-1));
			if (head) {
				const mid = at.y + TOP + card.h / 2;
				const xc = head.x + head.w + GAP_X / 2;
				lines.push([head.x + head.pw, mid, xc, mid], [xc, mid, xc, comb], [xc, comb, last, comb]);
			} else {
				const px = centre(parent);
				lines.push([px, bottom(parent), px, comb], [px, comb, last, comb]);
			}
			for (const d of these) lines.push([centre(d), comb, centre(d), at.y + TOP]);
		}
		for (const d of [...these].reverse()) if (d.kids.some((k) => !placed.has(k))) row(null, d.kids, d.x + d.w + GAP_X, d, null);
	}
	for (const s of sources) row(tree.get(s), tree.get(s).kids, 0, null, tree.get(s).source === 'variant' ? LABELS.variant : LABELS.source);
	if (roots.some((k) => !placed.has(k))) row(null, roots, PAD, null, LABELS.earlier);

	const width = Math.max(...nodes.map((n) => n.x + n.w)) + PAD;
	return { nodes, rows, lines, width, height: y - GAP_Y + PAD_B, card, tree };
}

/** the drawn box that shows `key`: its card, or the strip it is folded into (null when it isn't in the band) */
export const nodeOf = (layout, key) => layout?.nodes.find((n) => n.key === key || n.keys?.includes(key)) ?? null;
/** a strip's label: its first id + the cards folded in with it ("g3 +2") */
export const stripLabel = (strip) => `${shortId(strip.record?.id ?? strip.key)}${strip.keys.length > 1 ? ` +${strip.keys.length - 1}` : ''}`;

/** the card a key moves to – strips, running placeholders and folded bands are skipped (§3.5, §3.2): 'left' / 'right' in
 *  its row; 'up' / 'down' the row above / below at the nearest column (the pictures' centres) – over a band's edge the
 *  previous band's last row / the next band's first. bands = [{ shot, layout, folded }] in the board's order; at = { shot, key } (a strip's card
 *  too: it moves from the strip) → { shot, key } | null (the edge of the board) */
export function neighbour(bands, at, dir) {
	let b = bands.findIndex((x) => x.shot === at?.shot);
	const node = b >= 0 ? nodeOf(bands[b].layout, at.key) : null;
	if (!node) return null;
	const cardsOf = (band, r) => (band.folded ? [] : band.layout.rows[r].keys.map((k) => band.layout.nodes.find((n) => n.key === k)).filter((n) => n.kind !== 'strip' && n.kind !== 'running'));
	if (dir === 'left' || dir === 'right') {
		if (bands[b].folded) return null;
		const own = cardsOf(bands[b], node.row);
		const to = dir === 'left' ? own.filter((n) => n.x < node.x).at(-1) : own.find((n) => n.x > node.x);
		return to ? { shot: at.shot, key: to.key } : null;
	}
	if (dir !== 'up' && dir !== 'down') return null;
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
export const branchOf = (layout, key) => branch(layout.tree, key);
/** a click on a strip opens a LOOK (§3.5: "unfolding makes it always possible to dive in that tree"): the keys it unfolds –
 *  every card in the strip and what hangs under each */
export const lookKeys = (layout, key) => [...new Set((nodeOf(layout, key)?.keys ?? [key]).flatMap((k) => branchOf(layout, k)))];

/** X on a card (§3.5): the ids one reject marks – the card and what hangs under it that has a record, without the chosen
 *  and what is rejected already (it keeps its own stamp); [] when the card itself can't be rejected (a source – the
 *  render, a take, a dropped file: "Not rejectable: the chosen … and the sources" –, the chosen, rejected already – that
 *  X unrejects) */
export function rejectIds(layout, key) {
	const own = layout.tree.get(key);
	if (!own?.record || own.kind === 'source' || ['chosen', 'rejected'].includes(statusOf(own.record))) return [];
	return branchOf(layout, key).map((k) => layout.tree.get(k).record).filter((r) => r && !['chosen', 'rejected'].includes(statusOf(r))).map((r) => r.id);
}

/** X on a rejected card: what comes back, each to its mark before (`before` – one POST per mark): the card is the root
 *  of its X (its parent wasn't rejected by the same stamp) → everything that X folded in this band; else only the card.
 *  → [{ mark, marked, ids }] ([] when the card isn't rejected) */
export function unrejectIds(layout, key) {
	const n = layout.tree.get(key);
	const r = n?.record;
	if (statusOf(r) !== 'rejected') return [];
	const up = layout.tree.get(n.parent)?.record;
	const root = !(statusOf(up) === 'rejected' && up.marked === r.marked);
	const back = root && r.marked ? [...layout.tree.values()].map((x) => x.record).filter((x) => statusOf(x) === 'rejected' && x.marked === r.marked) : [r];
	const groups = new Map();
	for (const x of back) {
		const [mark, marked] = [x.before?.mark ?? null, x.before?.mark ? x.before.marked ?? null : null];
		const g = groups.get(`${mark}|${marked}`) ?? groups.set(`${mark}|${marked}`, { mark, marked, ids: [] }).get(`${mark}|${marked}`);
		g.ids.push(x.id);
	}
	return [...groups.values()];
}

/** a band head's figures: the results made (files – a dropped one too), how many of them are rejected, the money spent on
 *  the shot's jobs (USD – a failed job's cost too) */
export function bandFacts(records = []) {
	const files = records.filter((r) => r?.file);
	return {
		done: files.length,
		rejected: files.filter((r) => statusOf(r) === 'rejected').length,
		usd: records.reduce((s, r) => s + (Number(r?.cost?.usd) || 0), 0)
	};
}
