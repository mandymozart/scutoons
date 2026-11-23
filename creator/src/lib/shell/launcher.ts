// The launcher's pure parts (research/redesign/lap/workspace-thoughts.md points 24, 25, 37, 39; crafts-v2.html view
// `launch`): one ranked list of items, workspaces, views, assets and commands for what was typed, the most recent picks
// first while nothing is typed, and the keys that open it. Tests: test/shell.test.ts.

import { isGone } from './address.ts';
export type LaunchKind = 'item' | 'workspace' | 'view' | 'asset' | 'command';

/** the order kinds come in when they match equally well */
export const KIND_ORDER: readonly LaunchKind[] = ['item', 'workspace', 'view', 'command', 'asset'];

export interface LaunchEntry {
	/** unique across kinds: `item:<address>`, `workspace:<kind>`, `view:<tab>`, `asset:<key>`, `command:<id>` */
	id: string;
	kind: LaunchKind;
	label: string;
	/** muted after the name: the toon, the workspace, the kind's word */
	hint?: string;
	/** at the line's end: what it is – an item's kind, a file's kind (else the launcher's word for the entry's kind) */
	type?: string;
	/** an item's `kind:id` address */
	address?: string;
	keywords?: string;
	icon: string;
	/** where an item or workspace goes (a recent pick keeps it, so it opens before the lists are loaded) */
	href?: string;
}

/** what a pick leaves in the recents: enough to show and open it again */
export type LaunchPick = Pick<LaunchEntry, 'id' | 'kind' | 'label' | 'icon'> & Partial<Pick<LaunchEntry, 'hint' | 'address' | 'href'>>;

export const MAX_PICKS = 8;
export const MAX_SHOWN = 14;

/** does every word of the query appear in the text (case-insensitive)? An empty query matches everything */
export function wordsIn(hay: string, query: string): boolean {
	const text = hay.toLowerCase();
	return query
		.toLowerCase()
		.split(/\s+/)
		.filter(Boolean)
		.every((word) => text.includes(word));
}

/** how well an entry matches: 0 none; the address typed exactly, then the name exactly, its start, a word's start, inside it, anywhere */
export function score(entry: LaunchEntry, query: string): number {
	const q = query.trim().toLowerCase();
	if (!q) return 1;
	const label = entry.label.toLowerCase();
	const address = entry.address?.toLowerCase() ?? '';
	if (!wordsIn(`${entry.label} ${entry.hint ?? ''} ${entry.address ?? ''} ${entry.keywords ?? ''}`, q)) return 0;
	if (address && address === q) return 1000;
	if (label === q) return 800;
	if (address && q.includes(':') && address.startsWith(q)) return 700;
	if (label.startsWith(q)) return 600;
	const first = q.split(/\s+/)[0];
	if (label.split(/[\s·/_:-]+/).some((w) => w.startsWith(first))) return 400;
	if (label.includes(q)) return 300;
	return 100;
}

/** a recent pick lifts an entry a little, the newest most */
const lift = (at: number): number => (at < 0 ? 0 : Math.max(0, 50 - at * 5));

/** the entries for a query, best first, each id once (the first given wins); ties: recent, kind, shorter name, name */
export function rank(entries: readonly LaunchEntry[], query: string, recents: readonly string[] = [], limit = MAX_SHOWN): LaunchEntry[] {
	const seen = new Set<string>();
	const scored: { e: LaunchEntry; s: number; r: number }[] = [];
	for (const e of entries) {
		if (seen.has(e.id)) continue;
		seen.add(e.id);
		const s = score(e, query);
		if (s > 0) scored.push({ e, s, r: recents.indexOf(e.id) });
	}
	return scored
		.sort(
			(a, b) =>
				b.s + lift(b.r) - (a.s + lift(a.r)) ||
				(a.r < 0 ? Infinity : a.r) - (b.r < 0 ? Infinity : b.r) ||
				KIND_ORDER.indexOf(a.e.kind) - KIND_ORDER.indexOf(b.e.kind) ||
				a.e.label.length - b.e.label.length ||
				a.e.label.localeCompare(b.e.label)
		)
		.slice(0, limit)
		.map((x) => x.e);
}

/**
 * Nothing typed: the recent picks, newest first – each as the lists know it now, else as it was picked when it can still
 * open by itself (an item or workspace with its href); then `rest` to fill up.
 */
export function recentFirst(entries: readonly LaunchEntry[], picks: readonly LaunchPick[], rest: readonly LaunchEntry[] = [], limit = MAX_SHOWN): LaunchEntry[] {
	const byId = new Map(entries.map((e) => [e.id, e]));
	const out: LaunchEntry[] = [];
	const seen = new Set<string>();
	const add = (e: LaunchEntry | undefined) => {
		if (!e || seen.has(e.id) || out.length >= limit) return;
		seen.add(e.id);
		out.push(e);
	};
	for (const p of picks) add(byId.get(p.id) ?? (p.href ? { ...p } : undefined));
	for (const e of rest) add(e);
	return out;
}

/** the recents after a pick: it first, once, at most `max` */
export function remember(picks: readonly LaunchPick[], entry: LaunchEntry, max = MAX_PICKS): LaunchPick[] {
	const { id, kind, label, icon, hint, address, href } = entry;
	const pick: LaunchPick = { id, kind, label, icon, ...(hint ? { hint } : {}), ...(address ? { address } : {}), ...(href ? { href } : {}) };
	return [pick, ...picks.filter((p) => p.id !== id)].slice(0, max);
}

/** what the browser kept, checked: anything broken is left out */
export function readPicks(raw: unknown): LaunchPick[] {
	if (!Array.isArray(raw)) return [];
	const out: LaunchPick[] = [];
	for (const v of raw) {
		const p = v as Partial<LaunchPick> | null;
		if (!p || typeof p.id !== 'string' || typeof p.label !== 'string' || typeof p.icon !== 'string' || !KIND_ORDER.includes(p.kind as LaunchKind)) continue;
		if (p.href !== undefined && (typeof p.href !== 'string' || !p.href.startsWith('/'))) continue;
		if (typeof p.address === 'string' && isGone(p.address)) continue;
		out.push({ id: p.id, kind: p.kind as LaunchKind, label: p.label, icon: p.icon, ...(typeof p.hint === 'string' ? { hint: p.hint } : {}), ...(typeof p.address === 'string' ? { address: p.address } : {}), ...(p.href ? { href: p.href } : {}) });
		if (out.length >= MAX_PICKS) break;
	}
	return out;
}

export interface KeyLike {
	key: string;
	code: string;
	altKey: boolean;
	ctrlKey: boolean;
	metaKey: boolean;
	shiftKey: boolean;
}

/**
 * The keys that open (and close) Search: Alt Space, Cmd Space and Ctrl O / Cmd O anywhere, `/` outside a text field.
 * Ctrl Space is not one of them (a view full screen).
 */
export function isSearchKey(e: KeyLike, typing: boolean): boolean {
	if (e.shiftKey) return false;
	const space = e.code === 'Space' || e.key === ' ';
	if (space) return (e.altKey && !e.ctrlKey && !e.metaKey) || (e.metaKey && !e.ctrlKey && !e.altKey);
	const mod = (e.ctrlKey || e.metaKey) && !e.altKey;
	if (mod && e.key.toLowerCase() === 'o') return true;
	return e.key === '/' && !typing && !e.ctrlKey && !e.metaKey && !e.altKey;
}

/**
 * The keys that open (and close) the switcher: Ctrl K / Cmd K anywhere, and Alt Tab where the system leaves it to the
 * page (a browser on Windows never gets it: the system's own switcher has it).
 */
export function isSwitcherKey(e: KeyLike): boolean {
	if (e.shiftKey) return false;
	if ((e.ctrlKey || e.metaKey) && !e.altKey && e.key.toLowerCase() === 'k') return true;
	return e.altKey && !e.ctrlKey && !e.metaKey && e.key === 'Tab';
}
