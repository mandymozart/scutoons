// The texts of a tool's install and update bars (de/en – the apps pass their locale and their name).

export interface PwaLabels {
	/** "Install SCUTOONS Creator as an app" */
	offer: (app: string) => string;
	install: string;
	notNow: string;
	ok: string;
	/** the how-to by hand: iPhone / iPad Safari, Firefox on Android, a browser that can't */
	iosHowTo: string;
	firefoxHowTo: string;
	/** a Chromium browser without an install event now (installed already, or not offered yet) */
	chromiumHowTo: string;
	cannotHowTo: string;
	updateReady: (app: string) => string;
	later: string;
	reload: string;
}

const LABELS: Record<'de' | 'en', PwaLabels> = {
	de: {
		offer: (app) => `${app} als App installieren`,
		install: 'Installieren',
		notNow: 'Nicht jetzt',
		ok: 'OK',
		iosHowTo: 'Tippe auf Teilen und dann auf „Zum Home-Bildschirm“.',
		firefoxHowTo: 'Tippe auf das Menü ⋮ und dann auf „Zum Startbildschirm hinzufügen“ (oder „Installieren“).',
		chromiumHowTo: 'Tippe im Browser-Menü ⋮ auf „App installieren“. Schon installiert? Dann öffne die App vom Startbildschirm.',
		cannotHowTo: 'Dieser Browser kann keine Apps installieren – auf Android geht es mit Chrome oder Firefox, auf iPhone und iPad mit Safari.',
		updateReady: (app) => `Eine neue Version von ${app} ist da.`,
		later: 'Später',
		reload: 'Neu laden'
	},
	en: {
		offer: (app) => `Install ${app} as an app`,
		install: 'Install',
		notNow: 'Not now',
		ok: 'OK',
		iosHowTo: 'Tap Share, then “Add to Home Screen”.',
		firefoxHowTo: 'Tap the menu ⋮, then “Add to Home screen” (or “Install”).',
		chromiumHowTo: 'In the browser menu ⋮ tap “Install app”. Installed already? Then open the app from your home screen.',
		cannotHowTo: 'This browser can’t install apps – on Android use Chrome or Firefox, on iPhone and iPad Safari.',
		updateReady: (app) => `A new version of ${app} is ready.`,
		later: 'Later',
		reload: 'Reload'
	}
};

export function pwaLabels(locale: string): PwaLabels {
	return locale.startsWith('de') ? LABELS.de : LABELS.en;
}
