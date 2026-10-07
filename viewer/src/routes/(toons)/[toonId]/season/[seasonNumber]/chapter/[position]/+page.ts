import { error, redirect } from '@sveltejs/kit';
import { chapterAt } from '@scutoons/content';
import { localizeHref } from '$lib/paraglide/runtime.js';
import type { PageLoad } from './$types.ts';

/**
 * `/<toon>/season/S/chapter/K` (BACKLOG 10.41) → `/<toon>/chapter/N`, THE address of a chapter (printed QR codes): K = the
 * K-th entry with number ≠ 0 of season S's stored sequence (the prologue is not counted, drafts and coming-soon are –
 * @scutoons/content chapterAt). The query (`?label=`) goes along. Unknown season / position: the normal not-found page.
 */
export const load: PageLoad = async ({ params, parent, url }) => {
	const { seasons } = await parent();
	const digits = (value: string) => (/^\d{1,6}$/.test(value) ? Number(value) : NaN);
	const number = chapterAt(seasons, digits(params.seasonNumber), digits(params.position));
	if (number === null) error(404, 'Not found');
	redirect(307, localizeHref(`/${params.toonId}/chapter/${number}`) + url.search);
};
