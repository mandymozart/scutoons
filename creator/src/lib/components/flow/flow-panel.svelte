<script lang="ts">
	import { CloseIcon } from '@scutoons/ui';
	import type { ChapterReach, FlowNode } from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * The chapter's story flow (owner 2026-10-06, a DRAFT of the chapter level): a read-only, zoomable graph of what leads
	 * where – @scutoons/content `chapterFlow` over the editor's own `chapterReach`, so it agrees with the "unreachable"
	 * chips. Docked on the right like Problems / History (left of the Ask panel). The graph (Svelte Flow + dagre) is
	 * loaded only when the panel opens.
	 */
	let {
		groups,
		reach,
		chapter,
		current = -1,
		name,
		text,
		onopen,
		onclose
	}: {
		/** the groups `reach` was computed from (scenes in place of their refs) */
		groups: readonly any[] | null;
		reach: ChapterReach | null;
		chapter: number;
		/** the group open in the editor (-1: none) */
		current?: number;
		name: (group: any, index: number) => string;
		text: (value: unknown) => string;
		onopen: (node: FlowNode) => void;
		onclose: () => void;
	} = $props();

	const graph = import('$lib/components/flow/flow-graph.svelte');
</script>

<section class="flow" data-testid="flow-panel">
	<header>
		<h2>{m['creator.flow']()}</h2>
		<button class="ghost" onclick={onclose} aria-label={m['creator.close']()} title={m['creator.close']()}><CloseIcon /></button>
	</header>
	<p class="hint">{m['creator.flow_hint']()}</p>
	<div class="canvas">
		{#if !groups || !reach}
			<p class="hint">{m['creator.loading']()}</p>
		{:else if !groups.length}
			<p class="hint">{m['creator.flow_empty']()}</p>
		{:else}
			{#await graph}
				<p class="hint">{m['creator.loading']()}</p>
			{:then mod}
				<mod.default {groups} {reach} {chapter} {current} {name} {text} {onopen} />
			{:catch error}
				<p class="hint error">{String(error?.message ?? error)}</p>
			{/await}
		{/if}
	</div>
</section>

<style>
	.flow {
		display: flex;
		flex-direction: column;
		height: 100%;
		min-height: 0;
	}
	header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		border-bottom: var(--border-width-regular) solid var(--ink);
	}
	h2 {
		margin: var(--space-025) 0;
	}
	.hint {
		font-size: var(--type-caption-size);
		color: var(--text-muted);
		margin: var(--space-05) 0;
	}
	.error {
		color: var(--error);
	}
	.canvas {
		flex: 1;
		min-height: 0;
		position: relative;
		border: var(--border-width-regular) solid var(--ink);
	}
</style>
