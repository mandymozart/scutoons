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

/** the bar: installable, the moment reached (checked on a page change / start, not live), not snoozed */
export function shouldOffer(facts: InstallFacts, memory: InstallMemory, now: number): boolean {
	return canInstall(facts, memory) && momentReached(memory) && !snoozed(memory, now);
}
