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

/** minutes since midnight as a clock: 330 → "5:30", 1445 → "0:05" */
export function formatClock(minutes: number): string {
	const m = ((Math.round(minutes) % 1440) + 1440) % 1440;
	return `${Math.floor(m / 60)}:${String(m % 60).padStart(2, '0')}`;
}

/** `{variable}` → its value, `{variable:clock}` → minutes as a clock; unknown variables stay as written */
export function interpolateText(text: string, get: (variable: string) => Value): string {
	return String(text ?? '').replace(/{(\w+)(?::(clock))?}/g, (match, name: string, format?: string) => {
		const value = get(name);
		if (value === undefined || value === null) return match;
		if (format === 'clock' && Number.isFinite(Number(value))) return formatClock(Number(value));
		return String(value);
	});
}

/** what can happen from a point of a script (`scriptFlow`) */
export interface ScriptFlow {
	/** the events a run from the start can reach (by index) */
	reachable: Set<number>;
	/** some run reaches the script's end (it is done without leaving the page) */
	ends: boolean;
	/** some run navigates away (a `jumpTo` event or a picked choice's jumpTo) */
	jumps: boolean;
}

/** a test that can fail (an `if` with a variable): the event may be skipped */
const conditional = (e: any) => !!e?.if?.variable;

/**
 * Where a script can go from `start` (the runner's rules, viewer runner.ts): an `if` may skip its event, `divert` goes
 * to its named point (a missing one: on), `jumpTo` leaves the page (the run ends), `showChoices` waits for one of the
 * choices added since the last question – each choice's divert / jumpTo / nothing is a way on. Used for "unreachable"
 * (events no run reaches, groups after a script that always leaves) and "always leaves" (`!ends && jumps`). Pure and
 * bounded (states are visited once; a huge script stops early and counts as ending).
 */
export function scriptFlow(events: readonly unknown[] | null | undefined, start = 0): ScriptFlow {
	const list = (events ?? []) as any[];
	const reachable = new Set<number>();
	const flow: ScriptFlow = { reachable, ends: false, jumps: false };
	if (start < 0) return flow;
	const seen = new Set<string>();
	const todo: { i: number; pending: number[] }[] = [{ i: start, pending: [] }];
	const go = (i: number, pending: number[]) => todo.push({ i, pending });
	const point = (to: unknown) => (typeof to === 'string' && to ? labelIndex(list as VNEvent[], to) : -1);
	while (todo.length) {
		if (seen.size > 20_000) {
			flow.ends = true;
			break;
		}
		const { i, pending } = todo.pop()!;
		const key = `${i}|${pending.join(',')}`;
		if (seen.has(key)) continue;
		seen.add(key);
		if (i >= list.length) {
			flow.ends = true;
			continue;
		}
		reachable.add(i);
		const e = list[i];
		if (conditional(e)) go(i + 1, pending);
		switch (e?.type) {
			case 'divert': {
				const at = point(e.params?.to);
				go(at >= 0 ? at : i + 1, pending);
				break;
			}
			case 'jumpTo':
				flow.jumps = true;
				break;
			case 'addChoice':
				go(i + 1, [...pending, i]);
				break;
			case 'showChoices': {
				if (!pending.length) {
					go(i + 1, []);
					break;
				}
				for (const c of pending) {
					const actions = actionsOf(list[c]?.params?.onSelect);
					if (actions.some((a: any) => a && 'jumpTo' in a)) {
						flow.jumps = true;
						continue;
					}
					const divert = actions.find((a: any) => a && 'divert' in a) as { divert?: { to?: string } } | undefined;
					const at = divert ? point(divert.divert?.to) : -1;
					go(at >= 0 ? at : i + 1, []);
				}
				break;
			}
			default:
				go(i + 1, pending);
		}
	}
	return flow;
}
