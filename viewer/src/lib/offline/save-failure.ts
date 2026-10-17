// Why saving a chapter to the device failed (PWA phase 3, agents/research/PWA.md §2 "Phase 3"; owner 2026-10-17: saving
// "often fails" in a plain Chrome / Firefox tab on the phone with nothing but a "!"): the reason the reader is told
// instead of "didn't work", and which failures a download step tries again. Pure – test/save-failure.test.ts.

/**
 * offline – the device has no connection · network – the connection broke off (after the retries) · quota – the
 * browser's storage for SCUTOONS is full · server – the server or CDN refused a file / the file list · storage – the
 * browser refused to store (private window, storage blocked)
 */
export type SaveFailure = 'offline' | 'network' | 'quota' | 'server' | 'storage';

/** a failure we already know the reason of (a refused file, the file list) */
export class SaveError extends Error {
	constructor(
		readonly failure: SaveFailure,
		message: string
	) {
		super(message);
		this.name = 'SaveError';
	}
}

/** what went wrong – `online`: navigator.onLine at the time */
export function failureOf(error: unknown, online: boolean): SaveFailure {
	if (error instanceof SaveError) return error.failure;
	const name = (error as { name?: unknown } | null)?.name;
	// Chrome / Firefox / Safari: a DOMException named QuotaExceededError (old Firefox: NS_ERROR_DOM_QUOTA_REACHED)
	if (name === 'QuotaExceededError' || name === 'NS_ERROR_DOM_QUOTA_REACHED') return 'quota';
	if (!online) return 'offline';
	// fetch() rejects with a TypeError for every network failure (also a CORS one) – "Failed to fetch", "NetworkError …",
	// "Load failed"
	if (error instanceof TypeError) return 'network';
	return 'storage';
}

/** a download worth another try: the connection broke (TypeError), a timeout or an overloaded server */
export function retryable(error: unknown, status?: number): boolean {
	if (status !== undefined) return status === 408 || status === 429 || status >= 500;
	return error instanceof TypeError;
}

/** tries per file; the wait before try n is n × RETRY_WAIT ms */
export const TRIES = 3;
export const RETRY_WAIT = 1000;
