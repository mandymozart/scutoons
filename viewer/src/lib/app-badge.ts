// The app icon's badge (agents/research/PWA.md §2b, a proposal the owner may reject): it counts the "new chapter"
// notifications that came since the app was last opened (the worker sets it, lib/sw/push.ts). Opening the app – or
// coming back to it – clears it. No-op where the Badging API is missing (installed apps only: Chromium, iOS 16.4+).
import { BADGE_CACHE, BADGE_KEY } from './sw/push-rules.ts';

type BadgeNavigator = Navigator & { clearAppBadge?: () => Promise<void> };

async function clear() {
	const nav = navigator as BadgeNavigator;
	if (!nav.clearAppBadge || document.visibilityState !== 'visible') return;
	try {
		await nav.clearAppBadge();
		await (await caches.open(BADGE_CACHE)).delete(BADGE_KEY);
	} catch {
		/* nothing to clear */
	}
}

/** the root layout's onMount */
export function initAppBadge() {
	void clear();
	document.addEventListener('visibilitychange', () => void clear());
}
