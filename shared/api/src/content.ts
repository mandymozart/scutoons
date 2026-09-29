// Content store (server `Content` API, agents/MEMORY.md → "Creator editor").
// Chapter / season / toon configs and VN scene files are read from `{api}/content/<toon>/<locale>/<path>` first;
// static files (`{staticBase}/<toon>/<locale>/<path>`, shipped with the viewer) are the fallback (store not seeded,
// file not published, API unreachable). The editor endpoints need the role `creator`.
import type { ApiClient } from './client.ts';

type Fetch = typeof fetch;

/** How long the loader waits for the content store before it falls back to the static files. */
const STORE_TIMEOUT_MS = 4000;

/** Files the store manages (same whitelist as the server + only config files the loader reads). */
export const CONTENT_PATH = /^[a-z0-9_-]+(\/[a-z0-9_-]+)?\.json$/;

export interface ContentOptions {
	/** newest draft instead of the published version (needs a session with the creator role) */
	draft?: boolean;
}

export interface ContentVersion {
	id: number;
	status: 'draft' | 'published';
	created_at: string;
	author: string | null;
	author_email: string | null;
	message: string | null;
	size: number;
	toon?: string;
	locale?: string;
	path?: string;
}

export interface WrappedContent<T = unknown> {
	status: string;
	version: ContentVersion;
	published: { id: number; created_at: string } | null;
	content: T;
}

export interface ContentIndexFile {
	locale: string;
	path: string;
	latest: number;
	published: number | null;
	versions: number;
}

export interface ContentApiOptions {
	/** the configured API client (base URL + session token) */
	client: ApiClient;
	/**
	 * Base of the static fallback files without trailing slash: `/api` in the viewer (same origin), the viewer's
	 * `/api` URL in other apps. A getter is read on every call.
	 */
	staticBase: string | (() => string);
}

export type ContentApi = ReturnType<typeof createContentApi>;

export function createContentApi({ client, staticBase }: ContentApiOptions) {
	const staticRoot = () => (typeof staticBase === 'function' ? staticBase() : staticBase).replace(/\/+$/, '');

	function storeUrl(toon: string, locale: string, path: string, query = '') {
		return `${client.baseUrl}/content/${encodeURIComponent(toon)}/${encodeURIComponent(locale)}/${path}${query}`;
	}

	async function tryStore(fetchFn: Fetch, url: string, token: string | null): Promise<Response | null> {
		const controller = new AbortController();
		const timer = setTimeout(() => controller.abort(), STORE_TIMEOUT_MS);
		try {
			const response = await fetchFn(url, {
				headers: token ? { Authorization: `Bearer ${token}`, Accept: 'application/json' } : { Accept: 'application/json' },
				signal: controller.signal
			});
			return response.ok ? response : null;
		} catch {
			return null;
		} finally {
			clearTimeout(timer);
		}
	}

	/**
	 * A config file for the reader: content store (newest draft with `draft`, else published) → static file.
	 * Returns a Response like `fetch`, so load functions keep their `ok` / `json()` handling.
	 */
	async function fetchContent(
		fetchFn: Fetch,
		toon: string,
		locale: string,
		path: string,
		options: ContentOptions = {}
	): Promise<Response> {
		if (CONTENT_PATH.test(path)) {
			if (options.draft) {
				const token = client.getToken();
				if (token) {
					const draft = await tryStore(fetchFn, storeUrl(toon, locale, path, '?draft=1'), token);
					if (draft) return draft;
				}
			}
			const published = await tryStore(fetchFn, storeUrl(toon, locale, path), null);
			if (published) return published;
		}
		return fetchFn(`${staticRoot()}/${toon}/${locale}/${path}`);
	}

	const file = (toon: string, locale: string, path: string) =>
		`/content/${encodeURIComponent(toon)}/${encodeURIComponent(locale)}/${path}`;

	return {
		fetchContent,

		// ── Editor API (role creator) ─────────────────────────────────────────────────────────────────────

		/** Newest version (draft or published) with its meta; ApiError 404 when the store has none. */
		getDraft: <T = unknown>(toon: string, locale: string, path: string) =>
			client.get<WrappedContent<T>>(`${file(toon, locale, path)}?draft=1&wrap=1`),

		getVersion: <T = unknown>(toon: string, locale: string, path: string, id: number) =>
			client.get<WrappedContent<T>>(`${file(toon, locale, path)}?version=${id}&wrap=1`),

		/** Saves a new draft version. `base` = the version the edit started from (null = none) → 409 conflict if outdated. */
		putDraft: (
			toon: string,
			locale: string,
			path: string,
			content: unknown,
			base: number | null | undefined,
			message?: string
		) =>
			client.put<{ status: string; unchanged: boolean; version: ContentVersion }>(file(toon, locale, path), {
				content,
				message: message || undefined,
				...(base !== undefined ? { base } : {})
			}),

		publishVersion: (toon: string, locale: string, path: string, version?: number) =>
			client.post<{ status: string; version: ContentVersion }>('/content/publish', { toon, locale, path, version }),

		unpublishFile: (toon: string, locale: string, path: string) =>
			client.post<{ status: string; unpublished: number }>('/content/unpublish', { toon, locale, path }),

		restoreVersion: (id: number, message?: string) =>
			client.post<{ status: string; version: ContentVersion }>('/content/restore', { id, message }),

		listVersions: (toon: string, locale: string, path: string) =>
			client.get<{ status: string; versions: ContentVersion[] }>(
				`/content/versions?toon=${encodeURIComponent(toon)}&locale=${encodeURIComponent(locale)}&path=${encodeURIComponent(path)}`
			),

		contentIndex: (toon: string) =>
			client.get<{ status: string; files: ContentIndexFile[] }>(`/content/index?toon=${encodeURIComponent(toon)}`),

		/** The static fallback file (the state before the store was seeded), or null. */
		async getStatic<T = unknown>(toon: string, locale: string, path: string): Promise<T | null> {
			try {
				const response = await fetch(`${staticRoot()}/${toon}/${locale}/${path}`, { cache: 'no-cache' });
				if (!response.ok) return null;
				// a dev server answers unknown files with the SPA page
				if (!(response.headers.get('content-type') || '').includes('json')) return null;
				return (await response.json()) as T;
			} catch {
				return null;
			}
		}
	};
}
