// The Ask panel's open state, shared so a page can hand it a question (e.g. a problem's "Ask" button): `askAbout`
// opens the panel with the text in its field – the creator reads it and sends it, nothing is sent on its own.
export const ask = $state({ open: false, prefill: '' });

export function askAbout(text: string) {
	ask.prefill = text;
	ask.open = true;
}

/** the page has a panel docked on the right (Problems, History): with the Ask panel open the sidebar folds to its rail */
export const pagePanel = $state({ open: false });
