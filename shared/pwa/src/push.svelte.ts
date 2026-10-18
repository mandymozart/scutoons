// Web Push, the page side (agents/research/PWA.md §2b; the tools: research/PWA-APPS.md §5) – one `new PushSwitch(…)`
// per app: the reader's account page "Notifications" (its "new chapter"), the tools' "Benachrichtigungen" (the switch for
// this device + the person's kinds). Opt-in only – the browser's permission is asked on that tap, never on its own.
// Needs the service worker (production builds) and the server's VAPID key (GET /push/key; null = push is off there: no
// switch at all). iPhone / iPad: only the home-screen app may subscribe (iOS 16.4+) – Safari shows a hint instead.

/**
 * - `hidden`        nothing to show (no worker, no key on the server, a frame, an unsupported browser)
 * - `install-first` iOS Safari: works only in the home-screen app
 * - `off` / `on`    the switch
 * - `denied`        notifications are blocked in the browser's settings
 */
export type PushState = 'hidden' | 'install-first' | 'off' | 'on' | 'denied';

/** a team kind on the tools' page (server: Domain\Push\TeamPush::prefs) – `push` false = a mail channel (embassy.mail) */
export interface PushKind {
	kind: string;
	on: boolean;
	push: boolean;
}

/** the app's API client (@scutoons/api – with the session) */
export interface PushApi {
	get<T = unknown>(path: string): Promise<T>;
	post<T = unknown>(path: string, body?: unknown): Promise<T>;
	put<T = unknown>(path: string, body?: unknown): Promise<T>;
}

export interface PushSwitchOptions {
	/** viewer | creator | admin | embassy – the server sends a kind only to its app's browsers */
	app: string;
	api: PushApi;
	locale: () => string;
	/** iPhone / iPad Safari outside the home-screen app (the Install state's `iosHowTo`) */
	installFirst: () => boolean;
}

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

function sameKey(a: ArrayBuffer, b: Uint8Array): boolean {
	const x = new Uint8Array(a);
	return x.length === b.length && x.every((byte, i) => byte === b[i]);
}

export class PushSwitch {
	state = $state<PushState>('hidden');
	busy = $state(false);
	/** the last tap failed (network, the push service) */
	failed = $state(false);
	/** init() has finished: `state` is this device's answer (the reader's account page waits for it before it sends a
	 *  deep link to a group the reader can't see back to the index) */
	checked = $state(false);
	/** the tools: the person's kinds in this app (an account's choice – every device) */
	kinds = $state<PushKind[]>([]);
	#options: PushSwitchOptions;
	#key: string | null = null;
	#started = false;
	#checking: Promise<void> | null = null;

	constructor(options: PushSwitchOptions) {
		this.#options = options;
	}

	/** the page's onMount: what the switch shows (re-sends a live subscription – keeps locale + account fresh) */
	init(): Promise<void> {
		if (typeof window === 'undefined' || window.top !== window) {
			this.checked = true;
			return Promise.resolve();
		}
		this.#checking ??= this.#check().finally(() => (this.checked = true));
		return this.#checking;
	}

	async #check() {
		if (this.#started) return;
		this.#started = true;
		const { api, app } = this.#options;
		// the tools: the kinds come with the key (signed in); the reader: the key alone (guests too)
		if (app !== 'viewer') {
			try {
				const prefs = await api.get<{ key: string | null; kinds: PushKind[] }>(`/push/prefs?app=${encodeURIComponent(app)}`);
				this.kinds = prefs.kinds;
			} catch {
				this.kinds = [];
			}
		}
		const registration = await navigator.serviceWorker?.getRegistration().catch(() => undefined);
		if (!registration) return; // no worker: dev, the kill switch, private mode
		try {
			this.#key = (await api.get<{ key: string | null }>('/push/key')).key;
		} catch {
			this.#key = null;
		}
		if (!this.#key) return;
		if (this.#options.installFirst()) {
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
				await this.#options.api.post('/push/unsubscribe', { endpoint }).catch(() => {});
			}
			this.state = 'off';
		} catch {
			this.failed = true;
		} finally {
			this.busy = false;
		}
	}

	/** the tools: a kind on / off for this account (every device) */
	async setKind(kind: string, on: boolean) {
		const before = this.kinds;
		this.kinds = this.kinds.map((k) => (k.kind === kind ? { ...k, on } : k));
		this.failed = false;
		try {
			const answer = await this.#options.api.put<{ kinds: PushKind[] }>('/push/prefs', { kind, on });
			this.kinds = answer.kinds;
		} catch {
			this.kinds = before;
			this.failed = true;
		}
	}

	#store(subscription: PushSubscription) {
		const { api, app, locale } = this.#options;
		return api.post('/push/subscribe', { subscription: subscription.toJSON(), locale: locale(), ...(app === 'viewer' ? {} : { app }) });
	}
}
