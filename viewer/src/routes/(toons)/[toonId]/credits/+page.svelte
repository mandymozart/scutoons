<script lang="ts">
	import { resolveCredits } from '@scutoons/content';
	import { CreditsView } from '@scutoons/reader';
	import { asset } from '$lib/utils/cdn.ts';
	import LegacyCredits from './legacy-credits.svelte';
	import type { PageProps } from './$types.js';

	let { data }: PageProps = $props();

	const chapterTitles = $derived(Object.fromEntries((data.sequence ?? []).map((item) => [item.number, item.title])));
	const resolved = $derived(
		data.credits && data.people
			? resolveCredits(data.credits, data.people, data.roles, data.locale, { chapterTitles })
			: null
	);
</script>

<div class="view credits-view" data-credits-source={data.source}>
	{#if resolved}
		<CreditsView credits={resolved} imageUrl={asset} />
	{:else if data.legacy}
		<LegacyCredits credits={data.legacy} />
	{/if}
</div>

<style>
	.credits-view {
		max-width: 600px;
		margin: 5rem auto;
	}
</style>
