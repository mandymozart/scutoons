// The Nodes tab's layout (#282, RIG-EDITING.md §2 "Nodes – read-only first"): where the boxes, their ports and the cables
// of a wiring go on screen, in rem. Only placement: the graph – the nodes, their kinds and ports, the links resolved to
// ports, each node's depth and the path of each cable's value – is the core's (deck-space rig/wiring.rs, handed on by
// the engine host); the values come from the engine's analysis frame. A column per depth, the nodes of a column one
// under the other in the scene's order, a box per node with its inputs on the left and its outputs on the right, a
// cable as a curve from an output to an input.

export interface GraphPort {
	name: string;
	type: string;
}

export interface GraphNode {
	id: string;
	kind: string;
	track: string;
	depth: number;
	inputs: GraphPort[];
	outputs: GraphPort[];
	played: boolean;
}

export interface GraphLink {
	from: number;
	out: string;
	to: number;
	input: string;
	type: string;
	value: string;
}

export interface GraphIn {
	nodes: GraphNode[];
	links: GraphLink[];
}

/** a wiring as the core answers it: the graph, or none and why */
export type GraphWiring = GraphIn & { problem?: string };

/** the sizes the layout uses, in rem */
export const NODE = { width: 8.5, head: 1.5, row: 1, gapX: 2.5, gapY: 1, margin: 1 } as const;

export interface PlacedPort {
	name: string;
	type: string;
	/** the anchor a cable ends at (the box's edge) */
	x: number;
	y: number;
	/** a cable ends here */
	wired: boolean;
}

export interface PlacedNode {
	id: string;
	kind: string;
	track: string;
	played: boolean;
	x: number;
	y: number;
	w: number;
	h: number;
	inputs: PlacedPort[];
	outputs: PlacedPort[];
}

export interface PlacedCable {
	/** the link's index (its value is `values[index]`) */
	index: number;
	type: string;
	/** the SVG path (rem units) */
	d: string;
	/** where its value is written: the curve's middle */
	labelX: number;
	labelY: number;
	/** it runs back against the columns (a loop through a feedback) */
	back: boolean;
}

export interface GraphLayout {
	nodes: PlacedNode[];
	cables: PlacedCable[];
	width: number;
	height: number;
}

const portRows = (n: GraphNode) => Math.max(n.inputs.length, n.outputs.length, 1);

/** The wiring placed: a column per depth, the nodes of a column in the scene's order. */
export function layoutGraph(g: GraphIn): GraphLayout {
	const wired = new Set<string>();
	for (const l of g.links) wired.add(`${l.from}>${l.out}`).add(`${l.to}<${l.input}`);
	const columnY = new Map<number, number>();
	const nodes = g.nodes.map((n, i): PlacedNode => {
		const x = NODE.margin + n.depth * (NODE.width + NODE.gapX);
		const y = columnY.get(n.depth) ?? NODE.margin;
		const h = NODE.head + portRows(n) * NODE.row;
		columnY.set(n.depth, y + h + NODE.gapY);
		const row = (k: number) => y + NODE.head + (k + 0.5) * NODE.row;
		return {
			id: n.id,
			kind: n.kind,
			track: n.track,
			played: n.played,
			x,
			y,
			w: NODE.width,
			h,
			inputs: n.inputs.map((p, k) => ({ name: p.name, type: p.type, x, y: row(k), wired: wired.has(`${i}<${p.name}`) })),
			outputs: n.outputs.map((p, k) => ({ name: p.name, type: p.type, x: x + NODE.width, y: row(k), wired: wired.has(`${i}>${p.name}`) }))
		};
	});
	const cables: PlacedCable[] = [];
	g.links.forEach((l, index) => {
		const a = nodes[l.from]?.outputs.find((p) => p.name === l.out);
		const b = nodes[l.to]?.inputs.find((p) => p.name === l.input);
		if (!a || !b) return;
		const back = b.x <= a.x;
		// a forward cable bends half the way; one running back loops out past both boxes
		const bend = back ? NODE.gapX : Math.max((b.x - a.x) / 2, NODE.gapX / 2);
		const d = `M ${a.x} ${a.y} C ${a.x + bend} ${a.y}, ${b.x - bend} ${b.y}, ${b.x} ${b.y}`;
		cables.push({ index, type: l.type, d, labelX: (a.x + b.x) / 2, labelY: (a.y + b.y) / 2, back });
	});
	const width = Math.max(0, ...nodes.map((n) => n.x + n.w)) + NODE.margin;
	const height = Math.max(0, ...columnY.values()) - NODE.gapY + NODE.margin;
	return { nodes, cables, width, height };
}

/** A cable's value as its label: '–' when there is none (no sound yet, a node the browser does not play), else about
 *  three significant digits (an audio cable's value is its block's peak – a CV cable's pitch in Hz). */
export function cableText(v: number): string {
	if (!Number.isFinite(v)) return '–';
	const a = Math.abs(v);
	return a >= 100 ? v.toFixed(0) : a >= 10 ? v.toFixed(1) : v.toFixed(2);
}

/** the labels of every link, in the links' order (`read` gives a path's value) */
export function cableTexts(links: readonly GraphLink[], read: (path: string) => number): string[] {
	return links.map((l) => cableText(read(l.value)));
}

/** whether two label lists are the same (redraw only on a change) */
export function sameTexts(a: readonly string[], b: readonly string[]): boolean {
	return a.length === b.length && a.every((t, i) => t === b[i]);
}
