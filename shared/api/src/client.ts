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
	/** more request headers (e.g. `If-None-Match`) */
	headers?: Record<string, string>;
}

export interface ApiClientOptions {
	/** API base without trailing slash, e.g. `http://localhost:9000/api` (a getter is read on every request) */
	baseUrl: string | (() => string);
	/** session token for `Authorization: Bearer …`, null = anonymous */
	getToken?: () => string | null;
	/** sent as `X-Device-Id` (the server stores only an HMAC of it, for code activity stats) */
	getDeviceId?: () => string | null;
	/**
	 * `'include'` = send the single sign-on cookie `scutoons_sid` (HttpOnly, set by the API on `/auth/verify/`) with
	 * every request – the session clients of the apps. Leave it out for public clients (simple requests, any origin).
	 */
	credentials?: RequestCredentials;
	fetch?: typeof fetch;
}

export interface ApiClient {
	/** the resolved base URL (no trailing slash) */
	readonly baseUrl: string;
	getToken(): string | null;
	/**
	 * JSON request. `path` like `/me/` (leading slash, trailing slash as in the contract). A `Blob` / `File` body is sent
	 * raw with its own type (uploads); the answer is JSON either way.
	 * Throws ApiError for non-2xx and for `{status:"error"}` bodies; network failures → ApiError(0, 'network').
	 */
	request<T = any>(method: Method, path: string, body?: unknown, opts?: RequestOptions): Promise<T>;
	get<T = any>(path: string, opts?: RequestOptions): Promise<T>;
	post<T = any>(path: string, body?: unknown): Promise<T>;
	put<T = any>(path: string, body?: unknown): Promise<T>;
	delete<T = any>(path: string): Promise<T>;
	/**
	 * Raw GET for non-JSON answers (CSV, ZIP, PDF attachments): the `Response` of a 2xx, with the same headers
	 * (session, device id). Non-2xx → ApiError (JSON error body parsed when there is one).
	 */
	fetchRaw(path: string, opts?: RequestOptions): Promise<Response>;
}

export function createApiClient(options: ApiClientOptions): ApiClient {
	const base = () =>
		(typeof options.baseUrl === 'function' ? options.baseUrl() : options.baseUrl).replace(/\/+$/, '');
	const getToken = () => options.getToken?.() ?? null;

	async function send(method: Method, path: string, body: unknown, opts: RequestOptions, accept: string) {
		const headers: Record<string, string> = { ...opts.headers, Accept: accept };
		const deviceId = options.getDeviceId?.();
		if (deviceId) headers['X-Device-Id'] = deviceId;
		// a Blob / File is sent as it is (an upload, e.g. a claim's evidence file); everything else as JSON
		const raw = typeof Blob !== 'undefined' && body instanceof Blob;
		if (body !== undefined) headers['Content-Type'] = raw ? (body as Blob).type || 'application/octet-stream' : 'application/json';
		const token = opts.auth === false ? null : getToken();
		if (token) headers.Authorization = `Bearer ${token}`;

		try {
			return await (options.fetch ?? fetch)(base() + path, {
				method,
				headers,
				body: body === undefined ? undefined : raw ? (body as Blob) : JSON.stringify(body),
				credentials: options.credentials,
				signal: opts.signal
			});
		} catch (error) {
			throw new ApiError(0, 'network', 'Network error', error);
		}
	}

	function parse(text: string): any {
		if (!text) return null;
		try {
			return JSON.parse(text);
		} catch {
			return { message: text };
		}
	}

	function fail(response: Response, data: any): ApiError {
		return new ApiError(
			response.status,
			data?.error || (response.status === 401 ? 'unauthorized' : 'error'),
			data?.message || response.statusText || 'Request failed',
			data
		);
	}

	async function request<T = any>(
		method: Method,
		path: string,
		body?: unknown,
		opts: RequestOptions = {}
	): Promise<T> {
		const response = await send(method, path, body, opts, 'application/json');
		const data = parse(await response.text());
		if (!response.ok || data?.status === 'error') throw fail(response, data);
		return data as T;
	}

	async function fetchRaw(path: string, opts: RequestOptions = {}): Promise<Response> {
		const response = await send('GET', path, undefined, opts, '*/*');
		if (!response.ok) throw fail(response, parse(await response.text()));
		return response;
	}

	return {
		get baseUrl() {
			return base();
		},
		getToken,
		request,
		get: <T = any>(path: string, opts?: RequestOptions) => request<T>('GET', path, undefined, opts),
		post: <T = any>(path: string, body?: unknown) => request<T>('POST', path, body ?? {}),
		put: <T = any>(path: string, body?: unknown) => request<T>('PUT', path, body ?? {}),
		delete: <T = any>(path: string) => request<T>('DELETE', path),
		fetchRaw
	};
}
