import { m } from '$lib/paraglide/messages.js';

/**
 * The ActionBar's Info panel props shared by every creator page (@scutoons/ui `ActionBar` `info` / `InfoPanel`):
 * the translated labels and the link to the full style guide (`/guide`). The page adds its own `info` snippet
 * (usually `<PageHelp page="…" />`) and `infoTitle`.
 */
export function infoBar() {
	return {
		infoHref: '/guide',
		infoLabels: { toggle: m['creator.info'](), close: m['creator.info_close'](), guide: m['creator.info_guide']() }
	};
}
