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
	/** the newest version is the same document as the published one (a restore …): nothing new for readers (owner D6) */
	same_as_published?: boolean;
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
	/** toons (or `*`) whose physical catalog changes this account publishes (rule `physical`) */
	physical_of?: string[];
	to_review: ContentReview[];
	mine: ContentReview[];
}

/** The catalog kinds that are versioned documents (`item:<id>` …, server CatalogDocs – agents/CATALOG-IN-CREATOR.md) */
export const CATALOG_DOC_KINDS = { items: 'item', recipes: 'recipe', albums: 'album', products: 'product', editions: 'edition', 'home-scenes': 'home-scene', 'airlift-carriers': 'airlift-carrier' } as const;
export type CatalogDocKind = keyof typeof CATALOG_DOC_KINDS;

/** A catalog entry as a document (GET /api/catalog/docs/<kind>) */
export interface CatalogDoc {
	id: string;
	/** `item:<id>` … – the /entities key (drafts, versions, publish, restore) */
	key: string;
	/** the newest stored version (null: none stored – the table's data) */
	latest: number | null;
	published: number | null;
	/** in the catalog table (readers have it); false = a new entry, only drafted so far */
	live: boolean;
	/** a version newer than the published one */
	draft: boolean;
	/** the newest version is a tombstone (`removed: true`, D5): removed, or waiting to be published as removed */
	removed: boolean;
	versions: number;
	latest_at: string | null;
	latest_author: string | null;
	latest_message: string | null;
	/** publishing it needs the `physical` review rule (edition, product, kind physical, max_world …) */
	physical: boolean;
	/** the toons it touches (`*` = every toon) */
	toons: string[];
	/** the newest version's data */
	content: Record<string, unknown>;
}

/** a catalog document's key: `item:camera` → { kind: 'items', id: 'camera' } */
export function catalogDocKey(key: string): { kind: CatalogDocKind; id: string } | null {
	const [prefix, id, ...rest] = key.split(':');
	const kind = (Object.keys(CATALOG_DOC_KINDS) as CatalogDocKind[]).find((k) => CATALOG_DOC_KINDS[k] === prefix);
	return kind && id && !rest.length ? { kind, id } : null;
}

/** Does an account with these rules need a review to publish in this toon? */
export function needsReview(rules: { needs_review: string[] } | null | undefined, toon: string | null | undefined): boolean {
	return !!toon && !!rules && (rules.needs_review.includes('*') || rules.needs_review.includes(toon));
}

/** One entity WITH its document (GET /entities?kinds=…): a page that edits many documents (the world map's places) */
export interface ContentEntityWithContent<T = unknown> {
	key: string;
	kind: string;
	/** the id of its first version: the creation order (the world map's order) */
	first: number;
	latest: number;
	published: number | null;
	/** the newest version is the same document as the published one (owner D6) */
	same_as_published: boolean;
	/** the version `content` is (the newest, or the published one; null: none published) */
	version: number | null;
	content: T | null;
}

/** One save of a batch (POST /entities/batch) */
export interface BatchSave {
	key: string;
	content: unknown;
	/** the version the edit started from (null = a new key); left out = no conflict check (overwrite) */
	base?: number | null;
	message?: string;
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

		/** Every entity of some kinds with its newest (`draft`) or published document, in creation order */
		withContent: <T = unknown>(kinds: string[], content: 'draft' | 'published' = 'draft') =>
			client.get<{ status: string; entities: ContentEntityWithContent<T>[] }>(`/entities?kinds=${encodeURIComponent(kinds.join(','))}&content=${content}`),

		/**
		 * Several draft versions in one transaction: all or none (409 conflict / 422 invalid name the `key`); `deletes` =
		 * never-published place / route keys removed with it (409 `published` for one that is published)
		 */
		saveBatch: (saves: BatchSave[], message?: string, deletes: string[] = []) =>
			client.post<{ status: string; saved: { key: string; unchanged: boolean; version: ContentVersion }[]; deleted: string[] }>('/entities/batch', {
				saves: saves.map((s) => ({ ...s, message: s.message || undefined })),
				...(deletes.length ? { deletes } : {}),
				message: message || undefined
			}),

		/** Several documents live at once (one transaction) */
		publishMany: (versions: { key: string; version?: number }[]) =>
			client.post<{ status: string; published: { key: string; version: ContentVersion }[] }>('/entities/publish', { versions }),

		/** Every catalog entry of a kind as a document: its newest version, draft / live / physical */
		catalogDocs: (kind: CatalogDocKind) => client.get<{ status: string; kind: CatalogDocKind; entries: CatalogDoc[] }>(`/catalog/docs/${kind}`),

		/** Every stored entity of a toon; `''` = the platform-wide ones (characters, people, events …). */
		contentIndex: (toon: string) =>
			client.get<{ status: string; entities: ContentIndexEntity[] }>(`/entities?toon=${encodeURIComponent(toon)}`)
	};
}
