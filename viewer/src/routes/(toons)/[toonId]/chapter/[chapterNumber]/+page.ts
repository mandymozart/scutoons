import { error } from '@sveltejs/kit';
import type { PageLoad } from './$types.ts';
import { getLocale } from '$lib/paraglide/runtime.js';
import type { Chapter } from '@scutoons/content';

import { isItemPublic, sequenceStatus, Status } from '@scutoons/content';
import { m } from '$lib/paraglide/messages.js';
import { ApiError, fetchChapter } from '$lib/api.ts';

/** Why a chapter's content is not shown: `draft` (not public), `coming-soon` (listed, not released yet). */
export type ChapterGate = { reason: 'draft' | 'coming-soon'; number: number; title?: string; release?: string };

export const load: PageLoad = async ({ params, parent }) => {
	// One request per chapter: the library answers the chapter with its VN scenes inlined (agents/DATA.md). Started
	// before `parent()`, so it runs while the toon layout loads; the API applies the same visibility rules (403).
	const locale = getLocale();
	// chapter numbers start at 0 (the season's prologue); digits only, like the API route
	const number = /^\d{1,6}$/.test(params.chapterNumber) ? Number(params.chapterNumber) : NaN;
	const chapterPromise: Promise<Chapter | ApiError> = Number.isInteger(number) && number >= 0
		? fetchChapter(params.toonId, number, locale).catch((e) => (e instanceof ApiError ? e : new ApiError(0, 'network', String(e))))
		: Promise.resolve(new ApiError(404, 'not_found', 'Not found'));

	// Visibility = the season's status of this chapter in this locale (the toon layout loaded the season); the chapter
	// file's own `status` is not read. The URL itself always resolves (printed-URL rule) – only the content is gated.
	const { season, sequence } = await parent();
	const status = sequenceStatus(season, number);
	if (status === Status.COMING_SOON) {
		// title / release from the chapter overview (the layout loaded coming-soon items for the list)
		const item = sequence.find((i) => i.number === number);
		const gate: ChapterGate = { reason: 'coming-soon', number, title: item?.title || undefined, release: item?.release || undefined };
		return { chapter: null, gate };
	}
	// drafts (and chapters the season doesn't list) are readable in dev only, like the chapter overview
	if (status === null ? !import.meta.env.DEV : !isItemPublic({ status }, import.meta.env.DEV)) {
		const gate: ChapterGate = { reason: 'draft', number };
		return { chapter: null, gate };
	}

	const chapter = await chapterPromise;
	if (chapter instanceof ApiError) {
		if (chapter.code === 'coming_soon') return { chapter: null, gate: { reason: 'coming-soon', number } as ChapterGate };
		if (chapter.code === 'not_public') return { chapter: null, gate: { reason: 'draft', number } as ChapterGate };
		throw error(chapter.status === 404 ? 404 : 503, `${m['common.chapter']({ number: params.chapterNumber })} not found in ${locale.toUpperCase()}`);
	}
	return { chapter, gate: null };
};
