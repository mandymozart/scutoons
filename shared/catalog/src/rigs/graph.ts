// An instrument's node graph as graph.rs builds it (#269; the server: RigValidator::graph): known kinds, unique ids, at
// most one of each sink, links from an output to an input of the same type, one link per input, no cycle over the
// ordering links; plus `control_input` (instrument.rs) and an events node's patch (deck-events patch.rs). Pure TS over
// schema.json's node kinds – the creator shows these problems at the fields before Publish.
import { RIG_SCHEMA, type NodeKind } from './schema.ts';

export interface NodeRef {
	id: string;
	kind: string;
	params: Record<string, unknown>;
}
export interface LinkRef {
	from: string;
	to: string;
}

export const nodeKinds = (): Record<string, NodeKind> => RIG_SCHEMA.nodes;

/** scene.rs `split_ref`: `node.port`, a bare node = its `out` */
export function split(ref: string): [string, string] {
	const at = ref.indexOf('.');
	return at < 0 ? [ref, 'out'] : [ref.slice(0, at), ref.slice(at + 1)];
}

/** instrument.rs `canonical`: `node` → `node.out` */
export function canonical(ref: string): string {
	const [n, p] = split(ref);
	return `${n}.${p}`;
}

/** TOML has no null (scene.rs NodeDef params): a null anywhere in a param is refused */
export function hasNull(v: unknown): boolean {
	if (v === null) return true;
	if (Array.isArray(v)) return v.some(hasNull);
	if (v && typeof v === 'object') return Object.values(v).some(hasNull);
	return false;
}

/** the nodes of an instrument as { id, kind, params } – after the schema check (id and kind are strings) */
export function nodesOf(list: unknown): NodeRef[] {
	if (!Array.isArray(list)) return [];
	return list.flatMap((n) => {
		if (!n || typeof n !== 'object' || typeof n.id !== 'string' || typeof n.kind !== 'string') return [];
		const { id, kind, ...params } = n as Record<string, unknown> & { id: string; kind: string };
		return [{ id, kind, params }];
	});
}

export function linksOf(list: unknown): LinkRef[] {
	if (!Array.isArray(list)) return [];
	return list.flatMap((l) => (l && typeof l === 'object' && typeof l.from === 'string' && typeof l.to === 'string' ? [{ from: l.from, to: l.to }] : []));
}

/** node id → kind (unchecked) */
export const kindsOf = (nodes: NodeRef[]): Record<string, string> => Object.fromEntries(nodes.map((n) => [n.id, n.kind]));

/**
 * Node id → kind when the nodes and links form a valid graph; otherwise the problems (graph.rs `Graph::build`).
 */
export function buildGraph(who: string, nodes: NodeRef[], links: LinkRef[]): { kinds: Record<string, string> } | { problems: string[] } {
	const KINDS = nodeKinds();
	const p: string[] = [];
	const kinds: Record<string, string> = {};
	const index: Record<string, number> = {};
	for (const n of nodes) {
		if (!KINDS[n.kind]) p.push(`${who}: node '${n.id}': unknown kind '${n.kind}'`);
		if (n.id in kinds) p.push(`${who}: node id '${n.id}' is used twice`);
		index[n.id] = Object.keys(index).length;
		kinds[n.id] = n.kind;
	}
	for (const sink of RIG_SCHEMA.scene_only) {
		if (Object.values(kinds).filter((k) => k === sink).length > 1) p.push(`${who}: at most one ${sink} node`);
	}
	if (p.length) return { problems: p };
	const edges: [number, number][] = [];
	const inputs = new Set<string>();
	for (const l of links) {
		const [fn, fp] = split(l.from);
		const [tn, tp] = split(l.to);
		const at = `${who}: link ${l.from} → ${l.to}`;
		if (!(fn in kinds)) {
			p.push(`${at}: no node '${fn}'`);
			continue;
		}
		if (!(tn in kinds)) {
			p.push(`${at}: no node '${tn}'`);
			continue;
		}
		const fty = KINDS[kinds[fn]].outputs[fp];
		const tty = KINDS[kinds[tn]].inputs[tp];
		if (!fty) {
			p.push(`${at}: ${kinds[fn]} has no output '${fp}'`);
			continue;
		}
		if (!tty) {
			p.push(`${at}: ${kinds[tn]} has no input '${tp}'`);
			continue;
		}
		if (fty !== tty) {
			p.push(`${at}: a ${fty} output cannot feed a ${tty} input`);
			continue;
		}
		if (inputs.has(`${tn}.${tp}`)) {
			p.push(`${at}: the input already has a link`);
			continue;
		}
		inputs.add(`${tn}.${tp}`);
		// Link::orders: a control link into or out of an audio node crosses threads a frame late – it never orders
		const orders = fty !== 'control' || (KINDS[kinds[fn]].domain !== 'audio' && KINDS[kinds[tn]].domain !== 'audio');
		if (orders) edges.push([index[fn], index[tn]]);
	}
	if (!p.length && cyclic(nodes.length, edges)) p.push(`${who}: the graph has a cycle`);
	return p.length ? { problems: p } : { kinds };
}

/** Kahn: not every node reached = a cycle */
export function cyclic(n: number, edges: [number, number][]): boolean {
	const indeg = new Array<number>(n).fill(0);
	for (const [, to] of edges) indeg[to]++;
	const ready = indeg.flatMap((d, i) => (d === 0 ? [i] : []));
	let done = 0;
	while (ready.length) {
		const next = ready.shift()!;
		done++;
		for (const [from, to] of edges) if (from === next && --indeg[to] === 0) ready.push(to);
	}
	return done < n;
}

/** instrument.rs `control_input`: `node.port` names a control input of the node's kind; the problem, or null */
export function controlInput(who: string, kinds: Record<string, string>, path: string): string | null {
	const [node, port] = split(path);
	const kind = kinds[node];
	if (kind === undefined) return `${who}: no node '${node}' (${path})`;
	const def = nodeKinds()[kind];
	if (!def) return `${who}: unknown kind '${kind}'`;
	if (def.inputs[port] !== 'control') return `${who}: ${path} is not a control input of ${kind}`;
	return null;
}

/** every `node.port` of an instrument's nodes that is a control input (what a preset may set, a knob may show) */
export function controlInputs(nodes: NodeRef[]): string[] {
	const KINDS = nodeKinds();
	return nodes.flatMap((n) =>
		Object.entries(KINDS[n.kind]?.inputs ?? {})
			.filter(([, ty]) => ty === 'control')
			.map(([port]) => `${n.id}.${port}`)
	);
}

/** every `node.port` that is an audio output (what `out` may name) */
export function audioOutputs(nodes: NodeRef[]): string[] {
	const KINDS = nodeKinds();
	return nodes.flatMap((n) =>
		Object.entries(KINDS[n.kind]?.outputs ?? {})
			.filter(([, ty]) => ty === 'audio')
			.map(([port]) => `${n.id}.${port}`)
	);
}

/** every input `node.port` (any type) – what a preset may unlink or link into */
export function allInputs(nodes: NodeRef[]): string[] {
	const KINDS = nodeKinds();
	return nodes.flatMap((n) => Object.keys(KINDS[n.kind]?.inputs ?? {}).map((port) => `${n.id}.${port}`));
}

// ── events node (deck-events patch.rs; the server: RigValidator::events) ──────────────────────────────────────

/** deck-events' generators: kind → [inputs, outputs, params] (generators/*.rs `ports`) */
export const GENERATORS: Record<string, [string[], string[], string[]]> = {
	random_walk: [[], ['notes'], ['spread', 'rate', 'glide']],
	pulse: [[], ['out'], ['every']],
	arp: [['notes'], ['notes'], ['rate']]
};
export const ARP_MODES = ['up', 'down', 'updown'];

/** names.rs `event`: a domain word, then parts (camelCase allowed) */
const eventName = (name: string) => /^[a-z0-9_]+(?:\.[a-z][A-Za-z0-9_]*)+$/.test(name);
const isObj = (v: unknown): v is Record<string, unknown> => !!v && typeof v === 'object' && !Array.isArray(v);
const show = (v: unknown) => (v === null || v === undefined || typeof v === 'object' ? JSON.stringify(v) : String(v));

/** `Patch::from_table` + `build` on an events node's params: known generators, edges, `out`, ≤ KNOBS knobs, listen */
export function checkEvents(who: string, params: Record<string, unknown>): string[] {
	const p: string[] = [];
	const generators = params.generator ?? [];
	if (!Array.isArray(generators)) return [`${who}: generator must be a list`];
	const ports: Record<string, [string[], string[], string[]]> = {};
	generators.forEach((g, n) => {
		if (!isObj(g) || typeof g.id !== 'string' || typeof g.kind !== 'string') {
			p.push(`${who}: generator[${n}] must be { id, kind, … }`);
			return;
		}
		if (g.id in ports) p.push(`${who}: generator '${g.id}' twice`);
		if (!GENERATORS[g.kind]) {
			p.push(`${who}: generator '${g.id}': unknown kind '${g.kind}' (${Object.keys(GENERATORS).join(', ')})`);
			return;
		}
		if (g.kind === 'arp' && g.mode !== undefined && !ARP_MODES.includes(g.mode as string)) p.push(`${who}: ${g.id}: unknown mode '${show(g.mode)}' (up, down, updown)`);
		if (g.kind === 'pulse' && g.name !== undefined && !(typeof g.name === 'string' && eventName(g.name))) p.push(`${who}: ${g.id}: name must be an event name (domain.name)`);
		ports[g.id] = GENERATORS[g.kind];
	});
	const ref = (r: unknown, slot: 0 | 1 | 2, what: string): boolean => {
		if (typeof r !== 'string' || !r.includes('.')) {
			p.push(`${who}: '${show(r)}' is not <generator>.<port>`);
			return false;
		}
		const cut = r.lastIndexOf('.');
		const [g, port] = [r.slice(0, cut), r.slice(cut + 1)];
		if (!ports[g]) {
			p.push(`${who}: no generator '${g}' (${r})`);
			return false;
		}
		if (!ports[g][slot].includes(port)) {
			p.push(`${who}: ${g} has no ${what} '${port}'`);
			return false;
		}
		return true;
	};
	const edges = params.edge ?? [];
	if (!Array.isArray(edges)) p.push(`${who}: edge must be a list`);
	else
		edges.forEach((e, n) => {
			if (!isObj(e)) {
				p.push(`${who}: edge[${n}] must be { from, to, delayed? }`);
				return;
			}
			for (const k of Object.keys(e)) if (!['from', 'to', 'delayed'].includes(k)) p.push(`${who}: edge[${n}]: unknown field "${k}" (from, to, delayed)`);
			for (const k of ['from', 'to']) if (e[k] === undefined) p.push(`${who}: edge[${n}]: ${k} is required`);
			ref(e.from, 1, 'output');
			ref(e.to, 0, 'input');
			if (e.delayed !== undefined && typeof e.delayed !== 'boolean') p.push(`${who}: edge[${n}].delayed must be true or false`);
		});
	if (!('out' in params)) p.push(`${who}: out is required (<generator>.<port> the host plays)`);
	else ref(params.out, 1, 'output');
	const knobs = params.knobs ?? [];
	if (!Array.isArray(knobs)) p.push(`${who}: knobs must be a list`);
	else if (knobs.length > RIG_SCHEMA.event_knobs) p.push(`${who}: at most ${RIG_SCHEMA.event_knobs} knobs (a, b, c, d), not ${knobs.length}`);
	else for (const k of knobs) ref(k, 2, 'param');
	const listen = params.listen ?? [];
	if (!Array.isArray(listen)) p.push(`${who}: listen must be a list`);
	else
		listen.forEach((l, n) => {
			if (!isObj(l)) {
				p.push(`${who}: listen[${n}] must be { to, name | domain }`);
				return;
			}
			for (const k of Object.keys(l)) if (!['to', 'name', 'domain'].includes(k)) p.push(`${who}: listen[${n}]: unknown field "${k}" (to, name, domain)`);
			if (l.to === undefined) p.push(`${who}: listen[${n}]: to is required`);
			ref(l.to, 0, 'input');
			const name = l.name ?? null;
			const domain = l.domain ?? null;
			if ((name === null) === (domain === null)) p.push(`${who}: listen[${n}]: a \`name\` or a \`domain\`, one of them`);
			else if (name !== null && !(typeof name === 'string' && eventName(name))) p.push(`${who}: listen[${n}].name must be an event name (domain.name)`);
			else if (domain !== null && !(typeof domain === 'string' && /^[a-z0-9_]+$/.test(domain))) p.push(`${who}: listen[${n}].domain must be one plain word`);
		});
	if (!p.length && Array.isArray(edges) && eventCycle(edges as Record<string, unknown>[], ports)) p.push(`${who}: the generators loop without a delayed edge`);
	return p;
}

function eventCycle(edges: Record<string, unknown>[], ports: Record<string, unknown>): boolean {
	const index = Object.fromEntries(Object.keys(ports).map((k, i) => [k, i]));
	const pairs: [number, number][] = [];
	for (const e of edges) {
		if (e.delayed === true) continue;
		const gen = (r: unknown) => String(r).slice(0, String(r).lastIndexOf('.'));
		pairs.push([index[gen(e.from)], index[gen(e.to)]]);
	}
	return cyclic(Object.keys(ports).length, pairs);
}
