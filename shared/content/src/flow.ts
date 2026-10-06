// The story flow of a chapter as a graph (owner 2026-10-06, the creator's read-only "Flow" panel): what leads where –
// groups in reading order (linked groups, the follow-up bar, an authored end), the key points of each group's script
// (named points, questions with their choices, diverts, jumps), slice triggers / item taps that branch, and the places
// outside the chapter a jump leads to. The status of every node comes from `chapterReach` (reach.ts) – the single source
// of truth, so the graph agrees with the "unreachable" chips. Pure: no layout, no texts of the UI.
import { groupEnd, linksToNext, type ChainGroupLike } from './chains.ts';
import type { JumpToTarget } from './content.ts';
import { chapterReach, type ChapterReach, type Reach, type ToonFacts } from './reach.ts';
import { actionsOf } from './vn-script.ts';

/** a node's state: the reach of its item, or a condition that can never be true / a loop with no way out at it */
export type FlowStatus = Reach | 'never' | 'loop';

export type FlowNodeKind =
	/** a group of the chapter (a scene in place of its ref) */
	| 'group'
	/** a named point of a script (`label` event) */
	| 'label'
	/** a question: the choices before a `showChoices` */
	| 'choices'
	/** a `divert` event (to a named point of the same script) */
	| 'divert'
	/** a `jumpTo` event */
	| 'jump'
	/** a slice's trigger run (click / hover / scroll) or a placed item's tap that branches (a divert, a jump, a question) */
	| 'trigger'
	/** a place outside the chapter (`target`), or the next chapter (`next`) */
	| 'exit';

export interface FlowNode {
	id: string;
	kind: FlowNodeKind;
	status: FlowStatus;
	/** the group the node belongs to (exits: none) */
	group?: number;
	/** where it lives in the chapter (groups.g, …events.i, …slices.s.events.j, …slices.s.items.x.events.j): what to open */
	path?: (string | number)[];
	/** group: its label (or the caller's `name`); label / divert: the point's name; trigger: the slice number and how */
	name?: string;
	/** group: slice count; choices: the number of choices */
	count?: number;
	/** group: slices that can't be reached (indices) while the group itself can */
	deadSlices?: number[];
	/** group: it has a script */
	script?: boolean;
	/** trigger: `intersect` / `click` / `hover` / `item` */
	trigger?: string;
	/** trigger: the slice index */
	slice?: number;
	/** jump / exit: where to */
	target?: JumpToTarget;
	/** exit: the next chapter (the follow-up bar after the last chain or before an unlabelled group) */
	next?: boolean;
}

export type FlowEdgeKind =
	/** a linked group flows straight on (same page) */
	| 'continues'
	/** the follow-up bar leads to the next (labelled) group */
	| 'follow'
	/** on through the script (or past a point) */
	| 'then'
	/** a choice of a question */
	| 'choice'
	| 'divert'
	| 'jump'
	/** one of a random list (`anyOf`) */
	| 'random'
	/** the group's authored end (`Group.end`) */
	| 'end'
	/** a slice trigger / item tap of the group */
	| 'trigger';

export interface FlowEdge {
	id: string;
	from: string;
	to: string;
	kind: FlowEdgeKind;
	/** a choice's text */
	label?: string;
	/** on after a test that failed (`else`), a question whose choices may all be hidden (`none`), an end that goes at once (`auto`) */
	branch?: 'else' | 'none' | 'auto';
	/** the test on the way ("coins >= 3") */
	condition?: string;
	/** the test can never pass (its variable is set nowhere in the toon) */
	never?: boolean;
	/** the status of the node it leaves from (an edge out of something unreachable is dead too) */
	status: FlowStatus;
}

export interface ChapterFlow {
	nodes: FlowNode[];
	edges: FlowEdge[];
	reach: ChapterReach;
}

export interface FlowOptions {
	/** the chapter's number: a jump to this chapter leads to its groups instead of an exit */
	chapter?: number;
	/** the toon's facts (`toonFacts`), passed to `chapterReach` */
	facts?: ToonFacts;
	/** the reach the caller computed already (same groups and facts) – else computed here */
	reach?: ChapterReach;
	/** a group's name (default its label, else "#n") */
	name?: (group: any, index: number) => string;
	/** a text (a choice's label: a string or a locale map) as shown – default the string or the map's first text */
	text?: (value: unknown) => string;
}

/** a condition as one line: "coins >= 3", "met set", "name == Ana" */
export function conditionText(c: unknown): string {
	const x = c as { variable?: unknown; op?: unknown; value?: unknown } | null | undefined;
	if (!x || typeof x.variable !== 'string' || !x.variable) return '';
	if (x.op === 'set' || x.op === 'unset') return `${x.variable} ${x.op}`;
	return `${x.variable} ${typeof x.op === 'string' ? x.op : '=='} ${x.value ?? ''}`.trim();
}

const firstText = (v: unknown): string => {
	if (typeof v === 'string') return v;
	if (v && typeof v === 'object') for (const t of Object.values(v)) if (typeof t === 'string' && t.trim()) return t;
	return '';
};

const conditional = (e: any) => typeof e?.if?.variable === 'string' && !!e.if.variable;

/**
 * The flow of a chapter. `groups` = the chapter's groups with each scene in place of its ref (what `chapterReach` gets).
 */
export function chapterFlow(groups: readonly any[], options: FlowOptions = {}): ChapterFlow {
	const reach = options.reach ?? chapterReach(groups, options.facts);
	const text = options.text ?? firstText;
	const name = options.name ?? ((g: any, i: number) => (typeof g?.label === 'string' && g.label ? g.label : `#${i + 1}`));
	const nodes: FlowNode[] = [];
	const edges: FlowEdge[] = [];
	const byId = new Map<string, FlowNode>();
	const add = (n: FlowNode) => {
		if (!byId.has(n.id)) {
			byId.set(n.id, n);
			nodes.push(n);
		}
		return n.id;
	};
	const loops = new Set(reach.loops.map((p) => p.join('.')));
	const never = new Set(reach.neverTrue.map((p) => p.slice(0, -1).join('.')));
	const statusAt = (path: (string | number)[], base: Reach): FlowStatus => {
		const key = path.join('.');
		return loops.has(key) ? 'loop' : never.has(key) ? 'never' : base;
	};
	const edge = (from: string, to: string, kind: FlowEdgeKind, extra: Partial<FlowEdge> = {}) => {
		edges.push({ id: `${from}>${to}#${edges.length}`, from, to, kind, status: byId.get(from)?.status ?? 'reachable', ...extra });
	};

	// group labels → index (the first group with that label)
	const labelled = new Map<string, number>();
	groups.forEach((g, gi) => typeof g?.label === 'string' && g.label && !labelled.has(g.label) && labelled.set(g.label, gi));
	const groupId = (gi: number) => `g${gi}`;
	groups.forEach((g, gi) => {
		const slices: any[] = Array.isArray(g?.slices) ? g.slices : [];
		const status = reach.groups[gi] ?? 'unreachable';
		const dead = status === 'unreachable' ? [] : (reach.slices[gi] ?? []).flatMap((r, k) => (r === 'unreachable' ? [k] : []));
		add({
			id: groupId(gi),
			kind: 'group',
			status,
			group: gi,
			path: ['groups', gi],
			name: name(g, gi),
			count: slices.length,
			...(dead.length ? { deadSlices: dead } : {}),
			...(Array.isArray(g?.events) && g.events.length ? { script: true } : {})
		});
	});

	/** where a jump leads: groups of this chapter, else an exit node */
	const targets = (t: any): { id: string; random: boolean }[] => {
		if (!t || typeof t !== 'object' || typeof t.type !== 'string') return [];
		const here = t.type === 'chapter' && options.chapter !== undefined && Number(t.value) === options.chapter;
		const anyOf: string[] = Array.isArray(t.anyOf) ? t.anyOf.filter((l: unknown) => typeof l === 'string' && l) : [];
		if (here) {
			const labels = anyOf.length ? anyOf : typeof t.label === 'string' && t.label ? [t.label] : [];
			if (!labels.length) return groups.length ? [{ id: groupId(0), random: false }] : [];
			return labels.map((l) => {
				const gi = labelled.get(l);
				if (gi !== undefined) return { id: groupId(gi), random: anyOf.length > 0 };
				const target: JumpToTarget = { type: t.type, value: String(t.value), label: l };
				return { id: add({ id: `x:${t.type}:${t.value}:${l}`, kind: 'exit', status: 'unreachable', target }), random: anyOf.length > 0 };
			});
		}
		const target: JumpToTarget = { type: t.type, value: String(t.value ?? '') };
		if (typeof t.label === 'string' && t.label) target.label = t.label;
		if (anyOf.length) target.anyOf = anyOf;
		const id = `x:${t.type}:${target.value}:${target.label ?? ''}:${anyOf.join('|')}`;
		return [{ id: add({ id, kind: 'exit', status: 'unreachable', target }), random: false }];
	};
	const jumpEdges = (from: string, t: unknown, extra: Partial<FlowEdge>) => {
		for (const x of targets(t)) edge(from, x.id, x.random ? 'random' : 'jump', extra);
	};
	const cond = (e: any): Partial<FlowEdge> => {
		if (!conditional(e)) return {};
		return { condition: conditionText(e.if) };
	};

	/** what a pending way carries onto its edge */
	const carry = (p: Pending): Partial<FlowEdge> => ({
		...(p.label ? { label: p.label } : {}),
		...(p.branch ? { branch: p.branch } : {}),
		...(p.condition ? { condition: p.condition } : {}),
		...(p.never ? { never: true } : {})
	});
	type Pending = { from: string; kind: FlowEdgeKind; label?: string; branch?: FlowEdge['branch']; condition?: string; never?: boolean };
	/**
	 * A script's key points in order; returns what falls through its end. `id` = a node id for event j, `path` its path.
	 * `pending` = what leads into the script's start.
	 */
	const script = (events: any[], status: (j: number) => FlowStatus, id: (j: number) => string, path: (j: number) => (string | number)[], gi: number, pending: Pending[]): Pending[] => {
		const points = new Map<string, string>();
		events.forEach((e, j) => {
			if (e?.type === 'label' && typeof e.params?.name === 'string' && e.params.name && !points.has(e.params.name)) {
				points.set(e.params.name, add({ id: id(j), kind: 'label', status: status(j), group: gi, path: path(j), name: e.params.name }));
			}
		});
		let block: number[] = [];
		const enter = (to: string) => {
			for (const p of pending) edge(p.from, to, p.kind, carry(p));
			pending = [];
		};
		events.forEach((e, j) => {
			const dead = never.has(path(j).join('.'));
			switch (e?.type) {
				case 'label': {
					const at = points.get(e.params?.name);
					if (at !== id(j)) return; // a duplicate name: the first one counts
					enter(at);
					pending = [{ from: at, kind: 'then' }];
					return;
				}
				case 'addChoice':
					block.push(j);
					return;
				case 'divert':
				case 'jumpTo': {
					const at = add({
						id: id(j),
						kind: e.type === 'divert' ? 'divert' : 'jump',
						status: status(j),
						group: gi,
						path: path(j),
						...(e.type === 'divert' ? { name: String(e.params?.to ?? '') } : { target: e.params?.target })
					});
					enter(at);
					const extra = { ...cond(e), ...(dead ? { never: true } : {}) };
					if (e.type === 'divert') {
						const to = points.get(e.params?.to);
						if (to) edge(at, to, 'divert', extra);
						// a divert to a point that isn't there goes on (as the reader does)
						if (!to || conditional(e)) pending = [{ from: at, kind: 'then', ...(to ? { branch: 'else' } : {}) }];
					} else {
						jumpEdges(at, e.params?.target, extra);
						if (conditional(e)) pending = [{ from: at, kind: 'then', branch: 'else' }];
					}
					return;
				}
				case 'showChoices': {
					const choices = block;
					block = [];
					const at = add({ id: id(j), kind: 'choices', status: status(j), group: gi, path: path(j), count: choices.length });
					enter(at);
					const extra = { ...cond(e), ...(dead ? { never: true } : {}) };
					let sure = false;
					for (const c of choices) {
						const choice = events[c];
						const label = text(choice?.params?.label) || `#${choices.indexOf(c) + 1}`;
						const deadChoice = never.has(path(c).join('.'));
						if (!conditional(choice)) sure = true;
						const own: Partial<FlowEdge> = { label, ...(conditional(choice) ? { condition: conditionText(choice.if) } : extra.condition ? { condition: extra.condition } : {}), ...(deadChoice || dead ? { never: true } : {}) };
						const actions = actionsOf(choice?.params?.onSelect) as any[];
						const jump = actions.find((x) => x && 'jumpTo' in x);
						if (jump) {
							jumpEdges(at, jump.jumpTo?.target, own);
							continue;
						}
						const divert = actions.find((x) => x && 'divert' in x);
						const to = divert ? points.get(divert.divert?.to) : undefined;
						if (to) edge(at, to, 'choice', own);
						else pending.push({ from: at, kind: 'choice', ...own } as Pending);
					}
					// no choice, or only conditional ones: the question may be skipped
					if (!choices.length || !sure) pending.push({ from: at, kind: 'then', ...(choices.length ? { branch: 'none' as const } : {}) });
					if (conditional(e)) pending.push({ from: at, kind: 'then', branch: 'else' });
					return;
				}
			}
		});
		return pending;
	};

	/** what leaves each group when it is through (its script's end, or the group itself) */
	const through: Pending[][] = groups.map((g, gi) => {
		const events: any[] = Array.isArray(g?.events) ? g.events : [];
		const base = ['groups', gi, 'events'];
		const start: Pending[] = [{ from: groupId(gi), kind: 'then' }];
		if (!events.length) return start;
		return script(
			events,
			(j) => statusAt([...base, j], reach.events[gi]?.[j] ?? 'unreachable'),
			(j) => `g${gi}e${j}`,
			(j) => [...base, j],
			gi,
			start
		);
	});

	// slice triggers and item taps that branch: a node each, from the group
	groups.forEach((g, gi) => {
		const slices: any[] = Array.isArray(g?.slices) ? g.slices : [];
		slices.forEach((s, k) => {
			const sliceStatus = reach.slices[gi]?.[k] ?? 'unreachable';
			const runs = new Map<string, { trigger: string; events: any[]; idx: number[]; base: (string | number)[] }>();
			(Array.isArray(s?.events) ? s.events : []).forEach((e: any, j: number) => {
				if (!e?.trigger) return;
				const key = JSON.stringify(e.trigger);
				if (!runs.has(key)) runs.set(key, { trigger: String(e.trigger.type ?? ''), events: [], idx: [], base: ['groups', gi, 'slices', k, 'events'] });
				runs.get(key)!.events.push(e);
				runs.get(key)!.idx.push(j);
			});
			(Array.isArray(s?.items) ? s.items : []).forEach((item: any, x: number) => {
				if (!Array.isArray(item?.events) || !item.events.length) return;
				runs.set(`item${x}`, { trigger: 'item', events: item.events, idx: item.events.map((_: unknown, j: number) => j), base: ['groups', gi, 'slices', k, 'items', x, 'events'] });
			});
			let n = 0;
			for (const run of runs.values()) {
				const branches = run.events.some((e) => e?.type === 'divert' || e?.type === 'jumpTo' || e?.type === 'showChoices');
				if (!branches) continue;
				const id = `g${gi}s${k}t${n++}`;
				add({ id, kind: 'trigger', status: sliceStatus, group: gi, path: [...run.base, run.idx[0]], slice: k, trigger: run.trigger });
				edge(groupId(gi), id, 'trigger');
				// a lone divert goes into the group's script at its point
				const lone = run.events.length === 1 && run.events[0]?.type === 'divert' ? run.events[0] : null;
				if (lone) {
					const at = (Array.isArray(g?.events) ? g.events : []).findIndex((e: any) => e?.type === 'label' && e.params?.name === lone.params?.to);
					if (at >= 0 && byId.has(`g${gi}e${at}`)) {
						edge(id, `g${gi}e${at}`, 'divert', { ...cond(lone), ...(never.has([...run.base, run.idx[0]].join('.')) ? { never: true } : {}) });
						continue;
					}
				}
				// its own little script: what branches in it hangs off the trigger node (its end goes nowhere: the page goes on)
				script(
					run.events,
					(j) => statusAt([...run.base, run.idx[j]], sliceStatus),
					(j) => `${id}e${j}`,
					(j) => [...run.base, run.idx[j]],
					gi,
					[{ from: id, kind: 'then' }]
				);
			}
		});
	});

	// reading order: linked groups, the follow-up bar, an authored end, the next chapter
	groups.forEach((g, gi) => {
		const out = through[gi];
		if (!out.length) return;
		const link = (to: string, kind: FlowEdgeKind, extra: Partial<FlowEdge> = {}) => {
			for (const p of out) edge(p.from, to, p.kind === 'choice' ? 'choice' : kind, { ...carry(p), ...extra });
		};
		if (linksToNext(groups as ChainGroupLike[], gi)) return link(groupId(gi + 1), 'continues');
		const end = groupEnd(g);
		if (end) {
			for (const x of targets(end.target)) link(x.id, x.random ? 'random' : 'end', end.auto ? { branch: 'auto' } : {});
			return;
		}
		const next = groups[gi + 1];
		if (next && typeof next.label === 'string' && next.label) return link(groupId(gi + 1), 'follow');
		link(add({ id: 'x:next', kind: 'exit', status: 'unreachable', next: true }), 'follow');
	});

	// an exit is as reachable as the best way into it
	const rank: Record<FlowStatus, number> = { reachable: 3, conditional: 2, loop: 1, never: 1, unreachable: 0 };
	const best = new Map<string, number>();
	for (const e of edges) {
		const to = byId.get(e.to);
		if (to?.kind !== 'exit') continue;
		const r = e.never ? 0 : Math.min(rank[e.status], e.condition ? 2 : 3);
		best.set(e.to, Math.max(best.get(e.to) ?? 0, r));
	}
	for (const n of nodes) if (n.kind === 'exit') n.status = (best.get(n.id) ?? 0) >= 3 ? 'reachable' : (best.get(n.id) ?? 0) >= 1 ? 'conditional' : 'unreachable';
	return { nodes, edges, reach };
}
