import { cdnUrl, toonAsset } from '@scutoons/content';
import { CDN_SERVER } from '$lib/config.ts';

/** URL of a CDN file (`catpope/images/…`) on this app's CDN (VITE_CDN_URL). */
export function asset(path: string): string {
	return cdnUrl(CDN_SERVER, path);
}

/**
 * URL of a toon document's media path (a slice image, a video, a sound …): relative to the toon's folder, `../` out of
 * it (another toon's file – the asset picker and a paste across toons write that), a leading `/` from the CDN root –
 * @scutoons/content `toonAsset`, the one resolver of the reader and the creator (BACKLOG 10.39).
 */
export function toonUrl(toon: string | null | undefined, path: string | null | undefined): string {
	return path ? asset(toonAsset(toon, path)) : '';
}
