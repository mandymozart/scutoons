// Command builders the tools' palettes share (admin, creator, embassy): "go to" entries from nav links, the language
// switch and sign out. The apps pass their own texts (paraglide) and navigation (`goto`), so this stays framework-free.
import type { Command } from './command-palette.svelte';

export interface PaletteLink {
	id: string;
	label: string;
	href: string;
	hint?: string;
	keywords?: string;
	/** overrides the group given to `linkCommands` */
	group?: string;
}

/** One command per link: running it navigates to `href` with the app's `go` (SvelteKit `goto`). */
export function linkCommands(links: PaletteLink[], go: (href: string) => unknown, group?: string): Command[] {
	return links.map((link) => ({
		id: link.id,
		label: link.label,
		hint: link.hint,
		group: link.group ?? group,
		keywords: link.keywords,
		run: () => void go(link.href)
	}));
}

/** The locale the language switch goes to: the next one after `current` (de ↔ en). */
export function otherLocale<L extends string>(locales: readonly L[], current: L): L {
	return locales.find((l) => l !== current) ?? current;
}

/** "Switch language to EN": `label(locale)` builds the text, `setLocale` switches (paraglide reloads the app). */
export function languageCommand<L extends string>(options: {
	locales: readonly L[];
	current: L;
	setLocale: (locale: L) => unknown;
	label: (locale: L) => string;
	group?: string;
}): Command {
	const target = otherLocale(options.locales, options.current);
	return {
		id: 'locale',
		label: options.label(target),
		group: options.group,
		keywords: 'deutsch english language sprache',
		run: () => void options.setLocale(target)
	};
}

/** "Sign out" with the app's session logout. */
export function signOutCommand(options: { label: string; run: () => unknown; group?: string }): Command {
	return { id: 'logout', label: options.label, group: options.group, keywords: 'logout log out abmelden', run: () => void options.run() };
}
