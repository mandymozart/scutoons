<script lang="ts">
	import { page } from '$app/state';
	import SeasonCast from '$lib/components/toon/season-cast.svelte';
	import { loadToonLanguages } from '$lib/editor/languages.ts';
	import { m } from '$lib/paraglide/messages.js';

	const toon = $derived(page.params.toonId as string);
	const number = $derived(Number(page.params.seasonNumber));
	const languages = $derived(loadToonLanguages(toon));
</script>

{#await languages}
	<p class="pad">{m['creator.loading']()}</p>
{:then langs}
	{#key `${toon}/${number}`}
		<SeasonCast {toon} {number} languages={langs} />
	{/key}
{:catch error}
	<p class="pad error">{error instanceof Error ? error.message : String(error)}</p>
{/await}

<style>
	.pad {
		padding: 1rem;
	}
	.error {
		color: var(--tool-stop);
	}
</style>
