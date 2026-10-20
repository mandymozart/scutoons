// One SCUTOONS app opens another (owner 2026-10-18: "cross-referential app opening within the same phone if
// installed"; research/PWA-APPS.md §3a). Every cross-app link is a new window without an opener – desktop Chrome then
// opens the other installed app. Android keeps such a link inside the current installed app (Chrome's Custom Tab, the
// overlay with an address bar – the owner's phone, 2026-10-20). So on Android, inside an installed app, a cross-app
// link goes out as an Android intent for the https address: the system hands it to the app registered for that
// address (the other installed SCUTOONS app), else to the browser (`S.browser_fallback_url`). Outside an installed app
// (a browser tab, desktop, iOS) nothing changes.
//   <a href={url} target="_blank" rel="noopener" onclick={crossAppClick}>   ·   openApp(url) from a command

/** running as an installed app (any non-browser display mode, iOS home-screen apps, a TWA) */
export function runsInstalled(): boolean {
	if (typeof window === 'undefined') return false;
	const modes = ['standalone', 'fullscreen', 'minimal-ui', 'window-controls-overlay'];
	if (modes.some((mode) => window.matchMedia(`(display-mode: ${mode})`).matches)) return true;
	if ((navigator as Navigator & { standalone?: boolean }).standalone === true) return true;
	return document.referrer.startsWith('android-app://');
}

/** Android, inside an installed app: cross-app links go out as intents */
export function intentsNeeded(userAgent = typeof navigator === 'undefined' ? '' : navigator.userAgent, installed = runsInstalled()): boolean {
	return installed && /Android/.test(userAgent);
}

/**
 * An https address as an Android intent link: VIEW + BROWSABLE in a new task, the address itself as the browser
 * fallback (`intent://creator.scutoons.com/catpope#Intent;scheme=https;…;end`). Only http(s) addresses; anything else
 * comes back as it was. The address's own #fragment can't ride along (the intent uses #) – it is dropped.
 */
export function intentUrl(address: string): string {
	let url: URL;
	try {
		url = new URL(address);
	} catch {
		return address;
	}
	if (url.protocol !== 'https:' && url.protocol !== 'http:') return address;
	url.hash = '';
	const scheme = url.protocol.slice(0, -1);
	const rest = url.href.slice(url.protocol.length + 2); // host + path + query
	return `intent://${rest}#Intent;scheme=${scheme};action=android.intent.action.VIEW;category=android.intent.category.BROWSABLE;launchFlags=0x10000000;S.browser_fallback_url=${encodeURIComponent(url.href)};end`;
}

/** a cross-app link's click: on Android inside an installed app, the intent instead (the link's own target elsewhere) */
export function crossAppClick(event: MouseEvent) {
	if (event.defaultPrevented || event.button !== 0 || event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) return;
	if (!intentsNeeded()) return;
	const href = (event.currentTarget as HTMLAnchorElement | null)?.href;
	if (!href) return;
	event.preventDefault();
	location.href = intentUrl(href);
}

/** a command opens another app: the intent on Android inside an installed app, else a new window without an opener */
export function openApp(url: string) {
	if (intentsNeeded()) location.href = intentUrl(url);
	else window.open(url, '_blank', 'noopener');
}
