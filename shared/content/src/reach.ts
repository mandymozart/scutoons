// Reachability of a chapter's content (owner 2026-10-06, agents/DATA.md "Linked groups"): a graph over the chapter –
// groups, slices, script events, slice-event runs, their named points, diverts, jumps, choices and conditions – walked
// with visited sets (iterative, O(nodes + edges), never runs a script, cycles are harmless). Every item is reachable,
// reachable only through a condition, or unreachable; conditions that can never be true and divert loops with no way
// out are found on the way. The validator warns, the creator labels the items ("unreachable" chips).
import { groupEnd, linksToNext, type ChainGroupLike } from './chains.ts';
import { Trigger } from './enums.ts';
import { repeatOf, type Repeat } from './recurrence.ts';
import { actionsOf } from './vn-script.ts';
import { editDistance } from './expr/analyse.ts';
import { stuckPoints } from './stuck.ts';

export type Reach = 'reachable' | 'conditional' | 'unreachable';
type Path = (string | number)[];

/**
 * what the whole toon tells about a chapter (`toonFacts` over every document of the toon the caller has) – and, since
 * story variables are ONE save game for every toon (owner 2026-10-06, DATA.md "The save game"), what the OTHER toons do
 * with variables (`elsewhere`, from the server's flow index)
 */
export interface ToonFacts {
	/**
	 * variables something sets (setVariable, a choice's set / add, a picked-up item's id, an askText's answer) – in this
	 * toon or in any other (a variable set in toptoy is the one catpope reads)
	 */
	variables: ReadonlySet<string>;
	/** variables OTHER toons read (a test, an expression, a text's `{…}`): set here, read there = used */
	reads?: ReadonlySet<string>;
	/** group labels a jump target, an end, a random list or a map place refers to (entry points `?label=`) */
	labels: ReadonlySet<string>;
}

/** what `toonFacts` needs besides the documents */
export interface ToonFactsOptions {
	/**
	 * the toon the documents belong to: a chapter target naming another toon (`target.toon`) is no entry here. None: every
	 * target that names a toon counts as another toon's
	 */
	toon?: string;
	/** labels of this toon that OTHER toons' jumps lead to (`incomingJumps` over the server's flow index) */
	incoming?: Iterable<string>;
	/** story variables other toons set / read (`variablesElsewhere` over the server's flow index) */
	elsewhere?: { sets?: Iterable<string>; reads?: Iterable<string> };
}

/**
 * The facts of a toon from its documents (chapters, scenes, the map …; any JSON): walked iteratively, every object once
 * – a jump target is any `{ type, value, toon?, label?, anyOf? }`; one into another toon (`toon`) refers to that toon's
 * labels, not to these. `incoming`: labels other toons jump to (entry points too); `elsewhere`: other toons' variables.
 */
export function toonFacts(docs: Iterable<unknown>, options: ToonFactsOptions = {}): ToonFacts {
	const variables = new Set<string>(options.elsewhere?.sets ?? []);
	const labels = new Set<string>(options.incoming ?? []);
	const stack: unknown[] = [...docs];
	const seen = new Set<object>();
	while (stack.length) {
		const v = stack.pop();
		if (!v || typeof v !== 'object' || seen.has(v)) continue;
		seen.add(v);
		if (Array.isArray(v)) {
			for (const x of v) stack.push(x);
			continue;
		}
		const o = v as Record<string, any>;
		if (o.type === 'setVariable' && typeof o.params?.variable === 'string') variables.add(o.params.variable);
		if (askVariable(o)) variables.add(askVariable(o)!);
		if (typeof o.set?.variable === 'string') variables.add(o.set.variable);
		if (typeof o.add?.variable === 'string') variables.add(o.add.variable);
		if (o.action === 'pickup' && typeof o.item === 'string') variables.add(o.item);
		if (typeof o.type === 'string' && 'value' in o && !(typeof o.toon === 'string' && o.toon && o.toon !== options.toon)) {
			if (typeof o.label === 'string' && o.label) labels.add(o.label);
			if (Array.isArray(o.anyOf)) for (const l of o.anyOf) if (typeof l === 'string' && l) labels.add(l);
		}
		for (const key in o) stack.push(o[key]);
	}
	return { variables, labels, ...(options.elsewhere?.reads ? { reads: new Set(options.elsewhere.reads) } : {}) };
}

/**
 * Another story variable whose name looks like `name` (a typo? `visits_harbor` / `visits_harbour` – story variables are
 * shared by every toon, so a slip makes a second one): at most 1 edit for names of 5–7 letters, 2 from 8 on (none under
 * 5). Names that differ only in digits (`step1` / `step2`) are meant to differ; names that differ only in case count (a
 * likely slip). Internal `picked:` keys are never judged.
 */
export function similarVariable(name: string, known: Iterable<string>): string | undefined {
	if (name.startsWith('picked:')) return undefined;
	const limit = name.length >= 8 ? 2 : name.length >= 5 ? 1 : 0;
	if (!limit) return undefined;
	const digits = (s: string) => s.replace(/[0-9]+/g, '#');
	let best: string | undefined;
	let bestD = Infinity;
	for (const other of known) {
		if (other === name || other.startsWith('picked:') || Math.abs(other.length - name.length) > limit) continue;
		if (digits(other) === digits(name) && other.toLowerCase() !== name.toLowerCase()) continue;
		const d = editDistance(name.toLowerCase(), other.toLowerCase());
		const score = d === 0 ? 0.5 : d; // only the case differs
		if (score <= limit && score < bestD) {
			best = other;
			bestD = score;
		}
	}
	return best;
}

/** the story variable an askText event sets (scope toon – the default; scope reader sets `reader.name`, no variable) */
export function askVariable(e: unknown): string | null {
	const o = e as { type?: unknown; params?: { variable?: unknown; scope?: unknown } } | null | undefined;
	if (o?.type !== 'askText' || o.params?.scope === 'reader') return null;
	return typeof o.params?.variable === 'string' && o.params.variable ? o.params.variable : null;
}

/** an event that runs only if its test holds: a simple test (`{ variable, op, value }`) or an expression (`{ expr }`) */
export function isConditional(e: unknown): boolean {
	const c = (e as { if?: { variable?: unknown; expr?: unknown } } | null | undefined)?.if;
	return (typeof c?.variable === 'string' && !!c.variable) || (typeof c?.expr === 'string' && !!c.expr.trim());
}

/**
 * The key of a slice trigger: events with the same trigger run as one (one run per click / hover / scroll). The repeat
 * rule is part of it in its meaning (`repeatOf`: none = every time), not in its spelling.
 */
export function triggerKey(trigger: unknown): string {
	if (!trigger || typeof trigger !== 'object') return '';
	const { repeat, ...rest } = trigger as Record<string, unknown>;
	return `${JSON.stringify(rest)}|${JSON.stringify(repeatOf(repeat))}`;
}

/** how often a slice event's run / an item's taps run (`trigger.repeat`, `item.repeat`) */
export const runRepeat = (holder: unknown): Repeat => repeatOf((holder as { repeat?: unknown } | null | undefined)?.repeat);

/**
 * A condition that can never be true: its variable is set nowhere – in no toon (so it is always unset) – and the test
 * needs a value. Internal variables (`picked:` keys of placed items) are never judged.
 */
export function neverTrue(condition: unknown, facts: ToonFacts | undefined): boolean {
	const c = condition as { variable?: unknown; op?: unknown; value?: unknown } | null | undefined;
	if (!facts || !c || typeof c.variable !== 'string' || !c.variable || c.variable.startsWith('picked:')) return false;
	if (facts.variables.has(c.variable)) return false;
	switch (c.op) {
		case 'set':
		case '<':
		case '<=':
		case '>':
		case '>=':
			return true;
		case '==':
			return c.value !== undefined && c.value !== null && String(c.value) !== '';
		default:
			return false;
	}
}

export interface ChapterReach {
	/** per group */
	groups: Reach[];
	/** per group, per slice */
	slices: Reach[][];
	/** per group, per event of its script */
	events: Reach[][];
	/** events whose `if` can never be true (paths from the chapter: groups.g.events.i, …slices.s.events.j, …items.x.events.j) */
	neverTrue: Path[];
	/** diverts / questions caught in a loop with no way out (no end, no jump) */
	loops: Path[];
	/**
	 * slice trigger runs and item taps that run once for good (`repeat.mode` once – the save game checks them off): the
	 * path of the run's first event (…slices.s.events.j, …slices.s.items.x.events.0)
	 */
	once: Path[];
	/**
	 * questions where the reader may get stuck (stuck.ts `stuckPoints`: every choice conditional) – the showChoices'
	 * path; validation refuses them, the Flow graph marks them "may get stuck"
	 */
	stuck: Path[];
}

interface Edge {
	to: number;
	cond: boolean;
	/** inside a script (the loop check walks these) */
	script: boolean;
}

const conditional = isConditional;

/**
 * The chapter's reachability. `groups` = the chapter's groups with each scene in place of its ref. `facts` = the toon's
 * (`toonFacts`); without them every labelled group counts as an entry (a jump may lead there) and no condition is
 * judged. Entry points: the chapter's first group, a group the follow-up bar leads to (a labelled chain start after a
 * chain that can finish), labelled groups something refers to.
 */
export function chapterReach(groups: readonly any[], facts?: ToonFacts): ChapterReach {
	const out: Edge[][] = [];
	/** inputs an AND node needs (0 = any input) */
	const need: number[] = [];
	/** a script node that leaves (a jump) or finishes – the loop check walks back from these */
	const exit: boolean[] = [];
	/** the event behind a node, for the loop report */
	const where: (Path | null)[] = [];
	const loopKind: boolean[] = [];
	const node = (inputs = 0, path: Path | null = null) => {
		out.push([]);
		need.push(inputs);
		exit.push(false);
		where.push(path);
		loopKind.push(false);
		return out.length - 1;
	};
	const edge = (from: number, to: number, cond = false, script = false) => out[from].push({ to, cond, script });
	const never: Path[] = [];
	const once: Path[] = [];
	const entries: number[] = [];

	/**
	 * A script (a list of events run in order) from `entry`; `end` = its end node. Diverts look up named points in the
	 * same list. `idx`: the events' indices in their stored list (a slice's runs are parts of its events).
	 */
	const script = (events: any[], entry: number, end: number, base: Path, idx?: number[]): number[] => {
		const at = (j: number) => (idx ? idx[j] : j);
		const ids = events.map((_, j) => node(0, [...base, at(j)]));
		if (!events.length) {
			edge(entry, end, false, true);
			return ids;
		}
		edge(entry, ids[0], false, true);
		const point = (to: unknown) => {
			const at = typeof to === 'string' && to ? events.findIndex((e) => e?.type === 'label' && e.params?.name === to) : -1;
			return at >= 0 ? ids[at] : -1;
		};
		let block: number[] = [];
		events.forEach((e, j) => {
			const a = ids[j];
			const next = j + 1 < events.length ? ids[j + 1] : end;
			const cond = conditional(e);
			const dead = cond && neverTrue(e.if, facts);
			if (dead) never.push([...base, at(j), 'if']);
			const targets: { to: number; cond: boolean }[] = [];
			let flow = true;
			let leaves = false;
			switch (e?.type) {
				case 'divert': {
					const at = point(e.params?.to);
					targets.push({ to: at >= 0 ? at : next, cond: false });
					loopKind[a] = true;
					break;
				}
				case 'jumpTo':
					leaves = true;
					break;
				case 'addChoice':
					block.push(j);
					flow = false;
					break;
				case 'showChoices': {
					loopKind[a] = true;
					const choices = block;
					block = [];
					if (!choices.length) targets.push({ to: next, cond: false });
					let sure = false;
					for (const c of choices) {
						const choice = events[c];
						if (conditional(choice) && neverTrue(choice.if, facts)) continue; // reported at the choice itself
						const maybe = conditional(choice);
						if (!maybe) sure = true;
						const actions = actionsOf(choice?.params?.onSelect) as any[];
						if (actions.some((x) => x && 'jumpTo' in x)) {
							leaves = true;
							continue;
						}
						const divert = actions.find((x) => x && 'divert' in x);
						const at = divert ? point(divert.divert?.to) : -1;
						targets.push({ to: at >= 0 ? at : next, cond: maybe });
					}
					// only conditional choices: none may be shown – the question is skipped
					if (choices.length && !sure) targets.push({ to: next, cond: true });
					break;
				}
				default:
					flow = false;
			}
			if (!flow) {
				// not a branch: on to the next event whether it ran or was skipped
				edge(a, next, false, true);
				return;
			}
			if (!dead) {
				if (leaves) exit[a] = true;
				for (const t of targets) edge(a, t.to, cond || t.cond, true);
			}
			// the test fails: on to the next event (a test that can never pass: always)
			if (cond) edge(a, next, !dead, true);
		});
		return ids;
	};

	const gIn: number[] = [];
	const gate: number[] = [];
	const sliceNodes: number[][] = [];
	const eventNodes: number[][] = [];
	groups.forEach(() => gIn.push(node()));
	groups.forEach((g, gi) => {
		const base: Path = ['groups', gi];
		const slices: any[] = Array.isArray(g?.slices) ? g.slices : [];
		const events: any[] = Array.isArray(g?.events) ? g.events : [];
		const sNodes = slices.map((_, k) => node(0, [...base, 'slices', k]));
		sliceNodes.push(sNodes);
		const scriptEnd = node();
		exit[scriptEnd] = true;
		// the script starts when the group's first slice is seen (a group without slices: when it is reached)
		const start = sNodes.length ? sNodes[0] : gIn[gi];
		eventNodes.push(events.length ? script(events, start, scriptEnd, [...base, 'events']) : []);
		if (sNodes.length) edge(gIn[gi], sNodes[0]);
		let last = gIn[gi];
		slices.forEach((s, k) => {
			// slice events run per trigger (one run per click / hover / scroll), placed items' taps run their events
			const runs = new Map<string, { events: any[]; idx: number[] }>();
			(Array.isArray(s?.events) ? s.events : []).forEach((e: any, j: number) => {
				if (!e?.trigger) return;
				const key = triggerKey(e.trigger);
				if (!runs.has(key)) runs.set(key, { events: [], idx: [] });
				runs.get(key)!.events.push(e);
				runs.get(key)!.idx.push(j);
			});
			const gates: number[] = [];
			for (const run of runs.values()) {
				if (runRepeat(run.events[0]?.trigger).mode === 'once') once.push([...base, 'slices', k, 'events', run.idx[0]]);
				const scroll = run.events[0]?.trigger?.type === Trigger.INTERSECT;
				const single = run.events.length === 1 && run.events[0]?.type === 'divert' && !conditional(run.events[0]);
				const at = single ? events.findIndex((e) => e?.type === 'label' && e.params?.name === run.events[0].params?.to) : -1;
				if (at >= 0) {
					// one divert: the group's script from that point – the run ends with the script
					const ev = node(0, [...base, 'slices', k, 'events', run.idx[0]]);
					edge(sNodes[k], ev, false, true);
					edge(ev, eventNodes[gi][at], false, true);
					if (scroll) gates.push(scriptEnd);
					continue;
				}
				const end = node();
				exit[end] = true;
				script(run.events, sNodes[k], end, [...base, 'slices', k, 'events'], run.idx);
				if (scroll) gates.push(end);
			}
			(Array.isArray(s?.items) ? s.items : []).forEach((item: any, x: number) => {
				if (!Array.isArray(item?.events) || !item.events.length) return;
				if (runRepeat(item).mode === 'once') once.push([...base, 'slices', k, 'items', x, 'events', 0]);
				const end = node();
				exit[end] = true;
				script(item.events, sNodes[k], end, [...base, 'slices', k, 'items', x, 'events']);
			});
			// scrolling on past the slice: its scroll triggers' runs must have finished (the page waits for them)
			let done = sNodes[k];
			if (gates.length) {
				done = node(gates.length + 1);
				edge(sNodes[k], done);
				for (const g2 of gates) edge(g2, done);
			}
			if (k + 1 < slices.length) edge(done, sNodes[k + 1]);
			last = done;
		});
		// the group is through: its last slice, and its script finished (the page doesn't scroll on before)
		let through = last;
		if (events.length) {
			through = node(2);
			edge(last, through);
			edge(scriptEnd, through);
		}
		gate.push(through);
	});
	groups.forEach((g, gi) => {
		if (gi + 1 >= groups.length) return;
		const nextLabel = groups[gi + 1]?.label;
		// linked groups flow on; after a chain the follow-up bar leads to a labelled next group (no authored end)
		if (linksToNext(groups as ChainGroupLike[], gi)) edge(gate[gi], gIn[gi + 1]);
		else if (typeof nextLabel === 'string' && nextLabel && !groupEnd(g)) edge(gate[gi], gIn[gi + 1]);
	});
	if (groups.length) entries.push(gIn[0]);
	groups.forEach((g, gi) => {
		const label = g?.label;
		if (gi > 0 && typeof label === 'string' && label && (!facts || facts.labels.has(label))) entries.push(gIn[gi]);
	});

	const walk = (withConditions: boolean): Uint8Array => {
		const reached = new Uint8Array(out.length);
		const count = new Int32Array(out.length);
		const queue: number[] = [];
		for (const e of entries) if (!reached[e]) (reached[e] = 1), queue.push(e);
		while (queue.length) {
			const n = queue.pop()!;
			for (const e of out[n]) {
				if (e.cond && !withConditions) continue;
				if (need[e.to] > 0 && ++count[e.to] < need[e.to]) continue;
				if (!reached[e.to]) {
					reached[e.to] = 1;
					queue.push(e.to);
				}
			}
		}
		return reached;
	};
	const sure = walk(false);
	const maybe = walk(true);
	const reach = (n: number): Reach => (sure[n] ? 'reachable' : maybe[n] ? 'conditional' : 'unreachable');

	// loops: script nodes that are reached but can't get to an end or a jump (walking script edges backwards)
	const back: number[][] = out.map(() => []);
	out.forEach((edges, from) => edges.forEach((e) => e.script && back[e.to].push(from)));
	const free = new Uint8Array(out.length);
	const queue: number[] = [];
	exit.forEach((x, n) => x && ((free[n] = 1), queue.push(n)));
	while (queue.length) {
		const n = queue.pop()!;
		for (const from of back[n]) if (!free[from]) (free[from] = 1), queue.push(from);
	}
	const loops: Path[] = [];
	loopKind.forEach((k, n) => k && maybe[n] && !free[n] && where[n] && loops.push(where[n]!));

	return {
		groups: gIn.map(reach),
		slices: sliceNodes.map((nodes) => nodes.map(reach)),
		events: eventNodes.map((nodes) => nodes.map(reach)),
		neverTrue: never.map((p) => [...p]),
		loops,
		once,
		stuck: stuckPoints(groups).map((p) => p.path)
	};
}
