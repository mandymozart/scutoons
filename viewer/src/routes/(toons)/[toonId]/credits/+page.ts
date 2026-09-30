import { error } from '@sveltejs/kit';
import type { PageLoad } from './$types.ts';
import { ApiError, fetchCredits } from '$lib/api.ts';
import { getLocale } from '$lib/paraglide/runtime.js';

/**
 * Credits of a toon (printed URL `/<toon>/credits` – RULE: keep it working): the library's credits of the toon with the
 * people they name and the role vocabulary (agents/DATA.md), rendered in the reader's language.
 */
export const load: PageLoad = async ({ params }) => {
	const locale = getLocale();
	try {
		const { credits, people, roles } = await fetchCredits(params.toonId);
		if (credits && Array.isArray(credits.sections) && people) return { locale, credits, people, roles };
	} catch (e) {
		if (!(e instanceof ApiError) || e.status !== 404) throw error(503, 'Credits are not available right now');
	}
	error(404, 'Not found');
};
