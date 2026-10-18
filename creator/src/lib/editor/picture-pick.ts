// Reading a pixel of a slice picture for the eyedropper's fallback (owner 2026-10-18, where the browser has no
// EyeDropper): the picture is fetched with CORS (the CDN – bunny and the dev media server – sends
// `Access-Control-Allow-Origin: *`), drawn once into a canvas and kept (the last few pictures); the colour under a point
// (fractions of the picture) is its `#rrggbb`. The picture alone – pieces and text on top aren't drawn in.

/** what `sampleAt` needs of an ImageData (testable without a DOM) */
export interface Pixels {
	width: number;
	height: number;
	data: ArrayLike<number>;
}

/** the colour at fractions (0–1) of the picture, as `#rrggbb` (an opaque read: alpha is ignored) */
export function sampleAt(pixels: Pixels, fx: number, fy: number): string {
	const x = Math.min(pixels.width - 1, Math.max(0, Math.floor(fx * pixels.width)));
	const y = Math.min(pixels.height - 1, Math.max(0, Math.floor(fy * pixels.height)));
	const i = (y * pixels.width + x) * 4;
	return `#${[0, 1, 2].map((k) => Number(pixels.data[i + k]).toString(16).padStart(2, '0')).join('')}`;
}

const cache = new Map<string, Promise<Pixels | null>>();

/** the picture's pixels (null when it can't be read – no CORS, a network error) */
export function picturePixels(url: string): Promise<Pixels | null> {
	let hit = cache.get(url);
	if (!hit) {
		hit = (async () => {
			try {
				const response = await fetch(url, { mode: 'cors' });
				if (!response.ok) return null;
				const bitmap = await createImageBitmap(await response.blob());
				const canvas = document.createElement('canvas');
				canvas.width = bitmap.width;
				canvas.height = bitmap.height;
				const context = canvas.getContext('2d', { willReadFrequently: true });
				if (!context) return null;
				context.drawImage(bitmap, 0, 0);
				bitmap.close();
				return context.getImageData(0, 0, canvas.width, canvas.height);
			} catch {
				return null;
			}
		})();
		cache.set(url, hit);
		// keep the last four pictures
		while (cache.size > 4) cache.delete(cache.keys().next().value!);
	}
	return hit;
}
