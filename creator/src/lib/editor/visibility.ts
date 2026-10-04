// Chapter visibility has ONE source: the season's sequence status, per locale (season 1 → `sequence[].status`:
// published | draft | coming-soon; a toon may release de before en). The chapter file's own `status` is deprecated –
// the creator mirrors the season status into it on save, only for older viewer builds – and `published` is gone.
// A chapter always belongs to a season (owner rule): new chapters are created inside it, a new season adopts existing
// chapter files, and a chapter file without a season entry is a validation error.
import { Status, type SequenceItem } from '@scutoons/content';
import type { ContentDoc } from '$lib/editor/doc.svelte.ts';

export const STATUSES = Object.values(Status) as Status[];

/** Index of chapter `number` in the season sequence of `locale`, -1 when missing (or no season file). */
export function sequenceIndex(season: ContentDoc | null | undefined, locale: string, number: number): number {
	const sequence: SequenceItem[] = season?.data?.[locale]?.sequence ?? [];
	return Array.isArray(sequence) ? sequence.findIndex((i) => i?.type === 'chapter' && Number(i.number) === Number(number)) : -1;
}

/**
 * Season status of chapter `number` in `locale`: the status, null = the season has no entry for it,
 * undefined = no season file in this locale (nothing to compare with).
 */
export function seasonStatusOf(season: ContentDoc | null | undefined, locale: string, number: number): Status | null | undefined {
	if (!season || season.loading || season.data?.[locale] == null) return undefined;
	const index = sequenceIndex(season, locale, number);
	return index < 0 ? null : season.data[locale].sequence[index].status;
}

/** Sets the season status of chapter `number` in one locale (the one status control of the creator). */
export function setSeasonStatus(season: ContentDoc, locale: string, number: number, status: Status) {
	const index = sequenceIndex(season, locale, number);
	if (index < 0) return;
	season.setLocal(locale, ['sequence', index, 'status'], status, `status:${locale}:${number}`);
}

/** The changes `syncChapter` would make, per locale ([] = in sync). */
export function chapterSyncChanges(chapter: ContentDoc, season: ContentDoc | null | undefined, number: number): string[] {
	const changes: string[] = [];
	for (const locale of chapter.present) {
		const file = chapter.data[locale];
		if ('published' in file) changes.push(`${locale}: remove published`);
		const status = seasonStatusOf(season, locale, number);
		if (status && file.status !== status) changes.push(`${locale}: status ${file.status ?? '–'} → ${status}`);
	}
	return changes;
}

/**
 * Mirrors the season status into the chapter file of every locale and drops the old `published` flag
 * (runs before every save of a chapter). Returns true when it changed something (one undo step).
 */
export function syncChapter(chapter: ContentDoc, season: ContentDoc | null | undefined, number: number): boolean {
	if (!chapterSyncChanges(chapter, season, number).length) return false;
	chapter.checkpoint();
	for (const locale of chapter.present) {
		const file = chapter.data[locale];
		delete file.published;
		const status = seasonStatusOf(season, locale, number);
		if (status) file.status = status;
	}
	return true;
}

/** Mirrors the season status into every loaded chapter doc of the season (before a save); returns the changed ones. */
export function syncChapters(chapters: Record<number, ContentDoc>, season: ContentDoc | null | undefined): ContentDoc[] {
	const changed: ContentDoc[] = [];
	for (const [number, doc] of Object.entries(chapters)) {
		if (doc && !doc.loading && syncChapter(doc, season, Number(number))) changed.push(doc);
	}
	return changed;
}

/** Chapter numbers of a season doc in any of its locales. */
export function seasonChapterNumbers(season: ContentDoc | null | undefined): number[] {
	if (!season) return [];
	return [...new Set(season.present.flatMap((l) => (season.data[l]?.sequence ?? []).map((i: SequenceItem) => Number(i.number))))].sort((a, b) => a - b);
}
