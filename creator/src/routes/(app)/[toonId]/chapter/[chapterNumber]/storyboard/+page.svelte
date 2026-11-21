<script lang="ts">
	// A chapter's Storyboard page (#123, agents/research/STUDIO-IN-CREATOR.md): the toon's source language picks the
	// script beside the shots; the page is keyed by toon + chapter.
	import { page } from '$app/state';
	import StoryboardEditor from '$lib/components/storyboard/storyboard-editor.svelte';
	import { loadToonLanguages } from '$lib/editor/languages.ts';
	import { m } from '$lib/paraglide/messages.js';

	const toon = $derived(page.params.toonId as string);
	const chapterNumber = $derived(Number(page.params.chapterNumber));
	const languages = $derived(loadToonLanguages(toon));
</script>

{#await languages}
	<p class="pad">{m['creator.loading']()}</p>
{:then langs}
	{#key `${toon}/${chapterNumber}`}
		<StoryboardEditor {toon} {chapterNumber} source={langs.source} />
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
