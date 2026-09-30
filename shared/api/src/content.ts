// The content store by entity key (server `/api/entities`, docs/API.md "Entities") – the creator's write path. Every
// call needs a session with the role `creator`. Keys: `@scutoons/content` entities.ts (agents/DATA.md §2); readers use
// the library (./library.ts).
import type { ApiClient } from './client.ts';

export interface ContentVersion {
	id: number;
	status: 'draft' | 'published';
	created_at: string;
	author: string | null;
	author_email: string | null;
	message: string | null;
	size: number;
	key?: string;
	kind?: string;
	toon?: string | null;
	locale?: string | null;
}

export interface WrappedContent<T = unknown> {
	status: string;
	version: ContentVersion;
	published: { id: number; created_at: string } | null;
	content: T;
}

/** One stored entity of the index */
export interface ContentIndexEntity {
	key: string;
	kind: string;
	toon: string | null;
	locale: string | null;
	latest: number;
	published: number | null;
	versions: number;
}

export type ContentApi = ReturnType<typeof createContentApi>;

export function createContentApi(client: ApiClient) {
	const at = (key: string) => `/entities/${key}`;
	return {
		/** Newest version (draft or published) with its meta; ApiError 404 when nothing is stored. */
		getDraft: <T = unknown>(key: string) => client.get<WrappedContent<T>>(`${at(key)}?draft=1&wrap=1`),

		/** The published version with its meta; ApiError 404 when nothing is published. */
		getPublished: <T = unknown>(key: string) => client.get<WrappedContent<T>>(`${at(key)}?wrap=1`),

		getVersion: <T = unknown>(key: string, id: number) => client.get<WrappedContent<T>>(`${at(key)}?version=${id}&wrap=1`),

		/** Saves a new draft version. `base` = the version the edit started from (null = none) → 409 conflict if outdated. */
		putDraft: (key: string, content: unknown, base: number | null | undefined, message?: string) =>
			client.put<{ status: string; unchanged: boolean; version: ContentVersion }>(at(key), {
				content,
				message: message || undefined,
				...(base !== undefined ? { base } : {})
			}),

		publishVersion: (key: string, version?: number) =>
			client.post<{ status: string; version: ContentVersion }>(`${at(key)}/publish`, version !== undefined ? { version } : {}),

		unpublish: (key: string) => client.post<{ status: string; unpublished: number }>(`${at(key)}/unpublish`, {}),

		restoreVersion: (key: string, id: number, message?: string) =>
			client.post<{ status: string; version: ContentVersion }>(`${at(key)}/restore`, { id, message }),

		listVersions: (key: string) => client.get<{ status: string; versions: ContentVersion[] }>(`${at(key)}/versions`),

		/** Every stored entity of a toon; `''` = the platform-wide ones (characters, people, events …). */
		contentIndex: (toon: string) =>
			client.get<{ status: string; entities: ContentIndexEntity[] }>(`/entities?toon=${encodeURIComponent(toon)}`)
	};
}
