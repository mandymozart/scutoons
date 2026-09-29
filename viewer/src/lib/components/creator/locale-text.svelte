<script lang="ts">
	import type { ContentDoc } from '$lib/creator/doc.svelte.ts';
	import type { Path } from '$lib/creator/paths.ts';
	import { LENGTH_WARNING_RATIO, textOf } from '$lib/creator/validate.ts';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * One text field in every locale, side by side (text is per locale, inline in each chapter file).
	 * `string[]` values (paragraphs) are edited with a blank line between entries and stay arrays.
	 */
	let {
		doc,
		path,
		label = 'text',
		rows = 3,
		multiline = true
	}: { doc: ContentDoc; path: Path; label?: string; rows?: number; multiline?: boolean } = $props();

	const sourceText = $derived(textOf(doc.get(doc.primary, path)).trim());

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

<div class="locale-text" style:--cols={doc.present.length}>
	{#each doc.present as locale (locale)}
		{@const s = state(locale)}
		{@const value = doc.get(locale, path)}
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
	{/each}
</div>

<style>
	.locale-text {
		display: grid;
		grid-template-columns: repeat(var(--cols), minmax(0, 1fr));
		gap: 0.5rem;
	}
	.head {
		display: flex;
		gap: 0.3rem;
		align-items: center;
		font-size: 11px;
		margin-bottom: 0.15rem;
	}
	.pill {
		font-size: 9px;
		padding: 0 0.35em;
		border: 1px solid currentColor;
		text-transform: uppercase;
	}
	.source {
		background: var(--ink);
		color: var(--paper);
		border-color: var(--ink);
	}
	.miss {
		color: var(--error);
	}
	.warn {
		color: var(--warn);
	}
	.missing textarea,
	.missing input {
		border-color: var(--error);
		background: #fff3f2;
	}
	.long textarea,
	.long input {
		border-color: var(--warn);
		background: #fffaf0;
	}
	textarea {
		resize: vertical;
		min-height: 3em;
	}
	.mini {
		font-size: 10px;
		padding: 0 0.3em;
		margin-left: auto;
	}
</style>
