<script lang="ts">
	import { untrack } from 'svelte';
	import { kanonProblems, kanonTemplate, type KanonRole } from '@scutoons/content';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import { textLanguage } from '$lib/editor/text-language.svelte.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * A level's Kanon text in the character page's inspector (research/CHARACTER-PAGE.md §3a, round 2: "the text field
	 * could cover the entire panel"; K3, #82): ONE text field filling the panel, wrapped prose, in the header's text
	 * language (the source when the doc has no such language). An unwritten source text is empty and clean; another
	 * language without a text offers "Start empty" / "Copy the <source> text". While empty: "Start with the headings"
	 * (the role's sections, KANON.md §4); the rules' warnings show under the field. Saved with the page (MetaPage along).
	 */
	let { doc, role }: { doc: ContentDoc; role: KanonRole } = $props();

	const locale = $derived(textLanguage.chosen(doc.locales) ?? doc.primary);
	const text = $derived<string | null>(typeof doc.data[locale]?.text === 'string' ? doc.data[locale].text : null);
	const warnings = $derived(text ? kanonProblems(text, role) : []);

	// an unwritten source text: empty and clean (no "missing" noise; written once something is typed)
	$effect(() => {
		if (doc.loading || doc.data[doc.primary] != null) return;
		untrack(() => {
			doc.data[doc.primary] = { text: '' };
			doc.saved[doc.primary] = JSON.stringify({ text: '' });
		});
	});

	function input(value: string) {
		doc.setLocal(locale, ['text'], value, `kanon-text:${locale}`);
	}
</script>

<div class="kanon" data-testid="kanon-panel" data-locale={locale}>
	{#if doc.loading}
		<p class="sc-caption">{m['creator.loading']()}</p>
	{:else if text === null}
		<p class="sc-empty">{m['creator.kanon_missing_locale']({ locale, source: doc.primary })}</p>
		<p class="buttons">
			<button type="button" onclick={() => doc.createLocale(locale, () => ({ text: '' }))} data-testid="kanon-start-empty">{m['creator.script_start_empty']()}</button>
			{#if doc.data[doc.primary]?.text}
				<button type="button" class="secondary" onclick={() => doc.createLocale(locale)} data-testid="kanon-copy-source">{m['creator.script_copy_source']({ source: doc.primary })}</button>
			{/if}
		</p>
	{:else}
		{#if !text.trim()}
			<button type="button" class="secondary start" onclick={() => input(kanonTemplate(role, locale))} data-testid="kanon-start-headings">{m['creator.kanon_start_headings']()}</button>
		{/if}
		<textarea
			class="text"
			lang={locale}
			spellcheck="true"
			value={text}
			placeholder={kanonTemplate(role, locale).trim()}
			aria-label={m['creator.kanon_editor_label']({ locale })}
			oninput={(e) => input(e.currentTarget.value)}
			data-testid="kanon-text"
		></textarea>
		{#each warnings as w (w.line)}
			<p class="sc-caption warn">{m['creator.script_position']({ line: w.line, column: 1 })} · {w.message}</p>
		{/each}
	{/if}
</div>

<style>
	.kanon {
		flex: 1;
		min-height: 0;
		display: flex;
		flex-direction: column;
		gap: var(--space-05);
	}
	/* the field fills the panel: prose wraps, every space as typed (KANON.md §5) */
	.text {
		flex: 1;
		min-height: 15rem;
		width: 100%;
		box-sizing: border-box;
		resize: none;
		font-family: var(--font-mono);
		line-height: var(--type-body-line);
		white-space: pre-wrap;
		overflow-wrap: break-word;
		padding: var(--space-05) var(--space-075);
		border: var(--border-width-regular) solid var(--ink);
		background: var(--paper);
		color: var(--ink);
	}
	.start {
		align-self: flex-start;
	}
	.buttons {
		display: flex;
		gap: var(--space-05);
		margin: 0;
	}
	.warn {
		margin: 0;
		color: var(--state-warning);
	}
</style>
