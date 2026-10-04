// What the Ask panel sends as "where the person is": the section by default; a page that shows one thing (an
// organisation) names it here (`label`, shown above the field) with its id (`ref`, only for the agent), so "write
// them about the new album" needs no search first.
export const askPage = $state({ label: '', ref: '' });

/** a page that shows one thing: name it while the page is open (call inside an `$effect`, returns the cleanup) */
export function askAboutPage(label: string, ref: string) {
	askPage.label = label;
	askPage.ref = ref;
	return () => {
		if (askPage.ref !== ref) return;
		askPage.label = '';
		askPage.ref = '';
	};
}

/** a mail went out from a card, or the agent logged something / moved a lead: the open organisation page reloads */
export const changed = $state({ count: 0 });
