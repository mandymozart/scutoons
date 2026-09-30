import { creditsLabels, resolveCredits, type ResolvedCredits } from '@scutoons/content';
import { fetchCredits } from '$lib/api.ts';
import { getLocale } from '$lib/paraglide/runtime.js';
import type { PageLoad } from './$types.ts';

/**
 * The toon page's credits fine print (agents/DESIGN.md "Credits colophon"): the library's credits of the toon, resolved
 * in the reader's language. Streamed (not awaited), so the page and its chapter list don't wait for it; no credits →
 * empty (the season's supporters still show).
 */
export const load: PageLoad = ({ params }) => {
	const locale = getLocale();
	const empty: ResolvedCredits = { labels: creditsLabels(locale), lead: [], sections: [] };
	const credits: Promise<ResolvedCredits> = fetchCredits(params.toonId)
		.then(({ credits, people, roles }) => (credits && Array.isArray(credits.sections) ? resolveCredits(credits, people, roles, locale) : empty))
		.catch(() => empty);
	return { credits };
};
