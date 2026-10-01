import type { Checkpoint } from './content.ts';
import { Status } from './enums.ts';
import { isItemPublic } from './template.ts';

/**
 * Reading progress → the toon page's start button (agents/DESIGN.md "Start button"). The progress itself is the viewer's
 * game state per toon (`Checkpoint[]`, localStorage + `/api/progress/<toon>` for members); a chapter's checkpoint is
 * written while its slices are read (position) and when its last slice is reached (finished).
 */

/**
 * Chapter 0 of a season is its prologue (owner 2026-10-01: the music video "Roar Little Kitty" before chapter 1): a normal
 * chapter in the sequence, listed as "Prolog" / "Prologue", offered by the start button until it was watched, and never
 * the season's first chapter for "read again".
 */
export const PROLOGUE = 0;

export function isPrologue(chapter: number | null | undefined): boolean {
	return chapter === PROLOGUE;
}

/** The checkpoint uri of a chapter (chapter numbers are unique per toon) */
export function chapterCheckpointUri(toon: string, chapter: number): string {
	return `${toon}/chapter/${chapter}`;
}

/** Where a checkpoint's current read-through stands, 0..1 (older checkpoints without `position`: finished = 1, else 0) */
export function checkpointPosition(checkpoint: Checkpoint): number {
	const p = checkpoint.position;
	if (typeof p === 'number' && Number.isFinite(p)) return Math.min(1, Math.max(0, p));
	return checkpoint.isComplete ? 1 : 0;
}

/**
 * One chapter's progress for the chapter list: `position` of the current read-through (0..1) and whether it was ever read
 * to its end; null = never opened.
 */
export function chapterProgress(
	toon: string,
	chapter: number,
	checkpoints: Checkpoint[] | null | undefined
): { position: number; finished: boolean } | null {
	const uri = chapterCheckpointUri(toon, chapter);
	const cp = (Array.isArray(checkpoints) ? checkpoints : []).find((c) => c?.uri === uri);
	if (!cp) return null;
	return { position: checkpointPosition(cp), finished: !!cp.isComplete };
}

/**
 * The button's state:
 * - `prologue` nothing of the chapters read yet and the season's prologue (chapter 0) not watched to its end → chapter 0
 * - `start`    nothing of these chapters read yet → the first readable chapter (never the prologue)
 * - `continue` the last opened chapter is not read to its end → that chapter, `position` = how far (0..1)
 * - `next`     the last opened chapter was read to its end → the next readable chapter; after the last one, the first
 *              readable chapter that was never finished
 * - `again`    every readable chapter was finished (and the last opened one too) → the first readable chapter
 */
export type ReadingState =
	| { kind: 'prologue'; chapter: number }
	| { kind: 'start'; chapter: number }
	| { kind: 'continue'; chapter: number; position: number }
	| { kind: 'next'; chapter: number }
	| { kind: 'again'; chapter: number };

/**
 * @param readable the season's readable chapter numbers in reading order (published; "coming soon" / drafts never); a
 *   prologue (chapter 0) among them is only ever offered first (`prologue`), the rules below apply to the others
 * @param checkpoints the toon's checkpoints (any order; others than these chapters are ignored)
 */
export function readingState(toon: string, readable: number[], checkpoints: Checkpoint[] | null | undefined): ReadingState | null {
	const chapters = readable.filter((n) => !isPrologue(n));
	if (chapters.length === readable.length) return chaptersState(toon, chapters, checkpoints);
	const state = chaptersState(toon, chapters, checkpoints);
	if (state && state.kind !== 'start') return state;
	// nothing of the chapters read: the prologue until it was watched to its end (or when it is all there is)
	if (!state || !chapterProgress(toon, PROLOGUE, checkpoints)?.finished) return { kind: 'prologue', chapter: PROLOGUE };
	return state;
}

function chaptersState(toon: string, readable: number[], checkpoints: Checkpoint[] | null | undefined): ReadingState | null {
	if (!readable.length) return null;
	const byChapter = new Map<number, Checkpoint>();
	for (const cp of Array.isArray(checkpoints) ? checkpoints : []) {
		const n = readable.find((c) => chapterCheckpointUri(toon, c) === cp?.uri);
		if (n !== undefined) byChapter.set(n, cp);
	}
	// the last opened chapter = the checkpoint written last
	let last: number | null = null;
	for (const [n, cp] of byChapter) {
		if (last === null || time(cp.updated_at) > time(byChapter.get(last)!.updated_at)) last = n;
	}
	if (last === null) return { kind: 'start', chapter: readable[0] };
	const position = checkpointPosition(byChapter.get(last)!);
	if (position < 1) return { kind: 'continue', chapter: last, position };
	const after = readable[readable.indexOf(last) + 1];
	if (after !== undefined) return { kind: 'next', chapter: after };
	const unfinished = readable.find((n) => !byChapter.get(n)?.isComplete);
	if (unfinished !== undefined) return { kind: 'next', chapter: unfinished };
	return { kind: 'again', chapter: readable[0] };
}

/**
 * The checkpoint after reading up to `position` (0..1) of a chapter; `fresh` = the first update of this visit (a chapter
 * read to its end and opened again starts a new read-through). Returns null when nothing changes.
 */
export function advanceCheckpoint(current: Checkpoint | undefined, uri: string, position: number, fresh: boolean, now: string): Checkpoint | null {
	const p = Math.min(1, Math.max(0, position));
	if (!current) return { uri, isComplete: p >= 1, position: p, created_at: now, updated_at: now };
	const before = checkpointPosition(current);
	const next = fresh && before >= 1 ? p : Math.max(before, p);
	// the first update of a visit always touches `updated_at`: this chapter is now the last opened one
	if (!fresh && next === before && (current.isComplete || next < 1)) return null;
	return { ...current, isComplete: current.isComplete || next >= 1, position: next, updated_at: now };
}

function time(value: unknown): number {
	return typeof value === 'number' ? value : Date.parse(String(value ?? '')) || 0;
}

/**
 * What follows the end of the shown group (agents/DESIGN.md "Follow-up bar"):
 * - `group`       more groups follow in this chapter → the next one (`?label=`)
 * - `chapter`     the chapter's end, the next chapter is readable (published; drafts in dev) and the reader may read it
 *                 (a preview chapter, or a member)
 * - `members`     the next chapter is readable but needs a membership the reader lacks
 * - `coming-soon` the next chapter is coming soon (`release` when known), or a draft (not public here, so not announced:
 *                 no date) – the bar never skips a chapter number
 * - `season-end`  nothing follows in the season
 */
export type NextStep =
	| { kind: 'group'; label: string }
	| { kind: 'chapter'; chapter: number }
	| { kind: 'members'; chapter: number }
	| { kind: 'coming-soon'; chapter: number; release?: string }
	| { kind: 'season-end' };

export interface NextStepInput {
	/** the chapter's groups in order (a group is addressed by its `label`) */
	groups: { label?: string | null }[];
	/** index of the shown group */
	group: number;
	/** the shown chapter's number */
	chapter: number;
	/**
	 * the season's chapters in reading order with the season's status per locale – the whole `Season.sequence`, drafts
	 * included (they decide what comes next even where they aren't readable); `isPreview` / `release` from the loaded list
	 */
	sequence: { number: number; status?: string; isPreview?: boolean; release?: string | null }[];
	/** drafts are readable (dev) */
	includeDrafts?: boolean;
	/** the reader has a membership (device code or account) */
	member: boolean;
}

export function nextStep({ groups, group, chapter, sequence, includeDrafts = false, member }: NextStepInput): NextStep {
	// a following group without a label can't be addressed (the chapter page shows `?label=` or the first group)
	const following = groups[group + 1];
	if (following?.label) return { kind: 'group', label: following.label };
	const at = sequence.findIndex((item) => Number(item.number) === chapter);
	const next = at >= 0 ? sequence[at + 1] : sequence.find((item) => Number(item.number) > chapter);
	if (!next) return { kind: 'season-end' };
	const number = Number(next.number);
	if (next.status === Status.COMING_SOON) return { kind: 'coming-soon', chapter: number, ...(next.release ? { release: next.release } : {}) };
	if (!isItemPublic(next, includeDrafts)) return { kind: 'coming-soon', chapter: number };
	return next.isPreview || member ? { kind: 'chapter', chapter: number } : { kind: 'members', chapter: number };
}
