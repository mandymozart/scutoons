import type { StockistList } from '@scutoons/api';
import { fetchStockists } from '$lib/api.ts';

let pending: Promise<StockistList | null> | null = null;

/**
 * Every listed store (`/stockists/`, the Embassy's stores; agents/EMBASSY.md "Stores"), fetched once per page load; null
 * when the list is unavailable. The "Where to buy" page filters it in the browser, the product rows ask `storesFor`.
 */
export function loadStockists(): Promise<StockistList | null> {
	pending ??= fetchStockists()
		.then((list) => (list && Array.isArray(list.stores) ? list : null))
		.catch((e) => {
			console.warn('Stockists unavailable', e);
			pending = null;
			return null;
		});
	return pending;
}

/** How many listed stores carry a product (0 when unknown) */
export function storesFor(list: StockistList | null | undefined, productId: string | null | undefined): number {
	return productId ? (list?.products[productId] ?? 0) : 0;
}

/** The "Where to buy" page, filtered to one product */
export const storesHref = (productId: string) => `/stores?product=${encodeURIComponent(productId)}`;
