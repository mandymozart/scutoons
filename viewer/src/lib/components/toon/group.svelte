<script lang="ts">
	import { type Group, type RefLink } from '$lib/models/content.ts';
	import { Interaction, Renderer } from '$lib/models/enums.ts';
	import Horizontal from './renderer/horizontal.svelte';
	import Vertical from './renderer/vertical.svelte';

	// GroupRef is never called, but type wise easier to not create a third type
	let { group }: { group: Group } = $props();
	
	// assign renderer
	$inspect(group.renderer, group.interaction);
</script>


{#if group.renderer?.type === Renderer.VERTICAL}
	<Vertical group={group} />
{:else if group.renderer?.type === Renderer.HORIZONTAL}
	<Horizontal group={group} />
{:else}
	Renderer is missing
{/if}
