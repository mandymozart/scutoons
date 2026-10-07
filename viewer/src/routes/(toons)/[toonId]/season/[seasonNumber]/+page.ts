import { error, redirect } from '@sveltejs/kit';
import { localizeHref } from '$lib/paraglide/runtime.js';
import { shownSeasons } from '$lib/utils/chapter-name.ts';
import type { PageLoad } from './$types.ts';

/**
 * `/<toon>/season/S` (BACKLOG 10.41): the toon page at season S – its heading in the chapter list (`#season-S`) when the
 * toon lists two or more seasons, else the toon page. An unknown season: the normal not-found page.
 */
export const load: PageLoad = async ({ params, parent }) => {
	const { seasons } = await parent();
	const number = /^\d{1,4}$/.test(params.seasonNumber) ? Number(params.seasonNumber) : NaN;
	if (!seasons.some((s) => s.number === number)) error(404, 'Not found');
	redirect(307, localizeHref(`/${params.toonId}`) + (shownSeasons(seasons).length >= 2 ? `#season-${number}` : ''));
};
