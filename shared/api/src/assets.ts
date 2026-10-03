// The asset manager (server Domain\Assets, agents/ASSETS.md): content editors browse / look up / upload media
// (`/assets/*` – the creator's asset picker); admins replace, rename, relink, delete, scan, run jobs and see the storage
// (`/admin/assets/*`). Content keeps referencing the stable CDN paths these calls hand out.
import type { ApiClient } from './client.ts';
import { toQuery, type ListParams, type ListResult } from './admin.ts';

export type AssetKind = 'image' | 'video' | 'audio' | 'archive' | 'document' | 'other';
export const ASSET_KINDS: AssetKind[] = ['image', 'video', 'audio', 'archive', 'document', 'other'];
export type AssetQuality = 'low' | 'medium' | 'high';

/** One file of an asset version: the source ("original" / its marker) or a derived copy (480p, 1280w, webp, 2x …) */
export interface AssetRendition {
	id: number;
	label: string;
	quality: AssetQuality | null;
	/** CDN-root path (the string content stores, relative to the toon folder there) */
	path: string;
	size: number;
	mime: string | null;
	width: number | null;
	height: number | null;
	duration: number | null;
	origin: 'imported' | 'uploaded' | 'generated';
}

/** A list row: the asset with its current version */
export interface AssetSummary {
	id: string;
	slug: string;
	kind: AssetKind;
	folder: string | null;
	title: string;
	tags: string[];
	version: number;
	/** the current version's source file */
	path: string;
	width: number | null;
	height: number | null;
	duration: number | null;
	/** bytes of every file of the current version */
	size: number;
	/** references of its files (content + catalog) */
	usages: number;
	updated_at: string | null;
	renditions: AssetRendition[];
}

export interface AssetUsage {
	path: string;
	source: 'content' | 'catalog';
	/** entity key (content) or catalog file */
	ref: string;
	/** JSON pointer in the document */
	field: string;
	state: 'published' | 'draft' | 'config';
	toon: string | null;
	raw: string;
}

export interface AssetVersion {
	id: number;
	number: number;
	status: 'current' | 'archived' | 'purged';
	path: string;
	storage: string;
	size: number;
	sha256: string | null;
	mime: string | null;
	width: number | null;
	height: number | null;
	duration: number | null;
	archive_path: string | null;
	note: string | null;
	created_at: string | null;
	created_by: string | null;
	renditions: AssetRendition[];
}

export interface AssetJob {
	id: number;
	asset?: string | null;
	slug?: string | null;
	kind: string;
	status: 'queued' | 'running' | 'done' | 'failed';
	params?: Record<string, unknown>;
	log?: string | null;
	created_at: string | null;
	started_at?: string | null;
	finished_at: string | null;
}

export interface AssetDetail {
	status: 'success';
	asset: { id: string; slug: string; kind: AssetKind; folder: string | null; title: string; tags: string[]; status: string; created_at: string | null; updated_at: string | null };
	versions: AssetVersion[];
	usages: AssetUsage[];
	jobs: AssetJob[];
	cdn?: string;
}

export interface AssetList extends ListResult<AssetSummary> {
	folders: { folder: string | null; assets: number }[];
	/** the public media base (thumbnails) */
	cdn: string;
}

export interface AssetLookup {
	status: 'success';
	asset: AssetDetail | null;
	rendition: AssetRendition | null;
}

export interface AssetDeleteCheck {
	status: 'success';
	asset: { id: string; slug: string };
	files: string[];
	usages: AssetUsage[];
	can_delete: boolean;
	/** published content / catalog references – always block */
	blocking: number;
	/** draft-only references – an admin may force */
	drafts: number;
	archived_versions: number;
	note: string;
}

export interface AssetRelinkResult {
	status: 'success';
	map: Record<string, string>;
	content: { ref: string; changed: number; version: number | null; published: boolean; note: string }[];
	catalog: { ref: string; written: boolean; replacements: number; reason: string }[];
	manual: { ref: string; written: boolean; replacements: number; reason: string }[];
	usages: number;
}

export interface AssetSettingStatus {
	status: 'set' | 'missing' | 'invalid';
	/** non-secret values only (zone, endpoint, pull zone id, public URL) */
	value: string | null;
	note: string;
}

export interface AssetStorageStatus {
	status: 'success';
	primary: string;
	local: { where: string };
	bunny: { where: string; can_write: boolean; api_key: boolean; pull_zone: string } | null;
	settings: Record<string, AssetSettingStatus>;
	/** where the asset layer's files are loaded from (the primary storage) */
	public_base: string;
	/** the bunny.net public base (CDN_PUBLIC_URL) */
	bunny_public_base: string;
	totals: { assets: number; files: number; bytes: number; kinds: Record<string, { assets: number; files: number; bytes: number }> };
	bandwidth: { days: number; bandwidth: number; requests: number; cache_hit_rate: number; daily: { day: string; bytes: number }[] } | null;
	bandwidth_error: string | null;
}

export interface AssetMissing {
	path: string;
	usages: Omit<AssetUsage, 'path' | 'toon'>[];
}

export interface AssetOrphan {
	id: string;
	slug: string;
	kind: AssetKind;
	unused: string[];
	all_unused: boolean;
	size: number;
}

export type AssetsApi = ReturnType<typeof createAssetsApi>;

const enc = (value: string) => encodeURIComponent(value);

export function createAssetsApi(api: ApiClient) {
	return {
		list: (params: ListParams = {}) => api.get<AssetList>(`/assets/${toQuery(params)}`),
		get: (id: string) => api.get<AssetDetail>(`/assets/${enc(id)}`),
		lookup: (path: string) => api.get<AssetLookup>(`/assets/lookup/${toQuery({ path })}`),
		update: (id: string, body: { title?: string; tags?: string[] }) => api.put<AssetDetail>(`/assets/${enc(id)}`, body),
		/** a NEW file at `path` (409 `exists` when the path is taken) */
		upload: (file: Blob, path: string, meta: { title?: string; tags?: string[] } = {}) =>
			api.put<AssetDetail & { uploaded: string }>(
				`/assets/upload/${toQuery({ path, title: meta.title, tags: meta.tags?.join(',') })}`,
				file
			),
		/** new bytes for a file that is there (content editors): the old ones archived as a version, the CDN purged; 409 `unchanged` */
		replaceAt: (file: Blob, path: string, note?: string) =>
			api.put<AssetDetail & { replaced: string; archived_as: string; purged: boolean; stale_renditions: string[] }>(
				`/assets/replace/${toQuery({ path, note })}`,
				file
			),
		missing: () => api.get<{ status: 'success'; items: AssetMissing[] }>('/assets/missing/'),
		orphans: () => api.get<{ status: 'success'; items: AssetOrphan[] }>('/assets/orphans/'),

		// admin
		storage: (stats = false) => api.get<AssetStorageStatus>(`/admin/assets/storage/${toQuery({ stats: stats ? 1 : undefined })}`),
		scan: () => api.post<{ status: 'success'; import: Record<string, number>; usages: Record<string, number> }>('/admin/assets/scan/'),
		replace: (id: string, file: Blob, path?: string, note?: string) =>
			api.put<AssetDetail & { replaced: string; archived_as: string; purged: boolean; stale_renditions: string[] }>(
				`/admin/assets/${enc(id)}/file/${toQuery({ path, note })}`,
				file
			),
		rename: (id: string, slug: string, publish = false) => api.post<AssetDetail & { renamed: Record<string, string>; relink: AssetRelinkResult }>(`/admin/assets/${enc(id)}/rename/`, { slug, publish }),
		relink: (from: string, to: string, publish = false) => api.post<AssetRelinkResult>('/admin/assets/relink/', { from, to, publish }),
		deleteCheck: (id: string) => api.get<AssetDeleteCheck>(`/admin/assets/${enc(id)}/delete-check/`),
		remove: (id: string, force = false) => api.delete<{ status: 'success'; deleted: string[] }>(`/admin/assets/${enc(id)}/${toQuery({ force: force ? 1 : undefined })}`),
		enqueue: (id: string, kind: string, params: Record<string, unknown> = {}) => api.post<{ status: 'success'; job: AssetJob }>(`/admin/assets/${enc(id)}/jobs/`, { kind, params }),
		jobs: () => api.get<{ status: 'success'; items: AssetJob[]; kinds: string[] }>('/admin/assets/jobs/')
	};
}

/** The quality variants of a video asset from its renditions: { low: 480p, medium: 720p, high: 1080p } (CDN-root paths) */
export function qualityVariants(renditions: AssetRendition[]): Partial<Record<AssetQuality, string>> {
	const out: Partial<Record<AssetQuality, string>> = {};
	for (const r of renditions) if (r.quality && !out[r.quality]) out[r.quality] = r.path;
	return out;
}

/** "catpope/images/a.webp" → "images/a.webp" inside the toon folder; "../characters/x.webp" outside it */
export function toonRelativePath(path: string, toon: string): string {
	if (!toon) return path;
	return path.startsWith(`${toon}/`) ? path.slice(toon.length + 1) : `../${path}`;
}

/** the reverse: a content path of a toon document → its CDN-root path */
export function rootPath(value: string, toon: string): string {
	const parts: string[] = [];
	for (const segment of `${toon ? `${toon}/` : ''}${value.replace(/^\/+/, '')}`.split('/')) {
		if (!segment || segment === '.') continue;
		if (segment === '..') parts.pop();
		else parts.push(segment);
	}
	return parts.join('/');
}
