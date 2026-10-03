// What the media tooling needs from the app, provided ONCE in the app's root layout (agents/COMPONENTS.md "Shared
// media components"):
//
//   setupMedia({ assets: createAssetsApi(api), cdn: CDN_SERVER, locale: getLocale });
//
// A module setting, not a Svelte context: `uploadOrReplace` is called from event handlers and importers, where
// getContext is not available. One app = one API, one CDN.
import type { AssetsApi } from '@scutoons/api';
import { cdnUrl } from '@scutoons/content';
import { mediaLabels, type MediaLabels } from './labels.ts';

export interface MediaSetup {
	/** `createAssetsApi(client)` of the app's API client (session cookie) */
	assets: AssetsApi;
	/** the CDN base URL (e.g. `https://cdn.scutoons.com`), or a function CDN-root path → URL */
	cdn: string | ((path: string) => string);
	/** the app's UI locale (paraglide `getLocale`); read on every render */
	locale?: () => string;
	/** single texts overridden */
	labels?: Partial<MediaLabels>;
}

let current: MediaSetup | null = null;

export function setupMedia(setup: MediaSetup): void {
	current = setup;
}

function need(): MediaSetup {
	if (!current) throw new Error('@scutoons/media: call setupMedia({ assets, cdn, locale }) in the app root layout first');
	return current;
}

/** the app's assets API */
export function mediaAssets(): AssetsApi {
	return need().assets;
}

/** URL of a CDN-root path (`catpope/s1/cover.webp`) */
export function mediaUrl(path: string): string {
	const cdn = need().cdn;
	return typeof cdn === 'function' ? cdn(path) : cdnUrl(cdn, path);
}

/** the texts in the app's current locale */
export function mediaText(): MediaLabels {
	const setup = need();
	return mediaLabels(setup.locale?.() ?? 'en', setup.labels);
}
