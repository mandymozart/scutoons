// When the "Install SCUTOONS as an app" bar may show (agents/research/PWA.md "Install prompt", FEATURES 10.74 phase 4) –
// pure logic, tested in test/install-rules.test.ts; the page side is lib/install.svelte.ts.
//
// Never on the first visit: the bar is offered once the reader has come back (a 2nd visit = a 2nd browser session) or
// finished a chapter (its end bar showed) – and then only on the next page (a navigation or the next start), never
// over the chapter's end bar. Not inside an installed app, not in the creator's Reader frame, not after "Not now" for
// SNOOZE_DAYS, never again after the app was installed.

export const SNOOZE_DAYS = 30;
const DAY = 24 * 60 * 60 * 1000;

/** what this browser remembers (localStorage `scutoons_install`) */
export interface InstallMemory {
	/** browser sessions that opened SCUTOONS (the first one = 1) */
	visits: number;
	/** chapters whose end the reader reached */
	chapters: number;
	/** "Not now" – the time (ms) */
	snoozedAt: number | null;
	/** `appinstalled` came: never again on this browser */
	installed: boolean;
}

export const EMPTY_MEMORY: InstallMemory = { visits: 0, chapters: 0, snoozedAt: null, installed: false };

/** a stored value back (anything broken → empty) */
export function readMemory(raw: string | null): InstallMemory {
	if (!raw) return { ...EMPTY_MEMORY };
	try {
		const value = JSON.parse(raw) as Partial<InstallMemory> | null;
		if (!value || typeof value !== 'object') return { ...EMPTY_MEMORY };
		const count = (n: unknown) => (typeof n === 'number' && Number.isFinite(n) && n > 0 ? Math.floor(n) : 0);
		return {
			visits: count(value.visits),
			chapters: count(value.chapters),
			snoozedAt: typeof value.snoozedAt === 'number' && Number.isFinite(value.snoozedAt) ? value.snoozedAt : null,
			installed: value.installed === true
		};
	} catch {
		return { ...EMPTY_MEMORY };
	}
}

/** "Not now" still holds */
export function snoozed(memory: InstallMemory, now: number): boolean {
	return memory.snoozedAt !== null && now - memory.snoozedAt < SNOOZE_DAYS * DAY && now >= memory.snoozedAt;
}

export interface InstallFacts {
	/** the browser handed us `beforeinstallprompt` (Chromium; iOS never) */
	available: boolean;
	/** running as an installed app (display-mode standalone & co.) */
	standalone: boolean;
	/** inside a frame (the creator's dock Reader) */
	framed: boolean;
}

/** the account page's "Install app": whenever the browser can install, also when the bar is snoozed */
export function canInstall(facts: InstallFacts, memory: InstallMemory): boolean {
	return facts.available && !facts.standalone && !facts.framed && !memory.installed;
}

/** the reader has shown they come back: a 2nd visit or a finished chapter */
export function momentReached(memory: InstallMemory): boolean {
	return memory.visits >= 2 || memory.chapters >= 1;
}

/**
 * the bar: installable – by the browser's event, or by hand (`manual`: iPhone Safari, Firefox on Android – the bar then
 * leads to the how-to) – the moment reached (checked on a page change / start, not live), not snoozed
 */
export function shouldOffer(facts: InstallFacts & { manual?: boolean }, memory: InstallMemory, now: number): boolean {
	const installable = canInstall(facts, memory) || (!!facts.manual && !facts.standalone && !facts.framed);
	return installable && momentReached(memory) && !snoozed(memory, now);
}

/**
 * `?install` in the URL (testing on a real phone, PWA.md §6): the bar shows at once – whatever the moment and "Not now" –
 * on every browser except inside the app and the creator's frame; where nothing can install it explains why
 */
export function forcedOffer(way: InstallWay): boolean {
	return way !== 'installed' && way !== 'hidden';
}

/**
 * iPhone / iPad Safari (iOS has no install event – the account page shows "Share → Add to Home Screen" instead):
 * an iOS device (iPadOS reports a Mac with touch) in Safari itself – not Chrome / Firefox / Edge / Opera / the Google
 * app on iOS, not an in-app browser (Instagram, Facebook …; they send no "Version/… Safari/" or their own token).
 */
export function isIosSafari(userAgent: string, maxTouchPoints = 0): boolean {
	const ios = /iPhone|iPad|iPod/.test(userAgent) || (/Macintosh/.test(userAgent) && maxTouchPoints > 1);
	if (!ios || !/Safari\//.test(userAgent) || !/Version\//.test(userAgent)) return false;
	return !/CriOS|FxiOS|EdgiOS|OPiOS|OPT\/|GSA\/|YaBrowser|DuckDuckGo|FBAN|FBAV|Instagram|Line\//.test(userAgent);
}

/** Firefox on Android: installs only from its own menu (⋮ → "Add to Home screen") – it sends no install event */
export function isAndroidFirefox(userAgent: string): boolean {
	return /Android/.test(userAgent) && /Firefox\//.test(userAgent);
}

/**
 * Safari itself, not Chromium / Firefox (on a Mac too): deletes a site's storage – the saved chapters too – after 7 days
 * without a visit, unless SCUTOONS runs from the home screen / Dock (WebKit's tracking prevention).
 */
export function isSafari(userAgent: string, maxTouchPoints = 0): boolean {
	if (isIosSafari(userAgent, maxTouchPoints)) return true;
	return /Macintosh/.test(userAgent) && /Version\/[\d.]+.*Safari\//.test(userAgent) && !/Chrome|Chromium|Edg\/|OPR\/|Firefox/.test(userAgent);
}

/**
 * The account page's "App" item – what this browser can do:
 * installed (running as the app) · available (the browser's install dialog) · ios (Share → Add to Home Screen) ·
 * android-firefox (⋮ → Add to Home screen) · unsupported (this browser can't, or Chrome hasn't offered it: already
 * installed, not yet) · hidden (the creator's Reader frame).
 */
export type InstallWay = 'installed' | 'available' | 'ios' | 'android-firefox' | 'unsupported' | 'hidden';

export function installWay(facts: InstallFacts & { iosSafari: boolean; androidFirefox: boolean }, memory: InstallMemory): InstallWay {
	if (facts.framed) return 'hidden';
	if (facts.standalone) return 'installed';
	if (canInstall(facts, memory)) return 'available';
	if (facts.iosSafari) return 'ios';
	if (facts.androidFirefox) return 'android-firefox';
	return 'unsupported';
}

/** the how-to by hand (no install event): iPhone / iPad Safari, Firefox on Android */
export function manualWay(way: InstallWay): boolean {
	return way === 'ios' || way === 'android-firefox';
}
