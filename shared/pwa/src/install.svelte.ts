// "Install SCUTOONS as an app" (agents/research/PWA.md "Install prompt", every app: research/PWA-APPS.md): keeps the
// browser's `beforeinstallprompt` (instead of Chrome's own mini bar) and offers it at a good moment – the rules in
// ./install-rules.ts; each app makes one `new Install({ key })` and draws its own bar (the reader: install-prompt.svelte
// + the account page's "App"; the tools: ./install-bar.svelte).
// iPhone Safari and Firefox on Android send no such event: there the bar shows the how-to by hand (`way` 'ios' /
// 'android-firefox'). `?install` in the URL shows the bar at once (testing on a real phone, PWA.md §6).
import { EMPTY_MEMORY, canInstall, forcedOffer, installWay, isAndroidFirefox, isChromium, isIosSafari, manualWay, readMemory, shouldOffer, type InstallFacts, type InstallMemory, type InstallWay } from './install-rules.ts';

const browser = typeof window !== 'undefined';

export interface InstallOptions {
	/** localStorage key of what this browser remembers (one per app – each app installs on its own) */
	key: string;
	/** a page that explains installing itself (the reader's account page): no bar there */
	quietPath?: RegExp;
	/** dev builds: `__scFakeInstallPrompt()` in the console */
	dev?: boolean;
}

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

export class Install {
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
	/** a milestone is showing (the reader: a chapter's end bar): no bar over it – the next page offers it */
	#atMilestone = false;
	#options: InstallOptions;
	#counted = new Set<string>();
	#started = false;

	constructor(options: InstallOptions) {
		this.#options = options;
		if (!browser) return;
		// registered when the layout imports this module – the event can come before the app has mounted
		window.addEventListener('beforeinstallprompt', (event) => {
			event.preventDefault(); // no Chrome mini bar: we ask at our moment
			this.#event = event as BeforeInstallPromptEvent;
			this.#refresh();
			// late (it often comes a moment after the start): offer now – unless a milestone is up
			if (this.#started && !this.#atMilestone) this.check();
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
			{
				...this.#facts(),
				iosSafari: isIosSafari(navigator.userAgent, navigator.maxTouchPoints),
				androidFirefox: isAndroidFirefox(navigator.userAgent),
				chromium: isChromium(navigator.userAgent)
			},
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
			localStorage.setItem(this.#options.key, JSON.stringify(this.#memory));
		} catch {
			/* private window, blocked storage: this visit only */
		}
	}

	/** the app's root layout onMount: counts the visit, then the first check */
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
			raw = localStorage.getItem(this.#options.key);
		} catch {
			/* blocked storage */
		}
		this.#memory = readMemory(raw);
		let newVisit = true;
		try {
			// set once per browser session: a new session = a new visit
			const visitKey = `${this.#options.key}_visit`;
			newVisit = sessionStorage.getItem(visitKey) === null;
			sessionStorage.setItem(visitKey, '1');
		} catch {
			/* blocked storage: every load counts – still never the very first */
		}
		if (newVisit && !this.#framed) this.#remember({ visits: this.#memory.visits + 1 });
		if (this.#options.dev) (window as unknown as Record<string, unknown>).__scFakeInstallPrompt = fakeInstallPrompt;
		this.#refresh();
		this.check();
	}

	/** on the start and after each navigation: the moment to offer (never live over a milestone) */
	check() {
		this.#atMilestone = false;
		// a page that explains installing itself (the reader's account page "App" item)
		if (this.#options.quietPath?.test(location.pathname)) return void (this.offer = false);
		this.offer = this.#forced ? forcedOffer(this.way) : shouldOffer({ ...this.#facts(), manual: this.manual }, this.#memory, Date.now());
	}

	/** a milestone showed (the reader: a chapter's end bar, next-bar.svelte): counts once per key and visit */
	milestone(key: string) {
		this.#atMilestone = true;
		if (this.#counted.has(key)) return;
		this.#counted.add(key);
		this.#remember({ chapters: this.#memory.chapters + 1 });
	}

	/** "Install" (the bar, the reader's account page): the browser's own install dialog */
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

	/** the bar's "How?" / "OK" (no install event – iPhone Safari, Firefox on Android …): the how-to seen = snoozed */
	howTo() {
		this.offer = false;
		if (this.#forced) this.#forced = false;
		else this.#remember({ snoozedAt: Date.now() });
	}

	/** "Not now": away for SNOOZE_DAYS (an app's own install entry stays) */
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

