// Web Push in a service worker (agents/research/PWA.md §2b; the tools: research/PWA-APPS.md §5) – shows the notification
// the server sent (the payload: ./push-rules.ts), and a tap opens its page – in an open window of this app if there is
// one, only paths on the app's own origin. The reader also counts it on the app icon's badge (`badge: true`).
// Each app's src/service-worker.ts calls `registerPush(self, …)` (the tools through startToolWorker).
import { BADGE_CACHE, BADGE_KEY, nextBadge, noticeOf, safePath } from './push-rules.ts';

/** the bits of the worker scope push uses */
export interface PushScope {
	addEventListener(type: 'push', listener: (event: { data: { text(): string } | null; waitUntil(p: Promise<unknown>): void }) => void): void;
	addEventListener(
		type: 'notificationclick',
		listener: (event: { notification: { close(): void; data: unknown }; waitUntil(p: Promise<unknown>): void }) => void
	): void;
	registration: { showNotification(title: string, options: Record<string, unknown>): Promise<void> };
	clients: {
		matchAll(options: { type: 'window'; includeUncontrolled: boolean }): Promise<readonly WindowLike[]>;
		openWindow(url: string): Promise<unknown>;
	};
	location: { origin: string };
	navigator: { setAppBadge?: (count?: number) => Promise<void> };
}
interface WindowLike {
	url: string;
	focus(): Promise<WindowLike>;
	navigate(url: string): Promise<unknown>;
}

async function countBadge(sw: PushScope, tag: string) {
	if (!sw.navigator.setAppBadge) return;
	try {
		const cache = await caches.open(BADGE_CACHE);
		const next = nextBadge(await (await cache.match(BADGE_KEY))?.text(), tag);
		await cache.put(BADGE_KEY, new Response(next.stored, { headers: { 'Content-Type': 'application/json' } }));
		await sw.navigator.setAppBadge(next.count);
	} catch {
		/* no badge – the notification is what counts */
	}
}

async function show(sw: PushScope, data: { text(): string } | null, options: PushOptions) {
	let text: string | null = null;
	try {
		text = data?.text() ?? null;
	} catch {
		/* unreadable: the plain notice */
	}
	const notice = noticeOf(text);
	// every push shows a notification (Chrome / Safari require it – a silent push would cost the permission)
	await sw.registration.showNotification(notice.title, {
		body: notice.body,
		tag: notice.tag,
		icon: '/app-icon-192.png',
		data: { url: notice.url },
		lang: notice.url.startsWith('/en/') ? 'en' : 'de'
	});
	if (options.badge) await countBadge(sw, notice.tag);
}

async function open(sw: PushScope, url: string) {
	const target = new URL(url, sw.location.origin).href;
	const windows = await sw.clients.matchAll({ type: 'window', includeUncontrolled: true });
	const mine = windows.find((client) => new URL(client.url).origin === sw.location.origin);
	if (mine) {
		const focused = await mine.focus().catch(() => mine);
		await focused.navigate(target).catch(() => sw.clients.openWindow(target));
		return;
	}
	await sw.clients.openWindow(target);
}

export interface PushOptions {
	/** count the notifications on the app icon (the reader's "new chapter" – PWA.md §2b) */
	badge?: boolean;
}

export function registerPush(sw: PushScope, options: PushOptions = {}) {
	sw.addEventListener('push', (event) => event.waitUntil(show(sw, event.data, options)));
	sw.addEventListener('notificationclick', (event) => {
		event.notification.close();
		event.waitUntil(open(sw, safePath((event.notification.data as { url?: unknown } | null)?.url)));
	});
}
