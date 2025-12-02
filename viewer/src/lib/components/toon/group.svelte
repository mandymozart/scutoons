<script lang="ts">
	import { page } from '$app/state';
	import { getGameContext } from '$lib/contexts/game.svelte.ts';
	import { type Group } from '$lib/models/content.ts';
	import { Renderer } from '$lib/models/enums.ts';
	import VisualNovel from './interactions/visual-novel.svelte';
	import Horizontal from './renderer/horizontal.svelte';
	import Vertical from './renderer/vertical.svelte';

	let { group }: { group: Group } = $props();
	const game = getGameContext();

	$effect(() => {
		console.log(
			game.hasCheckpoint(
				`${page.params.toonId}/chapter/${page.params.chapterNumber}${page.url.searchParams.get('label') ? `?label=${page.url.searchParams.get('label')}` : ''}`
			)
		);
	});
</script>

{#if group.events}
	<VisualNovel {group} />
{/if}

{#if group.renderer?.type === Renderer.VERTICAL}
	<Vertical {group} />
{:else if group.renderer?.type === Renderer.HORIZONTAL}
	<Horizontal {group} />
{:else}
	Renderer is missing
{/if}
