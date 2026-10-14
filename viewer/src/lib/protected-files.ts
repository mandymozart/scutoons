// Files in the vault (the protected storage – scores' files, physical releases' audio; agents/research/SCORES.md §5,
// owner 2026-10-14): never a fixed address – the API answers a short-lived signed URL by the album's rules. A path
// outside the vault (game audio, an older album) is the plain CDN address as before.
import { albumFile } from '$lib/api.ts';
import { asset } from '$lib/utils/cdn.ts';

export const inVault = (path: string | null | undefined): boolean => !!path && (path.startsWith('albums/') || path.startsWith('scores/'));

/** the address of an album's zip (signed when it lives in the vault) */
export async function albumDownloadUrl(albumId: string, path: string): Promise<string> {
	return inVault(path) ? (await albumFile(albumId)).url : asset(path);
}

/** the address of one album track (signed when it lives in the vault) */
export async function albumTrackUrl(albumId: string, track: { id: string; src: string }): Promise<string> {
	return inVault(track.src) ? (await albumFile(albumId, track.id)).url : track.src;
}
