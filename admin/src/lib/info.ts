import { m } from '$lib/paraglide/messages.js';
import { CREATOR_URL } from '$lib/config.ts';

/**
 * The ActionBar's Info panel props shared by every admin page (@scutoons/ui `ActionBar` `info` / `InfoPanel`):
 * translated labels and the link to the full style guide (the creator's `/guide`). The page adds its `info` snippet
 * (`<PageHelp page="…" />`) and `infoTitle`.
 */
export function infoBar() {
	return {
		infoHref: `${CREATOR_URL}/guide`,
		infoLabels: { toggle: m['shell.info'](), close: m['shell.info_close'](), guide: m['shell.info_guide'](), fold: m['shell.fold_panel']() }
	};
}
