<script lang="ts">
	import ChapterItem from './chapter-item.svelte';
	import SongItem from './song-item.svelte';
	import type { Chapter, Song } from '$lib/models/content.ts';
	import type { LoadedSequenceItem } from '$lib/models/content.ts';

	let { items, toonId }: { items: LoadedSequenceItem[]; toonId: string } = $props();
</script>

<div class="chapters-list">
	{#each items as item}
		{#if item.type === 'song'}
			<SongItem song={item as Song} songNumber={item.number} {toonId} />
		{:else}
			<ChapterItem chapter={item as Chapter} chapterNumber={item.number} {toonId} />
		{/if}
	{/each}
</div>

<style>
	/* Toon View - Chapter Section */
	.chapters-list {
		overflow-y: auto;
		overflow-x: hidden;
	}
</style>
