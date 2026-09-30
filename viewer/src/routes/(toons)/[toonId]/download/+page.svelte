<script lang="ts">
	import { goto } from '$app/navigation';
	import MembershipBlocker from '$lib/components/chapter/membership-blocker.svelte';
	import { asset } from '$lib/utils/cdn.ts';
	import { m } from '$lib/paraglide/messages.js';
	import type { PageProps } from './$types.js';

	let { data, params }: PageProps = $props();

	function handleDownload(url: string) {
		window.open(url, '_blank');
	}

	function goBack() {
		goto(`/${params.toonId}`);
	}
</script>

<MembershipBlocker>
	<div class="view download-page">
		<div class="download-container">
			<h1>Downloads {data.toon.title}</h1>

			<!-- the season's downloads (season.download, agents/DATA.md) -->
			{#if data.download?.music}
				{@const musicUrl = data.download.music}
				<div class="download-item">
					<h2>Music</h2>
					<button onclick={() => handleDownload(asset(`${params.toonId}/${musicUrl}`))}>
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
</MembershipBlocker>

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
