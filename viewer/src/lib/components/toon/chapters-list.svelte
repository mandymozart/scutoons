<script lang="ts">
	import ChapterItem from './chapter-item.svelte';
	import type { Chapter } from '$lib/models/content.ts';
	import type { LoadedSequenceItem } from '$lib/models/content.ts';

	let { items, toonId }: { items: LoadedSequenceItem[]; toonId: string } = $props();
</script>

<div class="chapters-list">
	{#each items as item}
			<ChapterItem chapter={item as Chapter} chapterNumber={item.number} {toonId} />
	{/each}
</div>

<style>
	.chapters-list {
		overflow-y: auto;
		overflow-x: hidden;
	}
</style>
