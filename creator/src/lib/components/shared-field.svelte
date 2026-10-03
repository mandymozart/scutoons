<script lang="ts">
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import type { Path } from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';
	import ImagePicker from './image-picker.svelte';
	import ListField from './list-field.svelte';

	/**
	 * A language-neutral field (number, date, paths, names): edited once and written to every locale file
	 * (`setShared`). When the locale files differ today (or "per language" is chosen), every locale gets its own editor
	 * (`setLocal`) with the source first and the differing ones marked as overrides; "same for all" resets them.
	 */
	let {
		doc,
		path,
		label,
		kind = 'text',
		options = [],
		toon = '',
		known = [],
		placeholder = '',
		required = false,
		splittable = true,
		testid = ''
	}: {
		doc: ContentDoc;
		path: Path;
		label: string;
		kind?: 'text' | 'number' | 'date' | 'select' | 'list' | 'image' | 'nullable';
		options?: string[];
		toon?: string;
		known?: string[];
		placeholder?: string;
		/** required keys are never removed (empty number → 0, empty text → '') */
		required?: boolean;
		/** false: no "per language" switch (a value that is never translated); existing overrides still show */
		splittable?: boolean;
		testid?: string;
	} = $props();

	let split = $state(false);
	const overrides = $derived(doc.overrides(path));
	const perLocale = $derived(doc.present.length > 1 && (split || overrides.length > 0));
	const locales = $derived(perLocale ? doc.present : [doc.primary]);
	const id = $derived(testid || `field-${path.join('-')}`);

	function write(locale: string, value: unknown) {
		const key = `${perLocale ? locale : '*'}:${path.join('.')}`;
		if (perLocale) doc.setLocal(locale, path, value, key);
		else doc.setShared(path, value, key);
	}

	function parse(raw: string): unknown {
		if (kind === 'number') {
			if (raw.trim() === '') return required ? 0 : undefined;
			const n = Number(raw);
			return Number.isFinite(n) ? n : undefined;
		}
		if (kind === 'nullable') return raw.trim() === '' ? null : raw;
		if (raw === '' && !required) return undefined;
		return raw;
	}

	function sameForAll() {
		for (const l of overrides) doc.resetOverride(l, path);
		split = false;
	}

	const show = (v: unknown) => (v === undefined || v === null ? '' : String(v));
</script>

<div class="shared-field" class:per-locale={perLocale} data-testid={id}>
	<div class="head">
		<span class="label">{label}</span>
		{#if doc.present.length > 1 && (splittable || perLocale)}
			{#if perLocale}
				<button type="button" class="ghost mini" onclick={sameForAll} title={m['creator.same_for_all_hint']({ locale: doc.primary })} data-testid="{id}-same">{m['creator.same_for_all']()}</button>
			{:else}
				<button type="button" class="ghost mini" onclick={() => (split = true)} data-testid="{id}-split">{m['creator.per_language']()}</button>
			{/if}
		{/if}
	</div>
	{#each locales as l (l)}
		{@const value = doc.get(l, path)}
		<div class="row">
			{#if perLocale}
				<span class="loc">
					{l}
					{#if l === doc.primary}<span class="pill source">{m['creator.source']()}</span>{/if}
					{#if overrides.includes(l)}<span class="pill override">{m['creator.override']()}</span>{/if}
				</span>
			{/if}
			<div class="editor">
				{#if kind === 'list'}
					<ListField {value} onchange={(list) => write(l, list)} testid="{id}-{l}" />
				{:else if kind === 'image'}
					<ImagePicker {toon} label="{label}{perLocale ? ` · ${l}` : ''}" hideLabel value={typeof value === 'string' ? value : undefined} {known} onchange={(v) => write(l, v || (required ? '' : undefined))} />
				{:else if kind === 'select'}
					<select value={show(value)} onchange={(e) => write(l, parse(e.currentTarget.value))} data-testid="{id}-{l}">
						{#if !required}<option value="">–</option>{/if}
						{#each options as o}<option value={o}>{o}</option>{/each}
					</select>
				{:else}
					<input
						type={kind === 'number' ? 'number' : kind === 'date' ? 'date' : 'text'}
						value={show(value)}
						placeholder={kind === 'nullable' ? placeholder || 'null' : placeholder}
						oninput={(e) => write(l, parse(e.currentTarget.value))}
						data-testid="{id}-{l}"
					/>
				{/if}
			</div>
		</div>
	{/each}
</div>

<style>
	.shared-field {
		display: flex;
		flex-direction: column;
		gap: 0.2rem;
		min-width: 0;
	}
	.head {
		display: flex;
		align-items: center;
		gap: 0.3rem;
	}
	.label {
		font-size: var(--type-caption-size);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
		color: var(--muted);
	}
	.mini {
		font-size: var(--type-caption-size);
		padding: 0 0.3em;
		margin-left: auto;
	}
	.row {
		display: flex;
		flex-direction: column;
		gap: 0.1rem;
	}
	.per-locale .row {
		border-left: 2px solid var(--border-subtle);
		padding-left: 0.4rem;
	}
	.loc {
		font-size: var(--type-caption-size);
		display: flex;
		gap: 0.3rem;
		align-items: center;
	}
	.pill {
		font-size: var(--type-caption-size);
		letter-spacing: var(--tracking-label);
		color: var(--ink);
		padding: 0 0.35em;
		border: 1px solid currentColor;
		text-transform: uppercase;
	}
	.source {
		background: var(--ink);
		color: var(--paper);
		border-color: var(--ink);
	}
	.override {
		border-left: 0.5rem solid var(--primary-cyan);
		border-color: var(--cyan);
	}
	.editor :global(input),
	.editor :global(select) {
		width: 100%;
		box-sizing: border-box;
	}
</style>
