<script lang="ts">
	import { goto } from '$app/navigation';
	import { universeFlow, type FlowIndex, type FlowNode } from '@scutoons/content';
	import { api } from '$lib/api.ts';
	import { m } from '$lib/paraglide/messages.js';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { usePageNav } from '$lib/nav.svelte.ts';
	import { pickText } from '$lib/editor/toon-flow.ts';
	import type { FlowTextContext } from '$lib/components/flow/flow-texts.ts';
	import { FlowPictures } from '$lib/components/flow/flow-pictures.svelte.ts';

	/**
	 * The SCUTOONS-wide Flow (owner 2026-10-06, agents/DESIGN.md "Flow panel"; Universe, next to the world map): one box
	 * per toon (its seasons and chapters), the jumps between toons and to products / XR – from the server's flow index
	 * (`GET /api/creator/flow-index`, so the browser doesn't load every toon). A toon's box
	 * opens its toon page with its Flow (the seasons and chapters). Read-only; the graph is loaded only here.
	 */
	usePageNav({ fill: true });
	const graph = import('$lib/components/flow/flow-graph.svelte');
	let index = $state.raw<FlowIndex | null>(null);
	let error = $state<string | null>(null);
	$effect(() => {
		api.get<FlowIndex>('/creator/flow-index')
			.then((answer) => (index = answer))
			.catch((e) => (error = String(e?.message ?? e)));
	});
	const flow = $derived(index ? universeFlow(index, { text: (v) => pickText(v, getLocale()) }) : null);
	const status = (state: string) => (state === 'published' ? m['creator.status_published']() : state === 'draft' ? m['creator.status_draft']() : state);
	const ctx: FlowTextContext = { status };
	/** the toons' avatars (from the same index) */
	const pictures = $derived(index ? new FlowPictures('', { index }) : undefined);
	const link = (n: FlowNode) => n.kind === 'toon' || (n.kind === 'exit' && n.target?.type === 'toon');
	const open = (n: FlowNode) => goto(`/${n.toon ?? n.target?.value}?flow=1`);
</script>

<section class="universe-flow" data-testid="universe-flow">
	<h1 title={m['creator.flow_universe_hint']()}>{m['creator.flow_universe']()}</h1>
	<div class="canvas">
		{#if error}
			<p class="hint error">{error}</p>
		{:else if !flow}
			<p class="hint">{m['creator.loading']()}</p>
		{:else}
			{#await graph}
				<p class="hint">{m['creator.loading']()}</p>
			{:then mod}
				<mod.default {flow} {ctx} {link} {pictures} onopen={open} />
			{/await}
		{/if}
	</div>
</section>

<style>
	.universe-flow {
		display: flex;
		flex-direction: column;
		height: 100%;
		min-height: 0;
		box-sizing: border-box;
		padding: var(--space-1);
	}
	h1 {
		margin: 0;
	}
	.hint {
		font-size: var(--type-caption-size);
		color: var(--tool-muted);
		margin: var(--space-05) 0;
	}
	.error {
		color: var(--tool-stop);
	}
	.canvas {
		flex: 1;
		min-height: 20rem;
		margin-top: var(--space-05);
		position: relative;
		border: var(--border-width-regular) solid var(--tool-text);
	}
</style>
