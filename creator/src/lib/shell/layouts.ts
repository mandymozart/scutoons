// Saved workspace layouts (research/redesign/lap/workspace-thoughts.md points 9–15; crafts-v2.html view `save`): what a
// workspace starts from on a new device and what Reset goes back to – the member's last save, else their craft's default,
// else the code preset – and the own workspaces made from another one. The layout is the floats' own JSON; the server
// keeps it (server docs/API.md "Workspace layouts") and checks only its outer shape, the floats' readers check the inside.
// A missing or failing API (code goes live before its migration) = this browser's copy only. Pure; tests: test/shell.test.ts.
import type { MemberLayouts } from '@scutoons/api';
import { ROW } from './workspaces.ts';
import type { WorkspaceKind } from './address.ts';

/** a workspace made from another one: its own id, the workspace it shows (`kind`) and its name */
export interface OwnWorkspace {
	id: string;
	kind: WorkspaceKind;
	name: string;
}

export interface LayoutsState {
	/** each workspace's saved layout (a creator workspace's id or an own one's) */
	saved: Record<string, unknown>;
	own: OwnWorkspace[];
	/** each workspace's craft default: the craft and its layout */
	defaults: Record<string, { craft: string; layout: unknown }>;
}

export const NO_LAYOUTS: LayoutsState = { saved: {}, own: [], defaults: {} };

const OWN_ID = /^own-[a-z0-9]{1,40}$/;
const MAX_NAME = 80;

const isKind = (v: unknown): v is WorkspaceKind => ROW.includes(v as WorkspaceKind);
const isObject = (v: unknown): v is Record<string, unknown> => typeof v === 'object' && v !== null && !Array.isArray(v);

/** a name as the server keeps it: spaces folded, trimmed, at most 80 characters; '' = none */
export const cleanName = (name: string): string => name.replace(/\s+/g, ' ').trim().slice(0, MAX_NAME);

/** a new own workspace's id (`own-` and random letters; `taken` are not used again) */
export function ownId(taken: readonly string[], random: () => number = Math.random): string {
	for (;;) {
		const id = `own-${Math.floor(random() * 36 ** 6)
			.toString(36)
			.padStart(6, '0')}`;
		if (!taken.includes(id)) return id;
	}
}

/** the server's answer, checked; null when it has none (no layouts before its migration, or malformed) */
export function fromAnswer(raw: unknown): LayoutsState | null {
	const a = raw as Partial<MemberLayouts> | null;
	if (!isObject(a) || a.available !== true || !Array.isArray(a.layouts)) return null;
	const out: LayoutsState = { saved: {}, own: [], defaults: {} };
	for (const row of a.layouts) {
		if (!isObject(row) || typeof row.workspace !== 'string' || !isObject(row.layout)) continue;
		if (row.name !== null && row.name !== undefined) {
			if (!OWN_ID.test(row.workspace) || typeof row.name !== 'string' || !isKind(row.kind)) continue;
			out.own.push({ id: row.workspace, kind: row.kind, name: row.name });
		}
		out.saved[row.workspace] = row.layout;
	}
	for (const d of Array.isArray(a.defaults) ? a.defaults : []) {
		if (isObject(d) && typeof d.workspace === 'string' && typeof d.craft === 'string' && isObject(d.layout)) out.defaults[d.workspace] ??= { craft: d.craft, layout: d.layout };
	}
	return out;
}

/** the server's layouts, or null when the API is missing, fails or has none yet – never throws */
export async function readLayouts(mine: () => Promise<unknown>): Promise<LayoutsState | null> {
	try {
		return fromAnswer(await mine());
	} catch {
		return null;
	}
}

/** what this browser kept (the copy for a missing API), checked: anything broken is left out */
export function readKept(raw: unknown): LayoutsState {
	if (!isObject(raw)) return NO_LAYOUTS;
	const saved: Record<string, unknown> = {};
	for (const [id, layout] of Object.entries(isObject(raw.saved) ? raw.saved : {})) if (isObject(layout)) saved[id] = layout;
	const own = (Array.isArray(raw.own) ? raw.own : []).filter(
		(o): o is OwnWorkspace => isObject(o) && typeof o.id === 'string' && OWN_ID.test(o.id) && isKind(o.kind) && typeof o.name === 'string' && !!cleanName(o.name) && !!saved[o.id]
	);
	return { saved, own: own.map(({ id, kind, name }) => ({ id, kind, name: cleanName(name) })), defaults: {} };
}

/** what this browser keeps of it (the craft defaults come from the server each time) */
export const toKept = (state: LayoutsState): { saved: Record<string, unknown>; own: OwnWorkspace[] } => ({ saved: state.saved, own: state.own });

/** the layout a workspace starts from and Reset goes back to: its last save, else its craft's default, else null (the code preset). An own workspace never saved falls back to the one it was made from */
export function startOf(state: LayoutsState, id: string): unknown {
	if (state.saved[id]) return state.saved[id];
	const own = state.own.find((o) => o.id === id);
	if (own) return startOf(state, own.kind);
	return state.defaults[id]?.layout ?? null;
}

/** what Reset goes back to, in words: the last save, the craft default (which craft), or the preset */
export function startKind(state: LayoutsState, id: string): { kind: 'saved' } | { kind: 'craft'; craft: string } | { kind: 'preset' } {
	if (state.saved[id]) return { kind: 'saved' };
	const own = state.own.find((o) => o.id === id);
	if (own) return startKind(state, own.kind);
	const d = state.defaults[id];
	return d ? { kind: 'craft', craft: d.craft } : { kind: 'preset' };
}

/** a workspace saved (an own one with its name and the workspace it is made from: new, or renamed) */
export function withSaved(state: LayoutsState, id: string, layout: unknown, own?: { name: string; kind: WorkspaceKind }): LayoutsState {
	const saved = { ...state.saved, [id]: layout };
	if (!own) return { ...state, saved };
	const entry: OwnWorkspace = { id, kind: own.kind, name: cleanName(own.name) };
	const at = state.own.findIndex((o) => o.id === id);
	return { ...state, saved, own: at < 0 ? [...state.own, entry] : state.own.map((o, i) => (i === at ? entry : o)) };
}

/** a save taken back: a creator workspace back to its default, an own workspace gone */
export function without(state: LayoutsState, id: string): LayoutsState {
	const { [id]: _gone, ...saved } = state.saved;
	return { ...state, saved, own: state.own.filter((o) => o.id !== id) };
}

/** the workspace a kind shows: the own one chosen for it while it exists, else the creator's own */
export const shownWorkspace = (chosen: Readonly<Record<string, string>>, own: readonly OwnWorkspace[], kind: string): string => {
	const id = chosen[kind];
	return id && own.some((o) => o.id === id && o.kind === kind) ? id : kind;
};

/** the choices kept in this browser, checked */
export function readChosen(raw: unknown): Record<string, string> {
	if (!isObject(raw)) return {};
	return Object.fromEntries(Object.entries(raw).filter((e): e is [string, string] => isKind(e[0]) && typeof e[1] === 'string'));
}

/**
 * Own workspaces beside the one they are made from, in any list of a kind's entries (the bar's row, the dashboard's
 * cards): each entry's id is its workspace id, an own workspace's entry carries its name
 */
export function withOwn<T extends { kind: WorkspaceKind }>(entries: readonly T[], own: readonly OwnWorkspace[]): Array<T & { id: string; name?: string }> {
	return entries.flatMap((e) => [{ ...e, id: e.kind as string }, ...own.filter((o) => o.kind === e.kind).map((o) => ({ ...e, id: o.id, name: o.name }))]);
}
