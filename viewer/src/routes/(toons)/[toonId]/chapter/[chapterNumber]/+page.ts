import { error } from '@sveltejs/kit';
import type { PageLoad } from './$types.ts';
import { getLocale } from '$lib/paraglide/runtime.js';
import type { Chapter } from '@scutoons/content';

import { type Group } from '@scutoons/content';
import { m } from '$lib/paraglide/messages.js';
import { fetchContent } from '$lib/api.ts';

export const load: PageLoad = async ({ fetch, params }) => {
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

	if (chapter) return { chapter };
	error(404, 'Not found');
};
