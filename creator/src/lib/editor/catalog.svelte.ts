import type { Catalog, ItemDef } from '@scutoons/content';
import { authApi } from '$lib/api.ts';
import { getLocale } from '$lib/paraglide/runtime.js';

/**
 * The item catalog (`/api/items/`), loaded once per page load: related items of seasons / chapters, items placed on
 * slices (`Slice.items`). `itemCatalog` is reactive (null until loaded or when the API is unavailable).
 */
export const itemCatalog = $state<{ catalog: Catalog | null; loaded: boolean }>({ catalog: null, loaded: false });

let catalogPromise: Promise<Catalog | null> | null = null;

export function loadCatalog(): Promise<Catalog | null> {
	catalogPromise ??= authApi
		.fetchCatalog()
		.then((catalog) => {
			itemCatalog.catalog = catalog;
			itemCatalog.loaded = true;
			return catalog;
		})
		.catch((e) => {
			console.warn('Item catalog unavailable', e);
			catalogPromise = null;
			itemCatalog.loaded = true;
			return null;
		});
	return catalogPromise;
}

/** an item's name in the UI language (else en / de / the id) */
export function itemName(item: ItemDef | undefined, id: string): string {
	if (!item) return id;
	const locale = getLocale();
	return item.name[locale] ?? item.name.en ?? item.name.de ?? id;
}
