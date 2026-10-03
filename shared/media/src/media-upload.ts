// Browser-side media for the importers (slice import, character images, map places): WebP encoding and "upload, else a
// new version". The assets API is the app's (setupMedia).
import { ApiError } from '@scutoons/api';
import { mediaAssets } from './setup.ts';

const WEBP_QUALITY = 0.88;

/** the image as WebP, scaled down to `maxWidth` (never up); transparency is kept */
export async function toWebp(bitmap: ImageBitmap, maxWidth = bitmap.width): Promise<{ blob: Blob; width: number; height: number }> {
	const width = Math.min(maxWidth, bitmap.width);
	const height = Math.round((bitmap.height * width) / bitmap.width);
	const canvas = document.createElement('canvas');
	canvas.width = width;
	canvas.height = height;
	const ctx = canvas.getContext('2d');
	if (!ctx) throw new Error('no canvas');
	ctx.imageSmoothingEnabled = true;
	ctx.imageSmoothingQuality = 'high';
	ctx.drawImage(bitmap, 0, 0, width, height);
	const blob = await new Promise<Blob | null>((resolve) => canvas.toBlob(resolve, 'image/webp', WEBP_QUALITY));
	if (!blob) throw new Error('WebP encoding failed');
	return { blob, width, height };
}

/**
 * Upload to `path`; a file already there gets the new bytes (owner 2026-10-03: new versions without renaming – the old
 * bytes stay as the asset's previous version, the CDN is purged). The same bytes = nothing to do.
 */
export async function uploadOrReplace(blob: Blob, path: string, note?: string): Promise<'uploaded' | 'replaced' | 'unchanged'> {
	try {
		await mediaAssets().upload(blob, path);
		return 'uploaded';
	} catch (e) {
		if (!(e instanceof ApiError && e.code === 'exists')) throw e;
		try {
			await mediaAssets().replaceAt(blob, path, note);
			return 'replaced';
		} catch (again) {
			if (again instanceof ApiError && again.code === 'unchanged') return 'unchanged';
			throw again;
		}
	}
}
