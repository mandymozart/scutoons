// The asset manager (server Domain\Assets, agents/ASSETS.md): content editors browse / look up / upload media
// (`/assets/*` – the creator's asset picker); admins replace, rename, relink, delete, scan, run jobs and see the storage
// (`/admin/assets/*`). Content keeps referencing the stable CDN paths these calls hand out.
import { toonAsset } from '@scutoons/content';
import type { ApiClient } from './client.ts';
import { toQuery, type ListParams, type ListResult } from './admin.ts';

export type AssetKind = 'image' | 'video' | 'audio' | 'archive' | 'document' | 'model' | 'other';
/** `model` = a 3D model, binary glTF (.glb) only (agents/ASSETS.md) */
export const ASSET_KINDS: AssetKind[] = ['image', 'video', 'audio', 'archive', 'document', 'model', 'other'];
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
	/** references of this file (content + catalog) – in list rows only */
	usages?: number;
}

/**
 * A shape of the vector library (agents/research/VECTORS.md "The API"): an SVG asset with a shape row. The list's
 * `kind=shape` returns only these. Colour is no property to edit – a colouration is a preset applied by the page.
 */
export interface AssetShape {
	/** repeats along this axis (a band) */
	tile: 'x' | 'y' | null;
	/** one colour: use it as a CSS mask, the page gives the colour; false = show the file as it is */
	mask: boolean;
	/** the colours the file uses (measured) */
	colors: string[];
}
/** the detail's shape: + where it came from (provenance only) and the shape it was turned from */
export interface AssetShapeDetail extends AssetShape {
	source: { file?: string; page?: string; node?: string; frame?: string; copies?: string[]; part_of?: string } | null;
	derived: { from: string; rotate?: number } | null;
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
	/** only on shapes (the vector library) */
	shape?: AssetShape;
}

export interface AssetUsage {
	path: string;
	source: 'content' | 'catalog' | 'code';
	/** entity key (content), catalog entry `<kind>:<id>` (item:camera, product:dago_hoodie …) or code file */
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
	asset: {
		id: string;
		slug: string;
		kind: AssetKind;
		folder: string | null;
		title: string;
		tags: string[];
		status: string;
		created_at: string | null;
		updated_at: string | null;
		shape?: AssetShapeDetail;
	};
	versions: AssetVersion[];
	usages: AssetUsage[];
	jobs: AssetJob[];
	cdn?: string;
}

export interface AssetList extends ListResult<AssetSummary> {
	folders: { folder: string | null; assets: number }[];
	/** map tiles the other filters found but `tiles=hide` (the default) left out (research/ASSET-LISTS.md) */
	tiles_hidden: number;
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

/**
 * One row of the files a cleanup may delete (admin → Assets → Unused, server UnusedList): an asset, or a whole map tile
 * set (`id` = `tiles:<set>`, `slug` = the set's folder, `tiles` = how many) – research/ASSET-LISTS.md
 */
export interface AssetUnused {
	id: string;
	slug: string;
	kind: AssetKind;
	/** no current file is referenced: the cleanup deletes the whole asset, its history included */
	all_unused: boolean;
	/** `current` an unreferenced file of the current version · `old` left behind by a rename · `archive` old bytes a replace kept;
	 * a page carries the first few (`file_count` = all), `all: 1` every one */
	files: { path: string; what: 'current' | 'old' | 'archive'; size: number | null }[];
	file_count: number;
	size: number;
	/** the assets the row stands for (a tile set: one per tile) */
	assets: number;
	/** a map tile set: its tiles, else null */
	tiles: number | null;
}

export interface AssetUnusedTotals {
	rows: number;
	assets: number;
	files: number;
	bytes: number;
}

/** `totals` = everything unused, `matching` = what the filters find */
export interface AssetUnusedList extends ListResult<AssetUnused> {
	totals: AssetUnusedTotals;
	matching: AssetUnusedTotals;
}

export interface AssetCleanupResult {
	status: 'success';
	deleted: string[];
	assets_deleted: number;
	bytes: number;
	/** referenced by now (or not a cleanup candidate) – never deleted */
	skipped: { path: string; reason: string }[];
	failed: Record<string, string>;
}

export type AssetsApi = ReturnType<typeof createAssetsApi>;

const enc = (value: string) => encodeURIComponent(value);

export function createAssetsApi(api: ApiClient) {
	return {
		list: (params: ListParams = {}) => api.get<AssetList>(`/assets/${toQuery(params)}`),
		get: (id: string) => api.get<AssetDetail>(`/assets/${enc(id)}`),
		lookup: (path: string) => api.get<AssetLookup>(`/assets/lookup/${toQuery({ path })}`),
		/** `shape`: the shape's facts (makes the asset part of the vector library), null = no shape any more */
		update: (id: string, body: { title?: string; tags?: string[]; shape?: (AssetShape & Partial<Pick<AssetShapeDetail, 'source' | 'derived'>>) | null }) =>
			api.put<AssetDetail>(`/assets/${enc(id)}`, body),
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
		/** a score's file into the protected storage (research/SCORES.md §5) → its path `scores/<item>/<hash>.<ext>` */
		uploadProtected: (file: Blob, item: string, kind: 'musicxml' | 'pdf') =>
			api.put<{ path: string }>(`/assets/protected/${toQuery({ item, file: kind })}`, file),
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
		/** several files in one pass: one new draft per touched entity (BACKLOG 10.64) */
		relinkMap: (map: Record<string, string>, publish = false) => api.post<AssetRelinkResult>('/admin/assets/relink/', { map, publish }),
		/** `q, folder, kind, what (asset | files), tiles (all | hide | only), sort, page, page_size`; `fresh: 1` rebuilds the
		 * usage index first, `all: 1` = every matching row with all its files (the bulk delete) */
		unused: (params: ListParams = {}) => api.get<AssetUnusedList>(`/admin/assets/unused/${toQuery(params)}`),
		/** deletes these unused files; the server checks every path against the usage index (rebuilt first unless
		 * `rebuild: false` – a bulk delete's later batches) */
		cleanup: (paths: string[], rebuild = true) => api.post<AssetCleanupResult>('/admin/assets/cleanup/', rebuild ? { paths } : { paths, rebuild: false }),
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

/** the reverse: a content path of a toon document → its CDN-root path (@scutoons/content `toonAsset`: a leading `/` = the CDN root) */
export function rootPath(value: string, toon: string): string {
	return toonAsset(toon, value);
}
