<script lang="ts" module>
	import type { FlowNode } from '@scutoons/content';

	/** what a box of the Flow panel shows (flow-graph.svelte formats it) */
	export interface FlowNodeData extends Record<string, unknown> {
		flow: FlowNode;
		title: string;
		meta?: string;
		/** the warning chip: unreachable / never true / endless loop */
		chip?: string;
		hint?: string;
		/** the group open in the editor */
		current?: boolean;
	}
</script>

<script lang="ts">
	import { Handle, Position, type NodeProps, type Node } from '@xyflow/svelte';

	let { data }: NodeProps<Node<FlowNodeData>> = $props();
</script>

<Handle type="target" position={Position.Top} isConnectable={false} />
<div
	class="box kind-{data.flow.kind} is-{data.flow.status}"
	class:current={data.current}
	class:link={!!data.flow.path}
	title={data.hint ? `${data.title} – ${data.hint}` : data.title}
	data-testid="flow-node-{data.flow.id}"
>
	<span class="title">{data.title}</span>
	{#if data.meta || data.chip}
		<span class="meta">
			{#if data.chip}<span class="sc-chip is-warning">{data.chip}</span>{/if}
			{#if data.meta}<span class="text">{data.meta}</span>{/if}
		</span>
	{/if}
</div>
<Handle type="source" position={Position.Bottom} isConnectable={false} />

<style>
	/* DESIGN.md "Brutalist tools": ink on paper, 2px frames; a group is a strong box, a script point a lighter one, an
	   exit a pill; only through a condition = dashed; unreachable / never / loop = the warning colour (like the chips) */
	.box {
		box-sizing: border-box;
		width: 100%;
		height: 100%;
		display: flex;
		flex-direction: column;
		justify-content: center;
		gap: var(--space-025);
		padding: 0 var(--space-05);
		background: var(--paper);
		color: var(--ink);
		border: var(--border-width-regular) solid var(--ink);
		overflow: hidden;
		font-size: var(--type-caption-size);
		line-height: 1.25;
	}
	.link {
		cursor: pointer;
	}
	.title,
	.text {
		white-space: nowrap;
		overflow: hidden;
		text-overflow: ellipsis;
	}
	.kind-group .title {
		font-size: 1rem;
	}
	.meta {
		display: flex;
		align-items: center;
		gap: var(--space-025);
		min-width: 0;
		color: var(--text-muted);
	}
	.meta .sc-chip {
		flex: none;
	}
	.kind-label,
	.kind-choices,
	.kind-divert,
	.kind-jump,
	.kind-trigger {
		border-width: var(--border-width-hairline);
	}
	.kind-exit {
		border-radius: calc(infinity * 1rem);
		align-items: center;
		text-align: center;
	}
	.is-conditional {
		border-style: dashed;
	}
	.is-unreachable,
	.is-never,
	.is-loop {
		border-color: var(--badge-warning);
		border-width: var(--border-width-regular);
		border-style: solid;
	}
	.is-unreachable .title {
		color: var(--text-muted);
	}
	/* the group open in the editor: inverted (selected = inverted) */
	.current {
		background: var(--ink);
		color: var(--paper);
	}
	.current .meta {
		color: var(--paper);
	}
	.link:hover {
		outline: var(--border-width-focus) solid var(--ink);
	}
</style>
