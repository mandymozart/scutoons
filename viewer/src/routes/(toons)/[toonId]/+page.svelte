<script lang="ts">
	import type { PageProps } from './$types.js';
	import { asset } from '$lib/utils/cdn.ts';
	import ChaptersList from '$lib/components/toon/chapters-list.svelte';
	import StartButton from '$lib/components/toon/start-button.svelte';
	import { CreditsColophon } from '@scutoons/reader';
	import { m } from '$lib/paraglide/messages.js';
	import { localizeHref } from '$lib/paraglide/runtime.js';

	let { data, params }: PageProps = $props();
	let toon = $state(data.toon);
	let season = $state(data.season);
	let sequence = $state(data.sequence);

	const seasonNumber = $derived(season?.number ?? 1);

	$effect(()=>{
		toon = data.toon
		season = data.season
		sequence = data.sequence
	})
</script>

<div class="view toon-overview">
	<div class="toon-header">
		<img src={asset(`${params.toonId}/${toon.cover.square}`)} alt="Cover" class="Cover" />
		<h1>{toon.title}</h1>
		<p class="description">{toon.description}</p>
	</div>

	<section class="chapters-section">
		{#if season}
			<div class="chapters-header">
				<h2>{m['chapter.season_title']({ number: seasonNumber, title: season.title })}</h2>
				<p class="season-description">{season.description}</p>
				<!-- one start button: first chapter / continue / next / again, from the reading progress -->
				<StartButton toonId={params.toonId} season={seasonNumber} chapters={sequence} />
			</div>
			<!-- Chapter List -->
			<ChaptersList items={sequence} toonId={params.toonId} />
			{#if season.download?.music}
				<!-- the season's music (members: the download page) -->
				<div class="season-download" data-testid="season-download">
					<a class="btn btn-secondary" href={localizeHref(`/${params.toonId}/download`)}>{m['season.download']()}</a>
				</div>
			{/if}
		{/if}
	</section>

	<!-- the credits as fine print (+ the season's supporters) -->
	{#await data.credits then credits}
		{#if credits.lead.length || credits.sections.length || season?.supporters?.length}
			<section class="credits-section">
				<CreditsColophon {credits} supporters={season?.supporters} imageUrl={(path) => asset(`${params.toonId}/${path}`)} />
			</section>
		{/if}
	{/await}
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
	.chapters-header :global(.start-button) {
		margin-top: var(--space-1);
	}
	.season-download {
		padding: var(--space-15) 2rem 0;
		text-align: center;
	}
	.season-download .btn {
		display: inline-block;
		text-decoration: none;
	}
	@media (max-width: 768px) {
		.chapters-header,
		.season-download {
			padding-left: 1rem;
			padding-right: 1rem;
		}
	}

	.credits-section {
		margin: var(--space-3) auto;
	}
</style>
