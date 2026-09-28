// Shop data contract for static/api/products.json (version 2) and featured-products.json.
// One product per Bandcamp item; colours / test-press numbers are variants.
// `item_id` links to the item catalog (qr.scutoons.com /api/items/), `edition_id` to qr editions.json.

export type LocalizedText = { de: string; en: string } & Record<string, string>;

export interface ProductVariant {
	id: string;
	name: LocalizedText;
	soldOut: boolean;
	/** CDN path of the transparent item icon (catpope/items/<item_id>_<variant>.png) */
	icon: string | null;
	/** CDN paths, relative to the CDN root */
	images: string[];
	/** overrides the product price (e.g. test press 1/2 vs 2/2) */
	price?: number;
}

export interface Product {
	id: string;
	toon: string;
	item_id: string;
	edition_id: string | null;
	/** size of the QR edition (qr editions.json edition_count) — shared by all products of that edition */
	edition_size: number | null;
	name: string;
	category: string;
	link: string;
	/** base price in `currency` */
	price: number;
	/** pieces of this product on Bandcamp ("edition of N") */
	limited: number | null;
	/** Bandcamp "N remaining" at `checked_at` */
	remaining: number | null;
	sizes: string[];
	soldOutSizes: string[];
	credits: string[];
	icon: string;
	defaultVariant: string | null;
	variants: ProductVariant[];
}

export interface ProductCatalog {
	version: 2;
	currency: string;
	checked_at: string;
	source: string;
	products: Product[];
}

export interface FeaturedProduct {
	product: string;
	variant?: string;
}

export function isSoldOut(product: Product): boolean {
	return product.variants.every((v) => v.soldOut);
}

/** The variant to show first: the configured default if available, else the first available one. */
export function pickVariant(product: Product, preferred?: string | null): ProductVariant {
	const byId = (id?: string | null) => product.variants.find((v) => v.id === id);
	const wanted = byId(preferred) ?? byId(product.defaultVariant);
	if (wanted && !wanted.soldOut) return wanted;
	return product.variants.find((v) => !v.soldOut) ?? wanted ?? product.variants[0];
}

export function variantPrice(product: Product, variant?: ProductVariant): number {
	return variant?.price ?? product.price;
}

export function formatPrice(amount: number, currency: string, locale: string): string {
	return new Intl.NumberFormat(locale === 'de' ? 'de-AT' : 'en-GB', {
		style: 'currency',
		currency
	}).format(amount);
}

/** Pieces to advertise: the product's own Bandcamp limit, else the QR edition size. */
export function editionPieces(product: Product): number | null {
	return product.limited ?? product.edition_size;
}

export function localized(text: LocalizedText, locale: string): string {
	return text[locale] ?? text.en ?? text.de;
}
