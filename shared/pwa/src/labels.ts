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
	/** the tools' "Benachrichtigungen" page (research/PWA-APPS.md §5–6) */
	notifications: string;
	pushTitle: string;
	pushSwitch: string;
	pushHint: string;
	pushOff: string;
	pushInstallFirst: string;
	pushDenied: string;
	pushFailed: string;
	kindsTitle: string;
	kindsHint: string;
	kinds: Record<string, { label: string; hint: string }>;
	appTitle: string;
	installApp: string;
	installHint: string;
	installedNow: string;
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
		reload: 'Neu laden',
		notifications: 'Benachrichtigungen',
		pushTitle: 'Push-Nachrichten',
		pushSwitch: 'Auf diesem Gerät',
		pushHint: 'Dieses Gerät bekommt die Nachrichten, die du unten einschaltest.',
		pushOff: 'Push ist auf diesem Server aus, oder dieser Browser kann keine Push-Nachrichten.',
		pushInstallFirst: 'Auf iPhone und iPad geht das in der App: füge sie erst zum Home-Bildschirm hinzu und schalte es dort ein.',
		pushDenied: 'Benachrichtigungen sind in den Einstellungen dieses Browsers blockiert.',
		pushFailed: 'Hat nicht geklappt – versuch es nochmal.',
		kindsTitle: 'Worüber',
		kindsHint: 'Gilt für alle deine Geräte. Nie über deine eigenen Änderungen.',
		kinds: {
			'embassy.due': { label: 'Erinnerung als Push', hint: 'Wenn Kontakte dran sind – im Rhythmus deiner Erinnerung (täglich oder montags).' },
			'embassy.mail': { label: 'Erinnerung als E-Mail', hint: 'Dieselbe Erinnerung per Mail.' },
			'admin.proposals': { label: 'Neue Vorschläge', hint: 'Wenn jemand anderes einen Vorschlag einreicht.' },
			'creator.reviews': { label: 'Freigaben', hint: 'Wenn dich jemand um Freigabe bittet.' },
			'creator.drafts': { label: 'Offene Entwürfe', hint: 'Wenn andere in deinen Toons Entwürfe gespeichert und nicht veröffentlicht haben.' }
		},
		appTitle: 'App',
		installApp: 'App installieren',
		installHint: 'Auf deinem Startbildschirm, öffnet ohne Browserleiste.',
		installedNow: 'Du nutzt sie gerade als App.'
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
		reload: 'Reload',
		notifications: 'Notifications',
		pushTitle: 'Push messages',
		pushSwitch: 'On this device',
		pushHint: 'This device gets the messages you switch on below.',
		pushOff: 'Push is off on this server, or this browser can’t receive push messages.',
		pushInstallFirst: 'On iPhone and iPad this works in the app: add it to your home screen first, then switch it on there.',
		pushDenied: 'Notifications are blocked in this browser’s settings.',
		pushFailed: 'That didn’t work – try again.',
		kindsTitle: 'About',
		kindsHint: 'For all your devices. Never about your own changes.',
		kinds: {
			'embassy.due': { label: 'Reminder as push', hint: 'When contacts are due – in your reminder’s rhythm (daily or Mondays).' },
			'embassy.mail': { label: 'Reminder as e-mail', hint: 'The same reminder by mail.' },
			'admin.proposals': { label: 'New proposals', hint: 'When someone else submits a proposal.' },
			'creator.reviews': { label: 'Publishing requests', hint: 'When someone asks you to publish.' },
			'creator.drafts': { label: 'Open drafts', hint: 'When others saved drafts in your toons and didn’t publish them.' }
		},
		appTitle: 'App',
		installApp: 'Install app',
		installHint: 'On your home screen, opens without the browser bar.',
		installedNow: 'You’re using it as an app right now.'
	}
};

export function pwaLabels(locale: string): PwaLabels {
	return locale.startsWith('de') ? LABELS.de : LABELS.en;
}
