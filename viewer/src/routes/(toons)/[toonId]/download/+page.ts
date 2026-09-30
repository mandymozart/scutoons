import type { PageLoad } from './$types.ts';

/** The toon's settings (downloads) come with the toon layout's library answer (agents/DATA.md). */
export const load: PageLoad = async ({ parent }) => {
	const { toon } = await parent();
	return { toon };
};
