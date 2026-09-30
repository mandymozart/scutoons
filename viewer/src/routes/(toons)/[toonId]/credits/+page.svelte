<script lang="ts">
	import { resolveCredits } from '@scutoons/content';
	import { CreditsView } from '@scutoons/reader';
	import { asset } from '$lib/utils/cdn.ts';
	import type { PageProps } from './$types.js';

	let { data }: PageProps = $props();

	const chapterTitles = $derived(Object.fromEntries((data.sequence ?? []).map((item) => [item.number, item.title])));
	const resolved = $derived(resolveCredits(data.credits, data.people, data.roles, data.locale, { chapterTitles }));
</script>

<div class="view credits-view">
	<CreditsView credits={resolved} imageUrl={asset} />
</div>

<style>
	.credits-view {
		max-width: 600px;
		margin: 5rem auto;
	}
</style>
