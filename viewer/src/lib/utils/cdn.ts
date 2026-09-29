import { cdnUrl } from '@scutoons/content';
import { CDN_SERVER } from '../config.ts';

/** URL of a CDN file (`catpope/images/…`) on this app's CDN (VITE_CDN_URL). */
export function asset(path: string): string {
	return cdnUrl(CDN_SERVER, path);
}
