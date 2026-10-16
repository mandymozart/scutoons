// "Save chapter to device" (PWA phase 3, agents/research/PWA.md §2 "Phase 3", DATA.md §6, FEATURES 10.74): the rules,
// pure – no worker or DOM APIs – so they are tested (test/sw-saved-rules.test.ts); lib/sw/saved-cache.ts (the worker)
// and lib/offline/saved-chapters.svelte.ts (the page) carry them out.
//
// - A saved chapter is one cache `scutoons-chapter-<toon>-<n>-v1`: its media at the reader's quality level, its library
//   JSON (chapter, toon, cast, toons list, credits) + the item catalog, and a manifest (MANIFEST_PATH) written last – a
//   cache without one is an unfinished save. Never evicted by the media LRU; the kill switch deletes it (`scutoons-*`).
// - The list of files comes from the server (`/library/toons/<t>/chapters/<n>/files`): sizes + the levels each file is
//   played at, so the size is known before a byte moves.
// - Range requests (music streams, videos) are answered from the whole cached file with 206.

export type Level = 'low' | 'medium' | 'high';
export const LEVELS: readonly Level[] = ['low', 'medium', 'high'];

/** bump when the stored form changes (a new name = the old saves are not read any more) */
export const SAVED_VERSION = 'v1';
const PREFIX = 'scutoons-chapter-';
/** the manifest's key in a chapter cache (same origin as the app) */
export const MANIFEST_PATH = '/__scutoons/saved-chapter.json';

/** a file of the server's list */
export interface ChapterFile {
	path: string;
	/** bytes from the asset registry; null = unknown */
	size: number | null;
	kind: 'image' | 'video' | 'audio' | string;
	/** the levels that play it; none = every level */
	levels?: Level[];
	/** one media's files at its levels share a group (offline, the saved one answers for the others) */
	group?: number;
}

export interface ChapterFiles {
	toon: string;
	chapter: number;
	locale: string | null;
	version: string;
	/** members only (not a free preview) */
	gated: boolean;
	files: ChapterFile[];
}

/** what a cache holds – written last, so a cache without it is an unfinished save */
export interface SavedManifest {
	toon: string;
	chapter: number;
	locale: string;
	title: string;
	/** how the toon page names it ("Kapitel 1", "Prolog", "Staffel 2 · Kapitel 1") and the toon's title – for the account page */
	label?: string;
	toonTitle?: string;
	level: Level;
	videos: boolean;
	/** bytes stored */
	bytes: number;
	/** media URLs (the worker answers them from this cache) */
	urls: string[];
	/** library / catalog URLs stored with it */
	docs: string[];
	/** another level's file of a saved media → the saved file's URL (the reader's level may differ offline) */
	aliases: Record<string, string>;
	version: string;
	gated: boolean;
	/** a gated chapter: when the access it was saved with ends (ISO / SQL UTC); null = no end known */
	accessUntil: string | null;
	savedAt: number;
}

export function savedCacheName(toon: string, chapter: number): string {
	return `${PREFIX}${toon}-${chapter}-${SAVED_VERSION}`;
}

export function isSavedCacheName(name: string): boolean {
	return name.startsWith(PREFIX);
}

/** a chapter cache of another version (`-v0`): deleted */
export function isStaleSavedCacheName(name: string): boolean {
	return isSavedCacheName(name) && !name.endsWith(`-${SAVED_VERSION}`);
}

/** without a size from the registry: an estimate by kind (counted, never shown as exact) */
export const UNKNOWN_SIZE: Record<string, number> = { image: 40 * 1024, audio: 2 * 1024 * 1024, video: 5 * 1024 * 1024 };

export function fileSize(file: ChapterFile): number {
	return file.size ?? UNKNOWN_SIZE[file.kind] ?? 64 * 1024;
}

/** the files one level needs (with or without the videos) */
export function filesFor(files: ChapterFile[], level: Level, videos = true): ChapterFile[] {
	return files.filter((f) => (!f.levels || f.levels.includes(level)) && (videos || f.kind !== 'video'));
}

export interface SavePlan {
	files: ChapterFile[];
	bytes: number;
	/** bytes of the videos in it (the "without videos" offer) */
	videoBytes: number;
}

export function planSave(files: ChapterFile[], level: Level, videos = true): SavePlan {
	const picked = filesFor(files, level, videos);
	const bytes = picked.reduce((sum, f) => sum + fileSize(f), 0);
	const videoBytes = filesFor(files, level, true)
		.filter((f) => f.kind === 'video')
		.reduce((sum, f) => sum + fileSize(f), 0);
	return { files: picked, bytes, videoBytes };
}

/**
 * The stand-ins: every file of a quality group that is not saved → the group's saved file (the reader's "Automatic"
 * level can come out differently offline – the probe fails → low). Paths in, paths out.
 */
export function aliasesFor(files: ChapterFile[], saved: ChapterFile[]): Record<string, string> {
	const keep = new Map<number, string>();
	for (const f of saved) if (f.group !== undefined && !keep.has(f.group)) keep.set(f.group, f.path);
	const kept = new Set(saved.map((f) => f.path));
	const out: Record<string, string> = {};
	for (const f of files) {
		if (f.group === undefined || kept.has(f.path)) continue;
		const stand = keep.get(f.group);
		if (stand) out[f.path] = stand;
	}
	return out;
}

/** "without videos" is offered when the videos are this big and at least this share of the chapter */
export const VIDEO_OFFER_BYTES = 10 * 1024 * 1024;
export const VIDEO_OFFER_SHARE = 0.3;

export function offerWithoutVideos(plan: SavePlan): boolean {
	return plan.videoBytes >= VIDEO_OFFER_BYTES && plan.videoBytes >= plan.bytes * VIDEO_OFFER_SHARE;
}

/** on mobile data a download this big is warned about first */
export const CELLULAR_WARN_BYTES = 20 * 1024 * 1024;

export interface ConnectionHint {
	saveData?: boolean;
	type?: string;
	effectiveType?: string;
}

/** the data policy's notes for the save dialog (it always shows the size first) */
export function dataWarnings(bytes: number, c: ConnectionHint | undefined): { saveData: boolean; cellular: boolean } {
	return {
		saveData: !!c?.saveData,
		cellular: c?.type === 'cellular' && bytes >= CELLULAR_WARN_BYTES
	};
}

/** "about 18 MB" – rounded the way a reader reads it */
export function roundMb(bytes: number): string {
	const mb = bytes / (1024 * 1024);
	if (mb < 1) return '< 1';
	if (mb < 10) return String(Math.round(mb * 10) / 10);
	return String(Math.round(mb));
}

/**
 * The library + catalog answers a saved chapter needs to open offline – exactly the URLs the app asks
 * (lib/api.ts: the header-free public client; @scutoons/api createLibraryApi / createQrApi paths).
 */
export function documentUrls(api: string, toon: string, chapter: number, locale: string): string[] {
	const base = api.replace(/\/+$/, '');
	const t = `${base}/library/toons/${encodeURIComponent(toon)}`;
	const q = `?locale=${encodeURIComponent(locale)}`;
	return [`${t}/chapters/${chapter}${q}`, `${t}${q}`, `${t}/cast`, `${t}/credits`, `${base}/library/toons${q}`, `${base}/items/`, `${base}/editions/`];
}

/** the catalog reads a saved chapter keeps (network first, the saved copy offline) – the header-free client only */
export function isSavedDocRequest(
	request: { url: string; method: string; credentials?: string; headers: { has(name: string): boolean } },
	apiUrl: string
): boolean {
	if (request.method !== 'GET' || request.credentials === 'include') return false;
	if (request.headers.has('authorization') || request.headers.has('x-device-id')) return false;
	const url = new URL(request.url);
	const api = new URL(apiUrl);
	if (url.origin !== api.origin || url.search !== '') return false;
	const base = api.pathname.replace(/\/+$/, '');
	return url.pathname === `${base}/items/` || url.pathname === `${base}/editions/`;
}

/** CDN files the worker may hold for a saved chapter before it has read its index: audio, video, fonts */
const STREAMED = /\.(mp3|m4a|aac|ogg|oga|wav|flac|mp4|webm|mov|m4v|woff2?)$/i;

export function maySaveStreamed(url: URL, cdnOrigin: string): boolean {
	return url.origin === cdnOrigin && url.search === '' && STREAMED.test(url.pathname);
}

/**
 * A `Range` header against a file of `size` bytes: the bytes to send (inclusive), `null` = no range (send it whole),
 * `'unsatisfiable'` = 416. One range only (what media elements ask); a multi-range request gets the whole file.
 */
export function parseRange(header: string | null, size: number): { start: number; end: number } | null | 'unsatisfiable' {
	if (!header) return null;
	const m = /^bytes=(\d*)-(\d*)$/.exec(header.trim());
	if (!m) return null;
	const [, from, to] = m;
	if (from === '' && to === '') return null;
	let start: number;
	let end: number;
	if (from === '') {
		// the last N bytes
		const suffix = Number(to);
		if (suffix === 0) return 'unsatisfiable';
		start = Math.max(0, size - suffix);
		end = size - 1;
	} else {
		start = Number(from);
		end = to === '' ? size - 1 : Math.min(Number(to), size - 1);
	}
	if (start >= size || start > end) return 'unsatisfiable';
	return { start, end };
}

/** an access end (ISO 8601 or SQL `YYYY-MM-DD HH:MM:SS` UTC, like the membership context) has passed */
export function accessEnded(until: string | null | undefined, now = Date.now()): boolean {
	if (!until) return false;
	const t = Date.parse(/[TZ+]/.test(until.slice(10)) ? until : until.replace(' ', 'T') + 'Z');
	return !Number.isNaN(t) && now > t;
}

/** a saved chapter may be read offline: a free one always, a gated one until the access it was saved with ends */
export function savedReadable(m: Pick<SavedManifest, 'gated' | 'accessUntil'>, now = Date.now()): boolean {
	return !m.gated || !accessEnded(m.accessUntil, now);
}

/**
 * The access end a gated chapter is saved with: the latest of the reader's valid grants (device code, account);
 * null = a grant without an end (or none known – then the start's re-check decides)
 */
export function accessUntilOf(grants: (string | null | undefined)[]): string | null {
	if (!grants.length || grants.some((g) => !g)) return null;
	return grants.reduce<string | null>((latest, g) => {
		if (!latest) return g!;
		const parse = (v: string) => Date.parse(/[TZ+]/.test(v.slice(10)) ? v : v.replace(' ', 'T') + 'Z');
		return parse(g!) > parse(latest) ? g! : latest;
	}, null);
}

/**
 * On a start: which saved chapters go. Free ones never. A gated one: when the reader's access is KNOWN (online, the
 * session checked without an error, the device code re-checked) by that – no access = gone, access = kept (a renewed
 * membership outlives the end it was saved with); unknown (offline) by the access end it was saved with.
 */
export function toPurge<T extends Pick<SavedManifest, 'gated' | 'accessUntil'>>(saved: T[], access: { known: boolean; valid: boolean }, now = Date.now()): T[] {
	return saved.filter((m) => m.gated && (access.known ? !access.valid : accessEnded(m.accessUntil, now)));
}
