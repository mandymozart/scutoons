import { cdnUrl } from '@scutoons/content';
import { CDN_SERVER } from '../config.ts';

/** URL of a CDN file (`catpope/images/…`) on this app's CDN (VITE_CDN_URL). */
export function asset(path: string): string {
	return cdnUrl(CDN_SERVER, path);
}

/**
 * A toon's image on the landing page / in the menu: its tile image (toon settings `tile.image`, a path in the toon's CDN
 * folder, `../characters/…` for a registry portrait) when it has one, else its avatar. Announced toons always show the
 * tile – their avatar may give away what the announcement keeps back (Mausboss in her helmet).
 */
export function toonImage(toon: { id: string; avatar?: string; tile?: { image?: string } }): string {
	const path = toon.tile?.image || toon.avatar;
	return path ? asset(`${toon.id}/${path}`) : '';
}
