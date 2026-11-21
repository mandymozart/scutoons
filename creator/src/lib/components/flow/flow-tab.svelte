<script lang="ts" module>
	import type { ComponentProps } from 'svelte';
	import type FlowPanel from '$lib/components/flow/flow-panel.svelte';

	/** what a page hands the Flow tab: its level and that level's input */
	export type FlowInput =
		| { level: 'toon'; toon: string; season?: number }
		| ({ level: 'chapter' } & Omit<ComponentProps<typeof FlowPanel>, 'zoom' | 'onzoom'>);

	let mounts = 0;
</script>

<script lang="ts">
	import FlowPanelView from '$lib/components/flow/flow-panel.svelte';
	import ToonFlowPanel from '$lib/components/flow/toon-flow-panel.svelte';

	/**
	 * The Flow tab – one for every level: the toon board, a season and the chapter editor fill it. The zoom carries over
	 * into the next level's graph; the Flow only reads the current place and navigates only when a box is clicked.
	 */
	let { input }: { input: FlowInput } = $props();

	/** the zoom the next graph starts with (undefined: 1:1, the panel's first graph) */
	let zoom = $state<number | undefined>(undefined);
	const onzoom = (z: number) => (zoom = z);
	/** counts the mounts (the headless check: the same panel across pages) */
	const mount = ++mounts;
</script>

<section class="flow-tab" data-testid="flow-tab" data-mount={mount}>
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
	/* fills the tab edge to edge (it pans / zooms itself) */
	.flow-tab {
		display: flex;
		flex-direction: column;
		box-sizing: border-box;
		flex: 1;
		height: 100%;
		min-height: 16rem;
		overflow: hidden;
	}
</style>
