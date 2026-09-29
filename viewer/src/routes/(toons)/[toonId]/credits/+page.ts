import { error } from '@sveltejs/kit';
import type { PageLoad } from './$types.ts';
import { API_BASE } from '$lib/config.ts';
import { fetchContent } from '$lib/api.ts';
import { getLocale } from '$lib/paraglide/runtime.js';
import {
	CREDITS_FILE,
	CREDITS_SCOPE,
	NEUTRAL_LOCALE,
	PEOPLE_FILE,
	ROLES_FILE,
	type FullCredits,
	type PeopleRegistry,
	type RoleVocabulary,
	type ToonCredits
} from '@scutoons/content';

/**
 * Credits of a toon (printed URL `/<toon>/credits` – RULE: keep it working). Source: the content store's
 * language-neutral `<toon>/_/credits.json` + the global `scutoons/_/people.json` and `roles.json` (published), rendered
 * in the reader's language. Fallback while a toon has no credits.json: the old static `<toon>/<locale>/credits.json`
 * of the current locale (then `de`).
 */
export const load: PageLoad = async ({ fetch, params }) => {
	const locale = getLocale();
	const json = async <T>(response: Response): Promise<T | null> => {
		if (!response.ok) return null;
		try {
			return (await response.json()) as T;
		} catch {
			return null; // e.g. an HTML 404 page of the static host
		}
	};

	const credits = await json<ToonCredits>(await fetchContent(fetch, params.toonId, NEUTRAL_LOCALE, CREDITS_FILE));
	if (credits && Array.isArray(credits.sections)) {
		const [people, roles] = await Promise.all([
			fetchContent(fetch, CREDITS_SCOPE, NEUTRAL_LOCALE, PEOPLE_FILE).then((r) => json<PeopleRegistry>(r)),
			fetchContent(fetch, CREDITS_SCOPE, NEUTRAL_LOCALE, ROLES_FILE).then((r) => json<RoleVocabulary>(r))
		]);
		if (people) return { source: 'store' as const, locale, credits, people, roles, legacy: null };
	}

	for (const lang of [...new Set([locale, 'de'])]) {
		const legacy = await json<FullCredits>(await fetch(`${API_BASE}/${params.toonId}/${lang}/credits.json`));
		if (legacy) return { source: 'static' as const, locale, credits: null, people: null, roles: null, legacy };
	}
	error(404, 'Not found');
};
