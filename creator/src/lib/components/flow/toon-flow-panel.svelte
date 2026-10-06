<script lang="ts">
	import { CloseIcon } from '@scutoons/ui';
	import { toonFlow, type DeadPlace, type FlowNode } from '@scutoons/content';
	import { goto } from '$app/navigation';
	import { m } from '$lib/paraglide/messages.js';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { loadToonFlow, type ToonFlowData } from '$lib/editor/toon-flow.ts';
	import FlowDead from '$lib/components/flow/flow-dead.svelte';
	import type { FlowTextContext } from '$lib/components/flow/flow-texts.ts';
	import { FlowPictures } from '$lib/components/flow/flow-pictures.svelte.ts';

	/**
	 * The season's / toon's flow (owner 2026-10-06, agents/DESIGN.md "Flow panel"): one box per chapter (its status, its
	 * reach in the toon, unreachable groups), the season order, jumps and ends between chapters, exits to other seasons /
	 * toons / products; the toon level adds the map places. Dead things below the graph. A box opens its chapter (with its
	 * Flow), an exit its season / toon. Read-only; the graph is loaded only when it opens. The page docks it on the right
	 * like the chapter's Flow panel (the season: MetaPage's dock, the toon page: a snippet) – in the shell's right dock
	 * (usePageDock), left of the Ask panel, pushing the page, dragging wider.
	 */
	let { toon, season, onclose }: { toon: string; season?: number; onclose: () => void } = $props();

	const graph = import('$lib/components/flow/flow-graph.svelte');
	let data = $state.raw<ToonFlowData | null>(null);
	let error = $state<string | null>(null);
	$effect(() => {
		const id = toon;
		data = null;
		loadToonFlow(id, getLocale())
			.then((d) => (data = d))
			.catch((e) => (error = String(e?.message ?? e)));
	});
	const flow = $derived(data ? toonFlow(data.input, season !== undefined ? { season } : {}) : null);
	const status = (state: string) =>
		state === 'published' ? m['creator.status_published']() : state === 'coming-soon' ? m['creator.status_coming_soon']() : state === 'draft' ? m['creator.status_draft']() : state;
	const ctx: FlowTextContext = { status };
	/** the boxes' pictures: chapters' thumbnails, other chapters / toons (the flow index) */
	// svelte-ignore state_referenced_locally
	const pictures = new FlowPictures(toon);
	const link = (n: FlowNode) => n.kind === 'chapter' || n.kind === 'place' || (n.kind === 'exit' && !!n.target && ['chapter', 'season', 'toon'].includes(n.target.type));
	function open(n: FlowNode) {
		if (n.kind === 'chapter' || n.kind === 'place') return goto(`/${toon}/chapter/${n.chapter}?flow=1`);
		const t = n.target;
		if (!t) return;
		if (t.type === 'chapter') return goto(`/${t.toon || toon}/chapter/${t.value}?flow=1`);
		if (t.type === 'season') return goto(`/${toon}/season/${t.value}?flow=1`);
		if (t.type === 'toon') return goto(`/${t.value}?flow=1`);
	}
	const openDead = (at: DeadPlace) => goto(`/${toon}/chapter/${at.chapter}?flow=1`);
</script>

<section class="flow" data-testid="toon-flow" data-level={season !== undefined ? 'season' : 'toon'}>
	<header>
		<h2>{m['creator.flow']()}</h2>
		<button class="ghost" onclick={onclose} aria-label={m['creator.close']()} title={m['creator.close']()}><CloseIcon /></button>
	</header>
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
	<FlowDead dead={data?.dead} onopen={openDead} />
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
		margin-top: var(--space-05);
		position: relative;
		border: var(--border-width-regular) solid var(--ink);
	}
</style>
