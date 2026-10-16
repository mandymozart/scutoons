// Reading progress made offline – the page side (agents/research/PWA.md §2 "Offline reading progress", FEATURES 10.74,
// owner 2026-10-16 "alle Pakete angehen"). A member's save game write (contexts/world.svelte.ts `push`) that can't reach
// the API waits in the outbox (IndexedDB, lib/sw/outbox.ts) and goes out when the reader is back online: by the service
// worker's Background Sync (Chromium – also with every SCUTOONS tab closed), else by the page on `online` and on the next
// start. The API merges every write (idempotent), so the worker and the page may both send it.
// Guests have nothing to send (their save game lives in localStorage only).
import { browser } from '$app/environment';
import { clear, queue, queued, settle, type Sent } from '$lib/sw/outbox.ts';
import { PROGRESS_SYNC_TAG } from '$lib/sw/outbox-rules.ts';

/** asks the service worker to send the outbox when the browser is online (Background Sync, Chromium only) */
async function requestSync() {
	if (!('serviceWorker' in navigator) || !navigator.serviceWorker.controller) return;
	const registration = (await navigator.serviceWorker.ready) as ServiceWorkerRegistration & { sync?: { register(tag: string): Promise<void> } };
	await registration.sync?.register(PROGRESS_SYNC_TAG).catch(() => {});
}

export const progressOutbox = {
	/** a write that could not reach the API: kept (the newest per id) until it is sent */
	async hold(id: string, state: unknown, updatedAt: number) {
		if (!browser) return;
		try {
			await queue({ id, state, updatedAt, queuedAt: Date.now() });
		} catch {
			return; // no IndexedDB: localStorage keeps it, the next start's sync sends it
		}
		await requestSync().catch(() => {});
	},

	/** the API took the save game up to `updatedAt` */
	settle(id: string, updatedAt: number): Promise<unknown> {
		return browser ? settle(id, updatedAt).catch(() => {}) : Promise.resolve();
	},

	/** something waits to be sent */
	async waiting(id: string): Promise<boolean> {
		if (!browser) return false;
		return (await queued().catch(() => [])).some((q) => q.id === id);
	},

	/** a sign-out: nothing of this member is sent under the next one's session */
	clear(): Promise<unknown> {
		return browser ? clear().catch(() => {}) : Promise.resolve();
	},

	/** the service worker sent a queued write – its merged answer; returns the unsubscribe */
	onSynced(callback: (sent: Sent) => void): () => void {
		if (!browser || !('serviceWorker' in navigator)) return () => {};
		const listener = (event: MessageEvent) => {
			const data = event.data as ({ type?: string } & Sent) | null;
			if (data?.type === 'PROGRESS_SYNCED') callback(data);
		};
		navigator.serviceWorker.addEventListener('message', listener);
		return () => navigator.serviceWorker.removeEventListener('message', listener);
	}
};
