<script lang="ts">
	import { page } from '$app/state';
	import { DockHold } from '@scutoons/ui';
	import ConvertPage from '$lib/components/toon/convert-page.svelte';
	import { loadToonLanguages } from '$lib/editor/languages.ts';
	import { m } from '$lib/paraglide/messages.js';

	const toon = $derived(page.params.toonId as string);
	const languages = $derived(loadToonLanguages(toon));
</script>

{#await languages}
	<p class="pad">{m['creator.loading']()}</p>
	<!-- the last page's Flow / Problems / Info stay docked until this page takes them over (UI-CONCEPTS E.3 b) -->
	<DockHold />
{:then langs}
	{#key `${toon}/${langs.languages.join(',')}`}
		<ConvertPage {toon} languages={langs} />
	{/key}
{:catch error}
	<p class="pad error">{error instanceof Error ? error.message : String(error)}</p>
{/await}

<style>
	.pad {
		padding: 1rem;
	}
	.error {
		color: var(--error);
	}
</style>
