<script lang="ts">
	import type { Snippet } from 'svelte';
	import { Button } from '@scutoons/ui/editor';
	import { m } from '$lib/paraglide/messages.js';
	import { readingMinutes } from '$lib/editor/writing.ts';
	import { SOURCE_MARK } from '$lib/editor/text-language.svelte.ts';

	/**
	 * A writing page's part of the status bar (lap 2 C14): the words, the reading time, the languages (the shown one
	 * bright – the translator's bar changes it), Compare, the save state. No Save button (autosave), no refresh (RULES H19).
	 * `lead` is the page's own first word (Compare: how many paragraphs are written).
	 */
	let {
		words,
		languages = [],
		source = '',
		shown = '',
		comparing = false,
		oncompare,
		saving,
		dirty,
		savedAt,
		lead,
		children
	}: {
		words: number | null;
		languages?: readonly string[];
		source?: string;
		shown?: string;
		comparing?: boolean;
		oncompare?: () => void;
		saving: boolean;
		dirty: boolean;
		savedAt: number | null;
		lead?: string;
		/** the page's own controls before the save state (Kanon: Edit while the text is read-only) */
		children?: Snippet;
	} = $props();

	const time = (at: number) => new Date(at).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
	const minutes = $derived(words === null ? 0 : readingMinutes(words));
</script>

<span class="writing-status" data-testid="writing-status">
	{#if lead}<span data-testid="writing-lead">{lead}</span>{:else if words !== null}
		<span data-testid="writing-words">{m['creator.writing_words']({ count: words })}</span>
		{#if minutes}<span class="muted">{m['creator.writing_minutes']({ count: minutes })}</span>{/if}
	{/if}
	{#if languages.length > 1}
		<span class="langs" title={m['creator.writing_languages_about']()} data-testid="writing-languages">
			{#each languages as l (l)}<span class="lang" class:on={l === shown}>{l.toUpperCase()}{l === source ? ` ${SOURCE_MARK}` : ''}</span>{/each}
		</span>
		{#if oncompare}
			<Button variant="framed" pressed={comparing} disabled={shown === source} title={shown === source ? m['creator.writing_compare_source']() : m['creator.writing_compare_about']({ source: source.toUpperCase() })} onclick={oncompare} data-testid="writing-compare">{m['creator.writing_compare']()}</Button>
		{/if}
	{/if}
	{@render children?.()}
	<span class="seam" aria-hidden="true"></span>
	<span class="muted save" data-testid="save-state">{saving ? m['creator.saving']() : dirty ? m['creator.writing_unsaved']() : savedAt ? m['creator.writing_saved_at']({ time: time(savedAt) }) : m['creator.writing_saved']()}</span>
</span>

<style>
	.writing-status {
		display: inline-flex;
		flex: none;
		align-items: center;
		gap: calc(var(--editor-pad) * 1.2);
		padding: 0 var(--editor-pad);
		white-space: nowrap;
	}
	.muted,
	.lang {
		color: var(--tool-muted);
	}
	.langs {
		display: inline-flex;
		gap: calc(var(--editor-pad) * 0.8);
		font-size: var(--editor-small);
		letter-spacing: 0.06em;
	}
	.lang.on {
		color: var(--tool-text);
	}
	.seam {
		align-self: stretch;
		width: var(--editor-edge);
		margin: var(--kit-inset) 0;
		background: var(--tool-line);
	}
</style>
