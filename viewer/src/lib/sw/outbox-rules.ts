// Reading progress made offline (PWA phase 3, agents/research/PWA.md §2 "Offline reading progress", FEATURES 10.74): a
// member's save game write that could not reach the API waits in an outbox (IndexedDB `scutoons-outbox`, lib/sw/outbox.ts)
// and is sent when the reader is back online – by the service worker (Background Sync, Chromium) or by the page (`online`,
// the next start). Pure – tested in test/sw-outbox-rules.test.ts.
// The API merges every PUT /progress/<id>/ (checkpoints by uri, history per key, the map as sets – ProgressService), so a
// write sent twice (worker and page, a retry) changes nothing the second time: the outbox only keeps the newest one.

/** the Background Sync tag the page registers and the worker answers */
export const PROGRESS_SYNC_TAG = 'scutoons-progress';

/** one queued save game write: the whole state (the API merges) and its time */
export interface QueuedProgress {
	/** the progress id (`scutoons` = the save game) */
	id: string;
	state: unknown;
	/** the write's `updated_at` (ms) – what the API compares */
	updatedAt: number;
	queuedAt: number;
}

/** the outbox keeps one write per id: the newer one (the state is the device's whole save game, a superset) */
export function newer(queued: QueuedProgress | undefined, next: QueuedProgress): QueuedProgress {
	return queued && queued.updatedAt > next.updatedAt ? queued : next;
}

/** a write the API took covers every queued one up to its time */
export function settledBy(queued: QueuedProgress, sentUpdatedAt: number): boolean {
	return queued.updatedAt <= sentUpdatedAt;
}

/**
 * What a send's answer means for the queued write: `done` (taken – removed), `retry` (offline, the server's trouble,
 * rate limit – stays, Background Sync tries again later), `page` (no session the worker can use – stays for the page,
 * which sends with the reader's own session), `drop` (the API refused it for good – a broken state).
 */
export function sendOutcome(status: number | 'network'): 'done' | 'retry' | 'page' | 'drop' {
	if (status === 'network') return 'retry';
	if (status >= 200 && status < 300) return 'done';
	if (status === 401 || status === 403) return 'page';
	if (status === 408 || status === 429 || status >= 500) return 'retry';
	return 'drop';
}

/** the URL the save game is written to – the same as @scutoons/api `putProgress` */
export function progressUrl(api: string, id: string): string {
	return `${api.replace(/\/+$/, '')}/progress/${encodeURIComponent(id)}/`;
}

/** a failed push is queued when the network failed or the server had trouble – not for a refused write */
export function queueAfter(error: unknown): boolean {
	const status = (error as { status?: unknown } | null)?.status;
	return typeof status !== 'number' || status === 0 || status === 408 || status === 429 || status >= 500;
}
