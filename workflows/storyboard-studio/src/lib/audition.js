// The audition – the pure part (agents/research/VARIANTS-CANVAS.md §3.1, Q3 – owner 2026-10-20: "The current card";
// issue #62 stage 4; test/audition.test.mjs). While the board is open, the CURRENT shot's Generated clip on the timeline
// is the board's CURRENT card (the last clicked – none clicked: the card the board shows as current, its shot's chosen,
// the video first – currentCard; tagged "audition" on the clip); Play shows it among the picked shots' clips (Play
// switches the main to the Viewport – the audition holds while that Play runs; V goes back to the board). A rejected
// card, a source (the render, a layout variant, a take, a dropped file), a running job, a card whose file is gone or
// another shot's card → no pick: the clip is the chosen, as without the board (timeline-lanes.js clipOf – a rejected
// video is never its fallback either). Play's end, the board closed by hand (V, the switch, a source's Load) or another
// step: the chosen again. The timeline's Generated clips carry status bullets (✓ the chosen, ? review, ✕ rejected): GET
// /api/clips brings the log's marks; the board's own count instead when it read the log after the clips did (boardMarks
// – a board closed earlier keeps its records, a mark set since in another window or by hand comes with the clips).
// Images stay stills on the clips lane (STILL_MS) – showing the image.
import { statusOf } from './board-layout.js';

/** a record that is a source on the board: a dropped file (board-layout.js bandLayout – an upload without parents) */
const isDrop = (r) => r.generator === 'upload' && !r.parents?.length && !r.earlier;

/** a card key that is no record (the render, a layout variant, a take, a running job: "render:…", "take:…", "job:…") */
const isSourceKey = (key) => String(key ?? '').includes(':');

/**
 * The board's current card in `shot` – the last clicked there while it is still on the board (`has(key)`), else the
 * shot's chosen (the video first, else the image) – as Board shows it (its actions under it). → { shot, key } | null
 * (null: none – Board falls back to the band's first card, the render). `has` defaults to: a source key, or a record.
 */
export function currentCard({ shot, current, records = [], has = null }) {
	if (!shot) return null;
	const list = records ?? [];
	const known = has ?? ((key) => isSourceKey(key) || list.some((r) => r?.id === key));
	if (current?.shot === shot && known(current.key)) return current;
	const chosen = list.find((r) => r?.approved && r.kind === 'video') ?? list.find((r) => r?.approved);
	return chosen ? { shot, key: chosen.id } : null;
}

/**
 * The current shot's audition. `shot` = the current shot, `current` = the board's current card { shot, key } (null =
 * none clicked yet), `records` = the shot's records (GET /api/generated). → { id, file, url, video, ms, status } | null
 * (null: the clip stays the chosen)
 */
export function auditionPick({ shot, current, records = [] }) {
	if (!shot || current?.shot !== shot) return null;
	const r = (records ?? []).find((x) => x?.id === current.key);
	if (!r?.file || !r.url || r.missing || isDrop(r) || statusOf(r) === 'rejected') return null;
	const video = r.kind === 'video';
	const ms = video ? (r.ms > 0 ? r.ms : r.duration > 0 ? r.duration * 1000 : null) : null;
	return { id: r.id, file: r.file, url: r.url, video, ms, status: statusOf(r) };
}

/** the marks the board knows, by file: { [file]: 'review' | 'rejected' } – the chosen's ✓ comes from GET /api/clips */
export function marksOf(records = []) {
	const marks = {};
	for (const r of records ?? []) if (r?.file && !r.approved && (r.mark === 'review' || r.mark === 'rejected')) marks[r.file] = r.mark;
	return marks;
}

/** the board's marks for the clips lane – only when the board read the log after the clips did (`recordsAt` /
 *  `clipsAt`: when each GET was asked, performance.now()); else null: the clips' own marks (the log's) count */
export const boardMarks = (records, recordsAt, clipsAt) => (recordsAt >= clipsAt ? marksOf(records) : null);

/**
 * Whether the audition holds now – and whether Play carries it: while the board is open (`open`); Play's switch from
 * the board to the Viewport (`byPlay`, it held then – `was`) carries it while that Play runs (`playing`; `carried` – it
 * did so far); Play's end (pause, the sequence's end), the board closed by hand or another step (`stepped`) end it.
 * → { on, carried }
 */
export function holds({ open, was = false, byPlay = false, carried = false, playing = false, stepped = false }) {
	if (open) return { on: true, carried: false };
	const keep = !stepped && !!playing && (!!carried || (!!was && !!byPlay));
	return { on: keep, carried: keep };
}
