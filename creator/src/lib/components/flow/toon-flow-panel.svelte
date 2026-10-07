<script lang="ts">
	import { shellDock } from '@scutoons/ui';
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
	 * Flow), an exit its season / toon. Read-only; the graph is loaded only when it opens. Shown in the dock's `FlowDock`
	 * (the same panel as the chapter's: toon → season → chapter keep it mounted – UI-CONCEPTS E.3 b); a box navigates
	 * the main area, the dock's Flow follows (no `?flow=1`: the open Flow is the dock's).
	 */
	let { toon, season, zoom, onzoom }: { toon: string; season?: number; zoom?: number; onzoom?: (zoom: number) => void } = $props();

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
	/** a box opens its page – a maximized Flow steps back (owner 2026-10-06) */
	function open(n: FlowNode) {
		shellDock.restore();
		if (n.kind === 'chapter' || n.kind === 'place') return goto(`/${toon}/chapter/${n.chapter}`);
		const t = n.target;
		if (!t) return;
		if (t.type === 'chapter') return goto(`/${t.toon || toon}/chapter/${t.value}`);
		if (t.type === 'season') return goto(`/${toon}/season/${t.value}`);
		if (t.type === 'toon') return goto(`/${t.value}`);
	}
	const openDead = (at: DeadPlace) => (shellDock.restore(), goto(`/${toon}/chapter/${at.chapter}`));
</script>

<section class="flow" data-testid="toon-flow" data-level={season !== undefined ? 'season' : 'toon'}>
	<div class="canvas">
		{#if error}
			<p class="hint error">{error}</p>
		{:else if !flow}
			<p class="hint">{m['creator.loading']()}</p>
		{:else}
			{#await graph}
				<p class="hint">{m['creator.loading']()}</p>
			{:then mod}
				<!-- toon ↔ season: another graph – rebuilt, the zoom kept (FlowDock) -->
				{#key season}
					<mod.default {flow} {ctx} {link} {pictures} {zoom} {onzoom} onopen={open} />
				{/key}
			{/await}
		{/if}
	</div>
	<FlowDead dead={data?.dead} onopen={openDead} />
</section>

<style>
	.flow {
		display: flex;
		flex-direction: column;
		flex: 1;
		min-height: 0;
	}
	.hint {
		font-size: var(--type-caption-size);
		color: var(--text-muted);
		margin: var(--space-05) var(--space-1);
	}
	.error {
		color: var(--error);
	}
	.canvas {
		flex: 1;
		min-height: 0;
		position: relative;
	}
</style>
