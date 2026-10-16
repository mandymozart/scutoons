// "Save chapter to device" – the page side (PWA phase 3, agents/research/PWA.md §2 "Phase 3", DATA.md §6, FEATURES 10.74,
// owner 2026-10-16 "Go 2 and 3"). The page downloads (progress, cancel); the service worker only answers from the caches
// (lib/sw/saved-cache.ts). Rules: lib/sw/saved-rules.ts (tested).
// - One cache per chapter `scutoons-chapter-<toon>-<n>-v1`: the server's file list (`…/chapters/<n>/files`) at the
//   reader's quality level (with or without the videos), the chapter / toon / cast / toons-list / credits JSON and the
//   item catalog, the fonts the page uses; the manifest last (no manifest = an unfinished save, tidied away).
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
	documentUrls,
	isSavedCacheName,
	MANIFEST_PATH,
	planSave,
	savedCacheName,
	savedReadable,
	toPurge,
	type ChapterFile,
	type ChapterFiles,
	type Level,
	type SavedManifest
} from '$lib/sw/saved-rules.ts';

/** at most this many downloads at once */
const PARALLEL = 4;
/** the documents a chapter cannot open without (the rest – cast, credits, catalog – are kept when they answer) */
const REQUIRED_DOCS = 3;

export interface SaveJob {
	/** bytes so far / expected */
	done: number;
	total: number;
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
const manifestUrl = () => new URL(MANIFEST_PATH, location.origin).href;

/** the fonts this page loaded from the CDN – lettering needs them offline */
function loadedFonts(): string[] {
	const cdn = new URL(asset('x')).origin;
	return performance
		.getEntriesByType('resource')
		.map((entry) => entry.name)
		.filter((name) => /\.woff2?$/i.test(new URL(name).pathname) && new URL(name).origin === cdn);
}

class SavedChapters {
	/** what is saved on this device (manifests), newest first */
	list = $state<SavedManifest[]>([]);
	/** running saves by `<toon>/<n>` */
	jobs = $state<Record<string, SaveJob>>({});
	/** a service worker controls the page: saving is possible */
	supported = $state(false);
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
		if (!response.ok) throw new Error(`files ${response.status}`);
		return (await response.json()) as ChapterFiles;
	}

	async save(toon: string, chapter: number, options: SaveOptions): Promise<'saved' | 'cancelled' | 'failed'> {
		const id = key(toon, chapter);
		if (this.jobs[id]) return 'cancelled';
		const plan = planSave(options.list.files, options.level, options.videos);
		const controller = new AbortController();
		this.jobs = { ...this.jobs, [id]: { done: 0, total: plan.bytes, controller } };
		const name = savedCacheName(toon, chapter);
		this.#running.add(name);
		const progress = (bytes: number) => {
			const job = this.jobs[id];
			if (job) this.jobs = { ...this.jobs, [id]: { ...job, done: job.done + bytes } };
		};
		try {
			// the first save: ask the browser to keep the site's storage (no prompt in Chrome; Safari 17+ decides itself)
			if (!this.list.length) await navigator.storage?.persist?.().catch(() => false);
			await caches.delete(name);
			const cache = await caches.open(name);

			// the code of the pages it opens with (the shell caches app files only when used)
			await this.#preload(toon, chapter);
			// the documents: exactly the URLs the app asks (lib/api.ts, the public client)
			const docs = documentUrls(API_URL, toon, chapter, options.locale);
			const kept: string[] = [];
			for (const [i, url] of docs.entries()) {
				try {
					const response = await fetch(url, { credentials: 'omit', signal: controller.signal });
					if (!response.ok) throw new Error(`${url} ${response.status}`);
					await cache.put(url, response);
					kept.push(url);
				} catch (error) {
					if (controller.signal.aborted || i < REQUIRED_DOCS) throw error;
				}
			}

			// the media (+ the fonts the page uses), PARALLEL at a time
			const files: { url: string; file?: ChapterFile }[] = [
				...plan.files.map((file) => ({ url: new URL(asset(file.path)).href, file })),
				...loadedFonts().map((url) => ({ url }))
			];
			const urls = [...new Set(files.map((f) => f.url))];
			const media = await caches.open(MEDIA_CACHE);
			let stored = 0;
			let next = 0;
			const worker = async () => {
				while (next < files.length) {
					const { url, file } = files[next++];
					// not `stored += await …` – the workers run side by side, that would add to a stale total
					const bytes = await this.#store(cache, media, url, file?.kind ?? 'font', controller.signal, progress);
					stored += bytes;
				}
			};
			await Promise.all(Array.from({ length: PARALLEL }, worker));

			const aliases: Record<string, string> = {};
			for (const [from, to] of Object.entries(aliasesFor(options.list.files, plan.files)))
				aliases[new URL(asset(from)).href] = new URL(asset(to)).href;
			const manifest: SavedManifest = {
				toon,
				chapter,
				locale: options.locale,
				title: options.title,
				label: options.label,
				toonTitle: options.toonTitle,
				level: options.level,
				videos: options.videos,
				bytes: stored,
				urls,
				docs: kept,
				aliases,
				version: options.list.version,
				gated: options.list.gated,
				accessUntil: options.list.gated ? options.accessUntil : null,
				savedAt: Date.now()
			};
			await cache.put(manifestUrl(), new Response(JSON.stringify(manifest), { headers: { 'Content-Type': 'application/json' } }));
			this.#tellWorker();
			await this.refresh();
			return 'saved';
		} catch {
			await caches.delete(name).catch(() => {});
			return controller.signal.aborted ? 'cancelled' : 'failed';
		} finally {
			this.#running.delete(name);
			const { [id]: _done, ...rest } = this.jobs;
			this.jobs = rest;
		}
	}

	/** one file into the chapter's cache: from the media cache when it is there, else downloaded (counted as it comes) */
	async #store(cache: Cache, media: Cache, url: string, kind: string, signal: AbortSignal, progress: (bytes: number) => void): Promise<number> {
		const cached = kind === 'image' ? await media.match(url, { ignoreVary: true }) : undefined;
		if (cached) {
			const blob = await cached.blob();
			await cache.put(url, new Response(blob, { headers: { 'Content-Type': cached.headers.get('content-type') ?? blob.type } }));
			progress(blob.size);
			return blob.size;
		}
		// images: past the worker's media cache (`reload` – it would keep a second copy); audio / video / fonts: the
		// browser's HTTP cache may have them already
		const response = await fetch(url, { mode: 'cors', credentials: 'omit', cache: kind === 'image' ? 'reload' : 'default', signal });
		// a file the CDN doesn't have (a stale reference) is skipped – the reader shows its gap online too
		if (response.status === 404) return 0;
		if (!response.ok || !response.body) throw new Error(`${url} ${response.status}`);
		const reader = response.body.getReader();
		const parts: Uint8Array[] = [];
		let size = 0;
		for (;;) {
			const { done, value } = await reader.read();
			if (done) break;
			parts.push(value);
			size += value.byteLength;
			progress(value.byteLength);
		}
		const type = response.headers.get('content-type') ?? '';
		const blob = new Blob(parts as BlobPart[], { type });
		await cache.put(url, new Response(blob, { headers: { 'Content-Type': type, 'Content-Length': String(size) } }));
		return size;
	}

	cancel(toon: string, chapter: number) {
		this.jobs[key(toon, chapter)]?.controller.abort();
	}

	async remove(toon: string, chapter: number) {
		this.cancel(toon, chapter);
		await caches.delete(savedCacheName(toon, chapter)).catch(() => {});
		this.#tellWorker();
		await this.refresh();
	}

	async removeAll() {
		for (const id of Object.keys(this.jobs)) this.jobs[id]?.controller.abort();
		for (const name of await caches.keys()) if (isSavedCacheName(name)) await caches.delete(name);
		this.#tellWorker();
		await this.refresh();
	}

	/**
	 * On a start (root layout, once the session and the device code are checked): a reader without access loses the
	 * gated saves (`known` = online and checked; offline: by the end each was saved with); with access, the saves get
	 * the current access end (a renewal reaches offline reading).
	 */
	async checkAccess(access: { known: boolean; valid: boolean; until: string | null }) {
		await (this.#loaded ?? this.refresh());
		const gone = toPurge(this.list, access);
		for (const m of gone) await caches.delete(savedCacheName(m.toon, m.chapter)).catch(() => {});
		if (access.known && access.valid) {
			for (const m of this.list) {
				if (!m.gated || m.accessUntil === access.until) continue;
				const cache = await caches.open(savedCacheName(m.toon, m.chapter));
				await cache.put(manifestUrl(), new Response(JSON.stringify({ ...m, accessUntil: access.until }), { headers: { 'Content-Type': 'application/json' } }));
			}
		}
		if (gone.length) this.#tellWorker();
		await this.refresh();
	}

	#tellWorker() {
		navigator.serviceWorker?.controller?.postMessage({ type: 'SAVED_CHANGED' });
	}
}

/** One per page (module state). */
export const savedChapters = new SavedChapters();
