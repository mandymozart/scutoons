// The bar's workspace row (research/redesign/lap/workspace-thoughts.md points 30, 48): one flat row, most used first,
// never grouped by craft. An open item remembers its workspace; a workspace that does not fit the front item is dimmed
// and opens its own last item. Kept in this browser. Pure: test/shell.test.ts.
import type { Tab } from './tabs.ts';
import { AREAS, isGone, workspaceOf, type WorkspaceKind } from './address.ts';

export interface WorkspaceUse {
	/** how often each workspace came to the front */
	uses: Partial<Record<WorkspaceKind, number>>;
	/** the item each workspace showed last */
	last: Partial<Record<WorkspaceKind, { address: string; href: string }>>;
}

export const NO_USE: WorkspaceUse = { uses: {}, last: {} };

/** the workspaces the row offers, in their order before any use (Help lives in the menu) */
export const ROW: readonly WorkspaceKind[] = ['toon', 'chapter', 'storyboard', 'character', 'scene', 'catalog', 'map', 'experience', 'flow', 'sound', 'text', 'generate'];

/**
 * The workspaces the member's crafts start with (the server resolves them: GET /api/me/crafts `workspaces`), those this
 * creator knows, in their order. Empty: no crafts – every workspace.
 */
export const startKinds = (ids: readonly string[] | null | undefined): WorkspaceKind[] => (ids ?? []).filter((id, i, all): id is WorkspaceKind => ROW.includes(id as WorkspaceKind) && all.indexOf(id) === i);

/** the row's workspaces before any use: the crafts' first, in their order; without crafts the whole row */
const baseRow = (starts: readonly WorkspaceKind[]): readonly WorkspaceKind[] => (starts.length ? [...starts, ...ROW.filter((k) => !starts.includes(k))] : ROW);

/** where a workspace starts when it has shown nothing yet: an area's own list, else its empty page (Search opens an item of it) */
const STARTS: Partial<Record<WorkspaceKind, string>> = { scene: '/scenes', catalog: '/catalog/items', map: '/map', flow: '/flow', generate: '/generate', experience: '/experiences' };

/** a workspace's empty page: what the stage shows in a workspace that needs an item first */
export const EMPTY_PATH = '/workspace/';

export const startHref = (kind: WorkspaceKind): string => STARTS[kind] ?? `${EMPTY_PATH}${kind}`;

/** what Search starts with on a workspace's empty page: the address of the items it opens (a storyboard is a chapter's) */
const FINDS: Partial<Record<WorkspaceKind, string>> = { toon: 'toon:', chapter: 'chapter:', storyboard: 'chapter:', character: 'character:', sound: 'rig:', text: 'kanon:' };
export const findSeed = (kind: WorkspaceKind): string => FINDS[kind] ?? '';

/** the workspace whose empty page `pathname` is, else null */
export function emptyWorkspace(pathname: string): WorkspaceKind | null {
	if (!pathname.startsWith(EMPTY_PATH)) return null;
	const kind = decodeURIComponent(pathname.slice(EMPTY_PATH.length)).replace(/\/$/, '') as WorkspaceKind;
	return ROW.includes(kind) && !STARTS[kind] ? kind : null;
}

/** an address that only passes through (the old characters list goes Home): no workspace remembers it */
const passing = (address: string) => address === AREAS.characters || isGone(address);

/** the front item fits a workspace: it can be shown there (today one workspace per kind) */
export const fits = (address: string | null, kind: WorkspaceKind): boolean => !!address && !passing(address) && workspaceOf(address) === kind;

// ── an item and its workspace (point 48) ──
/** a workspace: a creator workspace (its kind) or an own one made from a kind */
export interface WorkspaceRef {
	id: string;
	kind: WorkspaceKind;
}

/** the workspace a kind shows: the own one chosen for it while it exists, else the kind's own */
const shownFor = (kind: WorkspaceKind, chosen: Readonly<Record<string, string>>, own: readonly WorkspaceRef[]): string => {
	const id = chosen[kind];
	return id && own.some((o) => o.id === id && o.kind === kind) ? id : kind;
};

/** a workspace id an item can still be shown in: its kind's own, or an own workspace of that kind that still exists */
const stillFits = (address: string, id: string | undefined, own: readonly WorkspaceRef[]): id is string => {
	if (!id) return false;
	const kind = workspaceOf(address);
	return !!kind && (id === kind || own.some((o) => o.id === id && o.kind === kind));
};

/**
 * The workspace an item comes to the front in. An open item remembers its workspace (`remembered`): clicking its tab
 * brings it back there while that workspace exists and fits. A new item opens in the front workspace if it fits, else
 * in the one its kind shows (the last picked of its kind). No workspace for the address: null.
 */
export function workspaceFor(address: string, remembered: string | undefined, front: WorkspaceRef | null, chosen: Readonly<Record<string, string>>, own: readonly WorkspaceRef[]): string | null {
	const kind = workspaceOf(address);
	if (!kind || passing(address)) return null;
	if (stillFits(address, remembered, own)) return remembered;
	if (front && front.kind === kind && stillFits(address, front.id, own)) return front.id;
	return shownFor(kind, chosen, own);
}

/**
 * A workspace tab clicked (`target`), the front item at `front` (its page `frontHref`): a workspace the item fits keeps
 * it (the item stays and remembers that workspace); one it doesn't fit (dimmed) opens its own last item (`entryHref`).
 */
export function workspaceClick(front: string | null, frontHref: string | null, target: WorkspaceRef, entryHref: string): { href: string; keeps: boolean } {
	if (front && frontHref && fits(front, target.kind)) return { href: frontHref, keeps: true };
	return { href: entryHref, keeps: false };
}

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
	/** where clicking it goes: its last item (as its open tab is now), else an open item of it, else its start or empty page */
	href: string;
}

/** every workspace with where it goes and whether an item of it is open, most used first (ties: the crafts' order, then the row's) */
function ranked(use: WorkspaceUse, tabs: readonly Tab[], starts: readonly WorkspaceKind[]): (RowEntry & { open: boolean })[] {
	const order = baseRow(starts);
	const entries = order.map((kind) => {
		const last = use.last[kind];
		const open = (last && tabs.find((t) => t.address === last.address)) ?? tabs.find((t) => !passing(t.address) && workspaceOf(t.address) === kind);
		return { kind, href: open?.href ?? last?.href ?? startHref(kind), open: !!open };
	});
	const rank = (k: WorkspaceKind) => use.uses[k] ?? 0;
	return entries.sort((a, b) => rank(b.kind) - rank(a.kind) || order.indexOf(a.kind) - order.indexOf(b.kind));
}

/** every workspace – the switcher's and Search's list: crafts (`starts`) order it, never leave one out */
export function workspaceList(use: WorkspaceUse, tabs: readonly Tab[], starts: readonly WorkspaceKind[] = []): RowEntry[] {
	return ranked(use, tabs, starts).map(({ kind, href }) => ({ kind, href }));
}

/**
 * The bar's row, flat and never grouped by craft: with crafts (`starts`) theirs and those used or open, else every
 * workspace; what it leaves out is in the switcher (its ⌄).
 */
export function workspaceRow(use: WorkspaceUse, tabs: readonly Tab[], starts: readonly WorkspaceKind[] = []): RowEntry[] {
	return ranked(use, tabs, starts)
		.filter((e) => !starts.length || starts.includes(e.kind) || e.open || !!use.uses[e.kind])
		.map(({ kind, href }) => ({ kind, href }));
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
		if (l && typeof l.address === 'string' && typeof l.href === 'string' && l.href.startsWith('/') && !passing(l.address) && workspaceOf(l.address) === kind) last[kind] = { address: l.address, href: l.href };
	}
	return { uses, last };
}
