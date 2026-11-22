// The story's projector for the Spores tab (#763, agents/research/redesign/NETWORK.md §2): a level of the story flow as
// @scutoons/content computes it (chapterFlow, groupFlow, toonFlow – the story's rules stay there) written as a weave of
// the spore contract. A box is a spore, a way between them a thread: on through the story an event, a jump or an end a
// scene; what leads outside the level ends in a far spore and a stub named by where it goes. Pure.
import type { Spores } from '@scutoons/api';
import type { FlowEdge, FlowNode } from '@scutoons/content';

type Weave = Spores.Weave;
type Spore = Spores.Spore;
type Thread = Spores.Thread;

/** what a level adds: its address (the weave's context) and the breadcrumb, root first */
export interface StoryLevel {
	context: string;
	levels: Spores.Level[];
}

const CONTAINERS = new Set(['group', 'slice', 'chapter', 'toon']);
const RULES = new Set(['choices', 'trigger', 'step']);

function family(n: FlowNode): Spores.Family {
	if (n.kind === 'exit') return 'far';
	if (CONTAINERS.has(n.kind)) return 'container';
	if (RULES.has(n.kind)) return 'rule';
	return 'action';
}

/** a way's signal: what takes the reader to another place is a scene, everything else happens on the way – an event */
function signal(e: FlowEdge, to: FlowNode | undefined): Spores.Signal {
	if (e.kind === 'jump' || e.kind === 'end' || to?.kind === 'exit') return 'scene';
	return 'event';
}

/** the words a way shows: a choice's text, else its kind where it says more than "on" */
function words(e: FlowEdge): string | undefined {
	if (e.label) return e.label;
	return e.kind === 'then' || e.kind === 'continues' || e.kind === 'follow' || e.kind === 'order' ? undefined : e.kind;
}

/** where an exit leads, as an address of the one grammar (chapter / toon / product / xr) */
function farOf(n: FlowNode): string {
	const t = n.target;
	if (n.next) return `chapter:next`;
	if (!t) return n.id;
	return `${t.type}:${t.toon ? `${t.toon}/` : ''}${t.value}`;
}

/** a box's detail line: its kind and, where it has one, its count or status */
function detail(n: FlowNode): string {
	const parts: string[] = [n.kind];
	if (n.count !== undefined) parts.push(String(n.count));
	if (n.status && n.status !== 'reachable') parts.push(String(n.status));
	return parts.join(' · ');
}

/** The weave of one story level: its boxes and ways as `chapterFlow` / `groupFlow` / `toonFlow` gave them. */
export function storyWeave(flow: { nodes: readonly FlowNode[]; edges: readonly FlowEdge[] }, at: StoryLevel): Weave {
	const byId = new Map(flow.nodes.map((n) => [n.id, n]));
	const ins = new Set(flow.edges.map((e) => e.to));
	const outs = new Map<string, Set<Spores.Signal>>();
	for (const e of flow.edges) {
		const s = outs.get(e.from) ?? new Set();
		s.add(signal(e, byId.get(e.to)));
		outs.set(e.from, s);
	}
	const spores: Spore[] = flow.nodes.map((n) => {
		const ports: Spores.Port[] = [];
		if (ins.has(n.id)) ports.push({ id: 'in', label: n.kind === 'exit' ? '' : 'enter', signal: n.kind === 'exit' ? 'far' : 'event', dir: 'in', many: true });
		for (const s of outs.get(n.id) ?? []) ports.push({ id: s, label: s === 'scene' ? 'leads to' : 'on', signal: s, dir: 'out' });
		const spore: Spore = {
			id: n.id,
			kind: n.kind,
			family: family(n),
			label: n.kind === 'exit' ? `→ ${n.name ?? farOf(n)}` : (n.name ?? n.kind),
			detail: detail(n),
			address: n.kind === 'exit' ? farOf(n) : `${at.context}/${n.id}`,
			driver: 'reader',
			ports
		};
		return spore;
	});
	const threads: Thread[] = flow.edges.map((e) => {
		const to = byId.get(e.to);
		const s = signal(e, to);
		const t: Thread = { id: e.id, from: { spore: e.from, port: s }, to: { spore: e.to, port: 'in' }, signal: s };
		const label = words(e);
		if (label) t.label = label;
		if (e.count && e.count > 1) t.count = e.count;
		if (to?.kind === 'exit') t.crosses = 'navigation';
		return t;
	});
	const stubs: Spores.Stub[] = flow.edges
		.filter((e) => byId.get(e.to)?.kind === 'exit')
		.map((e) => {
			const far = byId.get(e.to)!;
			return { thread: e.id, side: 'out', far: farOf(far), label: `→ ${far.name ?? farOf(far)}` };
		});
	const problems: Spores.Problem[] = flow.nodes
		.filter((n) => n.status === 'never' || n.status === 'stuck' || n.status === 'loop' || n.status === 'unreachable')
		.map((n) => ({ at: n.id, severity: n.status === 'stuck' ? 'error' : 'warning', text: String(n.status) }));
	return { scope: 'story', context: at.context, levels: at.levels, spores, threads, stubs, problems };
}
