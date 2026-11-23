// The bar's workspace row (research/redesign/lap/workspace-thoughts.md points 30, 48): one flat row, most used first,
// never grouped by craft. An open item remembers its workspace; a workspace that does not fit the front item is dimmed
// and opens its own last item. Kept in this browser. Pure: test/shell.test.ts.
import type { Tab } from './tabs.ts';
import { AREAS, workspaceOf, type WorkspaceKind } from './address.ts';

export interface WorkspaceUse {
	/** how often each workspace came to the front */
	uses: Partial<Record<WorkspaceKind, number>>;
	/** the item each workspace showed last */
	last: Partial<Record<WorkspaceKind, { address: string; href: string }>>;
}

export const NO_USE: WorkspaceUse = { uses: {}, last: {} };

/** the workspaces the row offers, in their order before any use (Help lives in the menu) */
export const ROW: readonly WorkspaceKind[] = ['toon', 'chapter', 'storyboard', 'character', 'scene', 'catalog', 'map', 'flow', 'sound', 'text'];

/** where a workspace starts when it has shown nothing yet (an area's own list); the others need an item first (a character opens from the Library) */
const STARTS: Partial<Record<WorkspaceKind, string>> = { scene: '/scenes', catalog: '/catalog/items', map: '/map', flow: '/flow' };

/** an address that only passes through (the old characters list goes Home): no workspace remembers it */
const passing = (address: string) => address === AREAS.characters;

/** the front item fits a workspace: it can be shown there (today one workspace per kind) */
export const fits = (address: string | null, kind: WorkspaceKind): boolean => !!address && workspaceOf(address) === kind;

/** the front item changed (`from`: the workspace in front before): its workspace remembers it and, entered anew, counts a use */
export function entered(use: WorkspaceUse, from: WorkspaceKind | null, address: string | null, href: string): WorkspaceUse {
	const kind = address && !passing(address) ? workspaceOf(address) : null;
	if (!kind || !address) return use;
	const was = use.last[kind];
	const counted = from !== kind;
	if (!counted && was?.address === address && was.href === href) return use;
	return {
		uses: counted ? { ...use.uses, [kind]: (use.uses[kind] ?? 0) + 1 } : use.uses,
		last: { ...use.last, [kind]: { address, href } }
	};
}

export interface RowEntry {
	kind: WorkspaceKind;
	/** where clicking it goes: its last item (as its open tab is now), else an open item of it, else its start */
	href: string;
}

/** the row: every workspace with somewhere to go, most used first (ties: the row's own order) */
export function workspaceRow(use: WorkspaceUse, tabs: readonly Tab[]): RowEntry[] {
	const entries: RowEntry[] = [];
	for (const kind of ROW) {
		const last = use.last[kind];
		const open = (last && tabs.find((t) => t.address === last.address)) ?? tabs.find((t) => !passing(t.address) && workspaceOf(t.address) === kind);
		const href = open?.href ?? last?.href ?? STARTS[kind];
		if (href) entries.push({ kind, href });
	}
	const rank = (k: WorkspaceKind) => use.uses[k] ?? 0;
	return entries.sort((a, b) => rank(b.kind) - rank(a.kind) || ROW.indexOf(a.kind) - ROW.indexOf(b.kind));
}

/** what the browser kept, checked: anything broken is left out */
export function readUse(raw: unknown): WorkspaceUse {
	const v = raw as Partial<WorkspaceUse> | null;
	if (!v || typeof v !== 'object') return NO_USE;
	const uses: WorkspaceUse['uses'] = {};
	const last: WorkspaceUse['last'] = {};
	for (const kind of ROW) {
		const n = (v.uses as Record<string, unknown> | undefined)?.[kind];
		if (typeof n === 'number' && Number.isFinite(n) && n > 0) uses[kind] = Math.floor(n);
		const l = (v.last as Record<string, unknown> | undefined)?.[kind] as { address?: unknown; href?: unknown } | undefined;
		if (l && typeof l.address === 'string' && typeof l.href === 'string' && l.href.startsWith('/') && workspaceOf(l.address) === kind) last[kind] = { address: l.address, href: l.href };
	}
	return { uses, last };
}
