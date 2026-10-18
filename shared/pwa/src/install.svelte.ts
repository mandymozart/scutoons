// "Install SCUTOONS as an app" on Android / Chromium (agents/research/PWA.md "Install prompt", FEATURES 10.74 phase 4):
// keeps the browser's `beforeinstallprompt` (instead of Chrome's own mini bar) and offers it at a good moment – the
// rules in lib/install-rules.ts, the bar in components/install-prompt.svelte, "Install app" on the account page.
// iPhone Safari and Firefox on Android send no such event: there the bar leads to the account page's how-to by hand
// (`way` 'ios' / 'android-firefox'). `?install` in the URL shows the bar at once (testing on a real phone, PWA.md §6).
import { browser, dev } from '$app/environment';
import { EMPTY_MEMORY, canInstall, forcedOffer, installWay, isAndroidFirefox, isIosSafari, manualWay, readMemory, shouldOffer, type InstallFacts, type InstallMemory, type InstallWay } from './install-rules.ts';

const KEY = 'scutoons_install';
/** set once per browser session: a new session = a new visit */
const VISIT_KEY = 'scutoons_install_visit';

interface BeforeInstallPromptEvent extends Event {
	prompt(): Promise<void>;
	userChoice: Promise<{ outcome: 'accepted' | 'dismissed'; platform: string }>;
}

/** an installed app (any non-browser display mode), also iOS home-screen apps and a TWA */
function runsInstalled(): boolean {
	const modes = ['standalone', 'fullscreen', 'minimal-ui', 'window-controls-overlay'];
	if (modes.some((mode) => window.matchMedia(`(display-mode: ${mode})`).matches)) return true;
	if ((navigator as Navigator & { standalone?: boolean }).standalone === true) return true;
	return document.referrer.startsWith('android-app://');
}

class Install {
	/** the bar shows */
	offer = $state(false);
	/** the browser can install now (the account page's entry) */
	available = $state(false);
	/** what this browser can do – the account page's "App" item (lib/install-rules.ts `installWay`) */
	way = $state<InstallWay>('unsupported');
	/** running as the installed app (display-mode standalone & co., iOS home-screen app) */
	installed = $state(false);
	/** `?install` was in the URL: the bar shows regardless of the moment and "Not now" (this visit) */
	#forced = false;
	#event: BeforeInstallPromptEvent | null = null;
	#memory: InstallMemory = { ...EMPTY_MEMORY };
	#framed = false;
	#standalone = false;
	/** a chapter's end bar is showing: no bar over it – the next page offers it */
	#atChapterEnd = false;
	#counted = new Set<string>();
	#started = false;

	constructor() {
		if (!browser) return;
		// registered when the layout imports this module – the event can come before the app has mounted
		window.addEventListener('beforeinstallprompt', (event) => {
			event.preventDefault(); // no Chrome mini bar: we ask at our moment
			this.#event = event as BeforeInstallPromptEvent;
			this.#refresh();
			// late (it often comes a moment after the start): offer now – unless a chapter's end bar is up
			if (this.#started && !this.#atChapterEnd) this.check();
		});
		window.addEventListener('appinstalled', () => {
			this.#remember({ installed: true });
			this.#event = null;
			this.offer = false;
			this.#refresh();
		});
	}

	#facts(): InstallFacts {
		return { available: this.#event !== null, standalone: this.#standalone, framed: this.#framed };
	}

	#refresh() {
		this.available = canInstall(this.#facts(), this.#memory);
		this.installed = this.#standalone;
		this.way = installWay(
			{ ...this.#facts(), iosSafari: isIosSafari(navigator.userAgent, navigator.maxTouchPoints), androidFirefox: isAndroidFirefox(navigator.userAgent) },
			this.#memory
		);
	}

	/** iPhone / iPad Safari, not the home-screen app (push asks to add it first) */
	get iosHowTo() {
		return this.way === 'ios';
	}

	/** installed by hand from the browser's own menu (the bar's button leads to the how-to) */
	get manual() {
		return manualWay(this.way);
	}

	#remember(change: Partial<InstallMemory>) {
		this.#memory = { ...this.#memory, ...change };
		try {
			localStorage.setItem(KEY, JSON.stringify(this.#memory));
		} catch {
			/* private window, blocked storage: this visit only */
		}
	}

	/** the root layout's onMount: counts the visit, then the first check */
	init() {
		if (!browser || this.#started) return;
		this.#started = true;
		this.#framed = window.top !== window;
		this.#standalone = runsInstalled();
		this.#forced = new URLSearchParams(location.search).has('install');
		window.matchMedia('(display-mode: standalone)').addEventListener('change', () => {
			this.#standalone = runsInstalled();
			if (this.#standalone) this.offer = false;
			this.#refresh();
		});
		let raw: string | null = null;
		try {
			raw = localStorage.getItem(KEY);
		} catch {
			/* blocked storage */
		}
		this.#memory = readMemory(raw);
		let newVisit = true;
		try {
			newVisit = sessionStorage.getItem(VISIT_KEY) === null;
			sessionStorage.setItem(VISIT_KEY, '1');
		} catch {
			/* blocked storage: every load counts – still never the very first */
		}
		if (newVisit && !this.#framed) this.#remember({ visits: this.#memory.visits + 1 });
		if (dev) (window as unknown as Record<string, unknown>).__scFakeInstallPrompt = fakeInstallPrompt;
		this.#refresh();
		this.check();
	}

	/** on the start and after each navigation: the moment to offer (never live over a chapter's end bar) */
	check() {
		this.#atChapterEnd = false;
		// the account page's "App" item says it all already
		if (/\/account\/offline\/?$/.test(location.pathname)) return void (this.offer = false);
		this.offer = this.#forced ? forcedOffer(this.way) : shouldOffer({ ...this.#facts(), manual: this.manual }, this.#memory, Date.now());
	}

	/** a chapter's end bar showed (components/chapter/next-bar.svelte): counts once per chapter and visit */
	chapterFinished(key: string) {
		this.#atChapterEnd = true;
		if (this.#counted.has(key)) return;
		this.#counted.add(key);
		this.#remember({ chapters: this.#memory.chapters + 1 });
	}

	/** "Install" (bar or account page): Chrome's own install dialog */
	async install() {
		const event = this.#event;
		this.offer = false;
		if (!event) return;
		// one prompt per event: Chrome sends a new one when it may ask again
		this.#event = null;
		this.#refresh();
		try {
			await event.prompt();
			const choice = await event.userChoice;
			// dismissed in Chrome's dialog = "Not now"; accepted: `appinstalled` follows
			if (choice.outcome === 'dismissed') this.#remember({ snoozedAt: Date.now() });
		} catch {
			/* the browser refused (no user gesture, already shown) – nothing to do */
		}
	}

	/** the bar's "How?" (no install event – iPhone Safari, Firefox on Android …): the account page's how-to; seen = snoozed */
	howTo() {
		this.offer = false;
		if (this.#forced) this.#forced = false;
		else this.#remember({ snoozedAt: Date.now() });
	}

	/** "Not now": away for SNOOZE_DAYS (the account page's entry stays) */
	notNow() {
		this.offer = false;
		this.#forced = false;
		this.#remember({ snoozedAt: Date.now() });
	}
}

/** dev only (no install event on the dev server): `__scFakeInstallPrompt()` in the console sends a fake one */
function fakeInstallPrompt(outcome: 'accepted' | 'dismissed' = 'dismissed') {
	const event = new Event('beforeinstallprompt', { cancelable: true }) as BeforeInstallPromptEvent;
	event.prompt = async () => {
		console.info('[install] fake prompt:', outcome);
		if (outcome === 'accepted') setTimeout(() => window.dispatchEvent(new Event('appinstalled')), 100);
	};
	event.userChoice = Promise.resolve({ outcome, platform: 'web' });
	window.dispatchEvent(event);
}

export const install = new Install();
