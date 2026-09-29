import { error } from '@sveltejs/kit';
import type { PageLoad } from './$types.ts';
import { getLocale } from '$lib/paraglide/runtime.js';
import type { Chapter } from '@scutoons/content';

import { type Group, isItemPublic, sequenceStatus, Status } from '@scutoons/content';
import { m } from '$lib/paraglide/messages.js';
import { fetchContent } from '$lib/api.ts';

/** Why a chapter's content is not shown: `draft` (not public), `coming-soon` (listed, not released yet). */
export type ChapterGate = { reason: 'draft' | 'coming-soon'; number: number; title?: string; release?: string };

export const load: PageLoad = async ({ fetch, params, parent }) => {
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

	// content store first (published versions), static /api files as fallback
	let response = await fetchContent(fetch, params.toonId, getLocale(), `chapter${params.chapterNumber}.json`);
	if(!response.ok) throw error(404,`${m['common.chapter']({number: params.chapterNumber})} not found in ${getLocale().toUpperCase()}`);
	const chapter: Chapter = await response.json();

	// populate Groups
	chapter.groups = await Promise.all(
		chapter.groups.map(async (group) => {
			if (group.ref) {
				const response2 = await fetchContent(fetch, params.toonId, getLocale(), group.ref.src);
				const populatedGroup: Group = await response2.json();
				return populatedGroup;
			}
			return group;
		})
	) as Group[];

	if (chapter) return { chapter, gate: null };
	error(404, 'Not found');
};
