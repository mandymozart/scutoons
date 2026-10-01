// Known image paths of a toon, collected from every entity the editor can reach (for the image path picker).
import { library } from '$lib/api.ts';
import { DEFAULT_SLICE, frameSliceSize, type Path } from '@scutoons/content';

const cache = new Map<string, Promise<string[]>>();

function collect(node: unknown, out: Set<string>) {
	if (Array.isArray(node)) {
		for (const item of node) collect(item, out);
		return;
	}
	if (!node || typeof node !== 'object') return;
	for (const [key, value] of Object.entries(node)) {
		if (typeof value === 'string') {
			if (/^(image|thumbnail|square|portrait|avatar|full|terrain)$/.test(key) && /\.(jpe?g|png|webp|gif|svg)$/i.test(value)) {
				out.add(value);
			}
		} else {
			collect(value, out);
		}
	}
}

/** Every image path used in the toon's settings, seasons, chapters and VN scenes (newest versions) of all locales. */
export function knownImages(toon: string, locales: readonly string[], extra: unknown[] = []): Promise<string[]> {
	const key = `${toon}:${locales.join(',')}`;
	if (!cache.has(key)) {
		cache.set(
			key,
			(async () => {
				const out = new Set<string>();
				await Promise.all(
					locales.map(async (locale) => {
						const bundle = await library.fetchToon(toon, locale, { draft: true }).catch(() => null);
						if (!bundle) return;
						collect(bundle.toon, out);
						for (const overview of bundle.seasons) collect(overview, out);
						const numbers = bundle.seasons.flatMap((o) => o.chapters.map((c) => c.number));
						const chapters = await Promise.all(numbers.map((n) => library.fetchChapter(toon, n, locale, { draft: true }).catch(() => null)));
						for (const chapter of chapters) collect(chapter, out);
					})
				);
				return [...out];
			})()
		);
	}
	return cache.get(key)!.then((list) => {
		const out = new Set(list);
		collect(extra, out);
		return [...out].sort(naturalCompare);
	});
}

/** "slice-2" before "slice-10" */
export function naturalCompare(a: string, b: string) {
	return a.localeCompare(b, undefined, { numeric: true, sensitivity: 'base' });
}

/** Pixel size of an image on the CDN (absolute URL), null when it can't be loaded. */
export function readImageSize(url: string): Promise<{ width: number; height: number } | null> {
	return new Promise((resolve) => {
		const img = new Image();
		img.onload = () => resolve(img.naturalWidth > 0 ? { width: img.naturalWidth, height: img.naturalHeight } : null);
		img.onerror = () => resolve(null);
		img.src = url;
	});
}

/**
 * Every slice carries the size of its image (owner 2026-09-30): read it from the CDN and write `width` / `height` when
 * they differ (a shared edit in every locale, undoable). A missing image keeps the current (standard) size. A player
 * video's poster may be larger (sharp behind the controls): its slice gets the poster's ratio at the slice width.
 */
export async function fillSliceSize(
	doc: { primary: string; get(locale: string, path: Path): any; setShared(path: Path, value: any, key?: string): void },
	slicePath: Path,
	url: (image: string) => string
): Promise<void> {
	// a video in a frame: the size follows the frame's ratio, not the image (SliceVideo.frame)
	const framed = frameSliceSize(doc.get(doc.primary, slicePath)?.video?.frame?.ratio);
	if (framed && !doc.get(doc.primary, slicePath)?.video?.viewport) {
		const current = doc.get(doc.primary, slicePath);
		if (current.width !== framed.width) doc.setShared([...slicePath, 'width'], framed.width, 'slice-size');
		if (current.height !== framed.height) doc.setShared([...slicePath, 'height'], framed.height, 'slice-size');
		return;
	}
	const image = doc.get(doc.primary, slicePath)?.image;
	if (typeof image !== 'string' || !image) return;
	const size = await readImageSize(url(image));
	const slice = doc.get(doc.primary, slicePath);
	if (!size || !slice || slice.image !== image) return;
	if (slice.video?.player === true && size.width !== DEFAULT_SLICE.width) {
		size.height = Math.round((size.height * DEFAULT_SLICE.width) / size.width);
		size.width = DEFAULT_SLICE.width;
	}
	if (slice.width !== size.width) doc.setShared([...slicePath, 'width'], size.width, 'slice-size');
	if (slice.height !== size.height) doc.setShared([...slicePath, 'height'], size.height, 'slice-size');
}
