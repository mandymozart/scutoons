// The service worker's phase 2 (agents/research/PWA.md §2 "Phase 2", FEATURES 10.74): media and library JSON beside the
// app shell. What qualifies: lib/sw/cache-rules.ts (tested). Worker-side only (src/service-worker.ts wires it).
// - Media `scutoons-media-v<n>`: cache first; a miss is fetched again WITH cors (the CDN answers
//   `Access-Control-Allow-Origin: *`) so the stored copy is a readable, sized response – never an opaque one (~7 MB of
//   quota each in Chrome). LRU by bytes, capped per data tier (the page tells the tier, `DATA_TIER`). Not per build:
//   the names are immutable, so an app update keeps it; bumping MEDIA_CACHE's number drops the old one on activate.
//   The look-ahead's `new Image()` prefetches (lib/lookahead.ts) pass through here, so they land in it too.
// - Library `scutoons-library-v<n>`: younger than LIBRARY_FRESH → the cached copy at once, revalidated behind (the
//   browser's HTTP cache revalidates with the ETag – no If-None-Match of ours, it would need a CORS preflight); older →
//   the network first, the cached copy after LIBRARY_TIMEOUT or offline.
// - Audio and video never (range requests) – a chapter saved to the device holds them (lib/sw/saved-cache.ts, phase 3).
import { API_URL, CDN_SERVER } from '$lib/config.ts';
import {
	evictions,
	isCacheableAnswer,
	isLibraryRequest,
	isMediaRequest,
	LIBRARY_CACHE,
	LIBRARY_FRESH,
	LIBRARY_MAX_ENTRIES,
	LIBRARY_TIMEOUT,
	MEDIA_CACHE,
	MEDIA_MAX_AGE,
	mediaCap,
	tierFromConnection,
	type DataTier,
	type MediaEntry
} from './cache-rules.ts';
import { allEntries, deleteDatabase, deleteEntries, getEntry, getTier, putEntry, putTier } from './media-index.ts';
import { savedDoc } from './saved-cache.ts';
import { deleteOutbox } from './outbox.ts';

/** the worker's FetchEvent, as far as used here (this file is type-checked with the DOM lib, not webworker) */
interface FetchEvent {
	request: Request;
	waitUntil(promise: Promise<unknown>): void;
}

// the names (bump the number when the stored form changes: activate deletes every other version) – in cache-rules.ts,
// so the page (saving a chapter copies from the media cache) needn't load this worker file
export { LIBRARY_CACHE, MEDIA_CACHE };
const RUNTIME = /^scutoons-(media|library)-/;
const CDN_ORIGIN = new URL(CDN_SERVER).origin;
/** when a library answer was stored (ms) – our own header on the stored copy */
const STORED_AT = 'x-scutoons-stored';
/** a hit's access time is written at most this often */
const TOUCH_EVERY = 60 * 1000;

export const isMedia = (request: Request) => isMediaRequest(request, CDN_ORIGIN);
export const isLibrary = (request: Request) => isLibraryRequest(request, API_URL);

let tier: DataTier | null = null;

async function currentTier(): Promise<DataTier> {
	if (!tier) tier = (await getTier().catch(() => undefined)) ?? null;
	const connection = (navigator as Navigator & { connection?: { saveData?: boolean; effectiveType?: string } }).connection;
	// data saver switched on later beats what the page said
	if (connection?.saveData) return 'none';
	return tier ?? tierFromConnection(connection);
}

/** the page's data tier (lib/data-budget.svelte.ts → `DATA_TIER`) */
export async function setTier(next: DataTier) {
	if (next !== tier) {
		tier = next;
		await putTier(next).catch(() => {});
	}
	// also when unchanged: told once per page load, a cheap check that the cache still fits
	await evict();
}

// ---------- media ----------

/** bytes in the media cache as far as this worker knows (null: not counted yet) */
let total: number | null = null;
let evicting: Promise<void> | null = null;

/** shrink the media cache to the tier's cap (least recently used first); `none` keeps what it has */
export function evict(): Promise<void> {
	evicting ??= (async () => {
		try {
			const current = await currentTier();
			const entries = await allEntries();
			total = entries.reduce((sum, e) => sum + e.size, 0);
			if (current === 'none') return;
			const quota = (await navigator.storage?.estimate?.().catch(() => undefined))?.quota;
			const drop = evictions(entries, mediaCap(current, quota));
			if (!drop.length) return;
			const cache = await caches.open(MEDIA_CACHE);
			for (const url of drop) await cache.delete(url);
			await deleteEntries(drop);
			const gone = new Set(drop);
			total = entries.filter((e) => !gone.has(e.url)).reduce((sum, e) => sum + e.size, 0);
		} catch {
			/* no bookkeeping (private mode): nothing to evict by */
		} finally {
			evicting = null;
		}
	})();
	return evicting;
}

async function keepMedia(cache: Cache, url: string, response: Response) {
	const size = (await response.clone().blob()).size;
	await cache.put(url, response);
	const now = Date.now();
	await putEntry({ url, size, used: now, fetched: now }).catch(() => {});
	if (total === null) return evict();
	total += size;
	const current = await currentTier();
	if (total > mediaCap(current)) await evict();
}

async function touch(url: string, entry: MediaEntry | undefined, hit: Response) {
	const now = Date.now();
	if (entry && now - entry.used < TOUCH_EVERY) return;
	// bookkeeping lost (or never written): note it now, as the oldest fetch
	const next = entry ?? { url, size: (await hit.clone().blob()).size, used: now, fetched: now };
	await putEntry({ ...next, used: now }).catch(() => {});
}

/** the CDN file with cors (readable, sized); without cors (a CDN without the header) the browser's own request, not kept */
async function fetchMedia(event: FetchEvent, cache: Cache): Promise<Response> {
	const { request } = event;
	let response: Response;
	try {
		response = await fetch(new Request(request, { mode: 'cors', credentials: 'omit' }));
	} catch {
		return fetch(request);
	}
	if (isCacheableAnswer(response, 'media') && (await currentTier()) !== 'none')
		event.waitUntil(keepMedia(cache, request.url, response.clone()).catch(() => {}));
	return response;
}

export async function media(event: FetchEvent): Promise<Response> {
	const url = event.request.url;
	const cache = await caches.open(MEDIA_CACHE);
	const hit = await cache.match(url, { ignoreVary: true });
	if (!hit) return fetchMedia(event, cache);
	const entry = await getEntry(url).catch(() => undefined);
	if (entry && Date.now() - entry.fetched > MEDIA_MAX_AGE) {
		// old: fetched again (a 304 from the HTTP cache costs nothing); offline → the old copy
		try {
			const fresh = await fetchMedia(event, cache);
			if (fresh.ok) return fresh;
		} catch {
			/* offline */
		}
	}
	event.waitUntil(touch(url, entry, hit).catch(() => {}));
	return hit;
}

/** the images the page loaded before this worker controlled it (`CACHE_MEDIA`, first install) – from the HTTP cache */
export async function cacheShown(urls: unknown) {
	if (!Array.isArray(urls) || (await currentTier()) === 'none') return;
	const cache = await caches.open(MEDIA_CACHE);
	for (const url of urls.slice(0, 200)) {
		if (typeof url !== 'string') continue;
		const request = new Request(url, { mode: 'cors', credentials: 'omit', cache: 'force-cache' });
		if (!isMedia(request) || (await cache.match(url))) continue;
		try {
			const response = await fetch(request);
			if (isCacheableAnswer(response, 'media')) await keepMedia(cache, url, response);
		} catch {
			/* offline / no cors: cached on its next use */
		}
	}
}

// ---------- library JSON ----------

async function keepLibrary(cache: Cache, url: string, response: Response) {
	const headers = new Headers(response.headers);
	headers.set(STORED_AT, String(Date.now()));
	await cache.put(url, new Response(await response.blob(), { status: response.status, statusText: response.statusText, headers }));
	const keys = await cache.keys();
	for (const old of keys.slice(0, Math.max(0, keys.length - LIBRARY_MAX_ENTRIES))) await cache.delete(old);
}

export async function library(event: FetchEvent): Promise<Response> {
	const { request } = event;
	const url = request.url;
	const cache = await caches.open(LIBRARY_CACHE);
	// a chapter saved to the device keeps its own copy (phase 3) – the fallback when this cache has none
	const hit = (await cache.match(url, { ignoreVary: true })) ?? (await savedDoc(url).catch(() => null));
	const network = fetch(request).then(async (response) => {
		if (isCacheableAnswer(response, 'library')) await keepLibrary(cache, url, response.clone()).catch(() => {});
		// gone or closed now (404 / 403 coming_soon, not_public): the stored copy goes too
		else if (response.status >= 400 && response.status < 500) await cache.delete(url);
		return response;
	});
	if (!hit) return network;
	event.waitUntil(network.then(() => {}, () => {}));
	if (Date.now() - Number(hit.headers.get(STORED_AT) || 0) < LIBRARY_FRESH) return hit;
	const fallback = network.then(
		(response) => (response.status >= 500 ? hit : response),
		() => hit
	);
	return Promise.race([fallback, new Promise<Response>((resolve) => setTimeout(() => resolve(hit), LIBRARY_TIMEOUT))]);
}

// ---------- lifecycle ----------

/** activate: other versions of the runtime caches go; the bookkeeping is reconciled with the media cache; evict */
export async function cleanUp() {
	for (const name of await caches.keys())
		if (RUNTIME.test(name) && name !== MEDIA_CACHE && name !== LIBRARY_CACHE) await caches.delete(name);
	try {
		const cache = await caches.open(MEDIA_CACHE);
		const cached = new Set((await cache.keys()).map((r) => r.url));
		const entries = await allEntries();
		const known = new Set(entries.map((e) => e.url));
		await deleteEntries(entries.filter((e) => !cached.has(e.url)).map((e) => e.url));
		// files without bookkeeping: noted as the least recently used
		for (const url of cached) {
			if (known.has(url)) continue;
			const response = await cache.match(url);
			if (response) await putEntry({ url, size: (await response.blob()).size, used: 0, fetched: Date.now() });
		}
	} catch {
		/* no IndexedDB: the cache works without LRU */
	}
	total = null;
	await evict();
}

/** the kill switch: every SCUTOONS cache (shell, media, library, the saved chapters) and the bookkeeping */
export async function clearAll() {
	for (const name of await caches.keys()) if (name.startsWith('scutoons-')) await caches.delete(name);
	await deleteDatabase();
	// the queued reading progress (lib/sw/outbox.ts) – the save game itself stays in the page's localStorage
	await deleteOutbox();
}
