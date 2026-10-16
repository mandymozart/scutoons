// The media cache's bookkeeping (agents/research/PWA.md §2 "Phase 2"): the Cache API keeps no sizes or access times, so
// the worker notes them here – one IndexedDB `scutoons-sw` with `media` ({ url, size, used, fetched }) and `meta`
// (the data tier the page last told). Worker-side only; any failure (private mode, blocked storage) just means no LRU –
// the cache still works and the next eviction run reconciles.
import type { DataTier, MediaEntry } from './cache-rules.ts';

export const DB_NAME = 'scutoons-sw';
const VERSION = 1;

let opening: Promise<IDBDatabase> | null = null;

function open(): Promise<IDBDatabase> {
	opening ??= new Promise<IDBDatabase>((resolve, reject) => {
		const request = indexedDB.open(DB_NAME, VERSION);
		request.onupgradeneeded = () => {
			const db = request.result;
			if (!db.objectStoreNames.contains('media')) db.createObjectStore('media', { keyPath: 'url' });
			if (!db.objectStoreNames.contains('meta')) db.createObjectStore('meta');
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

async function run<T>(store: 'media' | 'meta', mode: IDBTransactionMode, body: (s: IDBObjectStore) => IDBRequest<T> | void): Promise<T | undefined> {
	const db = await open();
	return new Promise<T | undefined>((resolve, reject) => {
		const tx = db.transaction(store, mode);
		const request = body(tx.objectStore(store));
		tx.oncomplete = () => resolve(request ? request.result : undefined);
		tx.onerror = () => reject(tx.error);
		tx.onabort = () => reject(tx.error);
	});
}

export const getEntry = (url: string) => run<MediaEntry>('media', 'readonly', (s) => s.get(url) as IDBRequest<MediaEntry>);
export const putEntry = (entry: MediaEntry) => run('media', 'readwrite', (s) => void s.put(entry));
export const allEntries = async () => (await run<MediaEntry[]>('media', 'readonly', (s) => s.getAll() as IDBRequest<MediaEntry[]>)) ?? [];

export async function deleteEntries(urls: string[]) {
	if (!urls.length) return;
	const db = await open();
	await new Promise<void>((resolve, reject) => {
		const tx = db.transaction('media', 'readwrite');
		const store = tx.objectStore('media');
		for (const url of urls) store.delete(url);
		tx.oncomplete = () => resolve();
		tx.onerror = () => reject(tx.error);
	});
}

export const getTier = () => run<DataTier>('meta', 'readonly', (s) => s.get('tier') as IDBRequest<DataTier>);
export const putTier = (tier: DataTier) => run('meta', 'readwrite', (s) => void s.put(tier, 'tier'));

/** the kill switch: the bookkeeping goes with the caches */
export function deleteDatabase(): Promise<void> {
	return new Promise((resolve) => {
		const request = indexedDB.deleteDatabase(DB_NAME);
		request.onsuccess = request.onerror = request.onblocked = () => resolve();
	});
}
