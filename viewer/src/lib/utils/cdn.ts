import { cdnUrl, toonAsset } from '@scutoons/content';
import { CDN_SERVER } from '../config.ts';

/** URL of a CDN file (`catpope/images/…`) on this app's CDN (VITE_CDN_URL). */
export function asset(path: string): string {
	return cdnUrl(CDN_SERVER, path);
}

/**
 * URL of a toon document's media path (a slice image, a video, a sound …): relative to the toon's folder, `../` out of
 * it (another toon's file), a leading `/` from the CDN root – @scutoons/content `toonAsset`, the one resolver (BACKLOG 10.39).
 */
export function toonUrl(toon: string | null | undefined, path: string | null | undefined): string {
	return path ? asset(toonAsset(toon, path)) : '';
}

/**
 * A toon's image on the landing page / in the menu: its tile image (toon settings `tile.image`, a path in the toon's CDN
 * folder, `../characters/…` for a registry portrait) when it has one, else its avatar. Announced toons always show the
 * tile – their avatar may give away what the announcement keeps back (Mausboss in her helmet).
 */
export function toonImage(toon: { id: string; avatar?: string; tile?: { image?: string } }): string {
	const path = toon.tile?.image || toon.avatar;
	return toonUrl(toon.id, path);
}
