import { redirect } from '@sveltejs/kit';
import type { PageLoad } from './$types.ts';
import { shopUrl } from '$lib/config.ts';
import { getLocale, isLocale } from '$lib/paraglide/runtime.js';

/**
 * PRINTED / SHARED URL (agents/PLATFORM.md "RULE: printed / public URLs never change"): `/shop`, `/de/shop`,
 * `/en/shop` and anything below (`/shop/product/<id>` from VN jumpTo) keep working as a redirect to the standalone
 * shop (`VITE_SHOP_URL`, shop.scutoons.com) in the locale of the URL prefix, else the reader's current locale.
 */
export const load: PageLoad = ({ url }) => {
	const prefix = url.pathname.split('/')[1];
	redirect(307, shopUrl(isLocale(prefix) ? prefix : getLocale()));
};
