<script lang="ts">
	import { CloseIcon, panelWidth } from '@scutoons/ui';
	import { toonFlow, type DeadPlace, type FlowNode } from '@scutoons/content';
	import { goto } from '$app/navigation';
	import { m } from '$lib/paraglide/messages.js';
	import { getLocale } from '$lib/paraglide/runtime.js';
	import { loadToonFlow, type ToonFlowData } from '$lib/editor/toon-flow.ts';
	import FlowDead from '$lib/components/flow/flow-dead.svelte';
	import type { FlowTextContext } from '$lib/components/flow/flow-texts.ts';

	/**
	 * The season's / toon's flow (owner 2026-10-06, agents/DESIGN.md "Flow panel"): one box per chapter (its status, its
	 * reach in the toon, unreachable groups), the season order, jumps and ends between chapters, exits to other seasons /
	 * toons / products; the toon level adds the map places. Dead things below the graph. A box opens its chapter (with its
	 * Flow), an exit its season / toon. Docked on the right like the chapter's Flow panel (left of the Ask panel, drags
	 * wider); read-only; the graph is loaded only when it opens.
	 */
	let { toon, season, onclose }: { toon: string; season?: number; onclose: () => void } = $props();

	const size = panelWidth('sc-panel-width:flow', 28);
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
	const link = (n: FlowNode) => n.kind === 'chapter' || n.kind === 'place' || (n.kind === 'exit' && !!n.target && ['chapter', 'season', 'toon'].includes(n.target.type));
	function open(n: FlowNode) {
		if (n.kind === 'chapter' || n.kind === 'place') return goto(`/${toon}/chapter/${n.chapter}?flow=1`);
		const t = n.target;
		if (!t) return;
		if (t.type === 'chapter') return goto(`/${toon}/chapter/${t.value}?flow=1`);
		if (t.type === 'season') return goto(`/${toon}/season/${t.value}?flow=1`);
		if (t.type === 'toon') return goto(`/${t.value}?flow=1`);
	}
	const openDead = (at: DeadPlace) => goto(`/${toon}/chapter/${at.chapter}?flow=1`);
</script>

<!-- a focusable separator is a widget (WAI-ARIA window splitter): drag the panel wider -->
<!-- svelte-ignore a11y_no_noninteractive_tabindex, a11y_no_noninteractive_element_interactions -->
<div class="resize" role="separator" aria-orientation="vertical" aria-label={m['creator.resize_panel']()} title={m['creator.resize_panel']()} tabindex="0" style:--panel-width={size.css} onpointerdown={size.start} onkeydown={size.key} ondblclick={size.reset}></div>
<aside class="dock" style:--panel-width={size.css} data-testid="toon-flow" data-level={season !== undefined ? 'season' : 'toon'}>
	<header>
		<h2>{m['creator.flow']()}</h2>
		<button class="ghost" onclick={onclose} aria-label={m['creator.close']()} title={m['creator.close']()}><CloseIcon /></button>
	</header>
	<p class="hint">{m['creator.flow_hint']()}</p>
	<div class="canvas">
		{#if error}
			<p class="hint error">{error}</p>
		{:else if !flow}
			<p class="hint">{m['creator.loading']()}</p>
		{:else}
			{#await graph}
				<p class="hint">{m['creator.loading']()}</p>
			{:then mod}
				<mod.default {flow} {ctx} {link} onopen={open} />
			{/await}
		{/if}
	</div>
	<FlowDead dead={data?.dead} onopen={openDead} />
</aside>

<style>
	.dock {
		position: fixed;
		top: var(--top, 3.5rem);
		/* left of the Ask panel: Ask belongs to the shell, the page's panels to the page */
		right: var(--ask-inset, 0rem);
		bottom: var(--action-bar-height, 0rem);
		width: var(--panel-width);
		max-width: 100vw;
		background: var(--paper);
		border-left: var(--border-width-regular) solid var(--ink);
		padding: var(--space-1);
		z-index: 20;
		box-sizing: border-box;
		display: flex;
		flex-direction: column;
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
	.resize {
		position: fixed;
		z-index: 21;
		top: var(--top, 3.5rem);
		bottom: var(--action-bar-height, 0rem);
		right: calc(var(--ask-inset, 0rem) + var(--panel-width) - 0.375rem);
		width: 0.75rem;
		cursor: col-resize;
		touch-action: none;
	}
	.resize:hover,
	.resize:focus-visible {
		background: linear-gradient(var(--ink), var(--ink)) center / var(--border-width-focus) 100% no-repeat;
		outline: none;
	}
	@media (max-width: 767px) {
		.resize {
			display: none;
		}
	}
</style>
