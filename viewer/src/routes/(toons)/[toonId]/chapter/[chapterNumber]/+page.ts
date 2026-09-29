import { error } from '@sveltejs/kit';
import type { PageLoad } from './$types.ts';
import { getLocale } from '$lib/paraglide/runtime.js';
import type { Chapter } from '$lib/models/content.ts';

import { type Group } from '$lib/models/content.ts';
import { m } from '$lib/paraglide/messages.js';
import { fetchContent, isDraftPreview } from '$lib/api/content.service.ts';

export const load: PageLoad = async ({ fetch, params, url }) => {
	// content store first (creators: newest drafts with `?preview=draft`), static /api files as fallback
	const draft = isDraftPreview(url);
	let response = await fetchContent(fetch, params.toonId, getLocale(), `chapter${params.chapterNumber}.json`, { draft });
	if(!response.ok) throw error(404,`${m['common.chapter']({number: params.chapterNumber})} not found in ${getLocale().toUpperCase()}`);
	const chapter: Chapter = await response.json();

	// populate Groups
	chapter.groups = await Promise.all(
		chapter.groups.map(async (group) => {
			if (group.ref) {
				const response2 = await fetchContent(fetch, params.toonId, getLocale(), group.ref.src, { draft });
				const populatedGroup: Group = await response2.json();
				return populatedGroup;
			}
			return group;
		})
	) as Group[];

	if (chapter) return { chapter };
	error(404, 'Not found');
};
