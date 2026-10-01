// The texts of the sign-in flow, the same in every app (de/en). Apps pass their locale; a component takes an optional
// `labels` object to override single texts.

export interface AuthLabels {
	loading: string;
	/** email step */
	loginIntro: string;
	signupIntro: string;
	email: string;
	emailPlaceholder: string;
	sendCode: string;
	/** apps without signup (creator, admin, embassy): only existing accounts */
	existingOnly: string;
	/** code step */
	codeSent: (email: string) => string;
	/** viewer: new addresses get a mail that explains the signup rule */
	codeSentNoAccount: string;
	otpLabel: string;
	verify: string;
	loggingIn: string;
	changeEmail: string;
	resend: string;
	marketingOptIn: string;
	/** errors */
	networkError: string;
	loginFailed: string;
	signupCodeRequired: string;
	signupCodeTaken: string;
	chooseItem: string;
	/** gate */
	signInTitle: (app: string) => string;
	deniedTitle: string;
	denied: (email: string) => string;
	switchAccount: string;
	openReader: string;
	/** signed in */
	signedInAs: string;
	signOut: string;
}

export const AUTH_LABELS: Record<'de' | 'en', AuthLabels> = {
	de: {
		loading: 'Lädt …',
		loginIntro: 'Melde dich mit deiner E-Mail an. Wir schicken dir einen Einmal-Code, kein Passwort.',
		signupIntro: 'Erstelle dein Konto mit deiner E-Mail. Wir schicken dir einen Einmal-Code, kein Passwort.',
		email: 'E-Mail',
		emailPlaceholder: 'du@beispiel.de',
		sendCode: 'Code senden',
		existingOnly: 'Hier meldest du dich mit deinem SCUTOONS-Konto an. Neue Konten entstehen im Reader.',
		codeSent: (email) =>
			`Wir haben einen Code an ${email} geschickt. Gib ihn hier ein oder tippe auf den Link in der Mail.`,
		codeSentNoAccount:
			'Falls es für diese Adresse noch kein Konto gibt, erklärt die E-Mail, wie du eins erstellst (du brauchst einen Code von einem SCUTOONS-Produkt oder einen Promo-Code).',
		otpLabel: 'Einmal-Code',
		verify: 'Anmelden',
		loggingIn: 'Anmelden …',
		changeEmail: 'Andere E-Mail',
		resend: 'Nochmal senden',
		marketingOptIn: 'Schickt mir Neuigkeiten zu neuen Veröffentlichungen',
		networkError: 'Der SCUTOONS-Server ist nicht erreichbar. Versuche es später nochmal.',
		loginFailed: 'Das hat nicht geklappt. Prüfe den Code und versuche es nochmal.',
		signupCodeRequired:
			'Für diese E-Mail gibt es noch kein Konto. Für ein Konto brauchst du einen Code von einem SCUTOONS-Produkt oder einen Promo-Code.',
		signupCodeTaken:
			'Dieser Code ist schon in einem anderen Konto, damit kann kein neues erstellt werden. Lesen kannst du trotzdem damit.',
		chooseItem: 'Wähle zuerst aus, was du bekommen hast.',
		signInTitle: (app) => `Anmeldung für ${app}`,
		deniedTitle: 'Kein Zugang',
		denied: (email) => `Dein Konto (${email}) hat keinen Zugang zu dieser App.`,
		switchAccount: 'Mit anderem Konto anmelden',
		openReader: 'Zum Reader',
		signedInAs: 'Angemeldet als',
		signOut: 'Abmelden'
	},
	en: {
		loading: 'Loading …',
		loginIntro: 'Sign in with your email. We send you a one-time code, no password.',
		signupIntro: 'Create your account with your email. We send you a one-time code, no password.',
		email: 'Email',
		emailPlaceholder: 'you@example.com',
		sendCode: 'Send code',
		existingOnly: 'Sign in with your SCUTOONS account. New accounts are created in the reader.',
		codeSent: (email) => `We sent a code to ${email}. Enter it here or tap the link in the mail.`,
		codeSentNoAccount:
			'If there is no account for this address yet, the mail explains how to create one (you need a code from a SCUTOONS product or a promo code).',
		otpLabel: 'One-time code',
		verify: 'Sign in',
		loggingIn: 'Signing in …',
		changeEmail: 'Use another email',
		resend: 'Send again',
		marketingOptIn: 'Send me news about new releases',
		networkError: "Can't reach the SCUTOONS server. Try again later.",
		loginFailed: 'That did not work. Check the code and try again.',
		signupCodeRequired:
			'There is no account for this email yet. An account needs a code from a SCUTOONS product or a promo code.',
		signupCodeTaken: "This code is already in someone else's account, so it can't create a new one. You can still read with it.",
		chooseItem: 'First choose what you got.',
		signInTitle: (app) => `${app} sign-in`,
		deniedTitle: 'No access',
		denied: (email) => `Your account (${email}) does not have access to this app.`,
		switchAccount: 'Switch account',
		openReader: 'Open the reader',
		signedInAs: 'Signed in as',
		signOut: 'Sign out'
	}
};

/** The texts of a locale (`de`, `en`, also `de-AT`); anything else → English */
export function authLabels(locale: string | null | undefined, overrides: Partial<AuthLabels> = {}): AuthLabels {
	const base = (locale ?? '').toLowerCase().startsWith('de') ? AUTH_LABELS.de : AUTH_LABELS.en;
	return { ...base, ...overrides };
}
