<script lang="ts">
	import { goto } from '$app/navigation';
	import MembershipBlocker from '$lib/components/chapter/membership-blocker.svelte';
	import { asset } from '$lib/utils/cdn.ts';
	import { m } from '$lib/paraglide/messages.js';
	import { localizeHref } from '$lib/paraglide/runtime.js';
	import type { PageProps } from './$types.js';
	import { albumAccess, albumDownload, seasonMusic } from '@scutoons/content';
	import { getInventoryContext } from '$lib/contexts/inventory.svelte.ts';
	import { getMembershipContext } from '$lib/contexts/membership.svelte.ts';

	let { data, params }: PageProps = $props();

	// who gets the music: the album's `access.download` (agents/ITEMS.md "Players, albums, related items") when the zip is
	// an album's download – `members` (the default, as before: the membership blocker), `everyone` (open, marketing) or
	// `owners` (readers who own a record of the album: retrieval of what they bought)
	const inventory = getInventoryContext();
	const membership = getMembershipContext();
	// the music = the album of the season's related items (older seasons: season.download.music – seasonMusic)
	const music = $derived(seasonMusic(inventory.catalog, data.season, params.toonId));
	const album = $derived(music?.album);
	const access = $derived(album ? albumAccess(album).download : 'members');
	const allowed = $derived(
		!!album && !!albumDownload(inventory.catalog, album, { owned: inventory.entries.map((e) => e.item_id), member: membership.isValid })
	);

	function handleDownload(url: string) {
		window.open(url, '_blank');
	}

	function goBack() {
		goto(localizeHref(`/${params.toonId}`));
	}
</script>

{#if access === 'members'}
	<MembershipBlocker>{@render downloads()}</MembershipBlocker>
{:else if allowed}
	{@render downloads()}
{:else}
	<div class="view download-page">
		<p data-testid="download-owners-only">{m['download.owners_only']()}</p>
		<button onclick={goBack}>← {m['download.read_webtoon']()}</button>
	</div>
{/if}

{#snippet downloads()}
	<div class="view download-page">
		<div class="download-container">
			<h1>Downloads {data.toon.title}</h1>

			<!-- the season's music: its album's zip (agents/ASSETS.md "Albums and downloads") -->
			{#if music?.download}
				{@const musicUrl = music.download}
				<div class="download-item">
					<h2>Music</h2>
					<button onclick={() => handleDownload(asset(musicUrl))}>
						{m['season.download']()}
					</button>
				</div>
			{/if}

			{#if data.download?.complete}
				{@const downloadUrl = data.download.complete}
	<div class="download-item">
		<h2>{data.toon.title} - Complete</h2>
		<button onclick={() => handleDownload(asset(`${params.toonId}/${downloadUrl}`))}>
			Download Complete
		</button>
	</div>
			{/if}

			<div class="back-section">
				<button onclick={goBack}>← {m['download.read_webtoon']()}</button>
			</div>
		</div>
	</div>
{/snippet}

<style>
	.download-page {
		max-width: 800px;
		margin: 0 auto;
		padding: 2rem;
	}

	.download-container h1 {
		margin-bottom: 2rem;
	}

	.download-item {
		margin-bottom: 2rem;
	}

	.download-item h2 {
		margin-bottom: 1rem;
	}

	.back-section {
		margin-top: 3rem;
	}
</style>
