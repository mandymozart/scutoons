/// <reference types="@sveltejs/kit" />
/// <reference no-default-lib="true"/>
/// <reference lib="esnext" />
/// <reference lib="webworker" />
// The reader's service worker (agents/research/PWA.md, FEATURES 10.74). Phase 1: the app shell (this file).
// Phase 2: slice images / posters / portraits / item icons from the CDN and the public library JSON (lib/sw/runtime-cache.ts,
// the rules in lib/sw/cache-rules.ts) – never auth, sessions, inventory, game state, codes, purchases, signed URLs.
// - index.html: network first; the cached copy (this build's, fetched at install) only when the network fails (offline)
// - /_app/immutable/*: this build's files from the cache (precached: what index.html needs + the root layout / error
//   nodes + CSS, ≈ 25 KB br; not the bundled fonts – they come from the CDN; the rest is cached when the app first
//   uses it – never a guess, LOADING.md data budget); unchanged files are carried over from the previous version's cache, so a deploy downloads only what changed
// - the few static files of the first paint: network first, the cache when offline
// - Phase 3: the chapters saved to the device (lib/sw/saved-cache.ts) – their files (also audio / video, ranges → 206)
//   and their library JSON + item catalog as the offline copy
// - NOTHING else: the rest of the API, audio / video, the vault's signed URLs, auth – not touched (the browser's own fetch)
// Updates: a new worker waits until the reader taps "Reload" (lib/pwa.svelte.ts, the update prompt).
// Kill switch: a build with VITE_SW=off makes this file remove itself and its caches (PWA.md "Kill switch").
import { build, files, version } from '$service-worker';
import { isDataTier } from '$lib/sw/cache-rules.ts';
import { cacheShown, cleanUp, clearAll, isLibrary, isMedia, library, media, setTier } from '$lib/sw/runtime-cache.ts';
import { maySaved, networkThenSaved, reloadIndex, savedFirst } from '$lib/sw/saved-cache.ts';
import { isSavedDocRequest } from '$lib/sw/saved-rules.ts';
import { API_URL } from '$lib/config.ts';

const sw = self as unknown as ServiceWorkerGlobalScope;

const PREFIX = 'scutoons-shell-';
const CACHE = PREFIX + version;
/** the key of this build's index.html */
const SHELL = '/';
const BUILD = new Set(build);
/** static files the boot screen and the first paint use; the rest of static/ only from the network */
const STATIC = files.filter((f) =>
	/^\/(css\/[^/]+\.css|SCUTOONS-logo-animated\.svg|site\.webmanifest|favicon[^/]*|android-chrome-[^/]+|icon-maskable-[^/]+|apple-touch-icon\.png)$/.test(f)
);
const STATIC_SET = new Set(STATIC);

/** data saver / 2G: precache only what the page has already loaded (index.html's files, from the HTTP cache) */
function frugal(): boolean {
	const connection = (sw.navigator as Navigator & { connection?: { saveData?: boolean; effectiveType?: string } }).connection;
	return !!connection?.saveData || /2g/.test(connection?.effectiveType ?? '');
}

async function install() {
	const cache = await caches.open(CACHE);
	const index = await fetch(SHELL, { cache: 'reload', credentials: 'same-origin' });
	if (!index.ok) throw new Error(`index.html answered ${index.status}`);
	const html = await index.clone().text();
	const refs = [...new Set(html.match(/\/_app\/immutable\/[^"'`\s)]+/g) ?? [])];
	// a deploy uploads file by file: an index.html of another build → install fails, the next update check tries again
	const stranger = refs.find((r) => !BUILD.has(r));
	if (stranger) throw new Error(`index.html is not this build's (${stranger})`);
	await cache.put(SHELL, index);

	// carry over what the previous versions cached and this build still has (same hashed name = same content)
	const have = new Set<string>();
	for (const name of await caches.keys()) {
		if (name === CACHE || !name.startsWith(PREFIX)) continue;
		const old = await caches.open(name);
		for (const request of await old.keys()) {
			const path = new URL(request.url).pathname;
			if (!BUILD.has(path) || have.has(path)) continue;
			const response = await old.match(request);
			if (response) {
				await cache.put(path, response);
				have.add(path);
			}
		}
	}

	const wanted = new Set([
		...refs,
		...build.filter((f) => /\/nodes\/[01]\.[^/]+\.js$/.test(f)),
		...(frugal() ? [] : build.filter((f) => f.endsWith('.css')))
	]);
	await cache.addAll([...wanted].filter((f) => !have.has(f)));
	// not hashed: revalidated (a 304 costs nothing)
	await cache.addAll(STATIC.map((f) => new Request(f, { cache: 'no-cache' })));
}

async function activate() {
	for (const name of await caches.keys()) if (name.startsWith(PREFIX) && name !== CACHE) await caches.delete(name);
	// the media / library caches outlive builds (immutable names); only their own version bump replaces them
	await cleanUp().catch(() => {});
	await reloadIndex();
	await sw.registration.navigationPreload?.enable();
	// the first install takes over the open page, so the app files it loads from now on are cached (an update only
	// activates by the reader's "Reload", lib/pwa.svelte.ts)
	await sw.clients.claim();
}

async function navigate(event: FetchEvent): Promise<Response> {
	try {
		const preloaded = (await event.preloadResponse) as Response | undefined;
		return preloaded ?? (await fetch(event.request));
	} catch {
		const cache = await caches.open(CACHE);
		return (await cache.match(SHELL)) ?? Response.error();
	}
}

async function immutable(request: Request, path: string): Promise<Response> {
	const cache = await caches.open(CACHE);
	const hit = await cache.match(path);
	if (hit) return hit;
	const response = await fetch(request);
	if (response.ok && response.type === 'basic') await cache.put(path, response.clone()).catch(() => {});
	return response;
}

async function staticFile(request: Request, path: string): Promise<Response> {
	try {
		return await fetch(request);
	} catch {
		const cache = await caches.open(CACHE);
		return (await cache.match(path)) ?? Response.error();
	}
}

/** the app files the page loaded before this worker controlled it (first install) – from the HTTP cache */
async function cacheLoaded(urls: unknown) {
	if (!Array.isArray(urls)) return;
	const cache = await caches.open(CACHE);
	for (const url of urls) {
		if (typeof url !== 'string' || !BUILD.has(url) || (await cache.match(url))) continue;
		try {
			const response = await fetch(url, { cache: 'force-cache' });
			if (response.ok) await cache.put(url, response);
		} catch {
			/* offline: cached on its next use */
		}
	}
}

if (__SW_OFF__) {
	// the kill switch: no fetch handler (everything from the network at once), caches gone, registration gone – the
	// open pages keep running and are uncontrolled from their next load on
	sw.addEventListener('install', () => void sw.skipWaiting());
	sw.addEventListener('activate', (event) => {
		event.waitUntil(
			(async () => {
				// every SCUTOONS cache (shell, media, library) and the media bookkeeping (IndexedDB scutoons-sw)
				await clearAll();
				await sw.registration.unregister();
			})()
		);
	});
} else {
	sw.addEventListener('install', (event) => event.waitUntil(install()));
	sw.addEventListener('activate', (event) => event.waitUntil(activate()));

	sw.addEventListener('fetch', (event) => {
		const { request } = event;
		if (request.method !== 'GET') return;
		const url = new URL(request.url);
		if (url.origin !== location.origin) {
			// a saved chapter's file first (phase 3), then the media cache (images) / the network
			if (isMedia(request)) return event.respondWith(savedFirst(event, () => media(event)));
			if (maySaved(request, url)) return event.respondWith(savedFirst(event, () => fetch(request)));
			if (isLibrary(request)) return event.respondWith(library(event));
			if (isSavedDocRequest(request, API_URL)) return event.respondWith(networkThenSaved(event));
			return;
		}
		if (request.mode === 'navigate') return event.respondWith(navigate(event));
		if (BUILD.has(url.pathname)) return event.respondWith(immutable(request, url.pathname));
		if (STATIC_SET.has(url.pathname)) return event.respondWith(staticFile(request, url.pathname));
	});

	sw.addEventListener('message', (event) => {
		const data = event.data as { type?: string; urls?: unknown; tier?: unknown } | null;
		if (data?.type === 'SKIP_WAITING') void sw.skipWaiting();
		else if (data?.type === 'VERSION') event.ports[0]?.postMessage(version);
		else if (data?.type === 'CACHE_LOADED') event.waitUntil(cacheLoaded(data.urls));
		else if (data?.type === 'DATA_TIER' && isDataTier(data.tier)) event.waitUntil(setTier(data.tier));
		else if (data?.type === 'CACHE_MEDIA') event.waitUntil(cacheShown(data.urls));
		// a chapter was saved to / removed from the device (lib/offline/saved-chapters.svelte.ts)
		else if (data?.type === 'SAVED_CHANGED') event.waitUntil(reloadIndex());
	});
}
