<script lang="ts">
	import { page } from '$app/state';
	import { DockHold } from '@scutoons/ui';
	import { entities, parseKanonSubject, type ToonLanguages } from '@scutoons/content';
	import ScriptEditor from '$lib/components/script/script-editor.svelte';
	import { loadToonLanguages } from '$lib/editor/languages.ts';
	import { PLATFORM_SOURCE_LANGUAGE, textLanguage } from '$lib/editor/text-language.svelte.ts';
	import { fileLabel } from '$lib/labels.ts';
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
</script>

{#if !parsed}
	<p class="pad error">{m['creator.kanon_unknown_subject']({ subject })}</p>
{:else}
	{#await languages}
		<p class="pad">{m['creator.loading']()}</p>
		<DockHold />
	{:then langs}
		{#if langs}
			{#key `${subject}/${langs.languages.join(',')}`}
				<ScriptEditor entity={entities.kanon(subject)} toon={parsed.kind === 'toon' ? parsed.toon : ''} languages={langs} title={fileLabel(entities.kanon(subject))} />
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
		color: var(--error);
	}
</style>
