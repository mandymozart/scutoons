<script lang="ts">
	import { Icon } from '@scutoons/ui';
	import { m } from '$lib/paraglide/messages.js';
	import { SOURCE_MARK, textLanguage } from '$lib/editor/text-language.svelte.ts';
	import SourceMark from '$lib/components/text/source-mark.svelte';

	/**
	 * The creator's content-language switch in the top bar: globe + the current code ("DE", "KO"), the source marked
	 * with the star, in a compact frame like the palette trigger (owner 2026-10-02, 2026-10-06; agents/DESIGN.md
	 * "Creator: content language (i18n) + translate mode"). A native select lies invisibly over it (its own list,
	 * keyboard and screen-reader support). Which ONE content language the translatable fields show – default the
	 * source; a translation shows its original as a muted line under each field. The interface language is elsewhere
	 * (command palette); this one only changes the content fields.
	 */
	let {
		languages,
		source
	}: {
		/** the content languages to offer (the toon's, source first) */
		languages: string[];
		/** the source language (the default, marked): the toon's, or the platform's on pages without a toon */
		source: string;
	} = $props();

	/** the remembered choice when this toon has it, else the source */
	const current = $derived(textLanguage.current(languages, source));
	const title = $derived(`${m['creator.text_language']()}: ${m['creator.text_language_hint']()}`);
</script>

<label class="text-language sc-control" class:chosen={current !== source} {title} data-testid="text-language">
	<Icon name="globe" />
	<span class="code" aria-hidden="true" data-testid="text-language-code">{current}</span>
	{#if current === source}<SourceMark />{/if}
	<select onchange={(e) => textLanguage.set(e.currentTarget.value)} aria-label={title} data-testid="text-language-select">
		{#each languages as l (l)}
			<option value={l} selected={current === l}>{l.toUpperCase()}{l === source ? ` ${SOURCE_MARK}` : ''}</option>
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
	/* a translation shown = a toggle that is on: inverted (DESIGN.md "Active / selected = inverted") */
	.text-language.chosen {
		background: var(--ink);
		color: var(--paper);
	}
	/* the native select covers the frame, invisible: it opens its own list */
	select {
		position: absolute;
		inset: 0;
		width: 100%;
		height: 100%; /* control-ok: the invisible native select covers the framed control */
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
