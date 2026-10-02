import type { ItemDef, Product, ProductCatalog } from '@scutoons/content';
import { fetchProducts } from '$lib/api.ts';

let pending: Promise<ProductCatalog | null> | null = null;

/** The shop catalog (`/products/`), fetched once per page load; null when it is unavailable */
export function loadProducts(): Promise<ProductCatalog | null> {
	pending ??= fetchProducts()
		.then((c) => (c && Array.isArray(c.products) ? c : null))
		.catch((e) => {
			console.warn('Products unavailable', e);
			pending = null;
			return null;
		});
	return pending;
}

/** The shop product of a catalog item (`item.product`), if there is one */
export function productOf(catalog: ProductCatalog | null | undefined, item: Pick<ItemDef, 'product'> | null | undefined): Product | undefined {
	if (!item?.product) return undefined;
	return catalog?.products.find((p) => p.id === item.product);
}
