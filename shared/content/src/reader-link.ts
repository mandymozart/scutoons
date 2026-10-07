/**
 * The creator's dock Reader (BACKLOG 10.49, UI-CONCEPTS E.6, agents/DOCKING.md "The Reader panel"): the creator frames the
 * viewer's draft preview and the two talk by `postMessage` – only these two messages, each checked on arrival:
 *
 * - viewer → creator `{ type: 'scutoons:reader-ready' }`: the framed viewer has mounted with its draft preview on; sent
 *   with the creator's origin as the target (another page framing the viewer never gets it).
 * - creator → viewer `{ type: 'scutoons:reader-show', toon, chapter, group, slice }`: scroll to that group / slice of
 *   that chapter (group / slice = indexes; slice -1 = the group's start). Sent with the viewer's origin as the target.
 *
 * Each side accepts a message only from the other's configured origin AND from the window it expects (the viewer: its
 * `window.parent`; the creator: its frame's `contentWindow`); the viewer only in its draft preview, only when framed.
 * Neither message carries a session or any other secret: the frame signs in with the shared session cookie.
 */
export const READER_READY = 'scutoons:reader-ready';
export const READER_SHOW = 'scutoons:reader-show';

export interface ReaderShow {
	toon: string;
	chapter: number;
	/** the group's index in the chapter */
	group: number;
	/** the slice's index in the group; -1 = the group's start */
	slice: number;
}

const index = (value: unknown, min: number): value is number => Number.isInteger(value) && (value as number) >= min && (value as number) < 100_000;

/** a well-formed "show" message's place, or null (anything else: ignored) */
export function parseReaderShow(data: unknown): ReaderShow | null {
	if (!data || typeof data !== 'object') return null;
	const d = data as Record<string, unknown>;
	if (d.type !== READER_SHOW) return null;
	if (typeof d.toon !== 'string' || !/^[a-z0-9][a-z0-9_-]{0,63}$/i.test(d.toon)) return null;
	if (!index(d.chapter, 0) || !index(d.group, 0) || !index(d.slice, -1)) return null;
	return { toon: d.toon, chapter: d.chapter, group: d.group, slice: d.slice };
}

/** the "show" message for a place */
export const readerShowMessage = (place: ReaderShow) => ({ type: READER_SHOW, ...place });

/** a "ready" message */
export const isReaderReady = (data: unknown): boolean => !!data && typeof data === 'object' && (data as { type?: unknown }).type === READER_READY;
