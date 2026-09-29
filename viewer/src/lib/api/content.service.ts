// Content store of the creator editor (qr.scutoons.com `api/lib/Content.php`, agents/MEMORY.md → "Creator editor").
// Chapter / season / toon configs and VN scene files are read from `{QR_API}/content/<toon>/<locale>/<path>` first;
// the static `/api/<toon>/<locale>/<path>` files shipped with the viewer are the fallback (store not seeded, file not
// published, API unreachable).
import { API_BASE, QR_API } from '$lib/config.ts';
import { api, getSessionToken } from './client.ts';

type Fetch = typeof fetch;

/** How long the loader waits for the content store before it falls back to the static files. */
const STORE_TIMEOUT_MS = 4000;

/** Files the store manages (same whitelist as the server + only config files the loader reads). */
export const CONTENT_PATH = /^[a-z0-9_-]+(\/[a-z0-9_-]+)?\.json$/;

export interface ContentOptions {
	/** creator preview (`?preview=draft`): newest draft instead of the published version (needs the creator role) */
	draft?: boolean;
}

function storeUrl(toon: string, locale: string, path: string, query = '') {
	return `${QR_API}/content/${encodeURIComponent(toon)}/${encodeURIComponent(locale)}/${path}${query}`;
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
 * A config file for the reader: content store (draft for creators in preview, else published) → static file.
 * Returns a Response like `fetch`, so load functions keep their `ok` / `json()` handling.
 */
export async function fetchContent(
	fetchFn: Fetch,
	toon: string,
	locale: string,
	path: string,
	options: ContentOptions = {}
): Promise<Response> {
	if (CONTENT_PATH.test(path)) {
		if (options.draft) {
			const token = getSessionToken();
			if (token) {
				const draft = await tryStore(fetchFn, storeUrl(toon, locale, path, '?draft=1'), token);
				if (draft) return draft;
			}
		}
		const published = await tryStore(fetchFn, storeUrl(toon, locale, path), null);
		if (published) return published;
	}
	return fetchFn(`${API_BASE}/${toon}/${locale}/${path}`);
}

/** `?preview=draft` on a reader URL */
export function isDraftPreview(url: URL): boolean {
	return url.searchParams.get('preview') === 'draft';
}

// ── Editor API (role creator) ─────────────────────────────────────────────────────────────────────────────

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

const file = (toon: string, locale: string, path: string) =>
	`/content/${encodeURIComponent(toon)}/${encodeURIComponent(locale)}/${path}`;

/** Newest version (draft or published) with its meta; ApiError 404 when the store has none. */
export const getDraft = <T = unknown>(toon: string, locale: string, path: string) =>
	api.get<WrappedContent<T>>(`${file(toon, locale, path)}?draft=1&wrap=1`);

export const getVersion = <T = unknown>(toon: string, locale: string, path: string, id: number) =>
	api.get<WrappedContent<T>>(`${file(toon, locale, path)}?version=${id}&wrap=1`);

/** Saves a new draft version. `base` = the version the edit started from (null = none) → 409 conflict if outdated. */
export const putDraft = (
	toon: string,
	locale: string,
	path: string,
	content: unknown,
	base: number | null | undefined,
	message?: string
) =>
	api.put<{ status: string; unchanged: boolean; version: ContentVersion }>(file(toon, locale, path), {
		content,
		message: message || undefined,
		...(base !== undefined ? { base } : {})
	});

export const publishVersion = (toon: string, locale: string, path: string, version?: number) =>
	api.post<{ status: string; version: ContentVersion }>('/content/publish', { toon, locale, path, version });

export const unpublishFile = (toon: string, locale: string, path: string) =>
	api.post<{ status: string; unpublished: number }>('/content/unpublish', { toon, locale, path });

export const restoreVersion = (id: number, message?: string) =>
	api.post<{ status: string; version: ContentVersion }>('/content/restore', { id, message });

export const listVersions = (toon: string, locale: string, path: string) =>
	api.get<{ status: string; versions: ContentVersion[] }>(
		`/content/versions?toon=${encodeURIComponent(toon)}&locale=${encodeURIComponent(locale)}&path=${encodeURIComponent(path)}`
	);

export const contentIndex = (toon: string) =>
	api.get<{ status: string; files: ContentIndexFile[] }>(`/content/index?toon=${encodeURIComponent(toon)}`);

/** The static file shipped with the viewer (the state before the store was seeded), or null. */
export async function getStatic<T = unknown>(toon: string, locale: string, path: string): Promise<T | null> {
	try {
		const response = await fetch(`${API_BASE}/${toon}/${locale}/${path}`, { cache: 'no-cache' });
		if (!response.ok) return null;
		// the dev server answers unknown files with the SPA page
		if (!(response.headers.get('content-type') || '').includes('json')) return null;
		return (await response.json()) as T;
	} catch {
		return null;
	}
}
