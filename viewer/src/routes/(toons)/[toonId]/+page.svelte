<script lang="ts">
	import type { PageProps } from './$types.js';
	import { asset, toonUrl } from '$lib/utils/cdn.ts';
	import ChaptersList from '$lib/components/toon/chapters-list.svelte';
	import SaveSeason from '$lib/components/toon/save-season.svelte';
	import StartButton from '$lib/components/toon/start-button.svelte';
	import Hero from '$lib/components/toon/hero.svelte';
	import RelatedItems from '$lib/components/toon/related-items.svelte';
	import { seasonMusic, seasonMusicDownload } from '@scutoons/content';
	import { getInventoryContext } from '$lib/contexts/inventory.svelte.ts';
	import { getMembershipContext } from '$lib/contexts/membership.svelte.ts';
	import { onMount } from 'svelte';
	import { CoverArt, CreditsColophon } from '@scutoons/reader';
	import ClampText from '$lib/components/clamp-text.svelte';
	import { m } from '$lib/paraglide/messages.js';
	import { localizeHref } from '$lib/paraglide/runtime.js';
	import { shownSeasons } from '$lib/utils/chapter-name.ts';

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
	// the download button only for a reader who may have the zip (the album's `access.download`; owner 2026-10-06: no
	// button without a code unless the album is a free download)
	const membership = getMembershipContext();
	const mayDownload = $derived(
		!!seasonMusicDownload(inventory.catalog, seasonSound, { owned: inventory.entries.map((e) => e.item_id), member: membership.isValid })
	);
	/** the record's row: its cut-out always, the download button only when allowed */
	const musicLink = $derived.by(() => {
		if (!musicCarrier || !musicAlbum || !seasonSound?.download) return null;
		return {
			item: musicCarrier,
			image: seasonSound.image ? asset(seasonSound.image) : null,
			href: mayDownload ? localizeHref(`/${params.toonId}/download`) : null
		};
	});
	let sequence = $state(data.sequence);

	const seasonNumber = $derived(season?.number ?? 1);
	// the chapter list grouped by season (BACKLOG 10.41): two or more seasons with listed chapters – each with its heading
	// (`#season-S`, where /<toon>/season/S lands); else the presented season's list
	const listSeasons = $derived.by(() => {
		const shown = shownSeasons(data.seasons);
		return shown.length >= 2
			? shown.map((s) => ({ number: s.number, title: s.season?.title ?? '', chapters: s.chapters }))
			: [{ number: seasonNumber, title: season?.title ?? '', chapters: sequence }];
	});
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
				src={toonUrl(params.toonId, cover.square)}
				alt="Cover"
				class="Cover"
				lettering={cover.lettering}
				title={toon.title}
				subtitle={season?.title ?? ''}
				lang={toon.language}
			/>
		{/if}
		<!-- the cover shows the title; title and toon description only for screen readers (owner 2026-10-03) -->
		<h1 class="sr-only">{toon.title}</h1>
	</div>

	<section class="chapters-section">
		{#if season && !desktop}
			<div class="chapters-header">
				<!-- longer than 3 lines: clamped with "…", a tap / click opens and closes it (owner 2026-10-03) -->
				<ClampText text={season.description ?? ''} class="season-description" />
				<!-- one start button: first chapter / continue / next / again, from the reading progress (desktop: in the hero) -->
				<StartButton toonId={params.toonId} season={seasonNumber} chapters={sequence} />
			</div>
		{/if}
		{#if season}
			<!-- Chapter List: left on desktop, the season's heading above it (owner 2026-10-03) -->
			<div class="list-column">
				{#each listSeasons as listed (listed.number)}
					<!-- the season's heading + "save the season to the device" (PWA phase 3, beside the rows' save buttons) -->
					<div class="list-head">
						<h2 class="list-heading" id="season-{listed.number}">{m['chapter.season_title']({ number: listed.number, title: listed.title })}</h2>
						<SaveSeason toonId={params.toonId} season={listed.number} chapters={listed.chapters} />
					</div>
					<ChaptersList items={listed.chapters} toonId={params.toonId} />
				{/each}
			</div>
			{#if desktop}
				<!-- desktop: cover, title and description are in the hero – this column only has music, related items, credits -->
				<div class="season-column">
					{#if mayDownload && !musicLink}
						{@render music()}
					{/if}
					{#if season.related_items?.length}
						<RelatedItems ids={season.related_items} music={musicLink} />
					{/if}
					{@render colophon()}
				</div>
			{/if}
			<!-- the season's related items (season.related_items): icon, name, price + "Kaufen" for physical products; the
			     season's music sits in its record's row (owner 2026-10-02) – the music block only without related items -->
			{#if !desktop && mayDownload && !musicLink}
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

<!-- the season's music (only for a reader who may download it), with the record's cut-out beside the button when the season has one -->
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
			<!-- `#credits`: the record player's "All credits" jumps here (owner 2026-10-16) – the credits come later than the page, so the jump waits for them -->
			<section class="credits-section" id="credits" {@attach (el) => { if (location.hash === '#credits') el.scrollIntoView({ block: 'start' }); }}>
				<CreditsColophon {credits} supporters={season?.supporters} imageUrl={(path) => toonUrl(params.toonId, path)} />
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
	/* the chapter list on the same edges as the description above it (phone / tablet; desktop: the grid's padding) */
	.list-column {
		padding: 0 2rem;
	}
	.sr-only {
		position: absolute;
		width: 1px;
		height: 1px;
		margin: -1px;
		overflow: hidden;
		clip: rect(0 0 0 0);
		white-space: nowrap;
		border: 0;
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
		.list-column,
		.season-download {
			padding-left: 1rem;
			padding-right: 1rem;
		}
	}

	.credits-section {
		margin: var(--space-3) auto;
	}

	/* the chapter list (all widths, owner 2026-10-03): the season's heading over a rule, rows with the thumbnail,
	   title (+ progress rule / release), "#n" / "Prolog" right-aligned, hairlines between */
	.list-head {
		display: flex;
		align-items: flex-end;
		justify-content: space-between;
		gap: 1rem;
		padding-bottom: 0.5rem;
		border-bottom: var(--border-width-regular) solid var(--ink);
	}
	.list-heading {
		margin: 0;
		min-width: 0;
		font-size: 1.25rem;
		font-weight: normal;
	}
	/* a further season's list (BACKLOG 10.41): room above its heading, also where /season/S scrolls to */
	.list-head:not(:first-child) {
		margin-top: 2rem;
	}
	.list-heading {
		scroll-margin-top: 4rem;
	}
	.list-column :global(.chapters-list) {
		padding: 0;
	}
	.list-column :global(.chapter-item) {
		grid-template-columns: 5.5rem minmax(0, 1fr) auto;
		gap: 1rem;
		padding: 0.5rem 0;
		border-bottom: var(--border-width-hairline) solid var(--border-subtle);
	}
	.list-column :global(.chapter-item .chapter-image img) {
		display: block;
		width: 5.5rem;
		height: 3.5rem;
		object-fit: cover;
	}
	.list-column :global(.chapter-item .chapter-info) {
		display: flex;
		flex-direction: column;
		align-items: flex-start;
		gap: 0.25rem;
	}
	.list-column :global(.chapter-item .chapter-title) {
		font-size: 1rem;
	}
	.list-column :global(.chapter-item .chapter-release) {
		font-size: 0.75rem;
	}
	.list-column :global(.chapter-item .chapter-release span) {
		display: inline;
		margin-right: 0.25rem;
	}
	.list-column :global(.chapter-item.published .chapter-release) {
		display: none;
	}
	.list-column :global(.chapter-item .chapter-progress) {
		width: 7rem;
		max-width: none;
		margin-top: 0.25rem;
	}
	.list-column :global(.chapter-item .chapter-number) {
		min-width: 0;
		justify-content: flex-end;
		font-size: 0.875rem;
	}
	.list-column :global(.chapter-item .chapter-label) {
		font-size: inherit;
	}

	/* ── desktop (≥ 1024 px, root 24 px): hero, then chapter list (cols 1–6) | season column (cols 8–12) ── */
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
			grid-column: 8 / span 5;
			display: flex;
			flex-direction: column;
			gap: 0.8333rem;
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
			grid-column: 1 / span 6;
			padding: 0;
		}
		.list-heading {
			font-size: 1.3333rem;
		}
		.list-column :global(.chapter-item) {
			grid-template-columns: 112px minmax(0, 1fr) 2.6667rem;
			gap: 0.8333rem;
			padding: 0.5rem 0;
		}
		.list-column :global(.chapter-item .chapter-image img) {
			width: 112px;
			height: 72px;
		}
		.list-column :global(.chapter-item .chapter-title) {
			font-size: 0.75rem;
		}
		.list-column :global(.chapter-item .chapter-release) {
			font-size: 0.5rem;
		}
		.list-column :global(.chapter-item .chapter-progress) {
			width: 5.3333rem;
		}
		.list-column :global(.chapter-item .chapter-number) {
			font-size: 0.6667rem;
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
