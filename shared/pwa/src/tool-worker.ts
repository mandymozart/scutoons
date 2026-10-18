// A tool's service worker (creator, admin, embassy – agents/research/PWA-APPS.md §2): what makes the app installable and
// start fast, NOTHING else. Each app's src/service-worker.ts calls `startToolWorker(self, …)`.
// - index.html: network first; this build's copy (fetched at install) only when the network fails – the app then shows
//   its own "no connection" state (every tool needs the API)
// - /_app/immutable/*: cache first (hashed names never change), cached on first use; index.html's own files at install;
//   unchanged files are carried over from the previous version's cache
// - the manifest, the app icons, the favicon: network first, the cache when offline
// - never: the API, the CDN, other origins, non-GET requests – a tool shows live data or nothing (no content cache)
// - Web Push: the team's notifications (./push-worker.ts – a tap opens the page in this app)
// Updates: a new worker waits until the page says SKIP_WAITING (./tool-pwa.svelte.ts, the update bar); the very first
// install takes the open page at once. Kill switch: `off` (VITE_SW=off) – the worker deletes this app's caches and
// unregisters itself.

import { registerPush, type PushScope } from './push-worker.ts';

/** the bits of the worker scope this file uses (the app's service-worker.ts passes `self`) */
export interface WorkerScope {
	addEventListener(type: 'install' | 'activate', listener: (event: ExtendableEventLike) => void): void;
	addEventListener(type: 'fetch', listener: (event: FetchEventLike) => void): void;
	addEventListener(type: 'message', listener: (event: MessageEventLike) => void): void;
	skipWaiting(): Promise<void>;
	clients: { claim(): Promise<void> };
	registration: { active: unknown; unregister(): Promise<boolean> };
	location: { origin: string };
}
interface ExtendableEventLike {
	waitUntil(promise: Promise<unknown>): void;
}
interface FetchEventLike extends ExtendableEventLike {
	request: Request;
	respondWith(response: Response | Promise<Response>): void;
}
interface MessageEventLike extends ExtendableEventLike {
	data: unknown;
	ports: readonly MessagePort[];
}

export interface ToolWorkerOptions {
	/** the app's name in the cache names: `scutoons-<app>-shell-<version>` */
	app: string;
	/** $service-worker `build`, `files`, `version` */
	build: string[];
	files: string[];
	version: string;
	/** the kill switch (VITE_SW=off) */
	off?: boolean;
}

/** the static files a tool's first paint and install need (the rest of static/ stays network only) */
export const TOOL_STATIC = /^\/(site\.webmanifest|favicon[^/]*|app-icon-[^/]+|SCUTOONS-logo[^/]*\.svg)$/;

export function startToolWorker(sw: WorkerScope & PushScope, options: ToolWorkerOptions) {
	const prefix = `scutoons-${options.app}-`;
	const cacheName = `${prefix}shell-${options.version}`;
	const build = new Set(options.build);
	const statics = new Set(options.files.filter((f) => TOOL_STATIC.test(f)));

	if (options.off) {
		sw.addEventListener('install', () => void sw.skipWaiting());
		sw.addEventListener('activate', (event) =>
			event.waitUntil(
				(async () => {
					for (const name of await caches.keys()) if (name.startsWith(prefix)) await caches.delete(name);
					await sw.registration.unregister();
				})()
			)
		);
		return;
	}

	registerPush(sw);

	sw.addEventListener('install', (event) =>
		event.waitUntil(
			(async () => {
				const cache = await caches.open(cacheName);
				const index = await fetch('/', { cache: 'reload', credentials: 'same-origin' });
				if (!index.ok) throw new Error(`index.html answered ${index.status}`);
				const html = await index.clone().text();
				const refs = [...new Set(html.match(/\/_app\/immutable\/[^"'`\s)]+/g) ?? [])];
				// a deploy uploads file by file: an index.html of another build → install fails, the next check tries again
				const stranger = refs.find((r) => !build.has(r));
				if (stranger) throw new Error(`index.html is not this build's (${stranger})`);
				await cache.put('/', index);
				// carried over from the previous version (same hashed name = same content), else fetched
				const old = (await caches.keys()).filter((n) => n.startsWith(`${prefix}shell-`) && n !== cacheName);
				for (const path of [...refs, ...statics]) {
					let response: Response | undefined;
					for (const name of old) if ((response = await (await caches.open(name)).match(path))) break;
					if (!response) response = await fetch(path, { credentials: 'same-origin' }).catch(() => undefined);
					if (response?.ok) await cache.put(path, response);
				}
				// the first worker takes the open page at once; an update waits for the page's "Reload"
				if (!sw.registration.active) await sw.skipWaiting();
			})()
		)
	);

	sw.addEventListener('activate', (event) =>
		event.waitUntil(
			(async () => {
				for (const name of await caches.keys()) if (name.startsWith(`${prefix}shell-`) && name !== cacheName) await caches.delete(name);
				await sw.clients.claim();
			})()
		)
	);

	sw.addEventListener('fetch', (event) => {
		const { request } = event;
		if (request.method !== 'GET') return;
		const url = new URL(request.url);
		if (url.origin !== sw.location.origin) return;
		if (request.mode === 'navigate') return event.respondWith(networkFirst(request, '/'));
		if (build.has(url.pathname)) return event.respondWith(cacheFirst(request, url.pathname));
		if (statics.has(url.pathname)) return event.respondWith(networkFirst(request, url.pathname));
	});

	sw.addEventListener('message', (event) => {
		const data = event.data as { type?: string } | null;
		if (data?.type === 'SKIP_WAITING') void sw.skipWaiting();
		else if (data?.type === 'VERSION') event.ports[0]?.postMessage(options.version);
	});

	async function networkFirst(request: Request, key: string): Promise<Response> {
		try {
			return await fetch(request);
		} catch (error) {
			const cached = await (await caches.open(cacheName)).match(key);
			if (cached) return cached;
			throw error;
		}
	}

	async function cacheFirst(request: Request, path: string): Promise<Response> {
		const cache = await caches.open(cacheName);
		const cached = await cache.match(path);
		if (cached) return cached;
		const response = await fetch(request);
		if (response.ok) await cache.put(path, response.clone());
		return response;
	}
}
