// Loading ahead while reading (agents/research/LOADING.md L6 + L2): a slice that comes within the data policy's distance
// (lib/data-budget.svelte.ts) gets its image fetched – exactly the URL its <img> will ask for, without crossorigin, so the
// lazy image is then served from the cache – and announces its music tracks / samples to the audio, which loads them
// only now (the default track and the first slices' sounds still start with the chapter).
// Only after the chapter's first screen is there (`firstScreenDone`), at idle, never on the `none` tier, never twice.
// With the service worker in control, the prefetched image passes through its media cache (same URL – lib/sw/
// runtime-cache.ts), so it also survives a reload and works offline.
import { dataBudget, estimateImageBytes } from '$lib/data-budget.svelte.ts';

export interface LookaheadTarget {
	/** the slice image's URL (as the <img> will request it); '' = none */
	image: () => string;
	width: number;
	height: number;
	/** the slice's music tracks it raises (volume > 0) and its samples */
	audio: () => { tracks: string[]; samples: string[] };
	/** the audio player's hook (null = no audio on this page) */
	want: ((wanted: { tracks: string[]; samples: string[] }) => void) | null;
}

const fetched = new Set<string>();
let firstScreen: Promise<void> | null = null;
let releaseFirstScreen: (() => void) | null = null;

/** Called by the chapter page once its first slices are loaded (or failed / timed out): the guesses may start. */
export function firstScreenDone() {
	if (!firstScreen) firstScreen = Promise.resolve();
	releaseFirstScreen?.();
	releaseFirstScreen = null;
}

/** A new chapter page: the guesses wait for its first screen again. */
export function resetFirstScreen() {
	firstScreen = new Promise((resolve) => (releaseFirstScreen = resolve));
}

/** Resolves once the chapter's first screen is there (at once outside a chapter). */
export function whenFirstScreen(): Promise<void> {
	return firstScreen ?? Promise.resolve();
}

const idle = (fn: () => void) =>
	typeof requestIdleCallback === 'function' ? requestIdleCallback(fn, { timeout: 1500 }) : setTimeout(fn, 200);

function prefetchImage(url: string, width: number, height: number) {
	if (!url || fetched.has(url)) return;
	if (!dataBudget.spend(estimateImageBytes(width, height))) return;
	fetched.add(url);
	const img = new Image();
	img.decoding = 'async';
	// a guess must never compete with what is on screen
	(img as HTMLImageElement & { fetchPriority?: string }).fetchPriority = 'low';
	img.src = url;
}

/** One observer per distance (the tier can change while reading): screens → observer. */
const observers = new Map<string, IntersectionObserver>();
const targets = new WeakMap<Element, { target: LookaheadTarget; imageDone: boolean; audioDone: boolean }>();

function observerFor(screens: number, kind: 'image' | 'audio'): IntersectionObserver {
	const key = `${kind}:${screens}`;
	let observer = observers.get(key);
	if (observer) return observer;
	const below = `${Math.round(screens * 100)}%`;
	const beside = `${Math.round(Math.min(screens, 2) * 100)}%`;
	observer = new IntersectionObserver(
		(entries) => {
			for (const entry of entries) {
				if (!entry.isIntersecting) continue;
				const state = targets.get(entry.target);
				if (!state) continue;
				observer!.unobserve(entry.target);
				if (kind === 'image' && !state.imageDone) {
					state.imageDone = true;
					const { target } = state;
					void whenFirstScreen().then(() => idle(() => prefetchImage(target.image(), target.width, target.height)));
				}
				if (kind === 'audio' && !state.audioDone) {
					state.audioDone = true;
					const wanted = state.target.audio();
					if (wanted.tracks.length || wanted.samples.length) state.target.want?.(wanted);
				}
			}
		},
		// below the screen (reading direction) and beside it (strips); 0 px above: what was read is read
		{ rootMargin: `0px ${beside} ${below} ${beside}` }
	);
	observers.set(key, observer);
	return observer;
}

/** The attachment for a slice's root element. */
export function lookahead(target: LookaheadTarget) {
	return (node: Element) => {
		const policy = dataBudget.policy;
		const state = { target, imageDone: false, audioDone: false };
		targets.set(node, state);
		const image = policy.imageScreens > 0 ? observerFor(policy.imageScreens, 'image') : null;
		const audio = target.want ? observerFor(policy.audioScreens, 'audio') : null;
		image?.observe(node);
		audio?.observe(node);
		return () => {
			image?.unobserve(node);
			audio?.unobserve(node);
			targets.delete(node);
		};
	};
}
