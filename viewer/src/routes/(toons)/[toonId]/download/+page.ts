import type { PageLoad } from './$types.ts';

/**
 * The downloads belong to the season: its music is the album of its related items (older seasons: season.download);
 * the toon layout's library answer has season 1 (agents/DATA.md, agents/ASSETS.md "Albums and downloads").
 */
export const load: PageLoad = async ({ parent }) => {
	const { toon, season } = await parent();
	return { toon, season, download: season.download ?? null };
};
