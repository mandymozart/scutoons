// The open tabs, pure (research/redesign/PLAN.md §2.4, the deck's tabs.rs): one tab per item, the front one is the URL,
// a closed front tab hands over to its neighbour, Home when none is left. Kept in this browser. Tests:
// test/shell.test.ts.

export interface Tab {
	/** the item's `kind:id` – the tab's identity */
	address: string;
	/** the page it was last on (a toon's cast, a chapter's slice) – where the tab goes back to */
	href: string;
	/** what it was called when last in front (a reload shows it before the names load) */
	label: string;
	/** the workspace it was last shown in (a kind, or an own workspace's id): its tab brings it back there */
	workspace?: string;
}

export interface TabsState {
	tabs: Tab[];
	/** the front tab's address; null = Home */
	front: string | null;
}

export const EMPTY: TabsState = { tabs: [], front: null };

/** a tab is limited to this many; the oldest behind the front one goes first */
export const MAX_TABS = 24;

/**
 * The page at `href` (its address, null for Home) came to the front: its tab takes the page (opened right of the front
 * one when new); a page inside the front tab's own item that names another item retargets nothing – it is that item's
 * tab. `label` names it when known.
 */
export function visit(state: TabsState, address: string | null, href: string, label?: string): TabsState {
	if (!address) return state.front === null ? state : { ...state, front: null };
	const at = state.tabs.findIndex((t) => t.address === address);
	if (at >= 0) {
		const old = state.tabs[at];
		const next = { ...old, href, label: label || old.label };
		if (state.front === address && old.href === href && next.label === old.label) return state;
		const tabs = [...state.tabs];
		tabs[at] = next;
		return { tabs, front: address };
	}
	const tab: Tab = { address, href, label: label || address };
	const from = state.tabs.findIndex((t) => t.address === state.front);
	const tabs = [...state.tabs];
	tabs.splice(from >= 0 ? from + 1 : tabs.length, 0, tab);
	while (tabs.length > MAX_TABS) tabs.splice(tabs.findIndex((t) => t.address !== address), 1);
	return { tabs, front: address };
}

/**
 * The front tab's page now names another item (a pick inside the page wrote the URL – the characters' registry, a
 * catalog entry): the tab follows it, unless that item has a tab of its own – then that one comes to the front.
 */
export function retarget(state: TabsState, address: string | null, href: string): TabsState {
	if (!address || !state.front || state.front === address) return visit(state, address, href);
	if (state.tabs.some((t) => t.address === address)) return visit(state, address, href);
	return { tabs: state.tabs.map((t) => (t.address === state.front ? { address, href, label: address } : t)), front: address };
}

/** the name a page gives its own item (a crumb of its own with the item's href, as a scene's or an experience's title) */
export const pageLabel = (crumbs: readonly { href: string; label: string }[], href: string | null): string => (href && crumbs.find((c) => c.href === href)?.label) || '';

export function rename(state: TabsState, address: string, label: string): TabsState {
	const tab = state.tabs.find((t) => t.address === address);
	if (!tab || !label || tab.label === label) return state;
	return { ...state, tabs: state.tabs.map((t) => (t.address === address ? { ...t, label } : t)) };
}

/** closes a tab; the front one hands over to the tab right of it, else left of it, else Home */
export function close(state: TabsState, address: string): TabsState {
	const at = state.tabs.findIndex((t) => t.address === address);
	if (at < 0) return state;
	const tabs = state.tabs.filter((t) => t.address !== address);
	if (state.front !== address) return { tabs, front: state.front };
	return { tabs, front: (tabs[at] ?? tabs[at - 1])?.address ?? null };
}

export function closeOthers(state: TabsState): TabsState {
	return { tabs: state.tabs.filter((t) => t.address === state.front), front: state.front };
}

export function closeAll(): TabsState {
	return { tabs: [], front: null };
}

/** the tab `by` steps along the strip from the front one, round at the ends (Home in front: from the start); none open: null */
export function stepTab(state: TabsState, by: number): Tab | null {
	const n = state.tabs.length;
	if (!n) return null;
	const at = state.tabs.findIndex((t) => t.address === state.front);
	const from = at < 0 ? (by > 0 ? -1 : 0) : at;
	return state.tabs[(((from + by) % n) + n) % n];
}

/** the item at `address` was shown in workspace `id`: it remembers that */
export function keepIn(state: TabsState, address: string, id: string): TabsState {
	const at = state.tabs.findIndex((t) => t.address === address);
	if (at < 0 || state.tabs[at].workspace === id) return state;
	const tabs = [...state.tabs];
	tabs[at] = { ...tabs[at], workspace: id };
	return { ...state, tabs };
}

export const frontTab = (state: TabsState): Tab | null => state.tabs.find((t) => t.address === state.front) ?? null;

/** what the browser kept, checked: a broken store opens empty; tabs of a `gone` address (an area removed) are left out */
export function readTabs(raw: unknown, gone: (address: string) => boolean = () => false): TabsState {
	const v = raw as Partial<TabsState> | null;
	if (!v || !Array.isArray(v.tabs)) return EMPTY;
	const tabs = v.tabs
		.filter((t): t is Tab => !!t && typeof t.address === 'string' && typeof t.href === 'string' && t.href.startsWith('/') && !gone(t.address))
		.map((t) => ({ address: t.address, href: t.href, label: typeof t.label === 'string' ? t.label : t.address, ...(typeof t.workspace === 'string' && t.workspace ? { workspace: t.workspace } : {}) }))
		.filter((t, i, all) => all.findIndex((o) => o.address === t.address) === i)
		.slice(0, MAX_TABS);
	const front = typeof v.front === 'string' && tabs.some((t) => t.address === v.front) ? v.front : null;
	return { tabs, front };
}
