<script lang="ts">
	import type { LocaleText } from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';
	import { textLanguage } from '$lib/editor/text-language.svelte.ts';
	import TextOriginal from '$lib/components/fields/text-original.svelte';

	/**
	 * A `{ <locale>: text }` value (role label, song title, bio …): one input per toon language, missing ones flagged –
	 * or only the creator's chosen text language, a translation with the source as a muted line under it (text-language.svelte.ts).
	 */
	let {
		value,
		languages,
		source,
		label,
		required = false,
		multiline = false,
		testid,
		onchange
	}: {
		value: LocaleText | undefined;
		/** every language, the source first */
		languages: string[];
		/** the source language (default: the first of `languages`) */
		source?: string;
		label: string;
		required?: boolean;
		multiline?: boolean;
		testid?: string;
		onchange: (value: LocaleText) => void;
	} = $props();

	const anyText = $derived(Object.values(value ?? {}).some((t) => t?.trim()));

	const sourceLanguage = $derived(source ?? languages[0] ?? '');
	const shown = $derived(textLanguage.visible(languages, sourceLanguage));
	const original = $derived(textLanguage.original(languages, sourceLanguage));

	function set(locale: string, text: string) {
		onchange({ ...(value ?? {}), [locale]: text });
	}
</script>

<fieldset class="locale-map" class:single={shown.length === 1} data-testid={testid}>
	<legend>{label}</legend>
	{#each shown as l (l)}
		{@const missing = !value?.[l]?.trim() && (required || anyText)}
		<label class:missing>
			<span class="lang">{l}{#if missing}<span class="pill">{m['creator.missing']()}</span>{/if}</span>
			{#if multiline}
				<textarea rows="3" value={value?.[l] ?? ''} oninput={(e) => set(l, e.currentTarget.value)}></textarea>
			{:else}
				<input type="text" value={value?.[l] ?? ''} oninput={(e) => set(l, e.currentTarget.value)} data-testid={testid ? `${testid}-${l}` : undefined} />
			{/if}
		</label>
		{#if original}
			<TextOriginal locale={original} text={value?.[original] ?? ''} oncopy={() => set(l, value?.[original!] ?? '')} testid={testid ? `${testid}-original` : undefined} />
		{/if}
	{/each}
</fieldset>

<style>
	.locale-map {
		border: 0;
		margin: 0;
		padding: 0;
		display: grid;
		grid-template-columns: repeat(auto-fit, minmax(10rem, 1fr));
		gap: 0.25rem 0.5rem;
		min-width: 0;
	}
	/* one language (or translate mode: the original above the field) = one column */
	.locale-map.single {
		grid-template-columns: minmax(0, 1fr);
	}
	legend {
		font-size: var(--type-caption-size);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
		color: var(--ink);
		font-weight: var(--font-weight-regular);
		padding: 0;
		margin-bottom: 0.15rem;
	}
	label {
		display: flex;
		flex-direction: column;
		gap: 0.1rem;
		min-width: 0;
	}
	.lang {
		font-size: var(--type-caption-size);
		color: var(--text-muted);
		display: flex;
		gap: 0.3rem;
		align-items: center;
	}
	.missing input,
	.missing textarea {
		border-color: var(--warn);
	}
	.pill {
		font-size: var(--type-caption-size);
		padding: 0 0.3em;
		background: var(--warn);
		color: var(--ink);
	}
	input,
	textarea {
		width: 100%;
		box-sizing: border-box;
	}
</style>
