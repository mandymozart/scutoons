// Season addresses and labels (BACKLOG 10.41, owner 2026-10-07). `/<toon>/chapter/N` stays THE address of a chapter
// (printed QR codes); `/<toon>/season/S/chapter/K` resolves to it: K = the K-th entry with number ≠ 0 in season S's
// `sequence`, in sequence order, counting EVERY stored entry (drafts and coming-soon too – a position never shifts when
// a status changes). Chapter 0 is the prologue ("Prolog" / "Prologue"): never counted. The creator's validator freezes
// the positions of listed chapters once published (`frozenSequenceProblems`), so a printed season address stays true.

import { isPrologue } from './progress.ts';

/** what the helpers need of a season: its `sequence` (stored order; `number` may come as a string from older data) */
export type PositionedSequence = readonly ({ number: number | string } | null | undefined)[] | null | undefined;

/** A season with its number and its stored sequence (the viewer: the toon bundle's seasons) */
export interface SeasonPlaces {
	number: number;
	sequence: PositionedSequence;
}

/** where a chapter sits: its season and its position there (null: the prologue, chapter 0 – not counted) */
export interface ChapterPlace {
	season: number;
	position: number | null;
}

const asNumber = (item: { number: number | string } | null | undefined): number | null => {
	const n = Number(item?.number);
	return item && String(item.number).trim() !== '' && Number.isInteger(n) ? n : null;
};

/** The chapters a season counts, in position order: index 0 = position 1 (every entry with a number ≠ 0) */
export function countedChapters(sequence: PositionedSequence): number[] {
	const out: number[] = [];
	for (const item of sequence ?? []) {
		const n = asNumber(item);
		if (n !== null && !isPrologue(n)) out.push(n);
	}
	return out;
}

/** The chapter number at `position` (1-based) of a season, or null (no such position) */
export function chapterAtPosition(sequence: PositionedSequence, position: number): number | null {
	if (!Number.isInteger(position) || position < 1) return null;
	return countedChapters(sequence)[position - 1] ?? null;
}

/** The position (1-based) of chapter `number` in a season, or null (not in it, or the prologue) */
export function chapterPosition(sequence: PositionedSequence, number: number): number | null {
	if (isPrologue(number)) return null;
	const at = countedChapters(sequence).indexOf(Number(number));
	return at < 0 ? null : at + 1;
}

/** Season + position of chapter `number`: the first season (by number) whose sequence lists it, or null */
export function chapterPlace(seasons: readonly SeasonPlaces[], number: number): ChapterPlace | null {
	const n = Number(number);
	for (const season of [...seasons].sort((a, b) => a.number - b.number)) {
		if (!(season.sequence ?? []).some((item) => asNumber(item) === n)) continue;
		return { season: season.number, position: chapterPosition(season.sequence, n) };
	}
	return null;
}

/** The chapter number of `/season/S/chapter/K`, or null (unknown season or position) */
export function chapterAt(seasons: readonly SeasonPlaces[], season: number, position: number): number | null {
	const found = seasons.find((s) => s.number === season);
	return found ? chapterAtPosition(found.sequence, position) : null;
}

/**
 * What a reader calls a chapter: the prologue (chapter 0), or "Season S · Chapter K" – `season` only when the toon has
 * two or more seasons (`seasons` = the ones the reader sees), else null ("Chapter K"). K is the position in its season;
 * a chapter no season lists keeps its number. Never the absolute number next to it.
 */
export function chapterLabel(seasons: readonly SeasonPlaces[], number: number): { prologue: boolean; season: number | null; chapter: number } {
	const n = Number(number);
	if (isPrologue(n)) return { prologue: true, season: null, chapter: 0 };
	const place = chapterPlace(seasons, n);
	return { prologue: false, season: place && seasons.length >= 2 ? place.season : null, chapter: place?.position ?? n };
}

/** a sequence entry's status per locale (a string = every locale) */
type EntryStatus = string | Record<string, unknown> | null | undefined;

/** Listed for readers: published (or no status) or coming-soon in any locale – a draft everywhere is not */
export function listedSomewhere(status: EntryStatus): boolean {
	if (status === null || status === undefined) return true;
	if (typeof status === 'string') return status !== 'draft';
	const values = Object.values(status);
	return values.length === 0 || values.some((v) => v !== 'draft');
}

export interface FrozenProblem {
	/** the entry in the edited sequence (`['sequence', i]`), or `['sequence']` for a removed chapter */
	path: (string | number)[];
	message: string;
}

/**
 * Positions frozen (BACKLOG 10.41, option A): compared with the last PUBLISHED version of a season, every chapter that
 * was listed there (published or coming-soon in any locale) keeps its position – not moved, not removed, nothing
 * inserted before it. Appending after the last entry is always fine; drafts may move as long as no listed chapter
 * shifts. The prologue (chapter 0) has no position and is free. `season` names the season in the messages.
 */
export function frozenSequenceProblems(
	published: readonly ({ number: number | string; status?: EntryStatus } | null | undefined)[] | null | undefined,
	edited: readonly ({ number: number | string } | null | undefined)[] | null | undefined,
	season: number | string = ''
): FrozenProblem[] {
	if (!Array.isArray(published) || !Array.isArray(edited)) return [];
	const out: FrozenProblem[] = [];
	const before = countedChapters(published);
	const after = countedChapters(edited);
	const at = season === '' ? 'season' : `season ${season}`;
	for (const item of published) {
		const n = asNumber(item);
		if (n === null || isPrologue(n) || !listedSomewhere(item?.status)) continue;
		const was = before.indexOf(n) + 1;
		const now = after.indexOf(n) + 1;
		if (now === was) continue;
		if (now === 0) {
			out.push({ path: ['sequence'], message: `chapter ${n} (${at}, position ${was}) is published and was removed – a listed chapter keeps its place (printed /season/…/chapter/${was} addresses); set it to draft instead` });
			continue;
		}
		const index = edited.findIndex((e) => asNumber(e) === n);
		out.push({
			path: ['sequence', index],
			message: `chapter ${n} moved from position ${was} to ${now} of the ${at} – a published / coming-soon chapter keeps its position (printed /season/…/chapter/${was} addresses); add new chapters at the end`
		});
	}
	return out;
}
