// Saved chapters, the later parts of PWA phase 3 (agents/research/PWA.md §2 "Phase 3", FEATURES 10.74, owner 2026-10-16
// "alle Pakete angehen"): refreshing a saved chapter whose version changed, and saving a whole season. Pure – no worker
// or DOM APIs – so they are tested (test/sw-saved-rules.test.ts); lib/offline/refresh-saved.svelte.ts and
// lib/offline/save-season.svelte.ts carry them out.
import { autoSaveAllowed, fileSize, offerWithoutVideos, planSave, type AutoSave, type ChapterFile, type ChapterFiles, type ConnectionHint, type Level, type SavedManifest, type SaveKind, type SavePlan } from './saved-rules.ts';

/** a refresh: the files the new version adds, the URLs it no longer needs, the bytes it costs */
export interface RefreshPlan {
	/** the version or the files differ */
	changed: boolean;
	/** to download (not in the saved chapter yet) */
	add: ChapterFile[];
	/** saved media URLs the new version doesn't name (deleted after the new manifest is written) */
	drop: string[];
	/** every media URL of the new version (+ the fonts kept) – the new manifest's `urls` */
	urls: string[];
	/** bytes to download (the documents come on top – a few KB) */
	bytes: number;
}

/** fonts are not in the server's list (the page's lettering fonts): a refresh keeps them */
const FONT = /\.woff2?$/i;

/**
 * What a refresh of a saved chapter does: the server's list at the level and video choice it was saved with, compared
 * with what it holds. `url` turns a list path into the CDN URL the manifest keeps (lib/utils/cdn.ts `asset`).
 */
export function planRefresh(saved: Pick<SavedManifest, 'version' | 'urls' | 'level' | 'videos'>, list: ChapterFiles, url: (path: string) => string): RefreshPlan {
	const plan = planSave(list.files, saved.level, saved.videos);
	const have = new Set(saved.urls);
	const wanted = new Map(plan.files.map((f) => [url(f.path), f]));
	const add = [...wanted].filter(([u]) => !have.has(u)).map(([, f]) => f);
	const drop = saved.urls.filter((u) => !wanted.has(u) && !FONT.test(new URL(u).pathname));
	const fonts = saved.urls.filter((u) => FONT.test(new URL(u).pathname));
	return {
		changed: list.version !== saved.version || add.length > 0 || drop.length > 0,
		add,
		drop,
		urls: [...new Set([...wanted.keys(), ...fonts])],
		bytes: add.reduce((sum, f) => sum + fileSize(f), 0)
	};
}

/** a refresh this small (a text fix, a swapped picture) runs by itself whatever the setting – unless data saver / tier none */
export const SMALL_REFRESH_BYTES = 1024 * 1024;

/**
 * How a changed saved chapter is refreshed: `auto` in the background, `ask` = the account page offers "Update" (the
 * reader taps). Data saver or the data tier `none` never refresh by themselves; a small refresh always does; a bigger one
 * when the setting "Save chapters on this device" would save a chapter now (On Wi-Fi on Wi-Fi, Always).
 */
export function refreshMode(bytes: number, setting: AutoSave, c: ConnectionHint | undefined, tier: string): 'auto' | 'ask' {
	if (c?.saveData || tier === 'none') return 'ask';
	if (bytes <= SMALL_REFRESH_BYTES) return 'auto';
	return autoSaveAllowed(setting, c) ? 'auto' : 'ask';
}

/** a saved chapter is checked for a new version at most this often (on a start, on `online`) */
export const REFRESH_EVERY_MS = 30 * 60 * 1000;

export function refreshDue(lastChecked: number | undefined, now = Date.now()): boolean {
	return !lastChecked || now - lastChecked >= REFRESH_EVERY_MS;
}

/** the kind a stored URL counts as (the account page's per-kind sizes after a refresh drops files) */
export function kindOfUrl(url: string): SaveKind {
	const path = new URL(url).pathname.toLowerCase();
	if (/\.(webp|png|jpe?g|avif|gif|svg)$/.test(path)) return 'image';
	if (/\.(mp3|m4a|aac|ogg|oga|wav|flac)$/.test(path)) return 'audio';
	if (/\.(mp4|webm|mov|m4v)$/.test(path)) return 'video';
	return 'data';
}

/** a season's chapter as the save needs it */
export interface SeasonChapter {
	number: number;
	/** the reader may read it (published; a free preview or the reader's access) */
	readable: boolean;
}

/** the chapters a "save the season" saves: readable and not saved yet, in the season's order */
export function seasonTargets(chapters: SeasonChapter[], saved: (number: number) => boolean): number[] {
	return chapters.filter((c) => c.readable && !saved(c.number)).map((c) => c.number);
}

/** the season's state for its button: nothing to save, some saved, all readable ones saved */
export function seasonState(chapters: SeasonChapter[], saved: (number: number) => boolean): 'none' | 'partial' | 'saved' {
	const readable = chapters.filter((c) => c.readable);
	if (!readable.length) return 'none';
	const count = readable.filter((c) => saved(c.number)).length;
	return count === readable.length ? 'saved' : 'partial';
}

export interface SeasonPlan {
	/** per chapter: its plan with and without the videos */
	chapters: { chapter: number; all: SavePlan; lean: SavePlan }[];
	bytes: number;
	leanBytes: number;
	/** the "Everything / Without videos" choice, judged on the whole season like one chapter */
	offerLean: boolean;
}

/** the size of a season's save – the server's lists of its chapters at one level */
export function planSeason(lists: ChapterFiles[], level: Level): SeasonPlan {
	const chapters = lists.map((list) => ({ chapter: list.chapter, all: planSave(list.files, level, true), lean: planSave(list.files, level, false) }));
	const bytes = chapters.reduce((sum, c) => sum + c.all.bytes, 0);
	const videoBytes = chapters.reduce((sum, c) => sum + c.all.videoBytes, 0);
	return {
		chapters,
		bytes,
		leanBytes: chapters.reduce((sum, c) => sum + c.lean.bytes, 0),
		offerLean: offerWithoutVideos({ files: [], bytes, videoBytes })
	};
}
