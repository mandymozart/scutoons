// The variants board's layout (agents/research/VARIANTS-CANVAS.md §3.2 + §3.5, issue #62) – pure: no Svelte, no DOM
// (test/board-layout.test.mjs). One BAND per picked shot; inside a band everything is laid out by the data – nobody
// drags or wires (owner 2026-10-20: rows of results, as recommended):
//   - column 0 = the SOURCES, a row each: the render without texts (the saved frame is the same card), one card per take
//     with a handheld render, the dropped files (`generator: 'upload'` without parents – a drop is a source);
//   - a parent's results form ONE row, oldest left (by g<n>); a generation's results open a new row under it, one column
//     right of it; a result hangs under its MAIN input (inputs[0]): a take's video / the render → that source card (Aleph
//     from take 3 with g5 as the keyframe sits in take 3's row), a generated file → that parent (parents[0] – the first
//     parent on the board); the others are named in its tooltip;
//   - a record without a parent on the board hangs under the source any input names (a take's render → that take's
//     card, the render / the saved frame / the approved copy → the render), else in the row "Earlier – no record" under
//     the sources – a file without a v2 record too (`earlier`): nothing is guessed;
//   - child rows follow right under their parent's row, the rightmost parent's first – no line crosses.
// Lines (2px ink, BoardLines.svelte) as segments [x1, y1, x2, y2]: a comb above each row with a tick into every result,
// a drop from the parent's caption into it – from a source out of its picture's right side. Sizes in rem at 100 %: one
// picture height for all bands (PIC_H), a card as wide as its shot's aspect but at least a caption wide (a narrow
// picture leaves room; another ratio is letterboxed in it). The board's keys: neighbour(); X: rejectIds / unrejectIds.

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
export const LABELS = { source: 'Source', earlier: 'Earlier – no record' };

/** a shot's aspect "9:16" → width / height (9 / 16 when unreadable) */
export function ratioOf(aspect) {
	const m = /^(\d+(?:\.\d+)?):(\d+(?:\.\d+)?)$/.exec(String(aspect ?? '').trim());
	return m && +m[1] > 0 && +m[2] > 0 ? +m[1] / +m[2] : 9 / 16;
}
/** a band's card: the picture h × pw, the column w (rem) */
export function cardSize(aspect, zoom = 1) {
	const h = PIC_H * zoom;
	const pw = h * ratioOf(aspect);
	return { h, pw, w: Math.max(pw, CAPTION_W * zoom) };
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
/** the source card an input key names in `shot`'s band – a take's (its handheld render or frame, generated-log.mjs
 *  TAKE_VIDEO / TAKE_FRAME), the render (clean, the saved frame; the approved copy only with `chosen` – it counts as its
 *  `from`, a parent, when that is known) – or null */
function sourceOf(key, shot, takes, chosen = true) {
	const k = bare(key);
	const take = new RegExp(`^out:[a-z0-9-]+/handheld/${esc(shot)}--([\\w-]+?)(?:--${PASS}\\.\\w+|/${PASS}/f\\d+\\.png)$`).exec(k)?.[1];
	if (take) return takes.includes(take) ? `take:${shot}:${take}` : null;
	if (new RegExp(`^out:[a-z0-9-]+/(?:clean|frames)/${esc(shot)}\\.png$`).test(k)) return `render:${shot}`;
	return chosen && new RegExp(`^boards:dataset/files/[a-z0-9-]+/${esc(shot)}/chosen\\.\\w+$`).test(k) ? `render:${shot}` : null;
}
/** where a record hangs (§3.2 "under its MAIN input"): every writer puts the main input first (jobs.mjs inputsOf,
 *  convert-generated-log.mjs) – a take's video or the render there → that source (Aleph: take 3's video + g5 as the
 *  keyframe hangs in take 3's row, the mockup's g10); else its first parent on the board (parents[0] = the main input
 *  when that is a generated file); else a source any input names; else null (the earlier row) */
function parentOf(r, nodes, shot, takes) {
	if (r.earlier) return null;
	return sourceOf(r.inputs?.[0]?.key, shot, takes, false)
		?? (r.parents ?? []).find((p) => p !== r.id && nodes.get(p)?.record)
		?? (r.inputs ?? []).map((i) => sourceOf(i?.key, shot, takes)).find(Boolean)
		?? null;
}

/**
 * A band's layout. `shot` = the shot id; `takes` = its takes with a handheld render (ids, in order); `records` = GET
 * /api/generated's records of the shot – only those with a file are cards (a failed / cancelled job has none: the board
 * shows results, the Generate panel the jobs); `aspect` = the shot's ("9:16"); `zoom` (1 = 100 %).
 * → { nodes: [{ key, kind: 'source' | 'result', source: 'render' | 'take' | 'upload' | null, take, record, parent (a key |
 *     null), kids (keys), x, y, w, pw, h, row }] (row by row), rows: [{ y, x, keys, label }], lines: [[x1, y1, x2, y2]],
 *     width, height, card: { w, pw, h } } – rem inside the band's body
 */
export function bandLayout({ shot, takes = [], records = [], aspect, zoom = 1 }) {
	const card = cardSize(aspect, zoom);
	const rowH = TOP + card.h + META;
	const nodes = new Map();
	const add = (key, f) => nodes.set(key, { key, kind: 'result', source: null, take: null, record: null, parent: null, kids: [], x: 0, y: 0, w: card.w, pw: card.pw, h: card.h, row: -1, ...f });
	const sources = [`render:${shot}`, ...takes.map((t) => `take:${shot}:${t}`)];
	add(sources[0], { kind: 'source', source: 'render' });
	takes.forEach((t, i) => add(sources[i + 1], { kind: 'source', source: 'take', take: t }));
	// oldest first: by g<n>, else the log's order (a stable sort)
	const results = records.filter((r) => r?.id && r.file && !nodes.has(r.id)).map((r, i) => ({ r, i })).sort((a, b) => nOf(a.r.id) - nOf(b.r.id) || a.i - b.i).map(({ r }) => r);
	for (const r of results) {
		const upload = r.generator === 'upload' && !r.parents?.length && !r.earlier;
		add(r.id, { kind: upload ? 'source' : 'result', source: upload ? 'upload' : null, take: r.take ?? null, record: r });
		if (upload) sources.push(r.id);
	}
	// each result under its main input on the board, else its source, else the earlier row (roots)
	const roots = [];
	for (const n of nodes.values()) {
		if (n.kind === 'source') continue;
		const parent = parentOf(n.record, nodes, shot, takes);
		if (parent) { n.parent = parent; nodes.get(parent).kids.push(n.key); } else roots.push(n.key);
	}
	// a loop of parents (a broken log) never reaches a source: it goes to the earlier row too
	const reached = new Set();
	const walk = (k) => { if (reached.has(k)) return; reached.add(k); nodes.get(k).kids.forEach(walk); };
	[...sources, ...roots].forEach(walk);
	for (const n of nodes.values()) if (!reached.has(n.key)) { if (n.parent) nodes.get(n.parent).kids = nodes.get(n.parent).kids.filter((k) => k !== n.key); n.parent = null; roots.push(n.key); walk(n.key); }

	const rows = [], lines = [], list = [], placed = new Set();
	let y = PAD_T;
	const centre = (n) => n.x + n.pw / 2;
	/** a row: its lead (a source) at column 0, then `items` from x0; then the child rows of its items, rightmost first */
	function row(lead, items, x0, parent, label) {
		const at = { y, x: lead ? PAD : x0, keys: [], label };
		rows.push(at);
		let x = at.x;
		const put = (n) => { placed.add(n.key); Object.assign(n, { x, y, row: rows.length - 1 }); at.keys.push(n.key); list.push(n); x += n.w + GAP_X; };
		const these = items.filter((k) => !placed.has(k)).map((k) => nodes.get(k));
		if (lead) put(lead);
		these.forEach(put);
		y += rowH + GAP_Y;
		if (these.length && (lead || parent)) {
			const comb = at.y + COMB;
			const last = centre(these.at(-1));
			if (lead) {
				const mid = at.y + TOP + card.h / 2;
				const xc = lead.x + lead.w + GAP_X / 2;
				lines.push([lead.x + lead.pw, mid, xc, mid], [xc, mid, xc, comb], [xc, comb, last, comb]);
			} else {
				const px = centre(parent);
				lines.push([px, parent.y + TOP + card.h + META, px, comb], [px, comb, last, comb]);
			}
			for (const n of these) lines.push([centre(n), comb, centre(n), at.y + TOP]);
		}
		for (const n of [...these].reverse()) if (n.kids.some((k) => !placed.has(k))) row(null, n.kids, n.x + n.w + GAP_X, n, null);
	}
	for (const s of sources) row(nodes.get(s), nodes.get(s).kids, 0, null, LABELS.source);
	if (roots.length) row(null, roots, PAD, null, LABELS.earlier);

	const width = Math.max(...list.map((n) => n.x + n.w)) + PAD;
	return { nodes: list, rows, lines, width, height: y - GAP_Y + PAD_B, card };
}

/** the card a key moves to: 'left' / 'right' in its row; 'up' / 'down' the row above / below at the nearest column (the
 *  pictures' centres) – over a band's edge the previous band's last row / the next band's first. bands = [{ shot,
 *  layout }] in the board's order; at = { shot, key } → { shot, key } | null (the edge of the board) */
export function neighbour(bands, at, dir) {
	let b = bands.findIndex((x) => x.shot === at?.shot);
	const node = b >= 0 ? bands[b].layout.nodes.find((n) => n.key === at.key) : null;
	if (!node) return null;
	const own = bands[b].layout.rows[node.row].keys;
	if (dir === 'left' || dir === 'right') {
		const key = own[own.indexOf(node.key) + (dir === 'left' ? -1 : 1)];
		return key ? { shot: at.shot, key } : null;
	}
	if (dir !== 'up' && dir !== 'down') return null;
	const step = dir === 'up' ? -1 : 1;
	const cx = node.x + node.pw / 2;
	let r = node.row + step;
	while (b >= 0 && b < bands.length) {
		const { nodes, rows } = bands[b].layout;
		if (r >= 0 && r < rows.length) {
			const best = rows[r].keys.map((k) => nodes.find((n) => n.key === k)).reduce((m, n) => (Math.abs(n.x + n.pw / 2 - cx) < Math.abs(m.x + m.pw / 2 - cx) ? n : m));
			return { shot: bands[b].shot, key: best.key };
		}
		b += step;
		r = b >= 0 && b < bands.length ? (step < 0 ? bands[b].layout.rows.length - 1 : 0) : -1;
	}
	return null;
}

/** a card and what hangs under it (keys, the card first, depth first) */
export function branchOf(layout, key) {
	const byKey = new Map(layout.nodes.map((n) => [n.key, n]));
	const out = [];
	const walk = (k) => { const n = byKey.get(k); if (!n || out.includes(k)) return; out.push(k); n.kids.forEach(walk); };
	walk(key);
	return out;
}

/** X on a card (§3.5): the ids one reject marks – the card and what hangs under it that has a record, without the chosen
 *  and what is rejected already (it keeps its own stamp); [] when the card itself can't be rejected (a source – the
 *  render, a take, a dropped file: "Not rejectable: the chosen … and the sources" –, the chosen, rejected already – that
 *  X unrejects) */
export function rejectIds(layout, key) {
	const byKey = new Map(layout.nodes.map((n) => [n.key, n]));
	const own = byKey.get(key)?.record;
	if (!own || byKey.get(key).kind === 'source' || ['chosen', 'rejected'].includes(statusOf(own))) return [];
	return branchOf(layout, key).map((k) => byKey.get(k).record).filter((r) => r && !['chosen', 'rejected'].includes(statusOf(r))).map((r) => r.id);
}

/** X on a rejected card: what comes back, each to its mark before (`before` – one POST per mark): the card is the root
 *  of its X (its parent wasn't rejected by the same stamp) → everything that X folded in this band; else only the card.
 *  → [{ mark, marked, ids }] ([] when the card isn't rejected) */
export function unrejectIds(layout, key) {
	const byKey = new Map(layout.nodes.map((n) => [n.key, n]));
	const n = byKey.get(key);
	const r = n?.record;
	if (statusOf(r) !== 'rejected') return [];
	const up = byKey.get(n.parent)?.record;
	const root = !(statusOf(up) === 'rejected' && up.marked === r.marked);
	const back = root && r.marked ? layout.nodes.map((x) => x.record).filter((x) => statusOf(x) === 'rejected' && x.marked === r.marked) : [r];
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
