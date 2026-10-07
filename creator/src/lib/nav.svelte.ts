import { getContext, setContext, type Snippet } from 'svelte';
import { panelWidth } from '@scutoons/ui';
import { chapterPlace, toonName as entryName, type SeasonPlaces, type ToonIndex } from '@scutoons/content';
import { getLocale } from '$lib/paraglide/runtime.js';
import { contentIndex, getDraft } from '$lib/api.ts';
import { contextKey } from '$lib/utils/shared.ts';

/**
 * Creator navigation (agents/DESIGN.md "Creator navigation"): ONE top bar with the breadcrumb of the full path
 * (SCUTOONS / toon / season / chapter / group / slice – the logo is the root crumb, UI-CONCEPTS E.2), a left sidebar with the children of the current level (+ the
 * Universe section) and the content. The root layout builds the crumbs of the route; a page adds deeper levels it
 * owns itself (the chapter editor: group, slice) and may replace the sidebar with its own children (the chapter tree,
 * the season's chapters). Alt+ArrowUp goes to the parent crumb, Alt+ArrowDown to the first child in the sidebar.
 */
export interface Crumb {
	href: string;
	label: string;
}

let nextOwner = 0;

export class CreatorNav {
	/** the route's crumbs (set by the layout) */
	base = $state<Crumb[]>([]);
	/** deeper crumbs a page owns (chapter editor: group, slice) */
	extra = $state<Crumb[]>([]);
	/** the page's own sidebar (children of the current level); null = the layout's default for the route */
	sidebar = $state<Snippet | null>(null);
	/** the page's own round buttons in the top bar, left of the Ask button (the chapter editor: Flow) */
	actions = $state<Snippet | null>(null);
	/** editors that scroll their own columns (AppShell mode "fill") */
	fill = $state(false);
	/** which page registered `extra` / `sidebar` / `fill` (a page leaving must not clear the next page's entries) */
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
export function usePageNav(options: { sidebar?: Snippet; actions?: Snippet; extra?: () => Crumb[]; fill?: boolean }) {
	const nav = getCreatorNav();
	const owner = ++nextOwner;
	$effect(() => {
		nav.owner = owner;
		nav.sidebar = options.sidebar ?? null;
		nav.actions = options.actions ?? null;
		nav.fill = !!options.fill;
		return () => {
			if (nav.owner !== owner) return;
			nav.sidebar = null;
			nav.actions = null;
			nav.fill = false;
			nav.extra = [];
		};
	});
	$effect(() => {
		const extra = options.extra?.() ?? [];
		if (nav.owner === owner || nav.owner === 0) nav.extra = extra;
	});
}

/** the Flow panel's width (every level: one stored width, 28rem by default, drags wider up to half the window) */
export const flowWidth = () => dockWidth('flow', 28);
/** a page panel's width in the shell's right dock (@scutoons/ui `useDock`): remembered per kind, folds to a rail */
export const dockWidth = (kind: string, normalRem: number) => panelWidth(`sc-panel-width:${kind}`, normalRem, undefined, { foldable: true });
