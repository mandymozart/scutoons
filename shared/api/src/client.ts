// Framework-agnostic JSON client for the SCUTOONS API (agents/ITEMS.md → HTTP API contract).
// The app injects the base URL, the session token and the device id; nothing here reads app config.

/** Error returned by the API: `{ status: "error", error: "<code>", message }`. Network failures → status 0, 'network'. */
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

export type Method = 'GET' | 'POST' | 'PUT' | 'DELETE';

export interface RequestOptions {
	/** false = never send the session token */
	auth?: boolean;
	signal?: AbortSignal;
}

export interface ApiClientOptions {
	/** API base without trailing slash, e.g. `http://localhost:9000/api` (a getter is read on every request) */
	baseUrl: string | (() => string);
	/** session token for `Authorization: Bearer …`, null = anonymous */
	getToken?: () => string | null;
	/** sent as `X-Device-Id` (the server stores only an HMAC of it, for code activity stats) */
	getDeviceId?: () => string | null;
	fetch?: typeof fetch;
}

export interface ApiClient {
	/** the resolved base URL (no trailing slash) */
	readonly baseUrl: string;
	getToken(): string | null;
	/**
	 * JSON request. `path` like `/me/` (leading slash, trailing slash as in the contract).
	 * Throws ApiError for non-2xx and for `{status:"error"}` bodies; network failures → ApiError(0, 'network').
	 */
	request<T = any>(method: Method, path: string, body?: unknown, opts?: RequestOptions): Promise<T>;
	get<T = any>(path: string, opts?: RequestOptions): Promise<T>;
	post<T = any>(path: string, body?: unknown): Promise<T>;
	put<T = any>(path: string, body?: unknown): Promise<T>;
}

export function createApiClient(options: ApiClientOptions): ApiClient {
	const base = () =>
		(typeof options.baseUrl === 'function' ? options.baseUrl() : options.baseUrl).replace(/\/+$/, '');
	const getToken = () => options.getToken?.() ?? null;

	async function request<T = any>(
		method: Method,
		path: string,
		body?: unknown,
		opts: RequestOptions = {}
	): Promise<T> {
		const headers: Record<string, string> = { Accept: 'application/json' };
		const deviceId = options.getDeviceId?.();
		if (deviceId) headers['X-Device-Id'] = deviceId;
		if (body !== undefined) headers['Content-Type'] = 'application/json';
		const token = opts.auth === false ? null : getToken();
		if (token) headers.Authorization = `Bearer ${token}`;

		let response: Response;
		try {
			response = await (options.fetch ?? fetch)(base() + path, {
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

	return {
		get baseUrl() {
			return base();
		},
		getToken,
		request,
		get: <T = any>(path: string, opts?: RequestOptions) => request<T>('GET', path, undefined, opts),
		post: <T = any>(path: string, body?: unknown) => request<T>('POST', path, body ?? {}),
		put: <T = any>(path: string, body?: unknown) => request<T>('PUT', path, body ?? {})
	};
}
