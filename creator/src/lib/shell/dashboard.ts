// The dashboard's pure parts (research/redesign/lap/crafts-v2.html views `dash`, `first`): the workspace cards – flat,
// the front one first, then most used – each with what is open in it; the welcome when nothing was used yet; the focus
// ring's step. Tests: test/shell.test.ts.
import type { Tab } from './tabs.ts';
import { workspaceOf, type WorkspaceKind } from './address.ts';
import { ROW, startHref, workspaceList, type WorkspaceUse } from './workspaces.ts';
import { shownWorkspace, withOwn, type OwnWorkspace } from './layouts.ts';

export interface WorkspaceCard {
	kind: WorkspaceKind;
	/** the workspace's id: its kind, or an own workspace's */
	id: string;
	/** an own workspace's name */
	name?: string;
	/** where opening the card goes: its item, its start or its empty page; null: none (an own workspace's kind decides) */
	href: string | null;
	/** the open item it shows (its last one while open), else null */
	shown: Tab | null;
	/** every open item of this workspace */
	open: Tab[];
	/** the front item's workspace (of its kind, the one it shows) */
	front: boolean;
}

/** nothing opened, nothing used yet: the welcome */
export const isWelcome = (use: WorkspaceUse, tabs: readonly Tab[], recent: number): boolean => !tabs.length && !recent && !Object.keys(use.uses).length;

/**
 * The cards: the front item's workspace first, then every other, most used first. The welcome shows every workspace,
 * empty, the member's crafts' (`starts`) first. An own workspace stands beside the one it is made from (`chosen`: the
 * one each kind shows).
 */
export function workspaceCards(
	use: WorkspaceUse,
	tabs: readonly Tab[],
	front: string | null,
	welcome = false,
	starts: readonly WorkspaceKind[] = [],
	own: readonly OwnWorkspace[] = [],
	chosen: Readonly<Record<string, string>> = {}
): WorkspaceCard[] {
	const current = front ? workspaceOf(front) : null;
	const card = (kind: WorkspaceKind, href: string | null): Omit<WorkspaceCard, 'id'> => {
		const open = tabs.filter((t) => workspaceOf(t.address) === kind);
		const last = use.last[kind]?.address;
		const shown = (kind === current ? open.find((t) => t.address === front) : null) ?? open.find((t) => t.address === last) ?? open[0] ?? null;
		return { kind, href: shown?.href ?? href, shown, open, front: kind === current };
	};
	const kinds = welcome ? [...starts, ...ROW.filter((k) => !starts.includes(k))].map((kind) => card(kind, startHref(kind))) : workspaceList(use, tabs, starts).map((e) => card(e.kind, e.href));
	if (!welcome && current && !kinds.some((c) => c.kind === current)) kinds.unshift(card(current, null));
	const cards: WorkspaceCard[] = withOwn(kinds, own).map((c) => ({ ...c, front: c.front && c.id === shownWorkspace(chosen, own, c.kind) }));
	const at = cards.findIndex((c) => c.front);
	if (at > 0) cards.unshift(...cards.splice(at, 1));
	return cards;
}

/** where the ring starts: on the card after the front one (app-switcher style), else the first */
export const firstRing = (cards: readonly WorkspaceCard[]): number => (cards.length > 1 && cards[0].front ? 1 : 0);

/** the ring `by` cards along, round the ends */
export const stepRing = (at: number, by: number, count: number): number => (count ? (((at + by) % count) + count) % count : 0);

/** the ring a row up or down in a grid of `columns` (stays when there is no card there) */
export function rowStep(at: number, by: 1 | -1, count: number, columns: number): number {
	const next = at + by * Math.max(1, columns);
	return next >= 0 && next < count ? next : at;
}

/** how long ago, in the dashboard's few words: now · 5 min · 3 h · 2 d (null: unknown) */
export function ago(at: number | undefined, now: number): { unit: 'now' | 'minute' | 'hour' | 'day'; n: number } | null {
	if (at === undefined || !Number.isFinite(at)) return null;
	const minutes = Math.max(0, Math.floor((now - at) / 60000));
	if (minutes < 1) return { unit: 'now', n: 0 };
	if (minutes < 60) return { unit: 'minute', n: minutes };
	const hours = Math.floor(minutes / 60);
	return hours < 24 ? { unit: 'hour', n: hours } : { unit: 'day', n: Math.floor(hours / 24) };
}

/** the name a welcome greets: the address' first part, capitalised (an account has no name of its own) */
export function greetName(email: string): string {
	const first = email.split('@')[0]?.split(/[._+-]/)[0] ?? '';
	return first ? first[0].toUpperCase() + first.slice(1) : '';
}
