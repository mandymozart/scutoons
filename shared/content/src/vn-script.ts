// The VN script's logic as pure functions (owner 2026-10-03, CUTOVER 10.27): one runner in the viewer uses them, the
// creator's forms and the validator read the same rules. A group's `events` is its script; `label` events are named points
// in it, a divert continues there (no page change); jumpTo is navigation only (another group / chapter, or a random one).
import type { VNAction, VNCondition, VNEvent } from './visual-novel.ts';

type Value = string | number | undefined;

/** does the reader's state pass the test? (numbers compare as numbers, `set` / `unset` = has a value at all) */
export function testCondition(condition: VNCondition | null | undefined, get: (variable: string) => Value): boolean {
	if (!condition || !condition.variable) return true;
	const have = get(condition.variable);
	const has = have !== undefined && have !== null && have !== '';
	if (condition.op === 'set') return has;
	if (condition.op === 'unset') return !has;
	const a = Number(have);
	const b = Number(condition.value);
	const numeric = has && Number.isFinite(a) && Number.isFinite(b) && condition.value !== '' && condition.value !== undefined;
	switch (condition.op) {
		case '==':
			return numeric ? a === b : String(have ?? '') === String(condition.value ?? '');
		case '!=':
			return numeric ? a !== b : String(have ?? '') !== String(condition.value ?? '');
		case '<':
			return numeric && a < b;
		case '<=':
			return numeric && a <= b;
		case '>':
			return numeric && a > b;
		case '>=':
			return numeric && a >= b;
		default:
			return true;
	}
}

/** a choice's actions as a list (one action or several) */
export function actionsOf(onSelect: VNAction | VNAction[] | null | undefined): VNAction[] {
	if (!onSelect) return [];
	return Array.isArray(onSelect) ? onSelect.filter(Boolean) : [onSelect];
}

/** the index of the named point `name` in a script (-1 when there is none) */
export function labelIndex(events: readonly VNEvent[] | null | undefined, name: string): number {
	return (events ?? []).findIndex((e) => e?.type === 'label' && e.params?.name === name);
}

/** every named point of a script, in order */
export function scriptLabels(events: readonly unknown[] | null | undefined): string[] {
	return (events ?? []).flatMap((e: any) => (e?.type === 'label' && typeof e.params?.name === 'string' && e.params.name ? [e.params.name] : []));
}

/** a random group label from `anyOf`, never `current` (unless it is the only one); null when the list is empty */
export function pickJumpLabel(anyOf: readonly string[] | null | undefined, current: string | null | undefined, random: () => number = Math.random): string | null {
	const all = (anyOf ?? []).filter((l) => typeof l === 'string' && l);
	if (!all.length) return null;
	const others = all.filter((l) => l !== current);
	const from = others.length ? others : all;
	return from[Math.min(from.length - 1, Math.floor(random() * from.length))];
}

/** minutes since midnight as a clock: 330 → "5:30", 1445 → "0:05" (texts: `{clock(t)}`, the expression registry) */
export function formatClock(minutes: number): string {
	const m = ((Math.round(minutes) % 1440) + 1440) % 1440;
	return `${Math.floor(m / 60)}:${String(m % 60).padStart(2, '0')}`;
}
