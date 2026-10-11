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
	/** with an unpublished draft (latest > published): the newest version's time, note and author (BACKLOG 10.60) */
	latest_at?: string;
	latest_message?: string | null;
	latest_author?: string | null;
}

/** A submit for review (server `/api/reviews`, docs/API.md "Reviews"), pinned to one version */
export interface ContentReview {
	id: number;
	key: string;
	toon: string;
	locale: string | null;
	/** the submitted version – what the reviewer publishes */
	version: number;
	/** the entity's newest version; greater than `version` = newer drafts, not part of the review */
	newest: number | null;
	state: 'pending' | 'published' | 'returned' | 'annulled';
	note: string | null;
	author: { id: string; email: string | null } | null;
	created_at: string;
	decided_by: { id: string; email: string | null } | null;
	decided_at: string | null;
	decision_note: string | null;
}

export interface ReviewOverview {
	status: string;
	/** toons (or `*`) where this account needs a review to publish */
	needs_review: string[];
	/** toons (or `*`) this account reviews */
	reviewer_of: string[];
	to_review: ContentReview[];
	mine: ContentReview[];
}

/** Does an account with these rules need a review to publish in this toon? */
export function needsReview(rules: { needs_review: string[] } | null | undefined, toon: string | null | undefined): boolean {
	return !!toon && !!rules && (rules.needs_review.includes('*') || rules.needs_review.includes(toon));
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

		/** Review before publish: this account's rules, the open submits it reviews, its own submits */
		reviews: () => client.get<ReviewOverview>('/reviews'),

		/** Submit versions for review (default: the newest) → 409 no_reviewer when nobody reviews the toon */
		submitReview: (items: { key: string; version?: number }[], note?: string) =>
			client.post<{ status: string; reviews: ContentReview[]; reviewers: string[] }>('/reviews', { items, note: note || undefined }),

		/** A reviewer publishes the submitted versions / sends them back (with an optional note to the author) */
		decideReviews: (action: 'publish' | 'return', ids: number[], note?: string) =>
			client.post<{ status: string; reviews: ContentReview[]; keys: string[] }>(`/reviews/${action}`, { ids, note: note || undefined }),

		/** Every stored entity of a toon; `''` = the platform-wide ones (characters, people, events …). */
		contentIndex: (toon: string) =>
			client.get<{ status: string; entities: ContentIndexEntity[] }>(`/entities?toon=${encodeURIComponent(toon)}`)
	};
}
