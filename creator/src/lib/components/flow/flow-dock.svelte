<script lang="ts" module>
	import type { ComponentProps } from 'svelte';
	import type FlowPanel from '$lib/components/flow/flow-panel.svelte';

	/** what a page hands the dock's Flow: its level and that level's input (UI-CONCEPTS E.3 b) */
	export type FlowInput =
		| { level: 'toon'; toon: string; season?: number }
		| ({ level: 'chapter' } & Omit<ComponentProps<typeof FlowPanel>, 'zoom' | 'onzoom'>);

	let mounts = 0;
</script>

<script lang="ts">
	import { CloseButton, DockFold } from '@scutoons/ui';
	import { m } from '$lib/paraglide/messages.js';
	import FlowPanelView from '$lib/components/flow/flow-panel.svelte';
	import ToonFlowPanel from '$lib/components/flow/toon-flow-panel.svelte';

	/**
	 * The Flow in the shell's right dock – ONE panel for every level (UI-CONCEPTS E.3 b, agents/DOCKING.md "Panels across
	 * pages"): the toon page, a season and the chapter editor dock it with `kind: 'flow'`, so moving toon → season →
	 * chapter (a box, the sidebar, a crumb) keeps this instance mounted – the header, the width, the fold stay, and the
	 * zoom carries over into the next level's graph (a new level is rebuilt around what's open, at the kept zoom; a new
	 * group / slice on the same level only updates the graph). The Flow only reads the current place; it navigates only
	 * when a box is clicked.
	 */
	let { input, onclose }: { input: FlowInput; onclose: () => void } = $props();

	/** the zoom the next graph starts with (undefined: 1:1, the panel's first graph) */
	let zoom = $state<number | undefined>(undefined);
	const onzoom = (z: number) => (zoom = z);
	/** counts the mounts (the headless check: the same panel across pages) */
	const mount = ++mounts;
</script>

<section class="flow-dock" data-testid="flow-dock" data-mount={mount}>
	<!-- the Ask panel's header (owner 2026-10-06): one control row, the title fills, edge to edge -->
	<header>
		<h2>{m['creator.flow']()}</h2>
		<DockFold label={m['creator.fold_panel']()} maximizeLabel={m['creator.maximize_panel']()} restoreLabel={m['creator.restore_panel']()} />
		<CloseButton label={m['creator.close']()} onclick={onclose} />
	</header>
	{#if input.level === 'chapter'}
		{@const { level: _, ...chapter } = input}
		{#key chapter.toon}
			<FlowPanelView {...chapter} {zoom} {onzoom} />
		{/key}
	{:else}
		{#key input.toon}
			<ToonFlowPanel toon={input.toon} season={input.season} {zoom} {onzoom} />
		{/key}
	{/if}
</section>

<style>
	/* fills the dock edge to edge (it pans / zooms itself) */
	.flow-dock {
		display: flex;
		flex-direction: column;
		box-sizing: border-box;
		height: 100%;
		min-height: 0;
		overflow: hidden;
	}
	header {
		display: flex;
		align-items: center;
		gap: var(--space-05);
		padding: var(--space-05) var(--space-1);
		border-bottom: var(--border-width-regular) solid var(--ink);
	}
	h2 {
		margin: 0;
		flex: 1;
	}
</style>
