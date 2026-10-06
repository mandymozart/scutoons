import type { GroupEnd } from './content.ts';
import { JumpToTargetType } from './enums.ts';
import { actionsOf } from './vn-script.ts';

/**
 * Linked groups ("chains", owner 2026-10-06; agents/DATA.md "Linked groups"): a group with `continues: true` flows
 * straight into the next group on the same chapter page. A chain is a maximal run of consecutive groups where every one
 * but the last continues. Any group may have a VN script or slice events (owner 2026-10-06): the scripts of a chain run
 * one after another on the page (viewer script-queue.ts); a script that jumps away is allowed too – the group after it
 * then needs a label (else nothing leads back to it).
 */

/** What the chain helpers read of a group (the content `Group`, or anything shaped like it) */
export interface ChainGroupLike {
	continues?: unknown;
	events?: unknown;
	slices?: { events?: unknown }[] | null;
}

/** The group runs a VN script (its `events`) or has slice events – the reader mounts the VN box for it */
export function hasVisualNovel(group: ChainGroupLike | null | undefined): boolean {
	if (!group) return false;
	if (group.events) return true;
	return Array.isArray(group.slices) && group.slices.some((slice) => Array.isArray(slice?.events) && slice.events.length > 0);
}

/** The group has a script to run (`events` with at least one event) */
export function hasScript(group: ChainGroupLike | null | undefined): boolean {
	return Array.isArray(group?.events) && group.events.length > 0;
}

/** The group's script can navigate away (a `jumpTo` event, or a choice whose `onSelect` jumps) */
export function scriptJumps(group: ChainGroupLike | null | undefined): boolean {
	if (!hasScript(group)) return false;
	return (group!.events as any[]).some(
		(e) => e?.type === 'jumpTo' || (e?.type === 'addChoice' && actionsOf(e.params?.onSelect).some((a: any) => a && 'jumpTo' in a))
	);
}

/** The group at `index` flows into the next one (it continues and a next group exists) */
export function linksToNext(groups: readonly (ChainGroupLike | null | undefined)[], index: number): boolean {
	const group = groups[index];
	return !!group && group.continues === true && index + 1 < groups.length;
}

/** The chain containing group `index`: the first and the last group's index (inclusive); a lone group is its own chain */
export function chainOf(groups: readonly (ChainGroupLike | null | undefined)[], index: number): { start: number; end: number } {
	if (index < 0 || index >= groups.length) return { start: index, end: index };
	let start = index;
	while (start > 0 && linksToNext(groups, start - 1)) start--;
	let end = index;
	while (linksToNext(groups, end)) end++;
	return { start, end };
}

/** The chains of a chapter in order (`chainOf` of every group, each once) */
export function chainsOf(groups: readonly (ChainGroupLike | null | undefined)[]): { start: number; end: number }[] {
	const out: { start: number; end: number }[] = [];
	for (let i = 0; i < groups.length; ) {
		const chain = chainOf(groups, i);
		out.push(chain);
		i = chain.end + 1;
	}
	return out;
}

/**
 * The slice numbering of a chain (one page): each group's first slice index on the page (`offset`) and the chain's
 * slice count. Slice indices on the page (`data-slice-index`) run on across the chain's groups.
 */
export function chainSlices(
	groups: readonly ({ slices?: unknown[] | null } | null | undefined)[],
	chain: { start: number; end: number }
): { offsets: number[]; count: number } {
	const offsets: number[] = [];
	let count = 0;
	for (let i = chain.start; i <= chain.end; i++) {
		offsets.push(count);
		count += groups[i]?.slices?.length ?? 0;
	}
	return { offsets, count };
}

/**
 * The group's authored end (`Group.end`) when it is usable: a known target type with a value. Only a chain's last group
 * ends anything (`nextStep` asks it), so an `end` on a group that flows on is never read.
 */
export function groupEnd(group: { end?: unknown } | null | undefined): GroupEnd | null {
	if (!group) return null;
	const end = group.end as Partial<GroupEnd> | null | undefined;
	if (!end || typeof end !== 'object' || !end.target || typeof end.target !== 'object') return null;
	const { type, value } = end.target;
	if (!(Object.values(JumpToTargetType) as string[]).includes(type as string)) return null;
	if (value === undefined || value === null || String(value) === '') return null;
	return { target: end.target, auto: end.auto === true };
}
