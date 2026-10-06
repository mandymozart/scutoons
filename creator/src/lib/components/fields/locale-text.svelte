<script lang="ts">
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import type { Path } from '@scutoons/content';
	import { LENGTH_WARNING_RATIO, textOf } from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';
	import { textLanguage } from '$lib/editor/text-language.svelte.ts';
	import TextOriginal from '$lib/components/fields/text-original.svelte';
	import SourceMark from '$lib/components/text/source-mark.svelte';

	/**
	 * One text field in the creator's chosen TEXT language (text-language.svelte.ts – the header's switch; default the
	 * source, marked with the star): exactly one language; a translation shows the source as a muted line under it
	 * (translate mode). A language-neutral document (chapter, scene, season) keeps the text as a locale map at `path`
	 * (`doc.text` / `doc.setText`); a per-locale document in each locale's file. `string[]` values (paragraphs) are
	 * edited with a blank line between entries and stay arrays. `type="date"`: a date per language (`release`).
	 */
	let {
		doc,
		path,
		label = m['creator.field_text'](),
		rows = 3,
		multiline = true,
		type = 'text'
	}: { doc: ContentDoc; path: Path; label?: string; rows?: number; multiline?: boolean; type?: 'text' | 'date' } = $props();

	const source = $derived(doc.textSource);
	const sourceText = $derived(textOf(doc.text(path, source)).trim());
	/** the one locale shown: the chosen text language, else the source */
	const shown = $derived(textLanguage.visible(doc.textLanguages, source));
	/** translate mode: the source shown read-only under the field */
	const original = $derived(textLanguage.original(doc.textLanguages, source));
	const key = (locale: string) => `text:${locale}:${path.join('.')}`;

	/** "Copy original": the source value into the translation (arrays stay arrays) */
	function copyOriginal(locale: string) {
		const value = doc.text(path, source);
		doc.setText(path, locale, Array.isArray(value) ? [...value] : (value ?? ''), key(locale));
	}

	function display(value: unknown) {
		return Array.isArray(value) ? value.join('\n\n') : typeof value === 'string' ? value : '';
	}

	function update(locale: string, input: string) {
		const current = doc.text(path, locale);
		const reference = doc.text(path, source);
		const asArray = Array.isArray(current) || ((current === undefined || current === '') && Array.isArray(reference));
		const value = asArray && input ? input.split(/\n{2,}/).map((p) => p.replace(/^\n+|\n+$/g, '')) : input;
		doc.setText(path, locale, value, key(locale));
	}

	function toggleArray(locale: string) {
		const current = doc.text(path, locale);
		doc.setText(path, locale, Array.isArray(current) ? current.join('\n') : [textOf(current)]);
	}

	function state(locale: string) {
		const text = textOf(doc.text(path, locale)).trim();
		if (locale === source) return { missing: !text, ratio: 1, long: false };
		const ratio = sourceText ? text.length / sourceText.length : 1;
		return { missing: !!sourceText && !text, ratio, long: type === 'text' && !!text && !!sourceText && ratio > LENGTH_WARNING_RATIO };
	}
</script>

<div class="locale-text">
	{#each shown as locale (locale)}
		{@const s = state(locale)}
		{@const value = doc.text(path, locale)}
		<div class="field">
			<label class:missing={s.missing} class:long={s.long}>
				<span class="head">
					<span>{label} · {locale}{#if locale === source}<SourceMark />{/if}</span>
					{#if s.missing}<span class="sc-chip is-error">{m['creator.missing']()}</span>{/if}
					{#if s.long}<span class="sc-chip is-warning">{Math.round(s.ratio * 100)}%</span>{/if}
					{#if Array.isArray(value)}<button type="button" class="ghost mini" title={m['creator.text_paragraphs_hint']()} onclick={() => toggleArray(locale)}>[ ]</button>{/if}
				</span>
				{#if type === 'date'}
					<input type="date" value={display(value)} oninput={(e) => update(locale, e.currentTarget.value)} data-testid="{path.join('-')}-{locale}" />
				{:else if multiline}
					<textarea
						{rows}
						lang={locale}
						value={display(value)}
						oninput={(e) => update(locale, e.currentTarget.value)}
						data-testid="text-{locale}"
					></textarea>
				{:else}
					<input
						type="text"
						lang={locale}
						value={display(value)}
						oninput={(e) => update(locale, e.currentTarget.value)}
						data-testid="text-{locale}"
					/>
				{/if}
			</label>
			{#if original && type === 'text'}
				<TextOriginal locale={original} text={display(doc.text(path, original))} oncopy={() => copyOriginal(locale)} testid="original-{path.join('-')}" />
			{/if}
		</div>
	{/each}
</div>


<style>
	.locale-text {
		display: grid;
		gap: var(--space-1);
	}
	.field {
		min-width: 0;
	}
	.head {
		display: flex;
		flex-wrap: wrap;
		gap: var(--space-05);
		align-items: center;
		margin-bottom: var(--space-025);
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
		font-weight: var(--font-weight-regular);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
		color: var(--ink);
	}
	/* state = the frame's colour + a thick left rule, no tinted fills */
	.missing textarea,
	.missing input {
		border-color: var(--error);
		border-left-width: 0.5em;
	}
	.long textarea,
	.long input {
		border-color: var(--warn);
		border-left-width: 0.5em;
	}
	textarea {
		resize: vertical;
		min-height: 3em;
	}
	.mini {
		margin-left: auto;
	}
</style>
