<script lang="ts">
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import type { Path } from '@scutoons/content';
	import { LENGTH_WARNING_RATIO, textOf } from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';
	import { textLanguage } from '$lib/editor/text-language.svelte.ts';
	import TextOriginal from './text-original.svelte';

	/**
	 * One text field in every locale, side by side (text is per locale, inline in each chapter file) – or only the
	 * creator's chosen TEXT language (text-language.svelte.ts); a translation then shows the source above it
	 * (translate mode: the source as a muted line under it). `string[]` values (paragraphs) are edited with a blank line between entries and stay arrays.
	 */
	let {
		doc,
		path,
		label = 'text',
		rows = 3,
		multiline = true
	}: { doc: ContentDoc; path: Path; label?: string; rows?: number; multiline?: boolean } = $props();

	const sourceText = $derived(textOf(doc.get(doc.primary, path)).trim());
	/** the locales shown: all present ones, or the chosen text language */
	const shown = $derived(textLanguage.visible(doc.present));
	/** translate mode: the source shown read-only above the field */
	const original = $derived(textLanguage.original(doc.present, doc.primary));

	/** "Copy original": the source value into the translation (arrays stay arrays) */
	function copyOriginal(locale: string) {
		const value = doc.get(doc.primary, path);
		doc.setLocal(locale, path, Array.isArray(value) ? [...value] : value, `text:${locale}:${path.join('.')}`);
	}

	function display(value: unknown) {
		return Array.isArray(value) ? value.join('\n\n') : typeof value === 'string' ? value : '';
	}

	function update(locale: string, input: string) {
		const current = doc.get(locale, path);
		const reference = doc.get(doc.primary, path);
		const asArray = Array.isArray(current) || (current === undefined && Array.isArray(reference));
		const value = asArray ? input.split(/\n{2,}/).map((p) => p.replace(/^\n+|\n+$/g, '')) : input;
		doc.setLocal(locale, path, value, `text:${locale}:${path.join('.')}`);
	}

	function toggleArray(locale: string) {
		const current = doc.get(locale, path);
		doc.setLocal(locale, path, Array.isArray(current) ? current.join('\n') : [textOf(current)]);
	}

	function state(locale: string) {
		const text = textOf(doc.get(locale, path)).trim();
		if (locale === doc.primary) return { missing: !text, ratio: 1, long: false };
		const ratio = sourceText ? text.length / sourceText.length : 1;
		return { missing: !!sourceText && !text, ratio, long: !!text && !!sourceText && ratio > LENGTH_WARNING_RATIO };
	}
</script>

<div class="locale-text" style:--cols={shown.length}>
	{#each shown as locale (locale)}
		{@const s = state(locale)}
		{@const value = doc.get(locale, path)}
		<div class="field">
			<label class:missing={s.missing} class:long={s.long}>
				<span class="head">
					<b>{label} · {locale}</b>
					{#if locale === doc.primary}<span class="pill source">{m['creator.source']()}</span>{/if}
					{#if s.missing}<span class="pill miss">{m['creator.missing']()}</span>{/if}
					{#if s.long}<span class="pill warn">{Math.round(s.ratio * 100)}%</span>{/if}
					{#if Array.isArray(value)}<button type="button" class="ghost mini" title="paragraphs (string[]) – click for one string" onclick={() => toggleArray(locale)}>[ ]</button>{/if}
				</span>
				{#if multiline}
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
			{#if original}
				<TextOriginal locale={original} text={display(doc.get(original, path))} oncopy={() => copyOriginal(locale)} testid="original-{path.join('-')}" />
			{/if}
		</div>
	{/each}
</div>

<style>
	.locale-text {
		display: grid;
		grid-template-columns: repeat(var(--cols), minmax(0, 1fr));
		gap: var(--space-1);
	}
	.field {
		min-width: 0;
	}
	.head b {
		font-weight: inherit;
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
	.pill {
		padding: 0.1em 0.4em;
		border: 0;
		background: var(--badge-neutral);
		color: var(--badge-neutral-text);
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
		font-weight: var(--font-weight-regular);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
	}
	/* filled chips (tokens.css --badge-*) */
	.source {
		background: var(--badge-neutral);
		color: var(--badge-neutral-text);
	}
	.miss {
		background: var(--badge-error);
		color: var(--badge-error-text);
	}
	.warn {
		background: var(--badge-warning);
		color: var(--badge-warning-text);
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
