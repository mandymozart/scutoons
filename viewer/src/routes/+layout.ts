export const csr = true;
export const prerender = false;
export const ssr = false;

import { error } from '@sveltejs/kit';
import type { LayoutLoad } from './$types.ts';
import { isToonListed } from '@scutoons/content';
import { fetchToons } from '$lib/api.ts';
import { getLocale } from '$lib/paraglide/runtime.js';
import { draftPreview } from '$lib/draft-preview.svelte.ts';
import { linkedPreview } from '$lib/draft-preview-link.ts';

/**
 * The toons of the home page and the menu from the API's library (agents/DATA.md): the published ones and the announced
 * ones (`announced`: listed as "soon", never a link; owner 2026-10-01). Drafts stay out, also in dev. In the reader's
 * language: a toon whose language status closes it in that language is not listed (agents/NEUTRAL.md step 4).
 */
export const load: LayoutLoad = async ({ url }) => {
	// `?draft=1` / `?draft=0` switches a creator's draft preview on / off (the creator's docked reader, "Open in the
	// viewer"): the address the app opened with is read when lib/draft-preview loads; this is for later navigations. The
	// API still checks the role – without it the preview switches itself off again (lib/api.ts)
	const link = linkedPreview(url.search);
	if (link !== null && link !== draftPreview.on) draftPreview.set(link);
	try {
		// draft toons (the sandbox, unreleased toons) with a creator's draft preview
		const toons = (await fetchToons(getLocale())).filter((t) => isToonListed(t) || (draftPreview.on && t.status === 'draft'));
		return { toons };
	} catch (e) {
		error(503, 'Not available');
	}
};
