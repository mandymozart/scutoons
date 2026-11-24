<script lang="ts">
	import { page } from '$app/state';
	import { entities, parseKanonSubject, type ToonLanguages } from '@scutoons/content';
	import KanonEditor from '$lib/components/writing/kanon-editor.svelte';
	import { loadToonLanguages } from '$lib/editor/languages.ts';
	import { PLATFORM_SOURCE_LANGUAGE, textLanguage } from '$lib/editor/text-language.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * A Kanon text (research/KANON.md §5, #80): `/kanon/<subject>` – `toon.toptoy`, `character.catpope.human`,
	 * `place.cathedral`. A toon's text takes the toon's languages; a character's or a place's the universe's (every
	 * toon's, the platform's `de` first – the layout's text languages).
	 */
	const subject = $derived(page.params.subject as string);
	const parsed = $derived(parseKanonSubject(subject));
	const universe = $derived<ToonLanguages | null>(
		textLanguage.page.languages.length ? ({ source: PLATFORM_SOURCE_LANGUAGE, languages: textLanguage.page.languages } as ToonLanguages) : null
	);
	const languages = $derived<Promise<ToonLanguages | null>>(parsed?.kind === 'toon' ? loadToonLanguages(parsed.toon) : Promise.resolve(universe));
	/** the text's name: its subject's parts after the kind ("Catpope · Human", "Cathedral") */
	const title = $derived(
		subject
			.split('.')
			.slice(1)
			.map((p) => p.charAt(0).toUpperCase() + p.slice(1))
			.join(' · ')
	);
</script>

{#if !parsed}
	<p class="pad error">{m['creator.kanon_unknown_subject']({ subject })}</p>
{:else}
	{#await languages}
		<p class="pad">{m['creator.loading']()}</p>
	{:then langs}
		{#if langs}
			{#key `${subject}/${langs.languages.join(',')}`}
				<KanonEditor entity={entities.kanon(subject)} subject={parsed} toon={parsed.kind === 'toon' ? parsed.toon : ''} languages={langs} {title} />
			{/key}
		{:else}
			<p class="pad">{m['creator.loading']()}</p>
		{/if}
	{:catch error}
		<p class="pad error">{error instanceof Error ? error.message : String(error)}</p>
	{/await}
{/if}

<style>
	.pad {
		padding: 1rem;
	}
	.error {
		color: var(--tool-stop);
	}
</style>
