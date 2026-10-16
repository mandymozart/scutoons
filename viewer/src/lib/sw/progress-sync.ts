// The service worker's part of offline reading progress (lib/sw/outbox.ts, rules ./outbox-rules.ts): the Background Sync
// event (Chromium) sends the queued save game writes once the browser is online again – also when no SCUTOONS page is
// open. The open pages take in the API's merged answer (`PROGRESS_SYNCED`). Browsers without Background Sync: the page
// sends on `online` and on the next start (lib/offline/progress-outbox.ts).
import { API_URL } from '$lib/config.ts';
import { flushOutbox } from './outbox.ts';
import { PROGRESS_SYNC_TAG } from './outbox-rules.ts';

/** the worker types this file needs (lib/sw/* is checked without the webworker lib – like saved-cache.ts) */
interface SyncEvent {
	readonly tag: string;
	waitUntil(promise: Promise<unknown>): void;
}
interface WorkerScope {
	clients: { matchAll(options: { type: 'window' }): Promise<{ postMessage(message: unknown): void }[]> };
}

/** `sync` handler: the progress tag only */
export function onSync(event: Event) {
	const sync = event as unknown as SyncEvent;
	if (sync.tag !== PROGRESS_SYNC_TAG) return;
	const worker = self as unknown as WorkerScope;
	sync.waitUntil(
		flushOutbox(API_URL, async (sent) => {
			for (const client of await worker.clients.matchAll({ type: 'window' })) client.postMessage({ type: 'PROGRESS_SYNCED', ...sent });
		})
	);
}
