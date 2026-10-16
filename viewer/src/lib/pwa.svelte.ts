// The service worker's page side (agents/research/PWA.md, FEATURES 10.74 phase 1): registers src/service-worker.ts,
// offers a new version with the update prompt (components/update-prompt.svelte) instead of a silent stale app, and
// carries out the kill switch (a VITE_SW=off build unregisters every worker and drops its caches).
import { browser, dev, version } from '$app/environment';

const CACHE_PREFIX = 'scutoons-shell-';
/** an SPA never navigates: look for a new worker when the reader comes back, at most this often */
const CHECK_EVERY = 30 * 60 * 1000;

/** the app files this page already loaded – the first worker caches them (from the HTTP cache), so offline works at once */
function loadedAppFiles(): string[] {
	return performance
		.getEntriesByType('resource')
		.map((entry) => new URL(entry.name))
		.filter((url) => url.origin === location.origin && url.pathname.startsWith('/_app/immutable/'))
		.map((url) => url.pathname);
}

/** the build a waiting worker belongs to (null: no answer within 2 s) */
function workerVersion(worker: ServiceWorker): Promise<string | null> {
	return new Promise((resolve) => {
		const channel = new MessageChannel();
		const timer = setTimeout(() => resolve(null), 2000);
		channel.port1.onmessage = (event) => {
			clearTimeout(timer);
			resolve(typeof event.data === 'string' ? event.data : null);
		};
		worker.postMessage({ type: 'VERSION' }, [channel.port2]);
	});
}

/** every worker and cache of this origin away (the kill switch; also by hand: PWA.md) */
export async function removeServiceWorker() {
	for (const registration of await navigator.serviceWorker.getRegistrations()) await registration.unregister();
	for (const name of await caches.keys()) if (name.startsWith(CACHE_PREFIX)) await caches.delete(name);
}

class Pwa {
	/** a new version is installed and waits for "Reload" */
	updateReady = $state(false);
	#waiting: ServiceWorker | null = null;
	#reloading = false;
	#lastCheck = Date.now();

	async init() {
		// not in dev (the dev server, HMR), not in the creator's Reader frame
		if (!browser || dev || !('serviceWorker' in navigator) || window.top !== window) return;
		if (__SW_OFF__) return removeServiceWorker().catch(() => {});
		let registration: ServiceWorkerRegistration;
		try {
			// updateViaCache none: the browser always asks the server for service-worker.js (also without our headers)
			registration = await navigator.serviceWorker.register('/service-worker.js', { updateViaCache: 'none' });
		} catch {
			return; // private mode, blocked storage: the reader works as before
		}
		if (registration.waiting && navigator.serviceWorker.controller) void this.#offer(registration.waiting);
		registration.addEventListener('updatefound', () => {
			const worker = registration.installing;
			worker?.addEventListener('statechange', () => {
				// the first install (no controller yet) has nothing to offer
				if (worker.state === 'installed' && navigator.serviceWorker.controller) void this.#offer(worker);
			});
		});
		navigator.serviceWorker.addEventListener('controllerchange', () => {
			if (this.#reloading) location.reload();
		});
		navigator.serviceWorker.ready.then((ready) => ready.active?.postMessage({ type: 'CACHE_LOADED', urls: loadedAppFiles() }));
		const check = () => {
			if (Date.now() - this.#lastCheck < CHECK_EVERY) return;
			this.#lastCheck = Date.now();
			registration.update().catch(() => {});
		};
		document.addEventListener('visibilitychange', () => document.visibilityState === 'visible' && check());
		setInterval(check, CHECK_EVERY);
	}

	async #offer(worker: ServiceWorker) {
		this.#waiting = worker;
		// this page already runs the waiting worker's build (a reload fetched the new index.html): take over silently
		if ((await workerVersion(worker)) === version) return worker.postMessage({ type: 'SKIP_WAITING' });
		this.updateReady = true;
	}

	/** "Reload": the new worker takes over, then the page reloads with the new version */
	apply() {
		this.#reloading = true;
		this.updateReady = false;
		if (!this.#waiting) return location.reload();
		this.#waiting.postMessage({ type: 'SKIP_WAITING' });
		// never stuck on the prompt: reload anyway when the takeover doesn't come
		setTimeout(() => location.reload(), 3000);
	}

	/** "Later": the prompt comes back on the next start */
	dismiss() {
		this.updateReady = false;
	}
}

export const pwa = new Pwa();
