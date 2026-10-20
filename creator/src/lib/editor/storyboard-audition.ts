// The audition – the pure part (#125; the port of #62's audition.js – agents/research/VARIANTS-CANVAS.md §3.1, Q3 –
// owner 2026-10-20: "The current card"). While the board is open, the CURRENT shot's Generated clip is the board's
// CURRENT card (the last clicked – none clicked: its shot's chosen, the video first – currentCard); a rejected card, a
// source (the render, a take, a dropped file), a running job or another shot's card → no pick: the clip is the chosen.
// The board's state publishes it (StoryboardPage.board.audition) – the clip timeline (#126) plays it; marksOf gives the
// timeline's Generated clips their status bullets; holds() says whether Play from the board carries it.
// Tested in test/storyboard-audition.test.ts.
import { idOfKey, isDrop, statusOf, type BoardRecord, type CardAt } from './storyboard-board-layout.ts';

/**
 * The board's current card in `shot` – the last clicked there while it is still on the board (`has(key)`), else the
 * shot's chosen (the video first, else the image). → { shot, key } | null (none: the board falls back to the render).
 * `has` defaults to: a source key, or a record of the shot.
 */
export function currentCard({ shot, current, records = [], has = null }: { shot: string | null; current: CardAt | null; records?: readonly BoardRecord[]; has?: ((key: string) => boolean) | null }): CardAt | null {
	if (!shot) return null;
	const known = has ?? ((key: string) => idOfKey(key) === null || records.some((r) => String(r.id) === key));
	if (current?.shot === shot && known(current.key)) return current;
	const done = records.filter((r) => r.shot === shot && r.status === 'succeeded');
	const chosen = done.find((r) => r.approved && r.kind === 'video') ?? done.find((r) => r.approved);
	return chosen ? { shot, key: String(chosen.id) } : null;
}

export interface Audition {
	id: number;
	video: boolean;
	status: ReturnType<typeof statusOf>;
}

/** The current shot's audition: the board's current card when it can play (a finished result of the shot, not
 *  rejected, not a drop) → { id, video, status } | null (the clip stays the chosen) */
export function auditionPick({ shot, current, records = [] }: { shot: string | null; current: CardAt | null; records?: readonly BoardRecord[] }): Audition | null {
	if (!shot || current?.shot !== shot) return null;
	const id = idOfKey(current.key);
	const r = id === null ? undefined : records.find((x) => x.id === id && x.shot === shot);
	if (!r || r.status !== 'succeeded' || isDrop(r) || statusOf(r) === 'rejected') return null;
	return { id: r.id, video: r.kind === 'video', status: statusOf(r) };
}

/** the marks by generation id: { [id]: 'review' | 'rejected' } – the chosen's ✓ is its `approved` */
export function marksOf(records: readonly BoardRecord[] | null | undefined = []): Record<number, 'review' | 'rejected'> {
	const marks: Record<number, 'review' | 'rejected'> = {};
	for (const r of records ?? []) if (!r.approved && (r.mark === 'review' || r.mark === 'rejected')) marks[r.id] = r.mark;
	return marks;
}

/**
 * Whether the audition holds now – and whether Play carries it: while the board is open (`open`); Play's switch from
 * the board to the Viewport (`byPlay`, it held then – `was`) carries it while that Play runs (`playing`; `carried`);
 * Play's end, the board closed by hand or another step (`stepped`) end it. → { on, carried }
 */
export function holds({ open, was = false, byPlay = false, carried = false, playing = false, stepped = false }: { open: boolean; was?: boolean; byPlay?: boolean; carried?: boolean; playing?: boolean; stepped?: boolean }): { on: boolean; carried: boolean } {
	if (open) return { on: true, carried: false };
	const keep = !stepped && !!playing && (!!carried || (!!was && !!byPlay));
	return { on: keep, carried: keep };
}
