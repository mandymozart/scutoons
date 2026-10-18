// A tool's service worker, page side (creator, admin, embassy – agents/research/PWA-APPS.md §2): registers
// /service-worker.js (./tool-worker.ts), says when a new version waits (the update bar, ./update-bar.svelte) and carries out
// the kill switch. Not in dev, not inside a frame. One instance per app: `new ToolPwa('creator')`.

/** an SPA never navigates: look for a new worker when the person comes back, at most this often */
const CHECK_EVERY = 30 * 60 * 1000;

export class ToolPwa {
	/** a new version is installed and waits for "Reload" */
	updateReady = $state(false);
	#app: string;
	#waiting: ServiceWorker | null = null;
	#reloading = false;
	#lastCheck = Date.now();

	constructor(app: string) {
		this.#app = app;
	}

	/** the root layout's onMount; `dev`: the dev server (no worker); `off`: the kill switch (VITE_SW=off) */
	async init({ dev, off = false }: { dev: boolean; off?: boolean }) {
		if (typeof window === 'undefined' || dev || !('serviceWorker' in navigator) || window.top !== window) return;
		if (off) {
			for (const registration of await navigator.serviceWorker.getRegistrations()) await registration.unregister();
			for (const name of await caches.keys()) if (name.startsWith(`scutoons-${this.#app}-`)) await caches.delete(name);
			return;
		}
		let registration: ServiceWorkerRegistration;
		try {
			registration = await navigator.serviceWorker.register('/service-worker.js', { updateViaCache: 'none' });
		} catch {
			return; // private mode, blocked storage: the tool works as before
		}
		const offer = (worker: ServiceWorker | null) => {
			if (!worker || !navigator.serviceWorker.controller) return; // the first install has nothing to offer
			this.#waiting = worker;
			this.updateReady = true;
		};
		if (registration.waiting) offer(registration.waiting);
		registration.addEventListener('updatefound', () => {
			const worker = registration.installing;
			worker?.addEventListener('statechange', () => worker.state === 'installed' && offer(worker));
		});
		navigator.serviceWorker.addEventListener('controllerchange', () => {
			if (this.#reloading) location.reload();
		});
		const check = () => {
			if (Date.now() - this.#lastCheck < CHECK_EVERY) return;
			this.#lastCheck = Date.now();
			registration.update().catch(() => {});
		};
		document.addEventListener('visibilitychange', () => document.visibilityState === 'visible' && check());
		setInterval(() => registration.update().catch(() => {}), 60 * 60 * 1000);
	}

	/** "Reload": the waiting worker takes over, then the page reloads (3 s fallback) */
	reload() {
		this.#reloading = true;
		if (this.#waiting) this.#waiting.postMessage({ type: 'SKIP_WAITING' });
		setTimeout(() => location.reload(), 3000);
	}

	/** "Later": the bar goes until the next start */
	later() {
		this.updateReady = false;
	}
}
