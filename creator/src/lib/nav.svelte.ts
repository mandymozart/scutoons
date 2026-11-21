import { getContext, setContext } from 'svelte';
import { chapterPlace, toonName as entryName, type SeasonPlaces, type ToonIndex } from '@scutoons/content';
import { getLocale } from '$lib/paraglide/runtime.js';
import { contentIndex, getDraft } from '$lib/api.ts';
import { contextKey } from '$lib/utils/shared.ts';

/**
 * Creator navigation: the path of the open item (toon / season / chapter / group / slice) – the bar's middle. The root
 * layout builds the crumbs of the route; a page adds deeper levels it owns itself (the chapter editor: group, slice).
 * Alt+ArrowUp goes to the parent crumb. A page's list is its Tree tab (`useTabs`, lib/shell/shell.svelte.ts).
 */
export interface Crumb {
	href: string;
	label: string;
}

let nextOwner = 0;

/** a route's id as the URL reads it: every creator page lives in the route group (app) (#126 – the phone page /h/ lives
 *  outside it), which SvelteKit keeps in `page.route.id` ("/(app)/map") but not in the URL ("/map") */
export const routeId = (id: string | null | undefined): string => (id ?? '/').replace(/^\/\(app\)(?=\/|$)/, '') || '/';

export class CreatorNav {
	/** the route's crumbs (set by the layout) */
	base = $state<Crumb[]>([]);
	/** deeper crumbs a page owns (chapter editor: group, slice) */
	extra = $state<Crumb[]>([]);
	/** the page fills the stage and scrolls its own columns (a canvas, an editor); else the stage scrolls the page */
	fill = $state(false);
	/** which page registered `extra` / `fill` (a page leaving must not clear the next page's entries) */
	owner = 0;

	/** every toon (library, drafts included), loaded by the layout once the creator is signed in */
	toons = $state<ToonIndex[]>([]);

	readonly crumbs = $derived([...this.base, ...this.extra]);

	/** toon → its seasons (number + stored sequence, newest versions) – where a chapter sits (the season crumb) */
	seasons = $state<Record<string, SeasonPlaces[]>>({});
	#seasonLoads = new Map<string, number>();

	/**
	 * (Re)loads a toon's seasons (the season crumb of a chapter, BACKLOG 10.45): the language-neutral season documents
	 * (a not yet converted season: its first language file). The last result stays while a reload runs (no flicker).
	 */
	async loadSeasons(toon: string): Promise<void> {
		const run = (this.#seasonLoads.get(toon) ?? 0) + 1;
		this.#seasonLoads.set(toon, run);
		try {
			const index = (await contentIndex(toon)).entities.filter((e) => e.kind === 'season');
			const chosen = new Map<number, string>();
			for (const e of [...index].sort((a, b) => Number(!!a.locale) - Number(!!b.locale) || a.key.localeCompare(b.key))) {
				const number = Number(e.key.split(':')[2]);
				if (Number.isInteger(number) && !chosen.has(number)) chosen.set(number, e.key);
			}
			const places = await Promise.all(
				[...chosen].map(async ([number, key]) => {
					const doc: any = await getDraft(key)
						.then((r) => r.content)
						.catch(() => null);
					return { number, sequence: Array.isArray(doc?.sequence) ? doc.sequence : [] };
				})
			);
			if (this.#seasonLoads.get(toon) === run) this.seasons[toon] = places;
		} catch {
			// no index (no access, offline): the chapter crumbs go without a season
		}
	}

	/** the season a chapter belongs to (the first that lists it); null: none lists it; undefined: not loaded yet */
	chapterSeason(toon: string, chapter: number): number | null | undefined {
		const seasons = this.seasons[toon];
		if (!seasons) return undefined;
		return chapterPlace(seasons, chapter)?.season ?? null;
	}

	/** A toon's display name in the UI locale (else its source language); the id only while the list is loading. */
	toonName(id: string): string {
		const t = this.toons.find((x) => x.id === id || x.uri === id);
		return (t && entryName(t, getLocale())) || id;
	}
	/** where "up" goes: the crumb before the last one */
	readonly parent = $derived(this.crumbs.length > 1 ? this.crumbs[this.crumbs.length - 2] : null);
}

const KEY = contextKey('creator-nav');

export function setCreatorNav() {
	return setContext(KEY, new CreatorNav());
}

export function getCreatorNav(): CreatorNav {
	return getContext<CreatorNav>(KEY);
}

/**
 * Registers a page's part of the navigation while the page is mounted (call during component init).
 * `extra` is a function so the crumbs follow the page's state.
 */
export function usePageNav(options: { extra?: () => Crumb[]; fill?: boolean }) {
	const nav = getCreatorNav();
	const owner = ++nextOwner;
	$effect(() => {
		nav.owner = owner;
		nav.fill = !!options.fill;
		return () => {
			if (nav.owner !== owner) return;
			nav.fill = false;
			nav.extra = [];
		};
	});
	$effect(() => {
		const extra = options.extra?.() ?? [];
		if (nav.owner === owner || nav.owner === 0) nav.extra = extra;
	});
}
