// The download steps of a saved chapter (PWA phase 3, agents/research/PWA.md §2 "Phase 3"), shared by a first save and
// a refresh (lib/offline/saved-chapters.svelte.ts): one media file into the chapter's cache, the library documents, the
// fonts the page uses, the manifest. Page side only.
import { asset } from '$lib/utils/cdn.ts';
import { MANIFEST_PATH, type SavedManifest, type SaveKind } from '$lib/sw/saved-rules.ts';
import { kindOfUrl } from '$lib/sw/saved-update-rules.ts';

/** at most this many downloads at once */
export const PARALLEL = 4;
/** the documents a chapter cannot open without (chapter, toon, toons list are the first three of `documentUrls`) */
export const REQUIRED_DOCS = 3;

export const manifestUrl = () => new URL(MANIFEST_PATH, location.origin).href;

/** the fonts this page loaded from the CDN – lettering needs them offline */
export function loadedFonts(): string[] {
	const cdn = new URL(asset('x')).origin;
	return performance
		.getEntriesByType('resource')
		.map((entry) => entry.name)
		.filter((name) => /\.woff2?$/i.test(new URL(name).pathname) && new URL(name).origin === cdn);
}

/**
 * One file into the chapter's cache: from the media cache when it is there, else downloaded (counted as it comes).
 * A 404 (a stale reference) is skipped – the reader shows its gap online too.
 */
export async function storeFile(cache: Cache, media: Cache, url: string, kind: string, signal: AbortSignal, progress: (bytes: number) => void): Promise<number> {
	const cached = kind === 'image' ? await media.match(url, { ignoreVary: true }) : undefined;
	if (cached) {
		const blob = await cached.blob();
		await cache.put(url, new Response(blob, { headers: { 'Content-Type': cached.headers.get('content-type') ?? blob.type, 'Content-Length': String(blob.size) } }));
		progress(blob.size);
		return blob.size;
	}
	// images: past the worker's media cache (`reload` – it would keep a second copy); audio / video / fonts: the
	// browser's HTTP cache may have them already
	const response = await fetch(url, { mode: 'cors', credentials: 'omit', cache: kind === 'image' ? 'reload' : 'default', signal });
	if (response.status === 404) return 0;
	if (!response.ok || !response.body) throw new Error(`${url} ${response.status}`);
	const reader = response.body.getReader();
	const parts: Uint8Array[] = [];
	let size = 0;
	for (;;) {
		const { done, value } = await reader.read();
		if (done) break;
		parts.push(value);
		size += value.byteLength;
		progress(value.byteLength);
	}
	const type = response.headers.get('content-type') ?? '';
	const blob = new Blob(parts as BlobPart[], { type });
	await cache.put(url, new Response(blob, { headers: { 'Content-Type': type, 'Content-Length': String(size) } }));
	return size;
}

/** many files, PARALLEL at a time */
export async function storeAll(cache: Cache, media: Cache, files: { url: string; kind: string }[], signal: AbortSignal, progress: (bytes: number, kind: string) => void) {
	let next = 0;
	const worker = async () => {
		while (next < files.length) {
			const { url, kind } = files[next++];
			await storeFile(cache, media, url, kind, signal, (bytes) => progress(bytes, kind));
		}
	};
	await Promise.all(Array.from({ length: PARALLEL }, worker));
}

/**
 * The library documents (exactly the URLs the app asks – lib/api.ts, the public client); the first REQUIRED_DOCS must
 * answer, the rest are kept when they do. All are fetched before any is stored, so a failed refresh leaves the saved
 * ones as they were; `keep`: a refresh keeps the saved copy of one that fails now.
 */
export async function storeDocs(cache: Cache, docs: string[], signal: AbortSignal, progress: (bytes: number) => void, keep: string[] = []): Promise<string[]> {
	const fetched: [string, Response][] = [];
	const kept: string[] = [];
	for (const [i, url] of docs.entries()) {
		try {
			const response = await fetch(url, { credentials: 'omit', signal });
			if (!response.ok) throw new Error(`${url} ${response.status}`);
			const blob = await response.blob();
			fetched.push([url, new Response(blob, { headers: response.headers })]);
			progress(blob.size);
			kept.push(url);
		} catch (error) {
			if (signal.aborted) throw error;
			if (keep.includes(url)) kept.push(url);
			else if (i < REQUIRED_DOCS) throw error;
		}
	}
	for (const [url, response] of fetched) await cache.put(url, response);
	return kept;
}

/** what a chapter cache holds per kind (the stored Content-Length, else the blob's size), without `skip` and the manifest */
export async function measureCache(cache: Cache, skip: Set<string>): Promise<Record<SaveKind, number>> {
	const out: Record<SaveKind, number> = { image: 0, audio: 0, video: 0, data: 0 };
	for (const request of await cache.keys()) {
		if (skip.has(request.url) || request.url === manifestUrl()) continue;
		const response = await cache.match(request);
		if (!response) continue;
		const length = Number(response.headers.get('content-length'));
		out[kindOfUrl(request.url)] += Number.isFinite(length) && length > 0 ? length : (await response.blob()).size;
	}
	return out;
}

/** the manifest – written last: a cache without one is an unfinished save */
export function writeManifest(cache: Cache, manifest: SavedManifest): Promise<void> {
	return cache.put(manifestUrl(), new Response(JSON.stringify(manifest), { headers: { 'Content-Type': 'application/json' } }));
}
