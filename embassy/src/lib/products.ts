// The shop catalog (`/products/`, public) for the stores' "carries" list: loaded once per page load.
import type { Product } from '@scutoons/content';
import { products } from '$lib/api.ts';
import { CDN_URL } from '$lib/config.ts';

let pending: Promise<Product[]> | null = null;

/** Every product a store can carry (empty when the catalog is unavailable – the ids still show) */
export function loadProducts(): Promise<Product[]> {
	pending ??= products
		.fetchProducts()
		.then((c) => (Array.isArray(c?.products) ? c.products : []))
		.catch(() => {
			pending = null;
			return [];
		});
	return pending;
}

/** A product icon (a CDN path like `catpope/items/awakening_vinyl.png`) as a URL */
export const productIcon = (product: Pick<Product, 'icon'> | undefined): string | null =>
	product?.icon ? `${CDN_URL}/${product.icon.replace(/^\/+/, '')}` : null;
