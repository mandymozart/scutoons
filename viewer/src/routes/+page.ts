import type { PageLoad } from './$types.ts';
import { isItemListed, Status, type CharacterRegistry, type FeaturedProduct, type ProductCatalog, type ToonIndex } from '@scutoons/content';
import { fetchFeaturedProducts, fetchProducts, fetchRegistry, fetchToon } from '$lib/api.ts';
import { getLocale } from '$lib/paraglide/runtime.js';

/** A published toon's newest season with something to read (the tile's "Staffel 1 · Das Erwachen") */
export interface TileSeason {
	number: number;
	title: string;
}

/**
 * The landing page (owner 2026-10-01, design "D · Brutal"), everything from the API:
 * - the toons of the root layout (published + announced), their tile colours from the character registry
 *   (`/library/characters`: the star's colour, unless the toon settings' `tile.color` says otherwise),
 * - per published toon its newest season with a published chapter (one library request per toon, cached 60 s),
 * - the featured editions (`/products/featured/`, chosen in the admin app) resolved against the catalog (`/products/`).
 * Each part may fail on its own: the page then renders without it.
 */
export const load: PageLoad = async ({ parent }) => {
	const { toons } = await parent();
	const locale = getLocale();
	const published = (toons as ToonIndex[]).filter((t) => t.status === Status.PUBLISHED && !t.announced);

	const [registry, seasons, products] = await Promise.all([
		fetchRegistry().catch((e): CharacterRegistry | null => (console.error('Error fetching characters', e), null)),
		Promise.all(
			published.map(async (toon): Promise<[string, TileSeason | null]> => {
				try {
					const bundle = await fetchToon(toon.id, locale);
					const readable = bundle.seasons.filter((s) =>
						s.chapters.some((c) => c.status === Status.PUBLISHED && isItemListed(c, import.meta.env.DEV))
					);
					const newest = readable.sort((a, b) => b.number - a.number)[0];
					return [toon.id, newest ? { number: newest.number, title: newest.season.title } : null];
				} catch (e) {
					console.error(`Error fetching toon ${toon.id}`, e);
					return [toon.id, null];
				}
			})
		).then((entries) => Object.fromEntries(entries) as Record<string, TileSeason | null>),
		Promise.all([fetchFeaturedProducts(), fetchProducts()])
			.then(([featured, catalog]) => (Array.isArray(featured) && catalog?.products ? { featured, catalog } : null))
			.catch((e) => (console.error('Error fetching products', e), null))
	]);

	return {
		registry,
		seasons,
		featured: products?.featured ?? ([] as FeaturedProduct[]),
		catalog: products?.catalog ?? (null as ProductCatalog | null)
	};
};
