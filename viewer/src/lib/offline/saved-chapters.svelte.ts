// "Save chapter to device" – the page side (PWA phase 3, agents/research/PWA.md §2 "Phase 3", DATA.md §6, FEATURES 10.74,
// owner 2026-10-16 "Go 2 and 3", "alle Pakete angehen"). The page downloads (progress, cancel); the service worker only
// answers from the caches (lib/sw/saved-cache.ts). Rules: lib/sw/saved-rules.ts + saved-update-rules.ts (tested); the
// download steps: ./save-files.ts; when a saved chapter is checked for a new version: ./refresh-saved.svelte.ts; a whole
// season: ./save-season.svelte.ts.
// - One cache per chapter `scutoons-chapter-<toon>-<n>-v1`: the server's file list (`…/chapters/<n>/files`) at the
//   reader's quality level (with or without the videos), the chapter / toon / cast / toons-list / credits JSON and the
//   item catalog, the fonts the page uses; the manifest last (no manifest = an unfinished save, tidied away).
// - A new version (`update`): only the changed files are downloaded, the documents again, then the new manifest, then
//   the files the new version no longer names go – a failed or stopped refresh leaves the saved version as it was.
// - Files the media cache already holds are copied, not downloaded again.
// - `navigator.storage.persist()` is asked on the first save (the browser may then not evict the saves).
// - Access: only offered for chapters the reader may read; a gated chapter keeps the access end it was saved with
//   (offline reading until then); on every start with the access known, a reader without access loses the gated ones.
// - The app code of the pages a saved chapter opens with (toon page, chapter page, account) is loaded through the worker
//   (`preloadCode` → the shell cache) when saving and on every start with saves – the shell caches app files only when
//   used, and a new build brings new chunks.
// Only with a service worker controlling the page (production builds; never in dev or the creator's Reader frame).
import { browser } from '$app/environment';
import { preloadCode } from '$app/navigation';
import { localizeHref } from '$lib/paraglide/runtime.js';
import { API_URL } from '$lib/config.ts';
import { asset } from '$lib/utils/cdn.ts';
import { MEDIA_CACHE } from '$lib/sw/cache-rules.ts';
import {
	aliasesFor,
	bytesByKind,
	documentUrls,
	fileSize,
	isSavedCacheName,
	planSave,
	savedCacheName,
	savedReadable,
	saveKindOf,
	toPurge,
	type ChapterFiles,
	type Level,
	type SavedManifest,
	type SaveKind
} from '$lib/sw/saved-rules.ts';
import type { RefreshPlan } from '$lib/sw/saved-update-rules.ts';
import { loadedFonts, manifestUrl, measureCache, storeAll, storeDocs, writeManifest } from './save-files.ts';
import { SaveError, failureOf, type SaveFailure } from './save-failure.ts';

export interface SaveJob {
	/** a first save, or a saved chapter taking in its new version */
	mode: 'save' | 'refresh';
	/** bytes so far / expected */
	done: number;
	total: number;
	/** the same per kind – the account page's bars (pictures, music, videos, the rest) */
	kinds: Record<SaveKind, { done: number; total: number }>;
	/** how the toon page names it – for the account page */
	label: string;
	controller: AbortController;
}

export interface SaveOptions {
	title: string;
	label: string;
	toonTitle: string;
	locale: string;
	level: Level;
	videos: boolean;
	list: ChapterFiles;
	/** the access end the chapter is saved with (gated chapters) */
	accessUntil: string | null;
}

const key = (toon: string, chapter: number) => `${toon}/${chapter}`;
const href = (path: string) => new URL(asset(path)).href;

/** another level's file of a saved media → the saved file, as CDN URLs */
function aliasUrls(list: ChapterFiles, saved: ChapterFiles['files']): Record<string, string> {
	const out: Record<string, string> = {};
	for (const [from, to] of Object.entries(aliasesFor(list.files, saved))) out[href(from)] = href(to);
	return out;
}

class SavedChapters {
	/** what is saved on this device (manifests), newest first */
	list = $state<SavedManifest[]>([]);
	/** running saves and refreshes by `<toon>/<n>` */
	jobs = $state<Record<string, SaveJob>>({});
	/** saved chapters with a new version waiting for the reader's tap (bytes to download) – refresh-saved.svelte.ts */
	outdated = $state<Record<string, number>>({});
	/** a service worker controls the page: saving is possible */
	supported = $state(false);
	/** why the last save / update of a chapter failed ("toon/n" → reason; gone when it saves again) */
	failures = $state<Record<string, SaveFailure>>({});
	#loaded: Promise<void> | null = null;
	/** the caches being filled on this page (not tidied away as unfinished) */
	#running = new Set<string>();

	init() {
		if (!browser || !('caches' in window) || !('serviceWorker' in navigator)) return;
		this.supported = !!navigator.serviceWorker.controller;
		navigator.serviceWorker.addEventListener('controllerchange', () => (this.supported = !!navigator.serviceWorker.controller));
		void this.refresh().then(() => {
			// this build's code for the pages the saves open with (an app update brings new chunks)
			if (this.supported && navigator.onLine) for (const m of this.list) void this.#preload(m.toon, m.chapter);
		});
	}

	/** the toon page's, the chapter page's and the account page's code into the shell cache (through the worker) */
	async #preload(toon: string, chapter: number) {
		for (const path of [`/${toon}`, `/${toon}/chapter/${chapter}`, '/account']) await preloadCode(localizeHref(path)).catch(() => {});
	}

	/** reads the manifests; unfinished saves (no manifest, not running here) are deleted */
	refresh(): Promise<void> {
		this.#loaded = (async () => {
			const out: SavedManifest[] = [];
			try {
				for (const name of await caches.keys()) {
					if (!isSavedCacheName(name)) continue;
					const cache = await caches.open(name);
					const manifest = (await (await cache.match(manifestUrl()))?.json().catch(() => null)) as SavedManifest | null;
					if (manifest) out.push(manifest);
					else if (!this.#running.has(name)) await caches.delete(name);
				}
			} catch {
				/* no Cache API */
			}
			this.list = out.sort((a, b) => b.savedAt - a.savedAt);
		})();
		return this.#loaded;
	}

	loaded(): Promise<void> {
		return this.#loaded ?? this.refresh();
	}

	get(toon: string, chapter: number): SavedManifest | undefined {
		return this.list.find((m) => m.toon === toon && m.chapter === chapter);
	}

	/** saved and still readable here (a gated one until the access it was saved with ends) */
	readable(toon: string, chapter: number): boolean {
		const saved = this.get(toon, chapter);
		return !!saved && savedReadable(saved);
	}

	job(toon: string, chapter: number): SaveJob | undefined {
		return this.jobs[key(toon, chapter)];
	}

	get bytes(): number {
		return this.list.reduce((sum, m) => sum + m.bytes, 0);
	}

	/** the server's list of a chapter's files (sizes + levels) – one request */
	async files(toon: string, chapter: number, locale: string): Promise<ChapterFiles> {
		const url = `${API_URL}/library/toons/${encodeURIComponent(toon)}/chapters/${chapter}/files?locale=${encodeURIComponent(locale)}`;
		const response = await fetch(url, { credentials: 'omit' });
		if (!response.ok) throw new SaveError('server', `files ${response.status}`);
		return (await response.json()) as ChapterFiles;
	}

	/**
	 * A job in `jobs` (the rows' rings, the account page's bars): `progress(bytes, kind, unplanned)` counts – documents
	 * and fonts have no size beforehand, they grow their kind's total as they come; `end()` takes it out.
	 */
	#begin(toon: string, chapter: number, mode: SaveJob['mode'], files: ChapterFiles['files'], label: string) {
		const id = key(toon, chapter);
		const name = savedCacheName(toon, chapter);
		const controller = new AbortController();
		const total = files.reduce((sum, f) => sum + fileSize(f), 0);
		const kinds = Object.fromEntries(Object.entries(bytesByKind(files)).map(([k, t]) => [k, { done: 0, total: t }])) as SaveJob['kinds'];
		this.jobs = { ...this.jobs, [id]: { mode, done: 0, total, kinds, label, controller } };
		if (this.failures[id]) {
			const { [id]: _was, ...rest } = this.failures;
			this.failures = rest;
		}
		this.#running.add(name);
		const progress = (bytes: number, kind: SaveKind, unplanned = false) => {
			const job = this.jobs[id];
			if (!job) return;
			const k = job.kinds[kind];
			const grown = unplanned ? bytes : 0;
			this.jobs = {
				...this.jobs,
				[id]: { ...job, done: job.done + bytes, total: job.total + grown, kinds: { ...job.kinds, [kind]: { done: k.done + bytes, total: k.total + grown } } }
			};
		};
		const end = () => {
			this.#running.delete(name);
			const { [id]: _done, ...rest } = this.jobs;
			this.jobs = rest;
		};
		return { name, controller, progress, end };
	}

	/** a failed save / update: its reason for the reader (./save-failure.ts), the real error in the console */
	#failed(toon: string, chapter: number, error: unknown) {
		const failure = failureOf(error, navigator.onLine);
		this.failures = { ...this.failures, [key(toon, chapter)]: failure };
		console.warn(`[saved] ${toon} ${chapter}: ${failure}`, error);
	}

	/** why the last save of this chapter failed (null: it didn't, or it saved since) */
	failure(toon: string, chapter: number): SaveFailure | null {
		return this.failures[key(toon, chapter)] ?? null;
	}

	async save(toon: string, chapter: number, options: SaveOptions): Promise<'saved' | 'cancelled' | 'failed'> {
		if (this.jobs[key(toon, chapter)]) return 'cancelled';
		const plan = planSave(options.list.files, options.level, options.videos);
		const { name, controller, progress, end } = this.#begin(toon, chapter, 'save', plan.files, `${options.toonTitle} · ${options.label}`);
		try {
			// the first save: ask the browser to keep the site's storage (no prompt in Chrome; Safari 17+ decides itself) –
			// never awaited: Firefox asks the reader in a bar and the promise waits for that answer (the save hung at 0 %)
			if (!this.list.length) void navigator.storage?.persist?.().catch(() => false);
			await caches.delete(name);
			const cache = await caches.open(name);

			// the code of the pages it opens with (the shell caches app files only when used)
			await this.#preload(toon, chapter);
			// the documents: exactly the URLs the app asks (lib/api.ts, the public client)
			const docs = await storeDocs(cache, documentUrls(API_URL, toon, chapter, options.locale), controller.signal, (b) => progress(b, 'data', true));
			// the media (+ the fonts the page uses – not in the plan: they add to the total as they come)
			const files = [...plan.files.map((f) => ({ url: href(f.path), kind: f.kind })), ...loadedFonts().map((url) => ({ url, kind: 'font' }))];
			await storeAll(cache, await caches.open(MEDIA_CACHE), files, controller.signal, (b, kind) => progress(b, saveKindOf(kind), kind === 'font'));

			const kinds = await measureCache(cache, new Set());
			await writeManifest(cache, {
				toon,
				chapter,
				locale: options.locale,
				title: options.title,
				label: options.label,
				toonTitle: options.toonTitle,
				level: options.level,
				videos: options.videos,
				bytes: Object.values(kinds).reduce((sum, b) => sum + b, 0),
				kinds,
				urls: [...new Set(files.map((f) => f.url))],
				docs,
				aliases: aliasUrls(options.list, plan.files),
				version: options.list.version,
				gated: options.list.gated,
				accessUntil: options.list.gated ? options.accessUntil : null,
				savedAt: Date.now()
			});
			this.tellWorker();
			await this.refresh();
			return 'saved';
		} catch (error) {
			await caches.delete(name).catch(() => {});
			if (controller.signal.aborted) return 'cancelled';
			this.#failed(toon, chapter, error);
			return 'failed';
		} finally {
			end();
		}
	}

	/**
	 * A saved chapter takes in its new version (`plan` from saved-update-rules.ts `planRefresh`): the new files, the
	 * documents again, the new manifest, then the files it no longer names go. Failed or stopped: the files it added go,
	 * the saved version stays readable as it was.
	 */
	async update(saved: SavedManifest, list: ChapterFiles, plan: RefreshPlan): Promise<'updated' | 'cancelled' | 'failed'> {
		const { toon, chapter } = saved;
		if (this.jobs[key(toon, chapter)]) return 'cancelled';
		const { name, controller, progress, end } = this.#begin(toon, chapter, 'refresh', plan.add, `${saved.toonTitle ?? toon} · ${saved.label ?? chapter}`);
		const cache = await caches.open(name);
		const had = new Set(saved.urls);
		const files = plan.add.map((f) => ({ url: href(f.path), kind: f.kind }));
		try {
			await this.#preload(toon, chapter);
			await storeAll(cache, await caches.open(MEDIA_CACHE), files, controller.signal, (b, kind) => progress(b, saveKindOf(kind)));
			const docs = await storeDocs(cache, documentUrls(API_URL, toon, chapter, saved.locale), controller.signal, (b) => progress(b, 'data', true), saved.docs);
			const drop = new Set(plan.drop);
			const kinds = await measureCache(cache, drop);
			const kept = planSave(list.files, saved.level, saved.videos).files;
			await writeManifest(cache, {
				...saved,
				bytes: Object.values(kinds).reduce((sum, b) => sum + b, 0),
				kinds,
				urls: plan.urls,
				docs,
				aliases: aliasUrls(list, kept),
				version: list.version,
				gated: list.gated,
				accessUntil: list.gated ? saved.accessUntil : null,
				refreshedAt: Date.now()
			});
			for (const url of drop) await cache.delete(url);
			const { [key(toon, chapter)]: _was, ...rest } = this.outdated;
			this.outdated = rest;
			this.tellWorker();
			await this.refresh();
			return 'updated';
		} catch (error) {
			for (const { url } of files) if (!had.has(url)) await cache.delete(url).catch(() => false);
			if (controller.signal.aborted) return 'cancelled';
			this.#failed(saved.toon, saved.chapter, error);
			return 'failed';
		} finally {
			end();
		}
	}

	cancel(toon: string, chapter: number) {
		this.jobs[key(toon, chapter)]?.controller.abort();
	}

	async remove(toon: string, chapter: number) {
		this.cancel(toon, chapter);
		await caches.delete(savedCacheName(toon, chapter)).catch(() => {});
		const { [key(toon, chapter)]: _was, ...rest } = this.outdated;
		this.outdated = rest;
		this.tellWorker();
		await this.refresh();
	}

	async removeAll() {
		for (const id of Object.keys(this.jobs)) this.jobs[id]?.controller.abort();
		for (const name of await caches.keys()) if (isSavedCacheName(name)) await caches.delete(name);
		this.outdated = {};
		this.tellWorker();
		await this.refresh();
	}

	/**
	 * On a start (root layout, once the session and the device code are checked): a reader without access loses the
	 * gated saves (`known` = online and checked; offline: by the end each was saved with); with access, the saves get
	 * the current access end (a renewal reaches offline reading).
	 */
	async checkAccess(access: { known: boolean; valid: boolean; until: string | null }) {
		await this.loaded();
		const gone = toPurge(this.list, access);
		for (const m of gone) await caches.delete(savedCacheName(m.toon, m.chapter)).catch(() => {});
		if (access.known && access.valid) {
			for (const m of this.list) {
				if (!m.gated || m.accessUntil === access.until) continue;
				await writeManifest(await caches.open(savedCacheName(m.toon, m.chapter)), { ...m, accessUntil: access.until });
			}
		}
		if (gone.length) this.tellWorker();
		await this.refresh();
	}

	/** the worker re-reads the manifests (its URL → cache index) */
	tellWorker() {
		navigator.serviceWorker?.controller?.postMessage({ type: 'SAVED_CHANGED' });
	}
}

/** One per page (module state). */
export const savedChapters = new SavedChapters();
