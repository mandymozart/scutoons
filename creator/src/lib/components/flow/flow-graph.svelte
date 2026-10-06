<script lang="ts">
	// Loaded only when the Flow panel opens (flow-panel.svelte imports it dynamically): Svelte Flow + dagre stay out of
	// the pages that never show it.
	import '@xyflow/svelte/dist/style.css';
	import { SvelteFlow, MiniMap, MarkerType, type Edge, type Node, type NodeTypes } from '@xyflow/svelte';
	import dagre from '@dagrejs/dagre';
	import { chapterFlow, type ChapterReach, type FlowEdge, type FlowNode, type JumpToTarget } from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';
	import FlowBox, { type FlowNodeData } from '$lib/components/flow/flow-node.svelte';
	import FlowControls from '$lib/components/flow/flow-controls.svelte';

	let {
		groups,
		reach,
		chapter,
		current = -1,
		name,
		text,
		onopen
	}: {
		groups: readonly any[];
		reach: ChapterReach;
		chapter: number;
		current?: number;
		name: (group: any, index: number) => string;
		text: (value: unknown) => string;
		onopen: (node: FlowNode) => void;
	} = $props();

	const nodeTypes: NodeTypes = { flow: FlowBox };

	// ── texts ───────────────────────────────────────────────────────────────────────────────────────────────────
	const triggerWord = (t: string | undefined) =>
		t === 'click' ? m['creator.trigger_click']() : t === 'hover' ? m['creator.trigger_hover']() : t === 'intersect' ? m['creator.trigger_intersect']() : t === 'item' ? m['creator.flow_item_tap']() : (t ?? '');
	function targetText(t: JumpToTarget | undefined): string {
		if (!t) return '';
		const value = t.value;
		const head =
			t.type === 'chapter'
				? m['creator.flow_chapter']({ value })
				: t.type === 'season'
					? m['creator.flow_season']({ value })
					: t.type === 'toon'
						? m['creator.flow_toon']({ value })
						: t.type === 'product'
							? m['creator.flow_product']({ value })
							: m['creator.flow_xr']({ value });
		if (t.anyOf?.length) return `${head} · ${m['creator.vn_random_one']({ list: t.anyOf.join(', ') })}`;
		return t.label ? `${head} · ${t.label}` : head;
	}
	/** a jump within this chapter: just the group(s) it leads to */
	function hereText(t: JumpToTarget | undefined): string | null {
		if (!t || t.type !== 'chapter' || Number(t.value) !== chapter) return null;
		if (t.anyOf?.length) return m['creator.vn_random_one']({ list: t.anyOf.join(', ') });
		return t.label ?? m['creator.vn_group_first']();
	}
	function boxText(n: FlowNode): { title: string; meta?: string } {
		switch (n.kind) {
			case 'group': {
				const parts = [m['creator.flow_slices']({ count: n.count ?? 0 })];
				if (n.script) parts.push(m['creator.script']());
				if (n.deadSlices?.length) parts.push(m['creator.flow_dead_slices']({ list: n.deadSlices.map((k) => k + 1).join(', ') }));
				return { title: n.name ?? '', meta: parts.join(' · ') };
			}
			case 'label':
				return { title: `${m['creator.vn_point']()} ${n.name ?? ''}` };
			case 'choices':
				return { title: m['creator.flow_question']({ count: n.count ?? 0 }) };
			case 'divert':
				return { title: m['creator.flow_divert']({ name: n.name ?? '' }) };
			case 'jump':
				return { title: `${m['creator.flow_jump']()} → ${hereText(n.target) ?? targetText(n.target)}` };
			case 'trigger':
				return { title: `${m['creator.flow_slice']({ number: (n.slice ?? 0) + 1 })} · ${triggerWord(n.trigger)}` };
			case 'exit':
				return { title: n.next ? `→ ${m['creator.flow_next_chapter']()}` : `→ ${targetText(n.target)}` };
		}
	}
	const chipOf = (n: FlowNode) =>
		n.status === 'unreachable' && n.kind !== 'exit'
			? m['creator.reach_unreachable']()
			: n.status === 'never'
				? m['creator.reach_never']()
				: n.status === 'loop'
					? m['creator.reach_loop']()
					: undefined;
	const hintOf = (n: FlowNode) =>
		n.status === 'unreachable'
			? m['creator.reach_unreachable_hint']()
			: n.status === 'never'
				? m['creator.reach_never_hint']()
				: n.status === 'loop'
					? m['creator.reach_loop_hint']()
					: n.status === 'conditional'
						? m['creator.flow_conditional']()
						: undefined;
	const clip = (s: string, n = 28) => (s.length > n ? `${s.slice(0, n - 1)}…` : s);
	function edgeText(e: FlowEdge): string {
		const parts: string[] = [];
		if (e.label) parts.push(clip(e.label));
		if (e.branch) parts.push(e.branch === 'else' ? m['creator.flow_else']() : e.branch === 'none' ? m['creator.flow_none']() : m['creator.flow_auto']());
		if (e.condition && e.branch !== 'else') parts.push(m['creator.vn_if_short']({ test: e.condition }));
		return parts.join(' · ');
	}

	// ── layout (dagre, top to bottom); sizes in rem, converted once for the layout ────────────────────────────────
	/** a box's size in rem (the box's CSS gets the same numbers) */
	const sizeOf = (n: FlowNode, d: FlowNodeData) => ({ w: n.kind === 'group' ? 13 : 12, h: (n.kind === 'group' ? 3.25 : 2) + (n.kind !== 'group' && d.chip ? 1.25 : 0) });
	const rem = () => parseFloat(getComputedStyle(document.documentElement).fontSize) || 16;

	let nodes = $state.raw<Node<FlowNodeData>[]>([]);
	let edges = $state.raw<Edge[]>([]);

	$effect(() => {
		const flow = chapterFlow(groups, { chapter, reach, name, text });
		const px = rem();
		const g = new dagre.graphlib.Graph({ multigraph: true });
		g.setGraph({ rankdir: 'TB', nodesep: px * 1.25, ranksep: px * 2.25, edgesep: px * 0.75, marginx: px, marginy: px });
		g.setDefaultEdgeLabel(() => ({}));
		const data = new Map<string, FlowNodeData>();
		for (const n of flow.nodes) {
			const t = boxText(n);
			const d: FlowNodeData = { flow: n, ...t, chip: chipOf(n), hint: hintOf(n), current: n.kind === 'group' && n.group === current };
			data.set(n.id, d);
			const s = sizeOf(n, d);
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
			const s = sizeOf(n, data.get(n.id)!);
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
			class: `flow-edge kind-${e.kind} from-${e.status}${e.never ? ' is-never' : ''}${e.condition ? ' is-conditional' : ''}`,
			markerEnd: { type: MarkerType.ArrowClosed, width: 16, height: 16, color: e.never || e.status === 'unreachable' ? 'var(--badge-warning)' : 'var(--ink)' }, // px-ok: the marker's own units
			deletable: false,
			selectable: false
		}));
	});

	const miniColor = (n: Node) => {
		const f = (n.data as FlowNodeData).flow;
		if (f.status === 'unreachable' || f.status === 'never' || f.status === 'loop') return 'var(--badge-warning)';
		return f.kind === 'group' ? 'var(--ink)' : 'var(--border-muted)';
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
			const f = (node.data as FlowNodeData).flow;
			if (f.path) onopen(f);
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
	.graph :global(.flow-edge.kind-continues path.svelte-flow__edge-path) {
		stroke-width: 3;
	}
	.graph :global(.flow-edge.kind-trigger path.svelte-flow__edge-path) {
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
