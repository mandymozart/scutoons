// Compare's actions and words (agents/research/VARIANTS-CANVAS.md §3.4, issue #62 stage 4): the board's selection IS
// Compare's sides (board.selected – A–D, the letters on the cards), `board.compare` whether the split is open. The rules
// (pure, node tests): compare.js. Opening (C, "Compare · n"): the selection – fewer than two: the current shot's chosen
// in front as A (the reference), with nothing selected the current card beside it; the selection becomes that, so the
// board's letters and Compare's agree. A click on a card while it is open → the last side (Board); Ctrl + click and
// More ▾ add / remove as before (toggleSelected – at most 4); a rejected card never enters (X on a side folds it on the
// board and empties that side – board.svelte.js reject); while it is open a side taken out stays empty (its letter
// free, the next click fills it – compare.js), closed, the selection closes up. The pane, its sides and the clock:
// Compare.svelte, CompareSide.svelte, compare-clock.svelte.js.
import { resizable } from '@scutoons/ui';
import { shortId, statusOf } from './board-layout.js';
import { remPx } from './board-view.svelte.js';
import { board, lengthOf, modelLabel, selection, under } from './board.svelte.js';
import { clickSide, fromText, sidesOnOpen } from './compare.js';
import { euro } from './generate.svelte.js';
import { notify } from './studio.svelte.js';

const kindOfNode = (node) => node?.record?.kind ?? (node?.source === 'take' ? 'video' : node ? 'image' : null);

/**
 * C / "Compare · n": open with the selection (completed as compare.js sidesOnOpen says) – or close. `shot` = the current
 * shot, `records` = its records, `current` = the current card { shot, key }, `nodeOf({ shot, key })` = a card's node.
 */
export function toggleCompare(at) {
	if (board.compare) return closeCompare();
	const node = at.current ? at.nodeOf(at.current) : null;
	const current = node ? { ...at.current, kind: kindOfNode(node), rejected: statusOf(node.record) === 'rejected' } : null;
	const now = selection();
	const { sides, why } = sidesOnOpen({ selected: now, shot: at.shot, records: at.records, current, kindOf: (c) => kindOfNode(at.nodeOf(c)) });
	if (!sides.length) return notify(why);
	if (sides.length !== board.selected.length || sides.some((s, i) => s.key !== board.selected[i]?.key)) board.selected = sides;
	board.compare = true;
	return true;
}
/** ✕, Esc, C: Compare closes – its empty sides and the cards gone close up (the letters on the board follow) */
export function closeCompare() {
	const now = selection().filter(Boolean);
	board.compare = false;
	if (now.length !== board.selected.length) board.selected = now;
}

/** a plain click on a card while Compare is open: it fills the first empty side, else it comes in as the last – A
 *  stays the reference */
export function compareClick(card, node) {
	const rejected = statusOf(node?.record) === 'rejected';
	const now = selection();
	const { selected, why } = clickSide(now, card, rejected);
	if (why === 'rejected') return notify(`${shortId(card.key)} is rejected – it stays out of Compare (X brings it back).`);
	if (selected !== now) board.selected = selected;
}

/** what a side says under its picture: { parts: model · length · seed · cost ([{ text, seed? }] – a narrow side leaves
 *  the seed out), line: them as one line (the tooltip), from: what it came from, prompt } */
export function sideFacts(node, media) {
	const r = node.record;
	const len = lengthOf(media?.ms ?? 0);
	const of = (parts, from = '', prompt = '') => {
		const list = parts.filter((p) => p?.text);
		return { parts: list, line: list.map((p) => p.text).join(' · '), from, prompt };
	};
	const t = (text) => ({ text });
	if (node.source === 'render') return of([t('the render without texts')], 'what the generators get');
	if (node.source === 'variant') return of([t(`${node.variant.id} – a kept layout`)], node.variant.note || 'no note');
	if (node.source === 'take') return of([t(`take ${node.take}`), t(media?.pass ?? 'render'), t(len)], 'its handheld render');
	if (!r) return of([]);
	if (r.earlier) return of([t('no record')], r.file ?? '');
	const cost = r.cost?.usd > 0 ? euro(r.cost.usd) : r.cost ? 'free' : '';
	// the model's name without its mode / its note ("Gen-4 Turbo – image to video" → "Gen-4 Turbo", "Qwen-Image-Edit
	// (comfy.mjs …)" → "Qwen-Image-Edit"; the whole is in the card's tooltip)
	const model = r.generator === 'upload' ? 'dropped' : modelLabel(r.model).split(/ – | \(/)[0] || r.generator;
	const others = (r.parents ?? []).filter((p) => p !== node.parent && p !== r.id).map(shortId);
	return of([t(model), t(len), { text: r.seed != null ? `seed ${r.seed}` : '', seed: true }, t(cost)], fromText(r, node.parent ? under(node.parent) : '', others), r.prompt ?? '');
}

// --- the split (Board) ----------------------------------------------------------------------------------------------------
/** Compare's width: normally SHARE of the room, at least NORMAL rem (the mockup's at 1440), the bands keep BOARD_MIN;
 *  dragged (the Divider), it is remembered in rem (@scutoons/ui resizable – `storyboard-studio:compare`, DOCKING) */
const [NORMAL, SHARE, MIN, BOARD_MIN] = [27, 0.45, 18, 18];
/** `el()` = the split element (the bands + Compare) → { size: the Divider's, width(): Compare's CSS width } */
export function compareSplit(el) {
	const room = () => el()?.clientWidth ?? innerWidth;
	const size = resizable({ storageKey: 'storyboard-studio:compare', edge: 'left', anchor: () => el()?.getBoundingClientRect().right ?? innerWidth, min: () => MIN * remPx(), normal: () => Math.max(NORMAL * remPx(), SHARE * room()), max: () => Math.max(MIN * remPx(), room() - BOARD_MIN * remPx()) });
	size.restore();
	return { size, width: () => `min(${size.width != null ? `${(size.width / remPx()).toFixed(2)}rem` : `max(${NORMAL}rem, ${SHARE * 100}%)`}, calc(100% - ${BOARD_MIN}rem))` };
}
