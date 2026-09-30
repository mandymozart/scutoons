import { error } from '@sveltejs/kit';
import type { PageLoad } from './$types.ts';
import { getLocale } from '$lib/paraglide/runtime.js';
import type { Chapter } from '@scutoons/content';

import { type Group, isItemPublic, sequenceStatus, Status } from '@scutoons/content';
import { m } from '$lib/paraglide/messages.js';
import { fetchContent } from '$lib/api.ts';

/** Why a chapter's content is not shown: `draft` (not public), `coming-soon` (listed, not released yet). */
export type ChapterGate = { reason: 'draft' | 'coming-soon'; number: number; title?: string; release?: string };

/** The chapter file with its referenced groups (VN scene files) filled in; null when the file is missing. */
async function loadChapter(fetch: typeof globalThis.fetch, toonId: string, locale: string, number: string): Promise<Chapter | null> {
	// content store first (published versions), static /api files as fallback
	const response = await fetchContent(fetch, toonId, locale, `chapter${number}.json`);
	if (!response.ok) return null;
	const chapter: Chapter = await response.json();
	// populate Groups (in parallel)
	chapter.groups = (await Promise.all(
		chapter.groups.map(async (group) => {
			if (group.ref) {
				const response2 = await fetchContent(fetch, toonId, locale, group.ref.src);
				const populatedGroup: Group = await response2.json();
				return populatedGroup;
			}
			return group;
		})
	)) as Group[];
	return chapter;
}

export const load: PageLoad = async ({ fetch, params, parent }) => {
	// Started before `parent()`: the toon layout loads the same chapterN.json (the content cache shares that request),
	// and the chapter's scene files load while the layout is still busy with the other chapters (agents/PERFORMANCE.md).
	const locale = getLocale();
	const chapterPromise = loadChapter(fetch, params.toonId, locale, params.chapterNumber);
	chapterPromise.catch(() => {}); // a gated chapter never awaits it

	// Visibility = the season's status of this chapter in this locale (the toon layout loaded the season); the chapter
	// file's own `status` is not read. The URL itself always resolves (printed-URL rule) – only the content is gated.
	const { season, sequence } = await parent();
	const number = Number(params.chapterNumber);
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
	if (!chapter) throw error(404, `${m['common.chapter']({ number: params.chapterNumber })} not found in ${locale.toUpperCase()}`);
	return { chapter, gate: null };
};
