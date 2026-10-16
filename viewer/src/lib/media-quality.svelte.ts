// The reader's media quality (agents/DESIGN.md "Media quality", @scutoons/content media-quality.ts): a member's stored
// setting (`/me/ user.media_quality`), else the device's (localStorage `scutoons_media_quality`, guests). "Automatic"
// is resolved once per page load: the Network Information API (Chromium) says "low" right away for data saver, a 2G/3G
// connection or < 1 Mbit/s; otherwise (and in Safari / Firefox, which have no such API) a measured download of the first
// media decides (512 KB of its low variant, `qualityForMbps`). Conservative: anything unknown or failing → low.
import { browser } from '$app/environment';
import {
	autoQuality,
	isMediaQualitySetting,
	MEDIA_QUALITY_DEFAULT,
	qualityForMbps,
	type MediaQuality,
	type MediaQualitySetting,
	type NetworkHint
} from '@scutoons/content';
import { getCurrentSession } from '$lib/contexts/session.svelte.ts';

export const MEDIA_QUALITY_KEY = 'scutoons_media_quality';
/** how much of the first media "Automatic" downloads to measure the bandwidth */
const PROBE_BYTES = 512 * 1024;
const PROBE_TIMEOUT_MS = 4000;
/** the measured "Automatic" level, kept for the browser session (research/LOADING.md L9: the probe ran on every page load) */
const MEASURED_KEY = 'scutoons_media_auto';

function readMeasured(): MediaQuality | null {
	try {
		const value = sessionStorage.getItem(MEASURED_KEY);
		return value === 'low' || value === 'medium' || value === 'high' ? value : null;
	} catch {
		return null;
	}
}

function readLocal(): MediaQualitySetting {
	try {
		const value = localStorage.getItem(MEDIA_QUALITY_KEY);
		return isMediaQualitySetting(value) ? value : MEDIA_QUALITY_DEFAULT;
	} catch {
		return MEDIA_QUALITY_DEFAULT;
	}
}

function networkHint(): NetworkHint | null {
	const c = (navigator as Navigator & { connection?: NetworkHint }).connection;
	return c ? { saveData: c.saveData, effectiveType: c.effectiveType, downlink: c.downlink } : null;
}

class MediaQualityState {
	/** this device's setting (guests; a member's last choice on this device) */
	local = $state<MediaQualitySetting>(browser ? readLocal() : MEDIA_QUALITY_DEFAULT);
	/** "Automatic" resolved: from the Network Information API right away, else after `measure` (null meanwhile) */
	auto = $state<MediaQuality | null>(browser ? autoQuality(networkHint()) : null);
	/** how "Automatic" was decided (for the account page's hint and the checks) */
	autoSource = $state<'network' | 'measured' | 'fallback' | null>(null);
	#measuring = false;

	constructor() {
		if (!browser) return;
		if (this.auto) this.autoSource = 'network';
		else {
			// measured earlier in this browser session: no second probe
			this.auto = readMeasured();
			if (this.auto) this.autoSource = 'measured';
		}
		// another tab changed it
		window.addEventListener('storage', (e) => {
			if (e.key === MEDIA_QUALITY_KEY) this.local = readLocal();
		});
	}

	/**
	 * The reader's setting: the member's (server) value, else this device's. null while a stored session is still
	 * loading (so a member's video doesn't load the guest's file first).
	 */
	get setting(): MediaQualitySetting | null {
		const session = getCurrentSession();
		// still checking whether this browser is signed in (single sign-on: the session is a cookie, `pending` until known)
		if (session?.pending) return null;
		const stored = session?.isLoggedIn ? session.user?.media_quality : undefined;
		return isMediaQualitySetting(stored) ? stored : this.local;
	}

	/** The level media load at; null = not known yet (session loading, or "Automatic" still measuring). */
	get level(): MediaQuality | null {
		const setting = this.setting;
		if (setting === null) return null;
		return setting === 'auto' ? this.auto : setting;
	}

	/** "Automatic" needs a measurement before the first media loads */
	get needsMeasure(): boolean {
		return this.setting === 'auto' && this.auto === null;
	}

	/** Stores a new setting: on this device always, for a member on the server too (`PUT /me/`). */
	async set(value: MediaQualitySetting) {
		this.local = value;
		try {
			localStorage.setItem(MEDIA_QUALITY_KEY, value);
		} catch {
			/* private mode: this page keeps it */
		}
		const session = getCurrentSession();
		if (session?.isLoggedIn) await session.setMediaQuality(value);
	}

	/** the account this device's choice was offered to (once per account and page load) */
	#carried: string | number | null = null;

	/**
	 * At sign-in (owner 2026-10-16, agents/research/ACCOUNT-PAGE.md §6.3): a quality chosen on this device as a guest goes
	 * into an account that has none yet – the account's own choice always wins; the untouched default isn't carried.
	 * The root layout calls it.
	 */
	async carryToAccount(): Promise<void> {
		const session = getCurrentSession();
		const user = session?.user;
		if (!session?.isLoggedIn || !user || isMediaQualitySetting(user.media_quality) || this.#carried === user.id) return;
		let chosen: string | null = null;
		try {
			chosen = localStorage.getItem(MEDIA_QUALITY_KEY);
		} catch {
			return;
		}
		if (!isMediaQualitySetting(chosen)) return;
		this.#carried = user.id;
		try {
			await session.setMediaQuality(chosen);
		} catch {
			// offline / refused: the device's choice still applies (`setting` falls back to it); the next sign-in tries again
		}
	}

	/** Measures the bandwidth with the first `PROBE_BYTES` of `url` (once per page load); failures count as slow. */
	async measure(url: string) {
		if (!browser || this.auto || this.#measuring) return;
		this.#measuring = true;
		const controller = new AbortController();
		const timer = setTimeout(() => controller.abort(), PROBE_TIMEOUT_MS);
		const started = performance.now();
		let bytes = 0;
		try {
			const res = await fetch(url, { headers: { Range: `bytes=0-${PROBE_BYTES - 1}` }, cache: 'no-store', signal: controller.signal });
			if (!res.ok || !res.body) throw new Error(`probe ${res.status}`);
			// a server that ignores the range sends the whole file: stop after PROBE_BYTES
			const reader = res.body.getReader();
			while (bytes < PROBE_BYTES) {
				const { done, value } = await reader.read();
				if (done) break;
				bytes += value.byteLength;
			}
			await reader.cancel().catch(() => {});
			const seconds = (performance.now() - started) / 1000;
			this.auto = qualityForMbps((bytes * 8) / seconds / 1e6);
			this.autoSource = 'measured';
		} catch {
			// timed out (slow), blocked (CORS) or failed: conservative, but a slow partial download still counts
			const seconds = (performance.now() - started) / 1000;
			this.auto = bytes > 64 * 1024 ? qualityForMbps((bytes * 8) / seconds / 1e6) : 'low';
			this.autoSource = bytes > 64 * 1024 ? 'measured' : 'fallback';
		} finally {
			clearTimeout(timer);
			this.#measuring = false;
			// a real measurement is kept for the session; a fallback (blocked, nothing arrived) may be retried next page
			if (this.auto && this.autoSource === 'measured') {
				try {
					sessionStorage.setItem(MEASURED_KEY, this.auto);
				} catch {
					/* private mode: this page only */
				}
			}
		}
	}
}

/** One per page (module state): the account page sets it, every video slice reads it. */
export const mediaQuality = new MediaQualityState();
