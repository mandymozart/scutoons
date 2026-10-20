// The variants board's state (agents/research/VARIANTS-CANVAS.md §3.1–§3.5, issue #62 stage 2; Board.svelte): the
// picked shots' records (GET /api/generated?ch&ids – one request) and kept layouts (GET /api/variants, per shot – the
// source cards after the render, Q5: A) – again with the chapter, the picked set and studio.version (a save, an
// approval, a finished job, a drop) –, their running jobs (board-sources.svelte.js), the current card (the last clicked
// – its actions show under it, its shot is the current shot) and the selection (Ctrl / ⌘ + click, at most 4, the letters
// A–D – Compare's sides, compare.svelte.js; while Compare is open a side taken out leaves its place empty – compare.js). The status (§3.5, one per card): chosen = approve (the existing flow: askDialog +
// POST /api/generated/approve – approve.mjs: the review + the dataset + `from`; one chosen per shot and kind, replaced)
// · review ↔ none (R) · reject ↔ unreject (X: the card and what hangs under it with ONE `marked` stamp, POST
// /api/generated/mark; X again brings back what that X marked, each to its mark before). The chosen and the sources
// (a dropped file too) can't be rejected, a rejected card can't be chosen or put in review (X first) and stays out of
// the selection; the render, the layout variants and the takes have no status. The view (stage 2b, none of it
// remembered): rejected cards fold to strips (board-layout.js) – a LOOK opens one strip with its branch until the
// current card leaves it or Esc, SHOW REJECTED opens all; a band folds to its head line; one ZOOM for all bands
// (board-zoom.js, board-view.svelte.js).
import { untrack } from 'svelte';
import { askDialog } from '@scutoons/ui';
import { api, ch, fail, loadChapter, notify, pickedShots, studio, toon } from './studio.svelte.js';
import { euro, gen, usdTitle } from './generate.svelte.js';
import { lookKeys, rejectIds, shortId, statusOf, unrejectIds } from './board-layout.js';
import { addSide, resolveSides, takeOut } from './compare.js';

export const board = $state({
	/** GET /api/generated's records per picked shot: { [shot]: record[] } */
	records: {},
	/** GET /api/variants per picked shot – its kept layouts: { [shot]: [{ id, note, created, shot }] } */
	variants: {},
	/** the picked shots' running jobs (board-sources.svelte.js pollJobs): { [shot]: job[] } */
	jobs: {},
	/** the current card { shot, key } – the last clicked (Board falls back to its shot's chosen / the render) */
	current: null,
	/** the selected cards [{ shot, key }] in the order picked – A, B, C, D: Compare's sides (compare.svelte.js); null =
	 *  an empty side (only while Compare is open – closed, the selection closes up) */
	selected: [],
	/** a status change runs (one at a time) */
	busy: false,
	/** a look into a strip (§3.5): { shot, keys } – the cards it unfolds; null = none */
	look: null,
	/** "Show rejected · n": every rejected card unfolded, in every band */
	showRejected: false,
	/** the folded bands { [shot]: true } – a newly picked shot opens unfolded */
	folded: {},
	/** one zoom for all bands (1 = 100 %) */
	zoom: 1,
	/** a video card's moment, clicked while it scrubs (More ▾ "Still: the frame at …"): { key, ms } – null = none */
	pin: null,
	/** Compare is open (§3.4 – the board left, Compare right; C, "Compare · n", ✕ / Esc) – not remembered */
	compare: false
});

let asked = 0;
const variantsOf = (t, c, id) => api('GET', `/api/variants?toon=${t}&ch=${c}&id=${id}`).then((v) => (Array.isArray(v) ? v : []));
/** the picked shots' records (the one shared GET) and the open bands' layout variants, shot by shot – a folded band
 *  loads nothing (§3.2): its variants come when it unfolds (toggleFold) */
export async function loadBoard() {
	const ids = pickedShots();
	const [c, t] = [ch(), toon()];
	const n = ++asked;
	if (!c || !ids.length) return void Object.assign(board, { records: {}, variants: {} });
	const open = ids.filter((id) => !board.folded[id]);
	// the jobs the Generate panel saw end before this GET: their records are in it (jobs.mjs writes a result's line before
	// its status flips) – their placeholders go in the same update, never beside their cards (pollJobs follows later)
	const over = new Set(gen.jobs.filter((j) => j.status !== 'queued' && j.status !== 'running').map((j) => j.id));
	const [list, variants] = await Promise.all([
		api('GET', `/api/generated?ch=${c}&ids=${ids.join(',')}`),
		Promise.all(open.map((id) => variantsOf(t, c, id)))
	]);
	if (n !== asked) return;
	const records = Object.fromEntries(ids.map((id) => [id, []]));
	for (const r of list) records[r.shot]?.push(r);
	board.records = records;
	if (Object.values(board.jobs).flat().some((j) => over.has(j.id))) board.jobs = Object.fromEntries(Object.entries(board.jobs).map(([s, js]) => [s, js.filter((j) => !over.has(j.id))]));
	const got = new Map(open.map((id, i) => [id, variants[i]]));
	board.variants = Object.fromEntries(ids.map((id) => [id, got.get(id) ?? board.variants[id] ?? []]));
}

/** Board: the records follow the chapter, the picked set and studio.version */
export function useBoard() {
	let lastError = '';
	$effect(() => {
		void [studio.chapter, studio.version, pickedShots().join(',')];
		untrack(() => loadBoard().then(() => (lastError = ''), (e) => {
			if (e?.message !== lastError) fail(e);
			lastError = e?.message ?? '';
		}));
	});
	// a shot taken out of the picked set forgets its fold, its look and its cards in the selection (= Compare's sides;
	// picked again: it opens unfolded)
	$effect(() => {
		const ids = new Set(pickedShots());
		untrack(() => {
			if (Object.keys(board.folded).some((id) => !ids.has(id))) board.folded = Object.fromEntries(Object.keys(board.folded).filter((id) => ids.has(id)).map((id) => [id, true]));
			if (board.look && !ids.has(board.look.shot)) board.look = null;
			const gone = board.selected.filter((s) => s && !ids.has(s.shot)).map((s) => s.key);
			if (gone.length) board.selected = takeOut(board.selected, gone, board.compare);
		});
	});
}

/** the current card – a look closes when it leaves the look's branch (§3.5) */
export function setCurrent(at) {
	board.current = at;
	if (board.look && !(at?.shot === board.look.shot && board.look.keys.includes(at.key))) board.look = null;
}
/** a click on a strip: a look into it and its branch – its first card the current one */
export function openLook(shot, layout, key) {
	board.look = { shot, keys: lookKeys(layout, key) };
	board.current = { shot, key };
}
/** a band's fold button / its folded line – unfolded, its layout variants load (folded, it loaded none) */
export function toggleFold(shot) {
	const unfold = !!board.folded[shot];
	board.folded = unfold ? Object.fromEntries(Object.keys(board.folded).filter((id) => id !== shot).map((id) => [id, true])) : { ...board.folded, [shot]: true };
	if (!unfold) return;
	const [c, t] = [ch(), toon()];
	variantsOf(t, c, shot).then((v) => {
		if (c === ch() && !board.folded[shot] && pickedShots().includes(shot)) board.variants = { ...board.variants, [shot]: v };
	}, fail);
}

/** whether a selected card is still on the board – Board says (its layouts know the cards); one gone (a layout variant
 *  taken, a record or a take's render gone) is an empty side, not a letter */
let alive = () => true;
export const setAlive = (fn) => void (alive = fn ?? (() => true));
/** the selection as it stands: its holes and its cards gone as null – Compare open, by place; closed, closed up */
export const selection = () => {
	const sides = resolveSides(board.selected, alive);
	return board.compare ? sides : sides.filter(Boolean);
};

/** Ctrl / ⌘ + click, More ▾: in or out of the selection – into the first empty side, else the end, a fifth replaces the
 *  last (D); out, Compare open: its side empties; a rejected card stays out (§3.5) */
export function toggleSelected(card, node) {
	const now = selection();
	const has = now.some((s) => s?.key === card.key);
	if (!has && statusOf(node?.record) === 'rejected') return notify(`${shortId(card.key)} is rejected – it stays out of Compare (X brings it back).`);
	board.selected = has ? takeOut(now, [card.key], board.compare) : addSide(now, card);
}

export async function guarded(fn) {
	if (board.busy) return;
	board.busy = true;
	try { await fn(); } catch (e) { fail(e); } finally { board.busy = false; }
}
const markIds = (ids, mark, marked) => api('POST', '/api/generated/mark', { ch: ch(), ids, mark, marked });

/** a source card without a record – the render, a layout variant, a take – in words */
export const sourceName = (node) => (node?.source === 'variant' ? node.variant.id : node?.source === 'take' ? `Take ${node.take}` : 'The render');
const noStatus = (node) => notify(`${sourceName(node)} is a source – sources take no status.`);

/** ✓ / Enter: the card becomes its shot's chosen of its kind (an image and a video each have theirs) – approve.mjs */
export async function choose(node) {
	const r = node?.record;
	if (!r) return noStatus(node);
	if (r.missing) return notify(`${shortId(r.id)}: its file is gone – nothing to approve.`, 'warning');
	if (r.approved) return notify(`${shortId(r.id)} is ${r.shot}'s chosen ${r.kind} already.`);
	// rejected cards stay out (§3.5: no Approve) – X brings one back first
	if (statusOf(r) === 'rejected') return notify(`${shortId(r.id)} is rejected – X brings it back first.`);
	const note = await askDialog({
		title: `The chosen ${r.kind}`,
		message: `${r.shot} · ${shortId(r.id)} becomes the shot's chosen ${r.kind} – approves the shot (review + dataset). The chosen ${r.kind} before stays on the board.`,
		label: 'Note (optional)', confirmLabel: 'Approve', skipLabel: 'Approve without note'
	});
	if (note === null) return;
	await guarded(async () => {
		const res = await api('POST', '/api/generated/approve', { toon: toon(), ch: ch(), id: r.shot, file: r.file, note });
		notify(res.out || (res.ok ? 'Approved.' : 'Not approved.'), res.ok ? 'success' : 'error');
		await loadChapter(true);
	});
}

/** ? / R: review ↔ none (the card alone; the chosen keeps its ✓) */
export function review(node) {
	const r = node?.record;
	if (!r) return noStatus(node);
	if (statusOf(r) === 'chosen') return notify(`${shortId(r.id)} is the chosen – approve another first.`);
	// a result wanted back: X on it (§3.5) – R would bring one card of a folded branch back alone
	if (statusOf(r) === 'rejected') return notify(`${shortId(r.id)} is rejected – X brings it back first.`);
	return guarded(async () => {
		await markIds([r.id], statusOf(r) === 'review' ? null : 'review', null);
		await loadBoard();
	});
}

/** ✕ / X: reject the card and its branch (one stamp) – or, rejected, bring back what that X marked */
export function reject(layout, node) {
	const r = node?.record;
	if (!r) return noStatus(node);
	if (statusOf(r) === 'chosen') return notify(`${shortId(r.id)} is the chosen – approve another first.`);
	// a dropped file is a source (§3.2) – "Not rejectable: the chosen … and the sources" (§3.5); one rejected before comes back
	if (node.kind === 'source' && statusOf(r) !== 'rejected') return notify(`${shortId(r.id)} is a source (dropped) – it can't be rejected.`);
	return guarded(async () => {
		if (statusOf(r) === 'rejected') {
			const groups = unrejectIds(layout, node.key);
			for (const g of groups) await markIds(g.ids, g.mark, g.marked);
			const n = groups.reduce((s, g) => s + g.ids.length, 0);
			if (n > 1) notify(`${shortId(r.id)} and ${n - 1} under it are back.`);
		} else {
			const ids = rejectIds(layout, node.key);
			await markIds(ids, 'rejected', new Date().toISOString());
			// rejected cards stay out of Compare (§3.5: "X on side B folds it and empties B" – the others keep their letters)
			if (board.selected.some((s) => s && ids.includes(s.key))) board.selected = takeOut(selection(), ids, board.compare);
			if (ids.length > 1) notify(`${shortId(r.id)} rejected with ${ids.length - 1} under it – X again brings them back.`);
		}
		await loadBoard();
	});
}

/** the status a button / key sets: the pressed one again = none (✓ stays – one chosen per kind, replaced, never cleared) */
export function setStatus(layout, node, status) {
	if (status === 'chosen') return choose(node);
	if (status === 'review') return review(node);
	if (status === 'rejected') return reject(layout, node);
}

// --- what a card says -------------------------------------------------------------------------------------------------
/** a model id → its label from the generators' list ("gen4_turbo" → "Gen-4 Turbo"); the id while that isn't loaded */
export const modelLabel = (id) => gen.generators.flatMap((g) => g.models ?? []).find((m) => m.id === id)?.label ?? id ?? '';
/** a length in ms → "5 s" / "6.2 s" */
export const lengthOf = (ms) => (ms > 0 ? `${Number((ms / 1000).toFixed(1))} s` : '');
const when = (d) => (d ? String(d).slice(0, 16).replace('T', ' ') : '');
const base = (key) => String(key ?? '').replace(/#.*$/, '').split('/').pop();
const STATUS = { chosen: 'the chosen', review: 'in review', rejected: 'rejected' };
/** a node key as words: "render:sb-25-2" → "the render", "take:sb-25-2:t3" → "take t3", "sb-25-2--g5" → "g5" */
export const under = (k) => (k.startsWith('render:') ? 'the render' : k.startsWith('take:') ? `take ${k.split(':')[2]}` : k.startsWith('variant:') ? k.split(':')[2] : shortId(k));

/** a card's caption: [the id, the muted line]; media = the card's picture ({ pass, ms } for a take) */
export function captionOf(node, media) {
	const r = node.record;
	if (node.source === 'render') return ['render', 'no texts'];
	if (node.source === 'variant') return [node.variant.id, node.variant.note || 'no note'];
	if (node.source === 'take') return [`take ${node.take}`, media?.pass ?? 'render'];
	if (node.source === 'upload') return [shortId(r.id), 'dropped'];
	return [shortId(r.id), r.earlier ? 'no record' : modelLabel(r.model) || r.generator || ''];
}

/** a strip's tooltip: its cards (the first's record), when they were rejected, what a click does */
export function stripTitle(strip, shot, tree) {
	const r = strip.record;
	const more = strip.keys.slice(1).map((k) => tree?.get(k)?.record ?? { id: k });
	const head = [`${shot} · ${shortId(r.id)}`, modelLabel(r.model) || r.generator, r.kind, r.cost?.usd != null && euro(r.cost.usd)].filter(Boolean).join(' · ');
	const rest = more.length ? ` – with ${more.length} more folded in (${more.map((x) => [shortId(x.id), modelLabel(x.model)].filter(Boolean).join(' ')).join(', ')})` : '';
	return `${head} – rejected ${when(r.marked)}${rest}${strip.lie ? ': the row lies down as one line' : ''}.
Click / Enter: unfold it for a look (X there: back) · X: back, as that X folded it`;
}

/** a card's tooltip: the record – generator, model, length, seed, cost, date; parents, inputs, prompt; the status */
export function titleOf(node, shot, media) {
	const r = node.record;
	if (node.source === 'render') return `${shot} · the render without texts – what the generators get (the saved frame has the texts)`;
	if (node.source === 'variant') return `${shot} · ${node.variant.id}${node.variant.note ? ` · ${node.variant.note}` : ''} – a kept layout (Keep as variant${node.variant.created ? ` ${when(node.variant.created)}` : ''}): its saved frame
Load: into the Viewport (unsaved) · Take this: it becomes the shot, the shot a variant`;
	if (node.source === 'take') return `${shot} · take ${node.take} · its handheld render (${media?.pass ?? '?'})${media?.ms ? ` · ${lengthOf(media.ms)}` : ''}`;
	if (r.earlier) return `${shot} · ${shortId(r.id)} · ${r.file} – no record (made before the record was kept – nothing guessed)${r.mark ? ` · ${STATUS[r.mark]}` : ''}${r.approved ? ' · the chosen' : ''}`;
	const head = [`${shot} · ${shortId(r.id)}`, r.generator === 'upload' ? 'dropped' : r.generator, modelLabel(r.model), r.kind, lengthOf(r.ms ?? (r.duration ? r.duration * 1000 : 0)),
		r.seed != null && `seed ${r.seed}`, r.cost?.usd != null && `${euro(r.cost.usd)} (${usdTitle(r.cost)})`, when(r.date)].filter(Boolean).join(' · ');
	const status = statusOf(r);
	return [
		head,
		r.parents?.length ? `from ${r.parents.map(shortId).join(' + ')}${node.parent && (r.parents.length > 1 || node.parent !== r.parents[0]) ? ` (hangs under ${under(node.parent)} – its main input)` : ''}` : '',
		r.inputs?.length ? `inputs: ${r.inputs.map((i) => `${i.role} ${i.key ? base(i.key) : i.name ?? '?'}${i.frame != null ? ` (${i.frame})` : ''}`).join(', ')}` : '',
		r.prompt ? `prompt: ${r.prompt.length > 200 ? `${r.prompt.slice(0, 200)} …` : r.prompt}` : '',
		status ? `${STATUS[status]}${r.marked && status !== 'chosen' ? ` ${when(r.marked)}` : ''}` : '',
		r.missing ? 'the file is gone (moved or deleted by hand)' : ''
	].filter(Boolean).join('\n');
}

/** a running job's placeholder: its caption, its estimate (euro – free for a free generator) and its tooltip */
export function runningOf(node, shot) {
	const j = node.job;
	const model = modelLabel(j.model) || j.generator || 'a job';
	const pct = j.progress != null ? ` ${Math.round(j.progress * 100)} %` : '';
	const cost = j.estimate?.usd > 0 ? `≈ ${euro(j.estimate.usd)}` : j.estimate ? 'free' : '';
	return {
		caption: [model, j.status],
		cost,
		title: [`${shot} · ${model} · ${j.status}${pct}${cost ? ` · ${cost}` : ''}${j.estimate?.usd > 0 ? ` (${usdTitle(j.estimate)})` : ''}`,
			`from ${under(node.parent ?? `render:${shot}`)} – its result lands here`, j.message || '', 'Cancel: the Generate panel\'s Jobs list'].filter(Boolean).join('\n')
	};
}
