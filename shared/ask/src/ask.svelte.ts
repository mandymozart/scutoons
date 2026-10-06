// The Ask panel's open state, shared so a page can hand it a question (e.g. a problem's "Ask" button, an organisation's
// "Draft a mail"): `askAbout` opens the panel with the text in its field – the person reads it and sends it, nothing
// is sent on its own. One state per app (each app is its own bundle).
export const ask = $state({ open: false, prefill: '' });

export function askAbout(text: string) {
	ask.prefill = text;
	ask.open = true;
}

