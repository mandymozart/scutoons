<script lang="ts">
	import { type Group } from '@scutoons/content';
	import { hasVisualNovel, Renderer } from '@scutoons/content';
	import VisualNovel from './interactions/visual-novel.svelte';
	import Horizontal from './renderer/horizontal.svelte';
	import Strip from './renderer/strip.svelte';
	import Vertical from './renderer/vertical.svelte';

	let {
		group,
		sliceOffset = 0,
		deferScript = false
	}: {
		group: Group;
		/**
		 * the page index of the group's first slice: linked groups (a chain, @scutoons/content chains.ts) share one page,
		 * their `data-slice-index` runs on across it
		 */
		sliceOffset?: number;
		/** a VN group further down a chain: its script starts when its first slice comes into view, not on mount */
		deferScript?: boolean;
	} = $props();
	// a VN script or slice triggers (e.g. a cucumber offer on a plain vertical group) need the VN box
	const vn = $derived(hasVisualNovel(group));
	/** the group's elements (display: contents – no box of its own, the renderers keep their layout) */
	let root = $state<HTMLElement>();
</script>

<div class="group" bind:this={root}>
	{#if vn}
		<VisualNovel {group} {root} deferred={deferScript} />
	{/if}

	{#if group.renderer?.type === Renderer.VERTICAL}
		<Vertical {group} {sliceOffset} />
	{:else if group.renderer?.type === Renderer.HORIZONTAL && group.renderer.params?.fullWidth}
		<!-- full width (owner 2026-10-06): a screen-high strip, moved sideways by the page scroll -->
		<Strip {group} {sliceOffset} />
	{:else if group.renderer?.type === Renderer.HORIZONTAL}
		<Horizontal {group} {sliceOffset} />
	{:else}
		Renderer is missing
	{/if}
</div>

<style>
	.group {
		display: contents;
	}
</style>
