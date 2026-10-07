import { error } from '@sveltejs/kit';
import type { PageLoad } from './$types.ts';
import { getLocale } from '$lib/paraglide/runtime.js';
import type { Chapter } from '@scutoons/content';

import { chapterPlace, sequenceStatus, Status } from '@scutoons/content';
import { m } from '$lib/paraglide/messages.js';
import { ApiError, fetchChapter } from '$lib/api.ts';
import { draftPreview, isReadable, showDrafts } from '$lib/draft-preview.svelte.ts';

/** Why a chapter's content is not shown: `draft` (not public), `coming-soon` (listed, not released yet). */
export type ChapterGate = {
	reason: 'draft' | 'coming-soon';
	number: number;
	title?: string;
	release?: string;
	/** coming soon: the key visual / its video as the episode's teaser (owner 2026-10-03) */
	keyvisual?: { image?: string; video?: string };
};

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
	const layout = await parent();
	// the season that lists the chapter (BACKLOG 10.41: a later season's chapters are read with their own season – status,
	// what comes next, its music); none: the presented one
	const place = Number.isInteger(number) ? chapterPlace(layout.seasons, number) : null;
	const own = place ? layout.seasons.find((s) => s.number === place.season) : undefined;
	const season = own?.season ?? layout.season;
	const sequence = own?.chapters ?? layout.sequence;
	const status = sequenceStatus(season, number);
	// a creator's draft preview reads coming-soon and draft chapters (the API checks the role)
	if (status === Status.COMING_SOON && !draftPreview.on) {
		// title / release from the chapter overview (the layout loaded coming-soon items for the list)
		const item = sequence.find((i) => i.number === number);
		// older chapters store the key visual as a plain image path
		const raw = item?.keyvisual as { image?: string; video?: string } | string | undefined;
		const kv = typeof raw === 'string' ? { image: raw } : raw;
		const keyvisual = kv?.image || kv?.video ? kv : undefined;
		const gate: ChapterGate = { reason: 'coming-soon', number, title: item?.title || undefined, release: item?.release || undefined, keyvisual };
		return { chapter: null, gate, season, sequence };
	}
	// drafts (and chapters the season doesn't list) are readable in dev only, like the chapter overview
	if (status === null ? !showDrafts() : !isReadable({ status })) {
		const gate: ChapterGate = { reason: 'draft', number };
		return { chapter: null, gate, season, sequence };
	}

	const chapter = await chapterPromise;
	if (chapter instanceof ApiError) {
		if (chapter.code === 'coming_soon') return { chapter: null, gate: { reason: 'coming-soon', number } as ChapterGate, season, sequence };
		if (chapter.code === 'not_public') return { chapter: null, gate: { reason: 'draft', number } as ChapterGate, season, sequence };
		throw error(chapter.status === 404 ? 404 : 503, `${m['common.chapter']({ number: params.chapterNumber })} not found in ${locale.toUpperCase()}`);
	}
	return { chapter, gate: null, season, sequence };
};
