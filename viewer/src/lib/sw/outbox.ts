// The outbox of reading progress made offline (rules: ./outbox-rules.ts): one IndexedDB `scutoons-outbox`, store
// `progress` (one QueuedProgress per id). Read and written by the page (lib/offline/progress-outbox.ts) and the service
// worker (Background Sync, `flushOutbox` below) – both send it the same way, the API merges, so a double send is harmless.
// Any IndexedDB failure (private mode, blocked storage) just means no outbox: the save game stays in localStorage and
// the next start's sync sends it (contexts/world.svelte.ts).
import { newer, progressUrl, sendOutcome, settledBy, type QueuedProgress } from './outbox-rules.ts';

export const OUTBOX_DB = 'scutoons-outbox';
const STORE = 'progress';

let opening: Promise<IDBDatabase> | null = null;

function open(): Promise<IDBDatabase> {
	opening ??= new Promise<IDBDatabase>((resolve, reject) => {
		const request = indexedDB.open(OUTBOX_DB, 1);
		request.onupgradeneeded = () => {
			if (!request.result.objectStoreNames.contains(STORE)) request.result.createObjectStore(STORE, { keyPath: 'id' });
		};
		request.onsuccess = () => {
			const db = request.result;
			// the kill switch deletes the database: let it
			db.onversionchange = () => {
				db.close();
				opening = null;
			};
			resolve(db);
		};
		request.onerror = () => reject(request.error);
		request.onblocked = () => reject(new Error('blocked'));
	}).catch((error) => {
		opening = null;
		throw error;
	});
	return opening;
}

/** one transaction on the store; `body` may read and write */
async function tx<T>(mode: IDBTransactionMode, body: (store: IDBObjectStore, done: (value: T) => void) => void): Promise<T | undefined> {
	const db = await open();
	return new Promise<T | undefined>((resolve, reject) => {
		const t = db.transaction(STORE, mode);
		let result: T | undefined;
		body(t.objectStore(STORE), (value) => (result = value));
		t.oncomplete = () => resolve(result);
		t.onerror = () => reject(t.error);
		t.onabort = () => reject(t.error);
	});
}

/** queue a write (the newer one of it and what waits) */
export function queue(entry: QueuedProgress): Promise<unknown> {
	return tx<void>('readwrite', (store) => {
		const read = store.get(entry.id);
		read.onsuccess = () => void store.put(newer(read.result as QueuedProgress | undefined, entry));
	});
}

/** the API took a write up to `updatedAt`: what it covers leaves the outbox */
export function settle(id: string, updatedAt: number): Promise<unknown> {
	return tx<void>('readwrite', (store) => {
		const read = store.get(id);
		read.onsuccess = () => {
			const queued = read.result as QueuedProgress | undefined;
			if (queued && settledBy(queued, updatedAt)) store.delete(id);
		};
	});
}

export async function queued(): Promise<QueuedProgress[]> {
	return (await tx<QueuedProgress[]>('readonly', (store, done) => {
		const all = store.getAll();
		all.onsuccess = () => done(all.result as QueuedProgress[]);
	})) ?? [];
}

/** a sign-out: nothing of the last member waits to be sent under the next one's session */
export function clear(): Promise<unknown> {
	return tx<void>('readwrite', (store) => void store.clear());
}

/** the kill switch: the outbox goes with the caches */
export function deleteOutbox(): Promise<void> {
	return new Promise((resolve) => {
		const request = indexedDB.deleteDatabase(OUTBOX_DB);
		request.onsuccess = request.onerror = request.onblocked = () => resolve();
	});
}

/** one send's result: the answer's merged state for the page to take in */
export interface Sent {
	id: string;
	state: unknown;
	updated_at: number;
}

/**
 * The service worker's Background Sync (Chromium): every queued write is sent with the reader's session cookie (the
 * single sign-on cookie – a worker can't read a Bearer token; then the page sends it). Throws when one should be tried
 * again (offline, server trouble) – the browser retries the sync later.
 */
export async function flushOutbox(api: string, onSent: (sent: Sent) => void): Promise<void> {
	let retry = false;
	for (const entry of await queued()) {
		let status: number | 'network';
		let body: { state?: unknown; updated_at?: number } | null = null;
		try {
			const response = await fetch(progressUrl(api, entry.id), {
				method: 'PUT',
				credentials: 'include',
				headers: { 'Content-Type': 'application/json', Accept: 'application/json' },
				body: JSON.stringify({ state: entry.state, updated_at: entry.updatedAt })
			});
			status = response.status;
			body = await response.json().catch(() => null);
		} catch {
			status = 'network';
		}
		const outcome = sendOutcome(status);
		if (outcome === 'done' || outcome === 'drop') await settle(entry.id, entry.updatedAt);
		if (outcome === 'done' && body?.state && typeof body.updated_at === 'number') onSent({ id: entry.id, state: body.state, updated_at: body.updated_at });
		if (outcome === 'retry') retry = true;
	}
	if (retry) throw new Error('progress: offline, tried again later');
}
