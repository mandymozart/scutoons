export const csr = true;
export const prerender = false;
export const ssr = false;

import { error } from '@sveltejs/kit';
import type { LayoutLoad } from './$types.ts';
import { isToonListed } from '@scutoons/content';
import { fetchToons } from '$lib/api.ts';
import { draftPreview } from '$lib/draft-preview.svelte.ts';

/**
 * The toons of the home page and the menu from the API's library (agents/DATA.md): the published ones and the announced
 * ones (`announced`: listed as "soon", never a link; owner 2026-10-01). Drafts stay out, also in dev.
 */
export const load: LayoutLoad = async () => {
	try {
		// draft toons (the sandbox, unreleased toons) with a creator's draft preview
		const toons = (await fetchToons()).filter((t) => isToonListed(t) || (draftPreview.on && t.status === 'draft'));
		return { toons };
	} catch (e) {
		error(503, 'Not available');
	}
};
