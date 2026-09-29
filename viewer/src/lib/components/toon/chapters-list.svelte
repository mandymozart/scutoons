<script lang="ts">
	import ChapterItem from './chapter-item.svelte';
	import type { LoadedSequenceItem } from '@scutoons/content';

	let { items, toonId }: { items: LoadedSequenceItem[]; toonId: string } = $props();
</script>

<div class="chapters-list">
	{#each items as item}
			<ChapterItem chapter={item} chapterNumber={item.number} {toonId} />
	{/each}
</div>

<style>
	.chapters-list {
		overflow-y: auto;
		overflow-x: hidden;
	}
</style>
