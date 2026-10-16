// The service worker's phase 3 (agents/research/PWA.md §2 "Phase 3", FEATURES 10.74): answers from the chapters the
// reader saved to the device. The page fills the caches (lib/offline/saved-chapters.svelte.ts); this file only reads
// them. Rules: lib/sw/saved-rules.ts (tested).
// - CDN files of a saved chapter (images, music, samples, videos, fonts): from its cache, before the media cache and the
//   network; a `Range` request (a streamed track, a video) gets 206 with the bytes from the whole cached file.
// - Library JSON + the item catalog: the network first (phase 2's rules), the saved copy when it fails.
// The index (URL → cache) is read from the caches' manifests at start and whenever the page says `SAVED_CHANGED`.
import { CDN_SERVER } from '$lib/config.ts';
import { isSavedCacheName, isStaleSavedCacheName, MANIFEST_PATH, maySaveStreamed, parseRange, type SavedManifest } from './saved-rules.ts';

interface FetchEvent {
	request: Request;
}

const CDN_ORIGIN = new URL(CDN_SERVER).origin;

/** media URL → the cache holding it; doc URL → the cache holding it; another level's URL → the saved URL */
let media: Map<string, string> | null = null;
let docs: Map<string, string> | null = null;
let aliases: Map<string, string> | null = null;
let loading: Promise<void> | null = null;

/** reads every saved chapter's manifest (unfinished saves – no manifest – are ignored; the page tidies them) */
export function reloadIndex(): Promise<void> {
	loading = (async () => {
		const nextMedia = new Map<string, string>();
		const nextDocs = new Map<string, string>();
		const nextAliases = new Map<string, string>();
		try {
			for (const name of await caches.keys()) {
				if (!isSavedCacheName(name)) continue;
				if (isStaleSavedCacheName(name)) {
					await caches.delete(name);
					continue;
				}
				const cache = await caches.open(name);
				const manifest = (await (await cache.match(new URL(MANIFEST_PATH, location.origin).href))?.json().catch(() => null)) as SavedManifest | null;
				if (!manifest) continue;
				for (const url of manifest.urls ?? []) nextMedia.set(url, name);
				for (const url of manifest.docs ?? []) nextDocs.set(url, name);
				for (const [from, to] of Object.entries(manifest.aliases ?? {})) nextAliases.set(from, to);
			}
		} catch {
			/* no Cache API: nothing saved */
		}
		media = nextMedia;
		docs = nextDocs;
		aliases = nextAliases;
	})();
	return loading;
}

const ready = () => (loading ??= reloadIndex());

/**
 * Whether a CDN request may be a saved chapter's file – decided synchronously (respondWith must be called at once):
 * known index → exact; before it is read → any audio / video / font (the rest goes through the media cache anyway).
 */
export function maySaved(request: Request, url: URL): boolean {
	// the media-quality probe (no-store) measures the network – never answered from the device
	if (url.origin !== CDN_ORIGIN || request.cache === 'no-store') return false;
	if (media && aliases) return media.has(url.href) || aliases.has(url.href);
	return maySaveStreamed(url, CDN_ORIGIN);
}

/** a saved file for this request (a range → 206 from the whole file), null when no saved chapter holds it */
export async function savedAnswer(request: Request): Promise<Response | null> {
	if (request.cache === 'no-store') return null;
	await ready();
	// another level's file of a saved media: the saved one stands in
	const url = media?.has(request.url) ? request.url : (aliases?.get(request.url) ?? request.url);
	const name = media?.get(url);
	if (!name) return null;
	const hit = await (await caches.open(name)).match(url, { ignoreVary: true });
	if (!hit) return null;
	const range = request.headers.get('range');
	if (!range) return hit;
	const blob = await hit.blob();
	const bytes = parseRange(range, blob.size);
	const type = hit.headers.get('content-type') ?? blob.type ?? 'application/octet-stream';
	if (bytes === null) return new Response(blob, { status: 200, headers: { 'Content-Type': type, 'Content-Length': String(blob.size), 'Accept-Ranges': 'bytes' } });
	if (bytes === 'unsatisfiable') return new Response(null, { status: 416, headers: { 'Content-Range': `bytes */${blob.size}` } });
	const part = blob.slice(bytes.start, bytes.end + 1);
	return new Response(part, {
		status: 206,
		statusText: 'Partial Content',
		headers: {
			'Content-Type': type,
			'Content-Length': String(part.size),
			'Content-Range': `bytes ${bytes.start}-${bytes.end}/${blob.size}`,
			'Accept-Ranges': 'bytes'
		}
	});
}

/** a CDN request: the saved copy first, else `next` (the media cache for images, the network for the rest) */
export async function savedFirst(event: FetchEvent, next: () => Promise<Response>): Promise<Response> {
	try {
		const saved = await savedAnswer(event.request);
		if (saved) return saved;
	} catch {
		/* a broken saved entry: the normal way */
	}
	return next();
}

/** the saved copy of a library / catalog answer (the offline fallback), null when none */
export async function savedDoc(url: string): Promise<Response | null> {
	await ready();
	const name = docs?.get(url);
	if (!name) return null;
	return (await (await caches.open(name)).match(url, { ignoreVary: true })) ?? null;
}

/** the item catalog / editions: the network first, the saved copy when it fails (offline) */
export async function networkThenSaved(event: FetchEvent): Promise<Response> {
	try {
		const response = await fetch(event.request);
		if (response.status < 500) return response;
		return (await savedDoc(event.request.url)) ?? response;
	} catch (error) {
		const saved = await savedDoc(event.request.url);
		if (saved) return saved;
		throw error;
	}
}
