// Screen Wake Lock (agents/research/PWA.md §2c, FEATURES 10.74 phase 4): the screen stays on while a chapter is open
// and the reader is reading, and while the record player plays. Each place holds the lock under its own name
// (`hold()` returns the release); the lock is held while any hold is active and the page is visible. The browser drops
// it when the page hides – it is asked for again when the page comes back. A chapter's hold lapses after IDLE_MS
// without a scroll / touch / key (a reader who put the phone down), and comes back with the next one.
// No-op where the API is missing (older Safari, Firefox before 126, insecure origins).
import { browser } from '$app/environment';

/** a reading hold without any input this long lets the screen sleep again */
export const IDLE_MS = 3 * 60 * 1000;
const ACTIVITY = ['scroll', 'wheel', 'touchstart', 'pointerdown', 'keydown'] as const;

interface Hold {
	idle: boolean;
	lastInput: number;
}

class WakeLock {
	/** the lock is held right now (for tests / the lab) */
	active = $state(false);
	#holds = new Map<string, Hold>();
	#sentinel: WakeLockSentinel | null = null;
	#pending = false;
	/** the last request (a refused one is not asked again on every scroll) */
	#lastTry = 0;
	#listening = false;
	#timer: ReturnType<typeof setInterval> | null = null;

	get supported(): boolean {
		return browser && 'wakeLock' in navigator;
	}

	/**
	 * Keep the screen on under `name` until the returned function is called.
	 * @param idle the hold lapses after IDLE_MS without input (reading); false = as long as it is held (music)
	 */
	hold(name: string, { idle = false }: { idle?: boolean } = {}): () => void {
		if (!this.supported) return () => {};
		this.#holds.set(name, { idle, lastInput: Date.now() });
		this.#listen();
		void this.#sync();
		return () => {
			this.#holds.delete(name);
			void this.#sync();
		};
	}

	#wanted(): boolean {
		if (document.visibilityState !== 'visible') return false;
		const now = Date.now();
		for (const hold of this.#holds.values()) if (!hold.idle || now - hold.lastInput < IDLE_MS) return true;
		return false;
	}

	async #sync() {
		const wanted = this.#wanted();
		if (wanted && !this.#sentinel && !this.#pending) {
			this.#pending = true;
			this.#lastTry = Date.now();
			try {
				const sentinel = await navigator.wakeLock.request('screen');
				this.#sentinel = sentinel;
				// the browser lets go by itself when the page hides (or the battery is low)
				sentinel.addEventListener('release', () => {
					if (this.#sentinel === sentinel) this.#sentinel = null;
					this.active = this.#sentinel !== null;
				});
			} catch {
				/* refused (hidden page, battery saver, a policy): try again on the next change */
			} finally {
				this.#pending = false;
			}
			// the holds may have ended while the request was out
			if (!this.#wanted()) await this.#drop();
		} else if (!wanted) {
			await this.#drop();
		}
		this.active = this.#sentinel !== null;
		this.#tick();
	}

	async #drop() {
		const sentinel = this.#sentinel;
		this.#sentinel = null;
		await sentinel?.release().catch(() => {});
	}

	/** an idle hold needs a look now and then (it lapses without an event) */
	#tick() {
		const idle = [...this.#holds.values()].some((h) => h.idle);
		if (idle && !this.#timer) this.#timer = setInterval(() => void this.#sync(), 30_000);
		else if (!idle && this.#timer) {
			clearInterval(this.#timer);
			this.#timer = null;
		}
	}

	#listen() {
		if (this.#listening) return;
		this.#listening = true;
		document.addEventListener('visibilitychange', () => void this.#sync());
		const input = () => {
			const now = Date.now();
			let lapsed = false;
			for (const hold of this.#holds.values()) {
				if (hold.idle && now - hold.lastInput >= IDLE_MS) lapsed = true;
				hold.lastInput = now;
			}
			if (lapsed || (!this.#sentinel && this.#holds.size && now - this.#lastTry > 5000)) void this.#sync();
		};
		for (const type of ACTIVITY) window.addEventListener(type, input, { passive: true, capture: true });
	}
}

export const wakeLock = new WakeLock();
