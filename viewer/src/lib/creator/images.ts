// Known image paths of a toon, collected from every config the editor can reach (for the image path picker).
import { fetchContent } from '$lib/api.ts';
import type { Season } from '@scutoons/content';

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

async function json(toon: string, locale: string, path: string) {
	try {
		const response = await fetchContent(fetch, toon, locale, path);
		if (!response.ok || !(response.headers.get('content-type') || '').includes('json')) return null;
		return await response.json();
	} catch {
		return null;
	}
}

/** Every image path used in toon, season, chapters and VN scene files (published or static) of all locales. */
export function knownImages(toon: string, locales: readonly string[], extra: unknown[] = []): Promise<string[]> {
	const key = `${toon}:${locales.join(',')}`;
	if (!cache.has(key)) {
		cache.set(
			key,
			(async () => {
				const out = new Set<string>();
				await Promise.all(
					locales.map(async (locale) => {
						const [toonFile, season] = await Promise.all([json(toon, locale, 'toon.json'), json(toon, locale, 'season1.json')]);
						collect(toonFile, out);
						collect(season, out);
						const chapters = await Promise.all(
							((season as Season | null)?.sequence ?? []).map((item) => json(toon, locale, `chapter${item.number}.json`))
						);
						const refs: string[] = [];
						for (const chapter of chapters) {
							collect(chapter, out);
							for (const g of chapter?.groups ?? []) if (g?.ref?.src) refs.push(g.ref.src);
						}
						(await Promise.all(refs.map((src) => json(toon, locale, src)))).forEach((g) => collect(g, out));
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
