<script lang="ts">
	import { type Group } from '@scutoons/content';
	import { hasVisualNovel, Renderer } from '@scutoons/content';
	import { setGroupScope } from '$lib/visual-novel/group-scope.ts';
	import VisualNovel from './interactions/visual-novel.svelte';
	import Horizontal from './renderer/horizontal.svelte';
	import Strip from './renderer/strip.svelte';
	import Vertical from './renderer/vertical.svelte';

	let {
		group,
		sliceOffset = 0,
		scope = '',
		deferScript = false,
		required = true,
		done = false
	}: {
		group: Group;
		/**
		 * the page index of the group's first slice: linked groups (a chain, @scutoons/content chains.ts) share one page,
		 * their `data-slice-index` runs on across it
		 */
		sliceOffset?: number;
		/** the group on the page (its index in the chapter): its text boxes, its script in the page's line */
		scope?: string;
		/** in linked groups: the script starts when the group's first slice comes into view, not on mount */
		deferScript?: boolean;
		/** the follow-up bar waits for its script (the groups from the shown one on) */
		required?: boolean;
		/** above the `?label=` target of linked groups: read already – no script, no triggers, text boxes shown (owner 2026-10-06) */
		done?: boolean;
	} = $props();
	// a VN script or slice triggers (e.g. a cucumber offer on a plain vertical group) need the VN
	const vn = $derived(hasVisualNovel(group));
	/** the group's elements (display: contents – no box of its own, the renderers keep their layout) */
	let root = $state<HTMLElement>();
	// its slices find their group: a slice event's divert goes into this script, its text boxes are this group's
	setGroupScope(() => ({ events: group.events, label: group.label ?? null, scope, done }));
</script>

<div class="group" bind:this={root}>
	{#if vn}
		<VisualNovel {group} {root} {scope} {sliceOffset} deferred={deferScript} {required} {done} />
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
