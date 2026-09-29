import { QR_API } from '$lib/config.ts';

export const SESSION_STORAGE_KEY = 'scutoons_session';
const DEVICE_STORAGE_KEY = 'scutoons_device';

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

/** Error returned by the qr.scutoons.com API: `{ status: "error", error: "<code>", message }`. */
export class ApiError extends Error {
	status: number;
	code: string;
	body: any;

	constructor(status: number, code: string, message: string, body?: any) {
		super(message);
		this.name = 'ApiError';
		this.status = status;
		this.code = code;
		this.body = body;
	}
}

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

type Method = 'GET' | 'POST' | 'PUT' | 'DELETE';

/**
 * JSON request against QR_API. `path` like `/me/` (leading slash, trailing slash as in the contract).
 * Throws ApiError for non-2xx and for `{status:"error"}` bodies; network failures → ApiError(0, 'network').
 */
export async function request<T = any>(
	method: Method,
	path: string,
	body?: unknown,
	opts: { auth?: boolean; signal?: AbortSignal } = {}
): Promise<T> {
	const headers: Record<string, string> = { Accept: 'application/json', 'X-Device-Id': getDeviceId() };
	if (body !== undefined) headers['Content-Type'] = 'application/json';
	const token = opts.auth === false ? null : getSessionToken();
	if (token) headers.Authorization = `Bearer ${token}`;

	let response: Response;
	try {
		response = await fetch(QR_API + path, {
			method,
			headers,
			body: body !== undefined ? JSON.stringify(body) : undefined,
			signal: opts.signal
		});
	} catch (error) {
		throw new ApiError(0, 'network', 'Network error', error);
	}

	let data: any = null;
	const text = await response.text();
	if (text) {
		try {
			data = JSON.parse(text);
		} catch {
			data = { message: text };
		}
	}

	if (!response.ok || data?.status === 'error') {
		throw new ApiError(
			response.status,
			data?.error || (response.status === 401 ? 'unauthorized' : 'error'),
			data?.message || response.statusText || 'Request failed',
			data
		);
	}
	return data as T;
}

export const api = {
	get: <T = any>(path: string, opts?: { auth?: boolean; signal?: AbortSignal }) =>
		request<T>('GET', path, undefined, opts),
	post: <T = any>(path: string, body?: unknown) => request<T>('POST', path, body ?? {}),
	put: <T = any>(path: string, body?: unknown) => request<T>('PUT', path, body ?? {})
};
