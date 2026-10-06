<script lang="ts">
	// Loaded only when a Flow panel / page opens (flow-panel.svelte, flow-dock.svelte import it dynamically): Svelte Flow +
	// dagre stay out of the pages that never show it. Draws any level's graph (@scutoons/content flow.ts) the same way.
	import '@xyflow/svelte/dist/style.css';
	import { SvelteFlow, MiniMap, MarkerType, type Edge, type Node, type NodeTypes } from '@xyflow/svelte';
	import dagre from '@dagrejs/dagre';
	import type { FlowEdge, FlowNode } from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';
	import FlowBox, { type FlowNodeData } from '$lib/components/flow/flow-node.svelte';
	import FlowControls from '$lib/components/flow/flow-controls.svelte';
	import { boxSize, boxText, chipOf, edgeText, hintOf, type FlowTextContext } from '$lib/components/flow/flow-texts.ts';

	let {
		flow,
		ctx = {},
		current = () => false,
		link = (n: FlowNode) => !!n.path,
		onopen
	}: {
		flow: { nodes: FlowNode[]; edges: FlowEdge[] };
		ctx?: FlowTextContext;
		/** the box of what's open in the editor (inverted) */
		current?: (n: FlowNode) => boolean;
		/** a click on the box opens something */
		link?: (n: FlowNode) => boolean;
		onopen: (node: FlowNode) => void;
	} = $props();

	const nodeTypes: NodeTypes = { flow: FlowBox };
	const rem = () => parseFloat(getComputedStyle(document.documentElement).fontSize) || 16;

	let nodes = $state.raw<Node<FlowNodeData>[]>([]);
	let edges = $state.raw<Edge[]>([]);

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
			g.setEdge(e.from, e.to, label ? { width: Math.min(label.length, 30) * px * 0.42, height: px * 1.1, labelpos: 'c' } : {}, e.id);
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
		edges = flow.edges.map((e) => ({
			id: e.id,
			source: e.from,
			target: e.to,
			label: labels.get(e.id) || undefined,
			class: `flow-edge kind-${e.kind} from-${e.status}${e.never ? ' is-never' : ''}${e.condition || e.status === 'conditional' ? ' is-conditional' : ''}`,
			markerEnd: { type: MarkerType.ArrowClosed, width: 16, height: 16, color: e.never || e.status === 'unreachable' ? 'var(--badge-warning)' : 'var(--ink)' }, // px-ok: the marker's own units
			deletable: false,
			selectable: false
		}));
	});

	const miniColor = (n: Node) => {
		const f = (n.data as FlowNodeData).flow;
		if (f.status === 'unreachable' || f.status === 'never' || f.status === 'loop') return 'var(--badge-warning)';
		return f.kind === 'group' || f.kind === 'chapter' || f.kind === 'toon' ? 'var(--ink)' : 'var(--border-muted)';
	};
	const miniSize = rem();
</script>

<div class="graph" data-testid="flow-graph">
	<SvelteFlow
		bind:nodes
		bind:edges
		{nodeTypes}
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
		<FlowControls />
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
