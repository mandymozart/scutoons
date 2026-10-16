// What the service worker may keep, and how much (agents/research/PWA.md §2 "Phase 2", FEATURES 10.74). Pure – no
// worker APIs – so the rules are tested (test/sw-cache-rules.test.ts) and src/service-worker.ts only carries them out.
//
// - Media (cdn.: slice images, posters, portraits, item icons, map tiles): images only, the public CDN only, never a
//   URL with a query (the vault's signed URLs carry ?token=…&expires=…, and live on another host anyway), never a
//   range / no-store request (the media-quality probe), never audio / video. Cache first, LRU by size per data tier.
// - Library JSON (api. /library/… public reads): only the header-free public client's requests (no credentials, no
//   Authorization, no device id), never `draft`, only answers marked `public` – auth, sessions, inventory, game state,
//   codes, purchases, signed URLs never qualify (they live outside /library/ or answer `no-store`).

export type DataTier = 'none' | 'light' | 'normal' | 'generous';

const MB = 1024 * 1024;

/** the media cache's size per data tier (lib/data-budget.svelte.ts tiers); `none` keeps nothing new */
export const MEDIA_CAPS: Record<DataTier, number> = {
	none: 0,
	light: 50 * MB,
	normal: 150 * MB,
	generous: 150 * MB
};

/** never more than this share of what the browser grants the origin (navigator.storage.estimate().quota) */
export const QUOTA_SHARE = 0.25;

/** a cached media file is fetched again after this long (names are immutable by rule – the CDN's own max-age) */
export const MEDIA_MAX_AGE = 30 * 24 * 60 * 60 * 1000;

/** the library JSON: younger than this → straight from the cache (revalidated behind); older → the network first */
export const LIBRARY_FRESH = 10 * 60 * 1000;
/** older than FRESH: how long the network may take before the cached copy answers */
export const LIBRARY_TIMEOUT = 3000;
/** at most this many library answers are kept (oldest written first out) */
export const LIBRARY_MAX_ENTRIES = 200;

const IMAGE = /\.(webp|png|jpe?g|avif|gif|svg)$/i;

export function isDataTier(value: unknown): value is DataTier {
	return value === 'none' || value === 'light' || value === 'normal' || value === 'generous';
}

/** the worker's own guess until the page tells it the tier (the same signals as tierOf, without the media quality) */
export function tierFromConnection(c: { saveData?: boolean; effectiveType?: string } | undefined): DataTier {
	if (c?.saveData || c?.effectiveType === 'slow-2g' || c?.effectiveType === '2g') return 'none';
	if (c?.effectiveType === '3g') return 'light';
	return 'normal';
}

/** the cap in bytes: the tier's, but never more than a share of the quota */
export function mediaCap(tier: DataTier, quota?: number): number {
	const cap = MEDIA_CAPS[tier];
	return quota && quota > 0 ? Math.min(cap, Math.floor(quota * QUOTA_SHARE)) : cap;
}

export interface RequestLike {
	url: string;
	method: string;
	mode?: string;
	credentials?: string;
	cache?: string;
	headers: { has(name: string): boolean; get(name: string): string | null };
}

/** a media request the cache may answer / keep: a GET for an image on the public CDN, whole file, no query */
export function isMediaRequest(request: RequestLike, cdnOrigin: string): boolean {
	if (request.method !== 'GET' || request.mode === 'navigate') return false;
	if (request.cache === 'no-store' || request.cache === 'reload') return false;
	// an <img> without crossorigin asks no-cors with credentials `include` (the default) – the worker fetches it again
	// with cors and no credentials (the CDN sets no cookies); only an explicit crossorigin=use-credentials stays out
	if (request.mode === 'cors' && request.credentials === 'include') return false;
	if (request.headers.has('range') || request.headers.has('authorization')) return false;
	const url = new URL(request.url);
	return url.origin === cdnOrigin && url.search === '' && IMAGE.test(url.pathname);
}

/** a public library read: GET <api>/library/…, the header-free client, not a draft */
export function isLibraryRequest(request: RequestLike, apiUrl: string): boolean {
	if (request.method !== 'GET') return false;
	if (request.credentials === 'include' || request.cache === 'no-store' || request.cache === 'reload') return false;
	if (request.headers.has('authorization') || request.headers.has('x-device-id')) return false;
	const url = new URL(request.url);
	const api = new URL(apiUrl);
	if (url.origin !== api.origin) return false;
	const base = api.pathname.replace(/\/+$/, '');
	if (!url.pathname.startsWith(`${base}/library/`)) return false;
	return !url.searchParams.has('draft');
}

/** an answer that may be kept: a whole 200 that says it is public (private / no-store answers never) */
export function isCacheableAnswer(response: { status: number; type: string; headers: { get(name: string): string | null } }, kind: 'media' | 'library'): boolean {
	if (response.status !== 200) return false;
	// opaque answers (no-cors) cost ~7 MB of quota each in Chrome and can't be checked – never kept
	if (response.type !== 'cors' && response.type !== 'basic') return false;
	const control = (response.headers.get('cache-control') ?? '').toLowerCase();
	if (/no-store|private/.test(control)) return false;
	if (kind === 'library') return /\bpublic\b/.test(control);
	return true;
}

export interface MediaEntry {
	url: string;
	size: number;
	/** last time the reader's page asked for it (ms) */
	used: number;
	/** when it was fetched (ms) */
	fetched: number;
}

/**
 * Which entries go so the cache fits `cap`: the least recently used first, down to 90 % of the cap (so the next
 * images don't evict one by one). Returns the URLs to delete.
 */
export function evictions(entries: MediaEntry[], cap: number): string[] {
	let total = entries.reduce((sum, e) => sum + e.size, 0);
	if (total <= cap) return [];
	const target = cap * 0.9;
	const out: string[] = [];
	for (const entry of [...entries].sort((a, b) => a.used - b.used)) {
		if (total <= target) break;
		out.push(entry.url);
		total -= entry.size;
	}
	return out;
}
