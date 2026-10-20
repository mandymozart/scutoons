// What the catalog editors need from the app, provided ONCE in the app's root layout (like @scutoons/media):
//
//   setupCatalog({ locale: getLocale, locales, adminUrl: '' });      // the admin
//   setupCatalog({ locale: getLocale, locales, adminUrl: ADMIN_URL }); // the creator (links to codes / batches)
//
// A module setting, not a Svelte context: kinds.ts' labels and text() are called outside components too.
import { crossAppClick } from '@scutoons/pwa';

export interface CatalogSetup {
	/** the app's UI locale (paraglide `getLocale`); read on every render */
	locale: () => string;
	/** the content languages a localized text has a line for (paraglide `locales`) */
	locales: readonly string[];
	/** where the admin's codes / batches pages live ('' = this app is the admin) */
	adminUrl?: string;
}

let current: CatalogSetup | null = null;

export function setupCatalog(setup: CatalogSetup): void {
	current = setup;
}

function need(): CatalogSetup {
	if (!current) throw new Error('@scutoons/catalog: call setupCatalog({ locale, locales }) in the app root layout first');
	return current;
}

export const catalogLocale = (): string => need().locale();
export const catalogLocales = (): readonly string[] => need().locales;
/** an admin page's URL (`/codes?edition_id=…`) – relative in the admin, absolute elsewhere */
export const adminHref = (path: string): string => `${(need().adminUrl ?? '').replace(/\/$/, '')}${path}`;
/**
 * the link attributes for an admin page: elsewhere (the creator) the admin is another app – a new window without an
 * opener (on Android inside the installed Creator: an intent, @scutoons/pwa cross-app.ts), so the installed Admin opens if there is one (research/PWA-APPS.md §3a); in the admin itself none
 */
export const adminLinkTarget = (): { target?: string; rel?: string; onclick?: (event: MouseEvent) => void } =>
	need().adminUrl ? { target: '_blank', rel: 'noopener', onclick: crossAppClick } : {};
