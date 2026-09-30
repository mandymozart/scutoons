import type { PageLoad } from './$types.ts';

/** The downloads belong to the season (season.download); the toon layout's library answer has season 1 (agents/DATA.md). */
export const load: PageLoad = async ({ parent }) => {
	const { toon, season } = await parent();
	return { toon, download: season.download ?? null };
};
