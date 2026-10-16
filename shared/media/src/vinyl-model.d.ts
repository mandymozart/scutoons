// Types of vinyl-model.js (plain JS: three.js has no bundled types and the repo carries no @types/three).
import type { VinylLook } from '@scutoons/content';

/** the pictures of a look (CDN-root paths) – what a page loads ahead */
export function vinylPictures(look: VinylLook | null | undefined): string[];

/** the record in its sleeve (groups "sleeve", "record") from a look; `url`: CDN-root path → URL */
export function buildVinyl(THREE: any, look: VinylLook, url?: (path: string) => string): Promise<any>;
