// How much the reader may load ahead (agents/research/LOADING.md L10 + L6, owner 2026-10-16: "do the data measures …
// tune it in"): ONE policy for every guess – the next slices' images, the chapter's later music tracks and samples, the
// first video. It never limits what the reader actually looks at, only what is fetched before it is needed.
//
// Tiers, from the Network Information API (Chromium; Safari / Firefox have none) and the reader's media quality:
//   none     – data saver, 2G: nothing ahead (the browser's own lazy loading only), audio only when its slice is there
//   light    – 3G, < 1.5 Mbit/s, or the reader chose / was measured "low": a little ahead
//   normal   – unknown or 4G on a phone: a few screens ahead
//   generous – Wi-Fi / Ethernet (Android reports the type): the old behaviour – the whole chapter's audio at once
// Speculative bytes are counted per browser session against the tier's budget (sessionStorage) – the account page's
// "Mittel ≈ 25 MB" estimate stays true: guesses never add more than the budget.
import { browser } from '$app/environment';
import { mediaQuality } from '$lib/media-quality.svelte.ts';

export type DataTier = 'none' | 'light' | 'normal' | 'generous';

interface Connection {
	saveData?: boolean;
	effectiveType?: string;
	downlink?: number;
	type?: string;
	addEventListener?: (type: 'change', listener: () => void) => void;
}

export interface TierPolicy {
	/** slice images fetched ahead of the screen, in screen heights below (and beside) it */
	imageScreens: number;
	/** a slice's music tracks and samples start loading this many screens before the slice */
	audioScreens: number;
	/** every music track and sample at the chapter's start (no waiting for their slices) */
	audioAll: boolean;
	/** the first slices' loop video starts after their images (else only when it is on screen) */
	eagerVideo: boolean;
	/** speculative bytes per browser session */
	budget: number;
}

const MB = 1024 * 1024;
/** tuned on ch1 (95 slices, median 24 KB, p90 44 KB; music 128k ≈ 16 KB/s; a slice ≈ 0.8 phone screens, read in 2–4 s) */
export const POLICIES: Record<DataTier, TierPolicy> = {
	none: { imageScreens: 0, audioScreens: 0.5, audioAll: false, eagerVideo: false, budget: 0 },
	light: { imageScreens: 1.5, audioScreens: 2, audioAll: false, eagerVideo: false, budget: 3 * MB },
	normal: { imageScreens: 4, audioScreens: 4, audioAll: false, eagerVideo: true, budget: 12 * MB },
	generous: { imageScreens: 10, audioScreens: 10, audioAll: true, eagerVideo: true, budget: 60 * MB }
};

const SPENT_KEY = 'scutoons_prefetch_bytes';

function connection(): Connection | undefined {
	return browser ? (navigator as Navigator & { connection?: Connection }).connection : undefined;
}

/** the tier from the connection + the reader's media quality (pure – tested) */
export function tierOf(c: Connection | undefined, quality: 'low' | 'medium' | 'high' | null): DataTier {
	if (c?.saveData) return 'none';
	const effective = c?.effectiveType;
	if (effective === 'slow-2g' || effective === '2g') return 'none';
	if (effective === '3g' || (typeof c?.downlink === 'number' && c.downlink > 0 && c.downlink < 1.5)) return 'light';
	if (quality === 'low') return 'light';
	if (c?.type === 'wifi' || c?.type === 'ethernet') return 'generous';
	return 'normal';
}

function readSpent(): number {
	try {
		return Number(sessionStorage.getItem(SPENT_KEY)) || 0;
	} catch {
		return 0;
	}
}

class DataBudget {
	/** bumped when the connection changes (the tier is derived from it) */
	#version = $state(0);
	spent = $state(browser ? readSpent() : 0);

	constructor() {
		if (!browser) return;
		connection()?.addEventListener?.('change', () => this.#version++);
	}

	get tier(): DataTier {
		void this.#version;
		return tierOf(connection(), mediaQuality.level);
	}

	get policy(): TierPolicy {
		return POLICIES[this.tier];
	}

	/** Whether a guess of `bytes` still fits the session's budget; counts it when it does. */
	spend(bytes: number): boolean {
		if (this.spent + bytes > this.policy.budget) return false;
		this.spent += bytes;
		try {
			sessionStorage.setItem(SPENT_KEY, String(Math.round(this.spent)));
		} catch {
			/* private mode: counted for this page only */
		}
		return true;
	}
}

/** One per page (module state). */
export const dataBudget = new DataBudget();

/**
 * The service worker sizes its media cache by the tier (agents/research/PWA.md §2 "Phase 2": none keeps nothing new,
 * light 50 MB, normal / generous 150 MB): told on load, on every change and when a worker takes over the page.
 */
function shareTier() {
	if (!('serviceWorker' in navigator)) return;
	const tell = (tier: DataTier) => navigator.serviceWorker.controller?.postMessage({ type: 'DATA_TIER', tier });
	navigator.serviceWorker.addEventListener('controllerchange', () => {
		tell(dataBudget.tier);
		// the first worker took over this page: the images it already showed go into the media cache too (from the
		// HTTP cache), so a chapter read on the first visit works offline from its first slice
		const urls = performance.getEntriesByType('resource').map((entry) => entry.name);
		navigator.serviceWorker.controller?.postMessage({ type: 'CACHE_MEDIA', urls });
	});
	$effect.root(() => {
		$effect(() => tell(dataBudget.tier));
	});
}

if (browser) shareTier();

/** a slice image's bytes before it is loaded: ~0.1 byte per pixel (ch1's WebPs: 390 × 625 ≈ 24 KB) */
export const estimateImageBytes = (width: number, height: number) => Math.max(8 * 1024, Math.round(width * height * 0.1));
