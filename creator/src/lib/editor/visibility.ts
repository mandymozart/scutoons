// Chapter visibility has ONE source: the season's sequence status, per locale (season 1 → `sequence[].status`:
// published | draft | coming-soon; a toon may release de before en). The season is one language-neutral document
// (agents/NEUTRAL.md): an entry's `status` is a locale map `{ de: 'published', en: 'draft' }` (a locale without one
// shows the source's). The chapter document has no status of its own any more (the library computes the deprecated
// mirror from the season); an older per-locale chapter's `status` / `published` are dropped on save.
// A chapter always belongs to a season (owner rule): new chapters are created inside it, a new season adopts existing
// chapter files, and a chapter without a season entry is a validation error.
import { resolveText, Status, type SequenceItem } from '@scutoons/content';
import type { ContentDoc } from '$lib/editor/doc.svelte.ts';

export const STATUSES = Object.values(Status) as Status[];

/** the season's sequence (the neutral document's, or a per-locale document's in `locale`) */
function sequenceOf(season: ContentDoc | null | undefined, locale: string): SequenceItem[] {
	const sequence = season?.data?.[season.neutral ? season.primary : locale]?.sequence;
	return Array.isArray(sequence) ? sequence : [];
}

/** Index of chapter `number` in the season sequence (of `locale` for a per-locale season), -1 when missing (or no season file). */
export function sequenceIndex(season: ContentDoc | null | undefined, locale: string, number: number): number {
	return sequenceOf(season, locale).findIndex((i) => i?.type === 'chapter' && Number(i.number) === Number(number));
}

/**
 * Season status of chapter `number` in `locale`: the status, null = the season has no entry for it,
 * undefined = no season document (in this locale) – nothing to compare with.
 */
export function seasonStatusOf(season: ContentDoc | null | undefined, locale: string, number: number): Status | null | undefined {
	if (!season || season.loading || season.data?.[season.neutral ? season.primary : locale] == null) return undefined;
	const index = sequenceIndex(season, locale, number);
	if (index < 0) return null;
	const status = sequenceOf(season, locale)[index].status as unknown;
	if (!season.neutral) return status as Status;
	// a locale without its own status shows the source's (like the reader)
	return (resolveText<Status>(status, [locale, season.textSource, ...season.textLanguages]) ?? null) as Status | null;
}

/** Sets the season status of chapter `number` in one locale (the one status control of the creator). */
export function setSeasonStatus(season: ContentDoc, locale: string, number: number, status: Status) {
	const index = sequenceIndex(season, locale, number);
	if (index < 0) return;
	if (season.neutral) season.setText(['sequence', index, 'status'], locale, status, `status:${locale}:${number}`);
	else season.setLocal(locale, ['sequence', index, 'status'], status, `status:${locale}:${number}`);
}

/** The changes `syncChapter` would make: fields a language-neutral chapter no longer has ([] = nothing to do). */
export function chapterSyncChanges(chapter: ContentDoc): string[] {
	const file = chapter.data[chapter.primary];
	if (!file || typeof file !== 'object') return [];
	return ['published', 'status', 'language'].filter((key) => key in file).map((key) => `remove ${key}`);
}

/**
 * Drops what the language-neutral chapter no longer stores (the old `published` flag, the deprecated `status` mirror –
 * the library computes it from the season –, `language`); runs before every save of a chapter. True when it changed
 * something (one undo step).
 */
export function syncChapter(chapter: ContentDoc): boolean {
	if (!chapterSyncChanges(chapter).length) return false;
	chapter.edit((file) => {
		delete file.published;
		delete file.status;
		delete file.language;
	});
	return true;
}

/** syncChapter for every loaded chapter doc of a season (before a save); returns the changed ones. */
export function syncChapters(chapters: Record<number, ContentDoc>): ContentDoc[] {
	return Object.values(chapters).filter((doc) => doc && !doc.loading && syncChapter(doc));
}

/** Chapter numbers of a season doc. */
export function seasonChapterNumbers(season: ContentDoc | null | undefined): number[] {
	if (!season) return [];
	return [...new Set(season.present.flatMap((l) => (season.data[l]?.sequence ?? []).map((i: SequenceItem) => Number(i.number))))].sort((a, b) => a - b);
}
