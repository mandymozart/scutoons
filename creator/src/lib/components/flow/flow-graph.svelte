<script lang="ts">
	// Loaded only when a Flow panel / page opens (flow-panel.svelte, flow-dock.svelte import it dynamically): Svelte Flow +
	// dagre stay out of the pages that never show it. Draws any level's graph (@scutoons/content flow.ts) the same way.
	import '@xyflow/svelte/dist/style.css';
	import { SvelteFlow, MiniMap, MarkerType, type Edge, type EdgeTypes, type Node, type NodeTypes } from '@xyflow/svelte';
	import dagre from '@dagrejs/dagre';
	import type { FlowEdge, FlowNode } from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';
	import FlowBox, { type FlowNodeData } from '$lib/components/flow/flow-node.svelte';
	import FlowControls from '$lib/components/flow/flow-controls.svelte';
	import FlowLine, { type FlowEdgeData } from '$lib/components/flow/flow-edge.svelte';
	import { boxSize, boxText, chipOf, edgeText, hintOf, type FlowTextContext } from '$lib/components/flow/flow-texts.ts';
	import { lazyPictures } from '$lib/components/flow/flow-pic.svelte';
	import { setFlowPictures, type FlowPictures } from '$lib/components/flow/flow-pictures.svelte.ts';

	let {
		flow,
		ctx = {},
		current = () => false,
		link = (n: FlowNode) => !!n.path,
		pictures,
		onopen
	}: {
		flow: { nodes: FlowNode[]; edges: FlowEdge[] };
		ctx?: FlowTextContext;
		/** the box of what's open in the editor (inverted) */
		current?: (n: FlowNode) => boolean;
		/** a click on the box opens something */
		link?: (n: FlowNode) => boolean;
		/** the boxes' pictures (images in the Flow): resolved in the boxes, loaded when a box comes near the view */
		pictures?: FlowPictures;
		onopen: (node: FlowNode) => void;
	} = $props();

	// svelte-ignore state_referenced_locally
	setFlowPictures(pictures);
	let canvas = $state<HTMLElement | null>(null);
	lazyPictures(() => canvas);
	/** zoomed far out: faces and pictures step back (the boxes keep their size) */
	let small = $state(false);
	const SMALL_ZOOM = 0.6;

	const nodeTypes: NodeTypes = { flow: FlowBox };
	const edgeTypes: EdgeTypes = { flow: FlowLine };
	const rem = () => parseFloat(getComputedStyle(document.documentElement).fontSize) || 16;

	let nodes = $state.raw<Node<FlowNodeData>[]>([]);
	let edges = $state.raw<Edge<FlowEdgeData>[]>([]);
	/** the box of what's open: the first view centres it */
	let focus = $state<string | undefined>(undefined);
	/** where reading starts (the first box that isn't drawn for context): the first view shows it when nothing is open */
	let entry = $state<string | undefined>(undefined);

	// layout: dagre, top to bottom; sizes in rem, converted once for the layout
	$effect(() => {
		const px = rem();
		const g = new dagre.graphlib.Graph({ multigraph: true });
		g.setGraph({ rankdir: 'TB', nodesep: px * 1.25, ranksep: px * 2.25, edgesep: px * 0.75, marginx: px, marginy: px });
		g.setDefaultEdgeLabel(() => ({}));
		const data = new Map<string, FlowNodeData>();
		for (const n of flow.nodes) {
			const d: FlowNodeData = { flow: n, ...boxText(n, ctx), chip: chipOf(n), hint: hintOf(n), current: current(n), link: link(n) };
			data.set(n.id, d);
			const s = boxSize(n, d);
			g.setNode(n.id, { width: s.w * px, height: s.h * px });
		}
		const labels = new Map<string, string>();
		for (const e of flow.edges) {
			const label = edgeText(e);
			labels.set(e.id, label);
			// a label is laid out like a box (dagre keeps it clear of the others): caption type ≈ 0.45rem per character
			g.setEdge(e.from, e.to, label ? { width: (label.length * 0.45 + 0.75) * px, height: px * 1.25, labelpos: 'c' } : {}, e.id);
		}
		dagre.layout(g);
		nodes = flow.nodes.map((n) => {
			const p = g.node(n.id);
			const s = boxSize(n, data.get(n.id)!);
			return {
				id: n.id,
				type: 'flow',
				position: { x: p.x - (s.w * px) / 2, y: p.y - (s.h * px) / 2 },
				data: data.get(n.id)!,
				style: `width: ${s.w}rem; height: ${s.h}rem;`,
				draggable: false,
				connectable: false,
				deletable: false
			};
		});
		focus = flow.nodes.find((n) => data.get(n.id)?.current)?.id;
		entry = flow.nodes.find((n) => !n.stub && n.kind !== 'exit')?.id;
		edges = flow.edges.map((e) => {
			const route = g.edge({ v: e.from, w: e.to, name: e.id }) as { points?: { x: number; y: number }[]; x?: number; y?: number } | undefined;
			const label = labels.get(e.id) || undefined;
			return {
			id: e.id,
			type: 'flow',
			source: e.from,
			target: e.to,
			label,
			data: { bends: (route?.points ?? []).slice(1, -1), ...(label && route?.x !== undefined && route.y !== undefined ? { labelAt: { x: route.x, y: route.y } } : {}) },
			class: `flow-edge kind-${e.kind} from-${e.status}${e.never ? ' is-never' : ''}${e.condition || e.status === 'conditional' ? ' is-conditional' : ''}`,
			markerEnd: { type: MarkerType.ArrowClosed, width: 16, height: 16, color: e.never || e.status === 'unreachable' ? 'var(--badge-warning)' : 'var(--ink)' }, // px-ok: the marker's own units
			deletable: false,
			selectable: false
			};
		});
	});

	const miniColor = (n: Node) => {
		const f = (n.data as FlowNodeData).flow;
		if (f.status === 'stuck') return 'var(--badge-error)';
		if (f.status === 'unreachable' || f.status === 'never' || f.status === 'loop') return 'var(--badge-warning)';
		return f.kind === 'group' || f.kind === 'chapter' || f.kind === 'toon' ? 'var(--ink)' : 'var(--border-muted)';
	};
	const miniSize = rem();
</script>

<div class="graph" class:flow-small={small} bind:this={canvas} data-testid="flow-graph">
	<SvelteFlow
		bind:nodes
		bind:edges
		{nodeTypes}
		{edgeTypes}
		minZoom={0.1}
		maxZoom={2}
		nodesDraggable={false}
		nodesConnectable={false}
		elementsSelectable={false}
		deleteKey={null}
		colorMode="light"
		proOptions={{ hideAttribution: true }}
		onnodeclick={({ node }) => {
			const d = node.data as FlowNodeData;
			if (d.link) onopen(d.flow);
		}}
	>
		<FlowControls {focus} {entry} onzoom={(zoom) => (small = zoom < SMALL_ZOOM)} />
		<MiniMap position="bottom-right" width={miniSize * 7.5} height={miniSize * 5.5} nodeColor={miniColor} nodeStrokeWidth={0} pannable zoomable ariaLabel={m['creator.flow']()} />
	</SvelteFlow>
</div>

<style>
	.graph {
		position: absolute;
		inset: 0;
	}
	/* Svelte Flow in the house look: paper, ink lines, the reader's caption type for edge labels */
	.graph :global(.svelte-flow) {
		--xy-background-color: var(--paper);
		--xy-edge-stroke: var(--ink);
		--xy-edge-stroke-width: 1.5;
		--xy-edge-label-background-color: var(--paper);
		--xy-edge-label-color: var(--ink);
		--xy-minimap-background-color: var(--paper);
		--xy-minimap-mask-background-color: color-mix(in srgb, var(--ink) 12%, transparent);
		--xy-node-border-radius: 0;
		font-family: inherit;
	}
	.graph :global(.svelte-flow__node-flow) {
		padding: 0;
		border: 0;
		background: transparent;
	}
	.graph :global(.svelte-flow__handle) {
		opacity: 0;
		pointer-events: none;
	}
	.graph :global(.svelte-flow__edge-label) {
		font-size: var(--type-caption-size);
		padding: 0 var(--space-025);
	}
	.graph :global(.flow-edge.is-conditional path.svelte-flow__edge-path) {
		stroke-dasharray: 6 4;
	}
	.graph :global(.flow-edge.kind-continues path.svelte-flow__edge-path),
	.graph :global(.flow-edge.kind-order path.svelte-flow__edge-path) {
		stroke-width: 3;
	}
	.graph :global(.flow-edge.kind-trigger path.svelte-flow__edge-path),
	.graph :global(.flow-edge.kind-reveal path.svelte-flow__edge-path),
	.graph :global(.flow-edge.kind-place path.svelte-flow__edge-path) {
		stroke-dasharray: 2 4;
	}
	.graph :global(.flow-edge.is-never path.svelte-flow__edge-path),
	.graph :global(.flow-edge.from-unreachable path.svelte-flow__edge-path) {
		stroke: var(--badge-warning);
	}
	.graph :global(.svelte-flow__minimap) {
		border: var(--border-width-regular) solid var(--ink);
		margin: var(--space-05);
	}
	.graph :global(.svelte-flow__panel.top.left) {
		margin: var(--space-05);
	}
</style>
