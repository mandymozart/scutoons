import type { GroupEnd } from './content.ts';
import { JumpToTargetType, Trigger } from './enums.ts';
import { labelIndex, scriptFlow } from './vn-script.ts';

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
	return hasScript(group) && scriptFlow(group!.events as unknown[]).jumps;
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

/** what of a chapter's groups the reader can never reach (`unreachable`) */
export interface Unreachable {
	/** group indices: they follow (on the page) a group that always leaves, and nothing leads to them (no label) */
	groups: number[];
	/** slices after a slice whose scroll trigger always leaves: group index → slice indices */
	slices: Map<number, number[]>;
}

/** the label a group can be reached by (`?label=`, a jump) */
const labelled = (g: unknown) => typeof (g as { label?: unknown } | null)?.label === 'string' && !!(g as { label: string }).label;

/**
 * The slice index from which the group's own slice events always leave the page: a scroll trigger (`intersect`, it fires
 * when the slice is seen) whose run – its events, or one divert into the group's script – always jumps. -1: none.
 */
function sliceExit(group: { events?: unknown; slices?: { events?: unknown }[] | null }): number {
	const slices = Array.isArray(group.slices) ? group.slices : [];
	for (let si = 0; si < slices.length; si++) {
		const events = Array.isArray(slices[si]?.events) ? (slices[si].events as any[]) : [];
		const runs = new Map<string, any[]>();
		for (const e of events) {
			if (e?.trigger?.type !== Trigger.INTERSECT) continue;
			const key = JSON.stringify(e.trigger);
			runs.set(key, [...(runs.get(key) ?? []), e]);
		}
		for (const run of runs.values()) {
			const script = Array.isArray(group.events) ? (group.events as any[]) : [];
			const divert = run.length === 1 && run[0]?.type === 'divert' && !run[0]?.if?.variable ? labelIndex(script, run[0].params?.to) : -1;
			const flow = divert >= 0 ? scriptFlow(script, divert) : scriptFlow(run);
			if (!flow.ends && flow.jumps) return si;
		}
	}
	return -1;
}

/**
 * Content the reader can never reach (owner 2026-10-06, the creator mutes it, validation warns): after a group whose
 * script always leaves the page (every run ends in a jumpTo) – or a slice whose scroll trigger always leaves – the rest of
 * that group and the following groups of its chain up to the next one with a label (a jump may lead there). A
 * conditional jump (`if`) or a choice that may stay never makes anything unreachable.
 */
export function unreachable(groups: readonly (ChainGroupLike | null | undefined)[]): Unreachable {
	const out: Unreachable = { groups: [], slices: new Map() };
	const cut = (from: number) => {
		for (let j = from + 1; j < groups.length && linksToNext(groups, j - 1) && !labelled(groups[j]); j++) {
			if (!out.groups.includes(j)) out.groups.push(j);
		}
	};
	groups.forEach((g, gi) => {
		if (!g || out.groups.includes(gi)) return;
		const exit = sliceExit(g);
		const count = Array.isArray(g.slices) ? g.slices.length : 0;
		if (exit >= 0) {
			if (exit + 1 < count) out.slices.set(gi, Array.from({ length: count - exit - 1 }, (_, k) => exit + 1 + k));
			cut(gi);
			return;
		}
		if (hasScript(g)) {
			const flow = scriptFlow(g.events as unknown[]);
			if (!flow.ends && flow.jumps) cut(gi);
		}
	});
	out.groups.sort((a, b) => a - b);
	return out;
}

/**
 * The events of a group's script no run reaches (owner 2026-10-06): not from the start, nor from a named point a slice
 * event diverts into – e.g. after an unconditional divert or jumpTo with no label leading back.
 */
export function unreachableEvents(group: ChainGroupLike | null | undefined): number[] {
	if (!hasScript(group)) return [];
	const script = group!.events as any[];
	const reach = new Set(scriptFlow(script).reachable);
	const slices = Array.isArray(group!.slices) ? group!.slices : [];
	for (const slice of slices) {
		for (const e of Array.isArray(slice?.events) ? (slice.events as any[]) : []) {
			// a slice event's divert (and a slice choice's divert) start the script at that point
			const tos = [e?.type === 'divert' ? e.params?.to : undefined, ...(e?.type === 'addChoice' ? [].concat(e.params?.onSelect ?? []).map((a: any) => a?.divert?.to) : [])];
			for (const to of tos) {
				const at = typeof to === 'string' && to ? labelIndex(script, to) : -1;
				if (at >= 0) for (const i of scriptFlow(script, at).reachable) reach.add(i);
			}
		}
	}
	return script.map((_, i) => i).filter((i) => !reach.has(i));
}
