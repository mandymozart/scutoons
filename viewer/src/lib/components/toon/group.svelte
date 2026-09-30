<script lang="ts">
	import { type Group } from '@scutoons/content';
	import { Renderer } from '@scutoons/content';
	import VisualNovel from './interactions/visual-novel.svelte';
	import Horizontal from './renderer/horizontal.svelte';
	import Vertical from './renderer/vertical.svelte';

	let { group }: { group: Group } = $props();
	// slice triggers (e.g. a cucumber offer on a plain vertical group) need the VN box too
	const hasSliceEvents = $derived(group.slices?.some((slice) => slice.events?.length) ?? false);
</script>

{#if group.events || hasSliceEvents}
	<VisualNovel {group} />
{/if}

{#if group.renderer?.type === Renderer.VERTICAL}
	<Vertical {group} />
{:else if group.renderer?.type === Renderer.HORIZONTAL}
	<Horizontal {group} />
{:else}
	Renderer is missing
{/if}
