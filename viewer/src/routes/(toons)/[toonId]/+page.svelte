<script lang="ts">
	import { goto } from '$app/navigation';
	import type { PageProps } from './$types.js';
	import { asset } from '$lib/utils/cdn.ts';
	import ChaptersList from '$lib/components/toon/chapters-list.svelte';

	let { data, params }: PageProps = $props();
	let toon = $state(data.toon);
	let season = $state(data.season);
	let sequence = $state(data.sequence);

	$effect(()=>{
		toon = data.toon
		season = data.season
		sequence = data.sequence
	})

	function navigateToCredits() {
		goto(`${params.toonId}/credits`);
	}
</script>

<div class="view toon-overview">
	<div class="toon-header">
		<img src={asset(`${params.toonId}/${toon.cover.square}`)} alt="Cover" class="Cover" />
		<h1>{toon.title}</h1>
		<p class="description">{toon.description}</p>
		<!-- Add your download section here if needed -->
		<div class="toon-download">
			<button onclick={() => goto(`/${params.toonId}/download`)}>Download Music</button>
		</div>
	</div>

	<section class="chapters-section">
		{#if season}
			<div class="chapters-header">
				<h2>Season 1: {season.title}</h2>
				<p class="season-description">{season.description}</p>
			</div>
			<!-- Chapter List -->
			<ChaptersList items={sequence} toonId={params.toonId} />
		{/if}
	</section>

	<section class="credits-section">
		<button onclick={navigateToCredits}>Credits</button>
	</section>
</div>

<style>
	.toon-overview {
		max-width: 800px;
		margin: 0 auto;
	}

	.toon-header {
		text-align: center;
		margin-bottom: 2rem;
	}

	.toon-header img {
		width: 100%;
		height: auto;
	}

	.chapters-header {
		margin-bottom: 2rem;
		padding: 0 2rem;
	}
	.toon-download {
		margin-top: 2rem;
	}

	/* Toon View - Credits Section */
	.credits-section {
		max-width: 800px;
		margin: 0 auto;
		padding: 2rem;
		text-align: center;
	}
</style>
