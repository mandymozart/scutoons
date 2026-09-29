// Browser-side ids stored per origin (localStorage). Safe to call during SSR / without storage.

export const SESSION_STORAGE_KEY = 'scutoons_session';
export const DEVICE_STORAGE_KEY = 'scutoons_device';

let deviceId: string | null = null;

/**
 * Random per-browser id (sent as `X-Device-Id`; the server only stores an HMAC of it, for code activity stats).
 * No device limits are enforced with it.
 */
export function getDeviceId(): string {
	if (deviceId) return deviceId;
	try {
		deviceId = localStorage.getItem(DEVICE_STORAGE_KEY);
		if (!deviceId) {
			deviceId = crypto.randomUUID();
			localStorage.setItem(DEVICE_STORAGE_KEY, deviceId);
		}
	} catch {
		deviceId ??= crypto.randomUUID();
	}
	return deviceId;
}

/** The session token of this origin (localStorage `scutoons_session`), null when signed out. */
export function getSessionToken(): string | null {
	try {
		return typeof localStorage === 'undefined' ? null : localStorage.getItem(SESSION_STORAGE_KEY);
	} catch {
		return null;
	}
}

export function setSessionToken(token: string | null) {
	try {
		if (token) localStorage.setItem(SESSION_STORAGE_KEY, token);
		else localStorage.removeItem(SESSION_STORAGE_KEY);
	} catch {
		/* storage unavailable */
	}
}
