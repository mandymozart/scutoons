import { getContext, setContext, type Snippet } from 'svelte';
import { panelWidth, type PanelWidth } from '@scutoons/ui';
import { toonName as entryName, type ToonIndex } from '@scutoons/content';
import { getLocale } from '$lib/paraglide/runtime.js';

/**
 * Creator navigation (agents/DESIGN.md "Creator navigation"): ONE top bar with the breadcrumb of the full path
 * (Creator / toon / season / chapter / group / slice), a left sidebar with the children of the current level (+ the
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
	/** the page's panel in the shell's right dock (Problems, History, Flow, …): one at a time; null = none open */
	dock = $state.raw<PageDock | null>(null);
	/** which page registered `extra` / `sidebar` / `fill` (a page leaving must not clear the next page's entries) */
	owner = 0;

	/** every toon (library, drafts included), loaded by the layout once the creator is signed in */
	toons = $state<ToonIndex[]>([]);

	readonly crumbs = $derived([...this.base, ...this.extra]);

	/** A toon's display name in the UI locale (else its source language); the id only while the list is loading. */
	toonName(id: string): string {
		const t = this.toons.find((x) => x.id === id || x.uri === id);
		return (t && entryName(t, getLocale())) || id;
	}
	/** where "up" goes: the crumb before the last one */
	readonly parent = $derived(this.crumbs.length > 1 ? this.crumbs[this.crumbs.length - 2] : null);
}

const KEY = Symbol('creator-nav');

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

/** a page's panel in the AppShell's right dock (agents/DOCKING.md) and its remembered width */
export interface PageDock {
	panel: Snippet;
	size: PanelWidth;
	/** the page that docked it */
	owner: object;
}

/**
 * Docks the page's open panel on the right while the page is mounted (call during component init): `current()` says
 * which one is open (its snippet and its width) or null. The shell makes room for it – left of Ask, never under it;
 * with Ask open too the sidebar folds to its rail. Pages never position panels themselves.
 */
export function usePageDock(current: () => { panel: Snippet; size: PanelWidth } | null) {
	const nav = getCreatorNav();
	const owner = {};
	$effect(() => {
		const open = current();
		nav.dock = open ? { ...open, owner } : null;
		return () => {
			if (nav.dock?.owner === owner) nav.dock = null;
		};
	});
}

/** the Flow panel's width (every level: one stored width, 28rem by default, drags wider up to half the window) */
export const flowWidth = () => panelWidth('sc-panel-width:flow', 28);
