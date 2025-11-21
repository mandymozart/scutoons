<script lang="ts">
	import { type Group } from '$lib/models/content.ts';
	import { Interaction, Renderer } from '$lib/models/enums.ts';
	import VisualNovel from './interactions/visual-novel.svelte';
	import Webtoon from './interactions/webtoon.svelte';
	import Horizontal from './renderer/horizontal.svelte';
	import Vertical from './renderer/vertical.svelte';

	let { group }: { group: Group } = $props();

	function onComplete() {}
</script>

{#if group.interaction === Interaction.VISUAL_NOVEL}
	<VisualNovel {group} {onComplete} />
{:else if group.interaction === Interaction.WEBTOON}
	<Webtoon {group} {onComplete} />
{:else}
	Interaction is missing
{/if}

{#if group.renderer?.type === Renderer.VERTICAL}
	<Vertical {group} />
{:else if group.renderer?.type === Renderer.HORIZONTAL}
	<Horizontal {group} />
{:else}
	Renderer is missing
{/if}
