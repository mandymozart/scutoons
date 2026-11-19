// The reader's live camera pages (research/LIVE-CAMERAS.md §2): what the routes read from the address and keep in the
// browser. The hold itself is the core's (@scutoons/live-deck CameraLink); this is the page's own state.

/** the phone's own secret for the live cameras: a drop or a reload is the same holder */
export const KEY_STORE = 'scutoons.live-camera.key';
/** the camera this browser held and the client id it held it as: a reload takes it again */
export const HELD_STORE = 'scutoons.live-camera.held';

/** what the pages need of localStorage (a test hands in a map) */
export type Store = Pick<Storage, 'getItem' | 'setItem' | 'removeItem'>;

/** the page's storage, null where the browser gives none */
export function store(): Store | null {
	try {
		return globalThis.localStorage ?? null;
	} catch {
		return null;
	}
}

/** this browser's key, made once and kept; without storage a key for the page's life */
export function cameraKey(s: Store | null, make: () => string = () => globalThis.crypto.randomUUID()): string {
	try {
		const kept = s?.getItem(KEY_STORE);
		if (kept) return kept;
		const key = make();
		s?.setItem(KEY_STORE, key);
		return key;
	} catch {
		return make();
	}
}

/** the client id this browser held `camera` as, if it held it when the page went */
export function heldBefore(s: Store | null, camera: string): string | null {
	try {
		const held = JSON.parse(s?.getItem(HELD_STORE) ?? 'null') as { camera?: string; client?: string } | null;
		return held?.camera === camera && held.client ? held.client : null;
	} catch {
		return null;
	}
}

/** keep what this browser holds now (`client` null: nothing) */
export function keepHeld(s: Store | null, camera: string | null, client: string | null): void {
	try {
		if (camera && client) s?.setItem(HELD_STORE, JSON.stringify({ camera, client }));
		else s?.removeItem(HELD_STORE);
	} catch {
		// a private window: a reload asks again
	}
}

/** the session's part of the address (`live`, `relay`, `quic`, `quichash`) for a link between the camera pages */
export function sessionQuery(search: URLSearchParams): string {
	const keep = new URLSearchParams();
	for (const k of ['live', 'relay', 'quic', 'quichash']) {
		const v = search.get(k);
		if (v !== null) keep.set(k, v);
	}
	const q = keep.toString();
	return q ? `?${q}` : '';
}

/** the page of one camera (its id in one path segment), the session kept */
export const cameraHref = (id: string, search: URLSearchParams): string => `/live/cameras/${encodeURIComponent(id)}${sessionQuery(search)}`;
/** the list, the session kept */
export const listHref = (search: URLSearchParams): string => `/live/cameras${sessionQuery(search)}`;

/** a camera's name for people */
export const cameraName = (c: { id: string; label?: string }): string => c.label?.trim() || c.id;
