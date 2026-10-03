<script lang="ts">
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * Translate mode (text-language.svelte.ts): one muted caption line under the field being translated – the source
	 * text, read-only, and a small "Copy original" (writes the source into the translation).
	 */
	let {
		locale,
		text,
		oncopy,
		testid
	}: {
		/** the source language */
		locale: string;
		/** the source text as shown (paragraphs joined by a blank line) */
		text: string;
		oncopy: () => void;
		testid?: string;
	} = $props();
</script>

<p class="original" data-testid={testid}>
	<span class="tag">{m['creator.text_original']({ locale: locale.toUpperCase() })}:</span>
	<span class="text" lang={locale}>{text || '–'}</span>
	{#if text}
		<button type="button" class="copy" onclick={oncopy} title={m['creator.text_copy_hint']({ locale: locale.toUpperCase() })} data-testid={testid ? `${testid}-copy` : undefined}
			>{m['creator.text_copy']()}</button
		>
	{/if}
</p>

<style>
	.original {
		margin: var(--space-025) 0 0;
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
		color: var(--text-muted);
		overflow-wrap: anywhere;
	}
	.text {
		white-space: pre-wrap;
	}
	/* a small text link, not a button with a frame (owner 2026-10-02: subtle) */
	.copy {
		all: unset;
		display: inline;
		margin-left: var(--space-05);
		color: var(--ink);
		text-decoration: underline;
		cursor: pointer;
		white-space: nowrap;
	}
	.copy:hover {
		text-decoration-thickness: 2px;
	}
	.copy:focus-visible {
		outline: var(--border-width-focus) solid var(--ink);
		outline-offset: var(--focus-offset);
	}
	/* touch screens: the DESIGN.md hit-target minimum */
	@media (pointer: coarse) {
		.copy {
			display: flex;
			align-items: center;
			width: fit-content;
			min-height: var(--control-height);
			margin-left: 0;
		}
	}
</style>
