/**
 * Universal panel tabs – the pure part. A tab is a panel's content unit (`TabSpec`); any tab can live in any panel.
 * A tab dragged into another panel lands before or after the tab it is dropped on; dragged out onto the stage it becomes
 * a panel of its own (a *free* panel, made from the tab's spec); a panel whose last tab leaves closes. A panel holding an
 * essential tab is essential itself (it folds, never closes), and a closed free panel gives its tabs back to their home
 * panels, so no tab is ever lost.
 */
import type { Dimension, PanelSpec, Rect, Size } from './floats.ts';

export interface TabSpec {
	/** What its arrangement is remembered by – never changes */
	id: string;
	name: string;
	icon: string;
	about: string;
	/** Its panel only folds, never closes */
	essential?: boolean;
	/** A free panel made from it asks for this size (rem) */
	size?: { w: Dimension; h: Dimension };
	min?: Size;
	/** Ids it was remembered by before (a stored layout naming one of them places this tab) */
	aliases?: readonly string[];
}

/**
 * Which panel holds which tabs (in order), the front tab of each, the free panels with the tab each was made from, and the
 * hidden tabs: a hidden tab keeps its place in its panel (shown again it comes back there) but its panel does not show it
 */
export interface Arrangement {
	panels: Record<string, string[]>;
	active: Record<string, string>;
	free: Record<string, string>;
	hidden?: string[];
}

/** Where a dragged tab would land: a panel and the index among its tabs; null = the stage */
export type DropTarget = { panel: string; index: number } | null;

export const FREE_PREFIX = 'free:';

const FREE_SIZE = { w: 20, h: 22 } as const;
const FREE_MIN = { w: 12, h: 8 } as const;

/** The arrangement the specs declare */
export const defaultArrangement = (panels: readonly PanelSpec[]): Arrangement => {
	const arrangement: Arrangement = { panels: {}, active: {}, free: {} };
	for (const spec of panels) {
		if (!spec.tabs?.length) continue;
		arrangement.panels[spec.id] = [...spec.tabs];
		arrangement.active[spec.id] = spec.tabs[0];
	}
	return arrangement;
};

export const panelOf = (a: Arrangement, tab: string): string | undefined =>
	Object.keys(a.panels).find((panel) => a.panels[panel].includes(tab));

/** A panel's shown tabs, in order */
export const tabsOf = (a: Arrangement, panel: string): readonly string[] => {
	const tabs = a.panels[panel] ?? [];
	return a.hidden?.length ? tabs.filter((t) => !a.hidden!.includes(t)) : tabs;
};

export const isHidden = (a: Arrangement, tab: string): boolean => !!a.hidden?.includes(tab);

/** A tab hidden: it stays in its panel's list; the panel's front tab moves to a shown neighbour */
export const hideTab = (a: Arrangement, tab: string): Arrangement => {
	const panel = panelOf(a, tab);
	if (!panel || isHidden(a, tab)) return a;
	const shown = tabsOf(a, panel);
	const rest = shown.filter((t) => t !== tab);
	const active = { ...a.active };
	if (activeOf(a, panel) === tab && rest.length) active[panel] = rest[Math.min(shown.indexOf(tab), rest.length - 1)];
	return { ...a, active, hidden: [...(a.hidden ?? []), tab] };
};

/** A hidden tab shown again, in its place, as its panel's front tab */
export const showTab = (a: Arrangement, tab: string): Arrangement => {
	const panel = panelOf(a, tab);
	if (!panel) return a;
	const hidden = (a.hidden ?? []).filter((t) => t !== tab);
	const { hidden: _was, ...rest } = a;
	return { ...rest, ...(hidden.length ? { hidden } : {}), active: { ...a.active, [panel]: tab } };
};

/** The front tab of a panel: the one it remembers, else its first */
export const activeOf = (a: Arrangement, panel: string): string | undefined => {
	const tabs = tabsOf(a, panel);
	const active = a.active[panel];
	return active && tabs.includes(active) ? active : tabs[0];
};

/** The tabs of an arrangement that no panel offers now – a kept layout naming a tab whose content is gone (no panel lists tabs: none) */
export const notOffered = (a: Arrangement, panels: readonly PanelSpec[]): string[] => {
	const offered = new Set(panels.flatMap((p) => p.tabs ?? []));
	return offered.size ? Object.values(a.panels).flat().filter((t) => !offered.has(t)) : [];
};

/** The panel that holds a tab by default */
export const homeOf = (panels: readonly PanelSpec[], tab: string): string | undefined => panels.find((p) => p.tabs?.includes(tab))?.id;

export const isEssential = (a: Arrangement, panel: string, tabs: readonly TabSpec[]): boolean =>
	tabsOf(a, panel).some((id) => tabs.find((t) => t.id === id)?.essential);

export const activate = (a: Arrangement, panel: string, tab: string): Arrangement =>
	tabsOf(a, panel).includes(tab) ? { ...a, active: { ...a.active, [panel]: tab } } : a;

/** `a` without `tab`: its panel closes when it was the last one, the panel's front tab moves to a neighbour */
const without = (a: Arrangement, tab: string): Arrangement => {
	const from = panelOf(a, tab);
	if (!from) return a;
	const tabs = a.panels[from];
	const rest = tabs.filter((t) => t !== tab);
	const panels = { ...a.panels };
	const active = { ...a.active };
	const free = { ...a.free };
	if (!rest.length) {
		delete panels[from];
		delete active[from];
		delete free[from];
	} else {
		panels[from] = rest;
		if (activeOf(a, from) === tab) active[from] = rest[Math.min(tabs.indexOf(tab), rest.length - 1)];
	}
	const hidden = a.hidden?.filter((t) => t !== tab);
	return { panels, active, free, ...(hidden?.length ? { hidden } : {}) };
};

/**
 * Move a tab into a panel before the tab at `index` (the end when `index` is past the last); in its own panel a reorder.
 * The moved tab becomes the front one.
 */
export const moveTab = (a: Arrangement, tab: string, to: string, index: number): Arrangement => {
	const from = panelOf(a, tab);
	if (!from || !a.panels[to]) return a;
	// the index counts the shown tabs: before the shown tab at it, or after the panel's last tab
	const shown = tabsOf(a, to);
	let at = index < shown.length ? a.panels[to].indexOf(shown[Math.max(0, index)]) : a.panels[to].length;
	if (from === to && a.panels[to].indexOf(tab) < at) at--;
	const rest = without(a, tab);
	const target = [...(rest.panels[to] ?? [])];
	target.splice(at, 0, tab);
	return { ...rest, panels: { ...rest.panels, [to]: target }, active: { ...rest.active, [to]: tab } };
};

/** A new free panel's id: by the tab, numbered when one by that name is still there */
export const freeId = (a: Arrangement, tab: string): string => {
	let id = `${FREE_PREFIX}${tab}`;
	for (let n = 2; a.panels[id]; n++) id = `${FREE_PREFIX}${tab}-${n}`;
	return id;
};

/** A tab torn out onto the stage: a free panel of its own (null: it is its panel's only tab – move that panel instead) */
export const tearOut = (a: Arrangement, tab: string): { arrangement: Arrangement; panel: string } | null => {
	const from = panelOf(a, tab);
	if (!from || a.panels[from].length < 2) return null;
	const panel = freeId(a, tab);
	const rest = without(a, tab);
	return {
		panel,
		arrangement: {
			panels: { ...rest.panels, [panel]: [tab] },
			active: { ...rest.active, [panel]: tab },
			free: { ...rest.free, [panel]: tab }
		}
	};
};

/**
 * A free panel closed: its tabs go back to their home panels (appended; a home panel that had emptied comes back). Any other
 * panel keeps its tabs – the floats close it and the panels menu brings it back.
 */
export const closeFree = (a: Arrangement, panel: string, panels: readonly PanelSpec[]): Arrangement => {
	if (!a.free[panel]) return a;
	let next = a;
	for (const tab of [...tabsOf(a, panel)]) {
		const home = homeOf(panels, tab);
		if (!home) continue;
		if (next.panels[home]) next = moveTab(next, tab, home, Infinity);
		else {
			const rest = without(next, tab);
			next = { ...rest, panels: { ...rest.panels, [home]: [tab] }, active: { ...rest.active, [home]: tab } };
		}
	}
	return next;
};

/** A free panel's spec, made from the tab it was torn out with */
export const freeSpec = (panel: string, tab: TabSpec): PanelSpec => ({
	id: panel,
	name: tab.name,
	icon: tab.icon,
	about: tab.about,
	place: { corner: 'top-left', dx: 0, dy: 0 },
	size: tab.size ?? FREE_SIZE,
	min: tab.min ?? FREE_MIN
});

/**
 * The arrangement from what storage holds: every known tab exactly once, in a known or a free panel; a tab missing goes
 * back to its home panel; anything else is dropped
 */
export const readArrangement = (stored: unknown, panels: readonly PanelSpec[], tabs: readonly TabSpec[]): Arrangement => {
	const fallback = defaultArrangement(panels);
	if (typeof stored !== 'object' || stored === null) return fallback;
	const s = stored as Partial<Record<keyof Arrangement, unknown>>;
	const record = (v: unknown): Record<string, unknown> => (typeof v === 'object' && v !== null && !Array.isArray(v) ? (v as Record<string, unknown>) : {});
	const known = new Set(tabs.map((t) => t.id));
	const alias = new Map(tabs.flatMap((t) => (t.aliases ?? []).map((a) => [a, t.id] as const)));
	const idOf = (v: unknown): unknown => (typeof v === 'string' ? (alias.get(v) ?? v) : v);
	const holders = new Set(panels.filter((p) => p.tabs).map((p) => p.id));
	const free = record(s.free);
	const seen = new Set<string>();
	const out: Arrangement = { panels: {}, active: {}, free: {} };
	for (const [panel, list] of Object.entries(record(s.panels))) {
		const madeFrom = idOf(free[panel]);
		const isFree = panel.startsWith(FREE_PREFIX) && typeof madeFrom === 'string' && known.has(madeFrom);
		if (!Array.isArray(list) || (!holders.has(panel) && !isFree)) continue;
		const ids: string[] = [];
		for (const raw of list) {
			const t = idOf(raw);
			if (typeof t !== 'string' || !known.has(t) || seen.has(t)) continue;
			seen.add(t);
			ids.push(t);
		}
		if (!ids.length) continue;
		out.panels[panel] = ids;
		const active = idOf(record(s.active)[panel]);
		out.active[panel] = typeof active === 'string' && ids.includes(active) ? active : ids[0];
		if (isFree) out.free[panel] = madeFrom as string;
	}
	const hidden = Array.isArray(s.hidden) ? [...new Set(s.hidden.map(idOf))].filter((t): t is string => typeof t === 'string' && seen.has(t)) : [];
	for (const tab of tabs) {
		if (seen.has(tab.id)) continue;
		const home = homeOf(panels, tab.id);
		if (!home) continue;
		out.panels[home] = [...(out.panels[home] ?? []), tab.id];
		out.active[home] ??= tab.id;
	}
	if (hidden.length) out.hidden = hidden;
	return out;
};

/** Where in a strip of tabs a pointer at `x` drops: before the first tab whose middle lies right of it */
export const dropIndex = (tabs: readonly Rect[], x: number): number => {
	const i = tabs.findIndex((r) => x < r.x + r.w / 2);
	return i < 0 ? tabs.length : i;
};

/** A panel under the pointer as a drop target: its rectangle, its tab strip and its tabs' rectangles */
export interface DropPanel {
	id: string;
	rect: Rect;
	strip: Rect;
	tabs: readonly Rect[];
}

const within = (r: Rect, x: number, y: number) => x >= r.x && x <= r.x + r.w && y >= r.y && y <= r.y + r.h;

/** The front-most panel under the pointer (the list front to back): over its strip by the tabs, elsewhere at its end */
export const dropTarget = (x: number, y: number, panels: readonly DropPanel[]): DropTarget => {
	const p = panels.find((o) => within(o.rect, x, y));
	if (!p) return null;
	return { panel: p.id, index: within(p.strip, x, y) ? dropIndex(p.tabs, x) : p.tabs.length };
};

/** The marker's x for a drop at `index`: between two tabs, before the first or after the last */
export const markerX = (tabs: readonly Rect[], strip: Rect, index: number): number => {
	if (!tabs.length) return strip.x;
	if (index <= 0) return tabs[0].x;
	const prev = tabs[Math.min(index, tabs.length) - 1];
	const next = tabs[index];
	return next ? (prev.x + prev.w + next.x) / 2 : prev.x + prev.w;
};
