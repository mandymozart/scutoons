// Places where a reader could get stuck (owner 2026-10-06: "Events queue takeovers have to be safe for accidental dead
// ends" – avoided by LOGIC and VALIDATION, no time-based patches). The viewer's runtime rules make the rest impossible
// (viewer visual-novel/box-state.ts: every waiting state has an exit the reader can take); what an author can still
// build wrong is found here, statically, and refused on save (validate.ts errors, the server's save guard the same) and
// marked in the Flow graph ("may get stuck").
//
// - a question whose choices can ALL be hidden (every addChoice before its showChoices has an `if`): one choice must be
//   unconditional – the reader always has a way on (the runtime skips an empty question, but the story then goes on
//   without the reader having chosen – the author meant a question)
// - a divert to a point that isn't in the group's script, a jump to a group label / chapter that doesn't exist (this
//   chapter's labels here; other chapters / toons where the caller knows them – validate.ts `checkTargetPlace`)
import type { Path } from './paths.ts';

/** an event that runs only if its test holds (reach.ts `isConditional` – kept here, reach.ts imports this file) */
const isConditional = (e: unknown): boolean => {
	const c = (e as { if?: { variable?: unknown; expr?: unknown } } | null | undefined)?.if;
	return (typeof c?.variable === 'string' && !!c.variable) || (typeof c?.expr === 'string' && !!c.expr.trim());
};

export type StuckCode = 'all-choices-hidden';

export interface StuckPoint {
	/** where (groups.g.events.i / groups.g.slices.s.events.i / … items.k.events.i) – the showChoices */
	path: Path;
	code: StuckCode;
	/** the conditional choices' paths */
	choices: Path[];
}

const isObj = (x: unknown): x is Record<string, any> => !!x && typeof x === 'object' && !Array.isArray(x);

/**
 * The questions of one event list (a group's script, a slice trigger's events, an item's taps) in script order: the
 * choices added since the last question (or the start, or an unconditional divert / jumpTo – the script left) and the
 * showChoices that asks. A choice added with an `if` may be hidden; when every one of them may be, the question may
 * have no choice at all.
 */
export function questionsOf(events: readonly unknown[], base: Path): { at: Path; choices: { path: Path; conditional: boolean }[] }[] {
	const out: { at: Path; choices: { path: Path; conditional: boolean }[] }[] = [];
	let choices: { path: Path; conditional: boolean }[] = [];
	events.forEach((e, i) => {
		if (!isObj(e)) return;
		const at = [...base, i];
		if ((e.type === 'divert' || e.type === 'jumpTo') && !isConditional(e)) choices = [];
		else if (e.type === 'addChoice') choices.push({ path: at, conditional: isConditional(e) });
		else if (e.type === 'showChoices') {
			out.push({ at, choices });
			choices = [];
		}
	});
	return out;
}

/** the event lists of a group: its script, each slice's events (a trigger's run), each placed item's tap events */
export function eventListsOf(group: unknown, base: Path): { events: unknown[]; path: Path }[] {
	if (!isObj(group)) return [];
	const lists: { events: unknown[]; path: Path }[] = [];
	if (Array.isArray(group.events)) lists.push({ events: group.events, path: [...base, 'events'] });
	(Array.isArray(group.slices) ? group.slices : []).forEach((s: unknown, si: number) => {
		if (!isObj(s)) return;
		if (Array.isArray(s.events)) lists.push({ events: s.events, path: [...base, 'slices', si, 'events'] });
		(Array.isArray(s.items) ? s.items : []).forEach((item: unknown, k: number) => {
			if (isObj(item) && Array.isArray(item.events)) lists.push({ events: item.events, path: [...base, 'slices', si, 'items', k, 'events'] });
		});
	});
	return lists;
}

/** every place in these groups (a chapter's, scenes in place of their refs) where the reader could get stuck */
export function stuckPoints(groups: readonly unknown[]): StuckPoint[] {
	const out: StuckPoint[] = [];
	groups.forEach((g, gi) => {
		for (const list of eventListsOf(g, ['groups', gi])) {
			for (const q of questionsOf(list.events, list.path)) {
				if (q.choices.length && q.choices.every((c) => c.conditional)) out.push({ path: q.at, code: 'all-choices-hidden', choices: q.choices.map((c) => c.path) });
			}
		}
	});
	return out;
}

/** the message of a stuck point (validation, the Flow graph's tooltip) */
export function stuckMessage(p: StuckPoint): string {
	switch (p.code) {
		case 'all-choices-hidden':
			return `every choice of this question has a condition – all of them can be hidden at once, so the reader may get no choice: give one choice no condition (e.g. "Go on")`;
	}
}
