<script lang="ts">
	import type { PageProps } from './$types.js';
	import { asset } from '$lib/utils/cdn.ts';
	import ChaptersList from '$lib/components/toon/chapters-list.svelte';
	import StartButton from '$lib/components/toon/start-button.svelte';
	import Hero from '$lib/components/toon/hero.svelte';
	import RelatedItems from '$lib/components/toon/related-items.svelte';
	import { albumAccess, ownsAlbum, seasonMusic } from '@scutoons/content';
	import { getInventoryContext } from '$lib/contexts/inventory.svelte.ts';
	import { onMount } from 'svelte';
	import { CoverArt, CreditsColophon } from '@scutoons/reader';
	import { m } from '$lib/paraglide/messages.js';
	import { localizeHref } from '$lib/paraglide/runtime.js';

	let { data, params }: PageProps = $props();
	let toon = $state(data.toon);
	let season = $state(data.season);

	// the season's music in its record's row (owner 2026-10-02): the album of the season's related items (its zip, its
	// cover = the record) – older seasons: their own download fields (seasonMusic, agents/ASSETS.md "Albums and
	// downloads"); without a carrying related item the separate music block stays
	const inventory = getInventoryContext();
	const seasonSound = $derived(seasonMusic(inventory.catalog, season, params.toonId));
	const musicAlbum = $derived(seasonSound?.album);
	const musicCarrier = $derived(
		seasonSound?.carrier ?? (musicAlbum ? (season?.related_items ?? []).find((id) => inventory.item(id)?.album === musicAlbum.id) : undefined)
	);
	/** the download button in that row: `everyone` / `members` (the download page gates members), `owners` only for owners */
	const musicLink = $derived.by(() => {
		if (!musicCarrier || !musicAlbum || !seasonSound?.download) return null;
		const owner = ownsAlbum(inventory.catalog, musicAlbum, inventory.entries.map((e) => e.item_id));
		return {
			item: musicCarrier,
			image: seasonSound.image ? asset(seasonSound.image) : null,
			href: albumAccess(musicAlbum).download === 'owners' && !owner ? null : localizeHref(`/${params.toonId}/download`)
		};
	});
	let sequence = $state(data.sequence);

	const seasonNumber = $derived(season?.number ?? 1);
	// the cover of the season this page presents (owner 2026-10-02: covers are per season); older data: the toon's
	const cover = $derived(season?.cover?.square ? season.cover : toon.cover);

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

	// the season description: 3 lines, then "…" – a tap opens it (only when it is longer)
	let descriptionEl = $state<HTMLElement>();
	let descriptionOpen = $state(false);
	let descriptionLong = $state(false);
	$effect(() => {
		const el = descriptionEl;
		void season?.description;
		if (!el) return;
		const measure = () => {
			if (descriptionOpen) return;
			descriptionLong = el.scrollHeight > el.clientHeight + 1;
		};
		measure();
		const observer = new ResizeObserver(measure);
		observer.observe(el);
		return () => observer.disconnect();
	});
</script>

<div class="view toon-overview">
	{#if desktop && season}
		<!-- desktop (≥ 1024 px): the trailer hero is the first screen (agents/DESIGN.md "Toon page desktop") -->
		<Hero toonId={params.toonId} {toon} {season} {seasonNumber} {sequence} />
	{/if}
	<div class="toon-header">
		{#if cover?.square}
			<!-- the season's cover, one for every language; with cover.lettering the title is set over the text-free art in the reader's language -->
			<CoverArt
				src={asset(`${params.toonId}/${cover.square}`)}
				alt="Cover"
				class="Cover"
				lettering={cover.lettering}
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
					{#if desktop && cover?.square}
						<!-- desktop: the season's cover at the column's width above its heading (owner 2026-10-03; mobile shows it at the top) -->
						<CoverArt
							src={asset(`${params.toonId}/${cover.square}`)}
							alt="Cover"
							class="season-cover"
							lettering={cover.lettering}
							title={toon.title}
							subtitle={season?.title ?? ''}
							lang={toon.language}
						/>
					{/if}
					<h2>{m['chapter.season_title']({ number: seasonNumber, title: season.title })}</h2>
					<!-- longer than 3 lines: clamped with "…", a tap / click opens and closes it (owner 2026-10-03); a button only then -->
					<!-- svelte-ignore a11y_no_noninteractive_tabindex -->
					<p
						class="season-description"
						class:clamped={!descriptionOpen}
						class:long={descriptionLong}
						bind:this={descriptionEl}
						role={descriptionLong ? 'button' : undefined}
						tabindex={descriptionLong ? 0 : undefined}
						aria-expanded={descriptionLong ? descriptionOpen : undefined}
						onclick={() => descriptionLong && (descriptionOpen = !descriptionOpen)}
						onkeydown={(e) => descriptionLong && (e.key === 'Enter' || e.key === ' ') && (e.preventDefault(), (descriptionOpen = !descriptionOpen))}
					>{season.description}</p>
					{#if !desktop}
						<!-- one start button: first chapter / continue / next / again, from the reading progress (desktop: in the hero) -->
						<StartButton toonId={params.toonId} season={seasonNumber} chapters={sequence} />
					{/if}
				</div>
				{#if desktop && seasonSound?.download && !musicLink}
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
			{#if !desktop && seasonSound?.download && !musicLink}
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
	<div class="season-download" class:with-image={!!seasonSound?.image} data-testid="season-download">
		{#if seasonSound?.image}
			<img
				class="season-download-image"
				src={asset(seasonSound.image)}
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
	.season-description.clamped {
		display: -webkit-box;
		-webkit-box-orient: vertical;
		-webkit-line-clamp: 3;
		line-clamp: 3;
		overflow: hidden;
	}
	.season-description.long {
		cursor: pointer;
	}
	.chapters-header :global(.season-cover) {
		display: block;
		width: 100%;
		margin: 0 0 0.8333rem;
	}
	.chapters-header :global(.start) {
		margin-top: var(--space-1);
	}
	.season-download {
		padding: var(--space-15) 2rem 0;
		text-align: center;
	}
	.season-download .btn {
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
			padding: 0 1.1667rem;
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
