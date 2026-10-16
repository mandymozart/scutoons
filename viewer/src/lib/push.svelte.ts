// Web Push, the page side (agents/research/PWA.md §2b, FEATURES 10.74 phase 4): the account page's "Notifications"
// switch. Opt-in only – the browser's permission is asked on that tap, never on its own. The only message is "a new
// chapter is out", sent by an admin (Domain\Push\NewChapterPush) – never automatically. Needs the service worker
// (production builds) and the server's VAPID key (GET /push/key; null = push is off there: no switch at all).
// iPhone / iPad: only the home-screen app may subscribe (iOS 16.4+) – Safari shows a hint instead.
import { browser } from '$app/environment';
import { api } from '$lib/api.ts';
import { getLocale } from '$lib/paraglide/runtime.js';
import { install } from '$lib/install.svelte.ts';

/**
 * - `hidden`        nothing to show (no worker, no key on the server, the creator's frame, an unsupported browser)
 * - `install-first` iOS Safari: works only in the home-screen app
 * - `off` / `on`    the switch
 * - `denied`        the reader blocked notifications in the browser's settings
 */
export type PushState = 'hidden' | 'install-first' | 'off' | 'on' | 'denied';

/** the VAPID key (base64url) as the bytes `pushManager.subscribe` wants */
function keyBytes(key: string): Uint8Array<ArrayBuffer> {
	const base64 = (key + '='.repeat((4 - (key.length % 4)) % 4)).replace(/-/g, '+').replace(/_/g, '/');
	const raw = atob(base64);
	const bytes = new Uint8Array(new ArrayBuffer(raw.length));
	for (let i = 0; i < raw.length; i++) bytes[i] = raw.charCodeAt(i);
	return bytes;
}

function supported(): boolean {
	return 'serviceWorker' in navigator && 'PushManager' in window && 'Notification' in window;
}

class Push {
	state = $state<PushState>('hidden');
	busy = $state(false);
	/** the last tap failed (network, the push service) */
	failed = $state(false);
	#key: string | null = null;
	#started = false;
	/** init() has finished: `state` is this device's answer (the account page waits for it before it sends a deep link
	 *  to a group the reader can't see back to the index) */
	checked = $state(false);
	#checking: Promise<void> | null = null;

	/** the account page's onMount: what the switch shows (re-sends a live subscription – keeps locale + account fresh) */
	init(): Promise<void> {
		if (!browser || window.top !== window) {
			this.checked = true;
			return Promise.resolve();
		}
		this.#checking ??= this.#check().finally(() => (this.checked = true));
		return this.#checking;
	}

	async #check() {
		if (this.#started) return;
		this.#started = true;
		const registration = await navigator.serviceWorker?.getRegistration().catch(() => undefined);
		if (!registration) return; // no worker: dev, the kill switch, private mode
		try {
			this.#key = (await api.get<{ key: string | null }>('/push/key')).key;
		} catch {
			this.#key = null;
		}
		if (!this.#key) return;
		if (install.iosHowTo) {
			this.state = 'install-first';
			return;
		}
		if (!supported()) return;
		if (Notification.permission === 'denied') {
			this.state = 'denied';
			return;
		}
		const subscription = await registration.pushManager.getSubscription().catch(() => null);
		this.state = subscription && Notification.permission === 'granted' ? 'on' : 'off';
		if (subscription && this.state === 'on') void this.#store(subscription).catch(() => {});
	}

	/** the switch on: the permission (this tap is the only place it is asked), subscribe, tell the server */
	async enable() {
		if (!this.#key || this.busy) return;
		this.busy = true;
		this.failed = false;
		try {
			const permission = await Notification.requestPermission();
			if (permission !== 'granted') {
				this.state = permission === 'denied' ? 'denied' : 'off';
				return;
			}
			const registration = await navigator.serviceWorker.ready;
			let subscription = await registration.pushManager.getSubscription();
			// a subscription for another key (the server's keys changed) can't receive ours
			const current = subscription?.options.applicationServerKey;
			if (subscription && current && !sameKey(current, keyBytes(this.#key))) {
				await subscription.unsubscribe();
				subscription = null;
			}
			subscription ??= await registration.pushManager.subscribe({ userVisibleOnly: true, applicationServerKey: keyBytes(this.#key) });
			await this.#store(subscription);
			this.state = 'on';
		} catch {
			this.failed = true;
		} finally {
			this.busy = false;
		}
	}

	/** the switch off: the browser forgets the subscription, the server deletes it */
	async disable() {
		if (this.busy) return;
		this.busy = true;
		this.failed = false;
		try {
			const registration = await navigator.serviceWorker.ready;
			const subscription = await registration.pushManager.getSubscription();
			if (subscription) {
				const endpoint = subscription.endpoint;
				await subscription.unsubscribe();
				await api.post('/push/unsubscribe', { endpoint }).catch(() => {});
			}
			this.state = 'off';
		} catch {
			this.failed = true;
		} finally {
			this.busy = false;
		}
	}

	#store(subscription: PushSubscription) {
		return api.post('/push/subscribe', { subscription: subscription.toJSON(), locale: getLocale() });
	}
}

function sameKey(a: ArrayBuffer, b: Uint8Array): boolean {
	const x = new Uint8Array(a);
	return x.length === b.length && x.every((byte, i) => byte === b[i]);
}

export const push = new Push();
