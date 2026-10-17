/// <reference lib="webworker" />
// Web Push in the service worker (agents/research/PWA.md §2b, FEATURES 10.74 phase 4): shows the "new chapter"
// notification the admin sent (the payload: lib/sw/push-rules.ts), counts it on the app icon's badge, and a tap opens
// the chapter – in an open SCUTOONS window if there is one. Registered by src/service-worker.ts (`registerPush`).
import { BADGE_CACHE, BADGE_KEY, nextBadge, noticeOf, safePath } from './push-rules.ts';

type BadgeNavigator = WorkerNavigator & { setAppBadge?: (count?: number) => Promise<void> };

async function countBadge(sw: ServiceWorkerGlobalScope, tag: string) {
	const nav = sw.navigator as BadgeNavigator;
	if (!nav.setAppBadge) return;
	try {
		const cache = await caches.open(BADGE_CACHE);
		const next = nextBadge(await (await cache.match(BADGE_KEY))?.text(), tag);
		await cache.put(BADGE_KEY, new Response(next.stored, { headers: { 'Content-Type': 'application/json' } }));
		await nav.setAppBadge(next.count);
	} catch {
		/* no badge – the notification is what counts */
	}
}

async function show(sw: ServiceWorkerGlobalScope, data: PushMessageData | null) {
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
	await countBadge(sw, notice.tag);
}

async function open(sw: ServiceWorkerGlobalScope, url: string) {
	const target = new URL(url, sw.location.origin).href;
	const windows = await sw.clients.matchAll({ type: 'window', includeUncontrolled: true });
	const mine = windows.find((client) => new URL(client.url).origin === sw.location.origin) as WindowClient | undefined;
	if (mine) {
		const focused = await mine.focus().catch(() => mine);
		await focused.navigate(target).catch(() => sw.clients.openWindow(target));
		return;
	}
	await sw.clients.openWindow(target);
}

export function registerPush(sw: ServiceWorkerGlobalScope) {
	sw.addEventListener('push', (event) => event.waitUntil(show(sw, event.data)));
	sw.addEventListener('notificationclick', (event) => {
		event.notification.close();
		event.waitUntil(open(sw, safePath((event.notification.data as { url?: unknown } | null)?.url)));
	});
}
