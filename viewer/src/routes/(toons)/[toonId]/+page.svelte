<script lang="ts">
	import type { PageProps } from './$types.js';
	import { asset } from '$lib/utils/cdn.ts';
	import ChaptersList from '$lib/components/toon/chapters-list.svelte';
	import StartButton from '$lib/components/toon/start-button.svelte';
	import Hero from '$lib/components/toon/hero.svelte';
	import RelatedItems from '$lib/components/toon/related-items.svelte';
	import { albumAccess, albumByDownload, ownsAlbum } from '@scutoons/content';
	import { getInventoryContext } from '$lib/contexts/inventory.svelte.ts';
	import { onMount } from 'svelte';
	import { CoverArt, CreditsColophon } from '@scutoons/reader';
	import { m } from '$lib/paraglide/messages.js';
	import { localizeHref } from '$lib/paraglide/runtime.js';

	let { data, params }: PageProps = $props();
	let toon = $state(data.toon);
	let season = $state(data.season);

	// the season's music in its record's row (owner 2026-10-02): the album whose download is the season's zip, and the
	// related item that carries it; without such an item the separate music block stays
	const inventory = getInventoryContext();
	const musicAlbum = $derived(season?.download?.music ? albumByDownload(inventory.catalog, `${params.toonId}/${season.download.music}`) : undefined);
	const musicCarrier = $derived(
		musicAlbum ? (season?.related_items ?? []).find((id) => inventory.item(id)?.album === musicAlbum.id) : undefined
	);
	/** the download button in that row: `everyone` / `members` (the download page gates members), `owners` only for owners */
	const musicLink = $derived.by(() => {
		if (!musicCarrier || !musicAlbum || !season?.download?.music) return null;
		const owner = ownsAlbum(inventory.catalog, musicAlbum, inventory.entries.map((e) => e.item_id));
		return {
			item: musicCarrier,
			image: season.download.image ? asset(`${params.toonId}/${season.download.image}`) : null,
			href: albumAccess(musicAlbum).download === 'owners' && !owner ? null : localizeHref(`/${params.toonId}/download`)
		};
	});
	let sequence = $state(data.sequence);

	const seasonNumber = $derived(season?.number ?? 1);

	// the desktop layout (hero + 12-column grid) from 1024 px; phones and tablets keep the single column
	const DESKTOP = '(min-width: 1024px)';
	let desktop = $state(typeof window !== 'undefined' && window.matchMedia(DESKTOP).matches);
	onMount(() => {
		const query = window.matchMedia(DESKTOP);
		const onChange = () => (desktop = query.matches);
		query.addEventListener('change', onChange);
		return () => query.removeEventListener('change', onChange);
	});

	$effect(()=>{
		toon = data.toon
		season = data.season
		sequence = data.sequence
	})
</script>

<div class="view toon-overview">
	{#if desktop && season}
		<!-- desktop (≥ 1024 px): the trailer hero is the first screen (agents/DESIGN.md "Toon page desktop") -->
		<Hero toonId={params.toonId} {toon} {season} {seasonNumber} {sequence} />
	{/if}
	<div class="toon-header">
		{#if toon.cover?.square}
			<!-- one cover for every language; with cover.lettering the title is set over the text-free art in the reader's language -->
			<CoverArt
				src={asset(`${params.toonId}/${toon.cover.square}`)}
				alt="Cover"
				class="Cover"
				lettering={toon.cover.lettering === true}
				title={toon.title}
				subtitle={season?.title ?? ''}
				lang={toon.language}
			/>
		{/if}
		<h1>{toon.title}</h1>
		<p class="description">{toon.description}</p>
	</div>

	<section class="chapters-section">
		{#if season}
			<div class="season-column">
				<div class="chapters-header">
					<h2>{m['chapter.season_title']({ number: seasonNumber, title: season.title })}</h2>
					<p class="season-description">{season.description}</p>
					{#if !desktop}
						<!-- one start button: first chapter / continue / next / again, from the reading progress (desktop: in the hero) -->
						<StartButton toonId={params.toonId} season={seasonNumber} chapters={sequence} />
					{/if}
				</div>
				{#if desktop && season.download?.music && !musicLink}
					{@render music()}
				{/if}
				{#if desktop && season.related_items?.length}
					<RelatedItems ids={season.related_items} music={musicLink} />
				{/if}
				{#if desktop}
					{@render colophon()}
				{/if}
			</div>
			<!-- Chapter List -->
			<div class="list-column">
				<ChaptersList items={sequence} toonId={params.toonId} />
			</div>
			<!-- the season's related items (season.related_items): icon, name, price + "Kaufen" for physical products; the
			     season's music sits in its record's row (owner 2026-10-02) – the music block only without related items -->
			{#if !desktop && season.download?.music && !musicLink}
				{@render music()}
			{/if}
			{#if !desktop && season.related_items?.length}
				<RelatedItems ids={season.related_items} music={musicLink} />
			{/if}
		{/if}
	</section>

	{#if !desktop}
		{@render colophon()}
	{/if}
</div>

<!-- the season's music (members: the download page), with the record's cut-out beside the button when the season has one -->
{#snippet music()}
	<div class="season-download" class:with-image={!!season?.download?.image} data-testid="season-download">
		{#if season?.download?.image}
			<img
				class="season-download-image"
				src={asset(`${params.toonId}/${season.download.image}`)}
				alt=""
				loading="lazy"
				decoding="async"
				data-testid="season-download-image"
			/>
		{/if}
		<a class="btn btn-secondary" href={localizeHref(`/${params.toonId}/download`)}>{m['season.download']()}</a>
	</div>
{/snippet}

<!-- the credits as fine print (+ the season's supporters): at the bottom, on desktop in the season column -->
{#snippet colophon()}
	{#await data.credits then credits}
		{#if credits.lead.length || credits.sections.length || season?.supporters?.length}
			<section class="credits-section">
				<CreditsColophon {credits} supporters={season?.supporters} imageUrl={(path) => asset(`${params.toonId}/${path}`)} />
			</section>
		{/if}
	{/await}
{/snippet}

<style>
	.toon-overview {
		max-width: 800px;
		margin: 0 auto;
	}

	.toon-header {
		text-align: center;
		margin-bottom: 2rem;
	}

	.toon-header :global(.Cover) {
		width: 100%;
	}

	.chapters-header {
		margin-bottom: 2rem;
		padding: 0 2rem;
	}
	.chapters-header :global(.start) {
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
	/* the record beside the centred button (phone / tablet): a smaller cut-out left of it */
	.season-download.with-image {
		display: flex;
		align-items: center;
		justify-content: center;
		gap: var(--space-1);
	}
	.season-download-image {
		display: block;
		flex: none;
		/* ≈ 77 px on a phone (14 px root), at most 96 px on a tablet (24 px root) */
		width: min(5.5rem, 96px);
		height: auto;
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

	/* ── desktop (≥ 1024 px, root 24 px): hero, then season (cols 1–5) | chapter list (cols 7–12) ── */
	@media (min-width: 1024px) {
		.toon-overview {
			max-width: none;
		}
		.toon-header {
			display: none;
		}
		.chapters-section {
			display: grid;
			grid-template-columns: repeat(12, minmax(0, 1fr));
			column-gap: 1.3333rem;
			padding: 3.3333rem 2rem 0;
			max-width: 75rem;
			margin: 0 auto;
		}
		.season-column {
			grid-column: 1 / span 5;
			display: flex;
			flex-direction: column;
			gap: 0.8333rem;
		}
		.chapters-header {
			padding: 0;
			margin: 0;
		}
		.chapters-header h2 {
			margin: 0 0 0.8333rem;
			font-size: 1.3333rem;
		}
		.season-description {
			margin: 0;
			font-size: 0.6667rem;
			line-height: 1.6;
		}
		.season-column .season-download {
			padding: 0.5rem 0 0;
			text-align: left;
		}
		.season-column .season-download .btn {
			font-size: 0.6667rem;
			padding: 0.5833rem 1.1667rem;
		}
		/* desktop: the record (≈ 150 px) left of the button, both on the column's left edge */
		.season-column .season-download.with-image {
			justify-content: flex-start;
			gap: 0.8333rem;
		}
		.season-column .season-download-image {
			width: 6.25rem;
		}
		.list-column {
			grid-column: 7 / span 6;
			border-top: var(--border-width-regular) solid var(--ink);
		}
		/* the chapter rows: thumbnail 112 × 72, title (+ progress rule / release), "#n" / "Prolog" right-aligned */
		.list-column :global(.chapter-item) {
			grid-template-columns: 112px minmax(0, 1fr) 2.6667rem;
			gap: 0.8333rem;
			padding: 0.5rem 0;
			border-bottom: var(--border-width-hairline) solid var(--border-subtle);
		}
		.list-column :global(.chapter-item .chapter-image img) {
			width: 112px;
			height: 72px;
			object-fit: cover;
		}
		.list-column :global(.chapter-item .chapter-info) {
			display: flex;
			flex-direction: column;
			align-items: flex-start;
			gap: 0.25rem;
		}
		.list-column :global(.chapter-item .chapter-title) {
			font-size: 0.75rem;
		}
		.list-column :global(.chapter-item .chapter-release) {
			font-size: 0.5rem;
		}
		.list-column :global(.chapter-item .chapter-release span) {
			display: inline;
			margin-right: 0.25rem;
		}
		.list-column :global(.chapter-item.published .chapter-release) {
			display: none;
		}
		.list-column :global(.chapter-item .chapter-progress) {
			width: 5.3333rem;
			max-width: none;
			margin-top: 0.25rem;
		}
		.list-column :global(.chapter-item .chapter-number) {
			min-width: 0;
			justify-content: flex-end;
			font-size: 0.6667rem;
		}
		.list-column :global(.chapter-item .chapter-label) {
			font-size: inherit;
		}
		/* the colophon in the season column: its own centred fine print, within the column width */
		.season-column .credits-section {
			margin: 1.3333rem 0 var(--space-3);
		}
		/* the colophon fills the column (no 36em measure): heading bars, running text and the logo box span it */
		.season-column .credits-section :global(.colophon) {
			max-width: none;
			margin: 0;
			padding: 0;
			/* fine print like on the phone (≈ 13 px at the 24 px root), not the 19.5 px the root scale would give */
			font-size: 0.5417rem;
		}
	}
</style>
