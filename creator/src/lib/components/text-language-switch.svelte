<script lang="ts">
	import { Icon } from '@scutoons/ui';
	import { m } from '$lib/paraglide/messages.js';
	import { ALL_TEXT_LANGUAGES, SOURCE_TEXT_LANGUAGE, textLanguage } from '$lib/editor/text-language.svelte.ts';

	/**
	 * The creator's content-language switch in the top bar: globe + the current code ("ALL", "EN") in a compact frame
	 * like the palette trigger (owner 2026-10-02; agents/DESIGN.md "Creator: content language (i18n) + translate mode").
	 * A native select lies invisibly over it (its own list, keyboard and screen-reader support). Which content language
	 * the translatable fields show – all side by side, or one (a translation with its original as a muted line under
	 * it). The interface language is elsewhere (command palette); this one only changes the content fields.
	 */
	let {
		languages,
		source = null
	}: {
		/** the content languages to offer (the toon's, source first) */
		languages: string[];
		/** the source language (marked), null when the page has none (universe pages) */
		source?: string | null;
	} = $props();

	/** the remembered choice when this toon has it, else "source" (the default) or "all" */
	const current = $derived(textLanguage.mode(languages));
	const code = $derived(
		current === ALL_TEXT_LANGUAGES ? m['creator.text_language_all']() : current === SOURCE_TEXT_LANGUAGE ? m['creator.text_language_source_short']() : current
	);
	const title = $derived(`${m['creator.text_language']()}: ${m['creator.text_language_hint']()}`);
</script>

<label class="text-language" class:chosen={current !== ALL_TEXT_LANGUAGES && current !== SOURCE_TEXT_LANGUAGE} {title} data-testid="text-language">
	<Icon name="globe" />
	<span class="code" aria-hidden="true">{code}</span>
	<select onchange={(e) => textLanguage.set(e.currentTarget.value)} aria-label={title} data-testid="text-language-select">
		<option value={SOURCE_TEXT_LANGUAGE} selected={current === SOURCE_TEXT_LANGUAGE}>{m['creator.text_language_source']()}</option>
		<option value={ALL_TEXT_LANGUAGES} selected={current === ALL_TEXT_LANGUAGES}>{m['creator.text_language_all']()}</option>
		{#each languages as l (l)}
			<option value={l} selected={current === l}>{l.toUpperCase()}{l === source ? ` · ${m['creator.source']()}` : ''}</option>
		{/each}
	</select>
</label>

<style>
	/* the palette trigger's compact footprint: 2px ink frame, control height, icon + a short word */
	.text-language {
		position: relative;
		flex: none;
		display: flex;
		align-items: center;
		gap: var(--space-025);
		box-sizing: border-box;
		min-height: var(--control-height);
		margin: 0;
		padding: 0 var(--space-05);
		border: var(--border-width-regular) solid var(--ink);
		background: var(--paper);
		color: var(--ink);
		white-space: nowrap;
		cursor: pointer;
	}
	.code {
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
	}
	/* a language chosen = a toggle that is on: inverted (DESIGN.md "Active / selected = inverted") */
	.text-language.chosen {
		background: var(--ink);
		color: var(--paper);
	}
	/* the native select covers the frame, invisible: it opens its own list */
	select {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%;
		min-height: 0;
		margin: 0;
		padding: 0;
		border: 0;
		opacity: 0;
		cursor: pointer;
	}
	.text-language:focus-within {
		outline: var(--border-width-focus) solid var(--ink);
		outline-offset: var(--focus-offset);
	}
</style>
