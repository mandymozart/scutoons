<script lang="ts">
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import type { Path } from '@scutoons/content';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * A shared field (written to every locale that doesn't override it). Empty input removes the key, so the
	 * renderer default applies (shown as placeholder). Per-locale overrides are listed and can be reset.
	 */
	let {
		doc,
		path,
		label,
		type = 'text',
		options = [],
		placeholder = '',
		step = 0.1,
		min,
		max,
		hint = '',
		required = false,
		editLocale = null
	}: {
		doc: ContentDoc;
		path: Path;
		label: string;
		type?: 'text' | 'number' | 'select' | 'bool' | 'color' | 'date';
		options?: (string | number | { value: string | number | boolean; label: string })[];
		placeholder?: string;
		step?: number;
		min?: number;
		max?: number;
		hint?: string;
		/** required keys are never removed (empty number → 0, empty text → '') */
		required?: boolean;
		/** set = edit only this locale (explicit override); null = shared */
		editLocale?: string | null;
	} = $props();

	const readLocale = $derived(editLocale ?? doc.primary);
	const value = $derived(doc.get(readLocale, path));
	const overrides = $derived(editLocale ? [] : doc.overrides(path));
	const opts = $derived(options.map((o) => (typeof o === 'object' ? o : { value: o, label: String(o) })));
	const id = $derived(`f-${path.join('-')}`);

	function write(next: unknown) {
		if (editLocale) doc.setLocal(editLocale, path, next);
		else doc.setShared(path, next);
	}

	function onInput(raw: string) {
		if (type === 'number') {
			if (raw.trim() === '') return write(required ? 0 : undefined);
			const n = Number(raw);
			if (Number.isFinite(n)) write(n);
			return;
		}
		if (raw === '' && !required) return write(undefined);
		if (type === 'select') {
			const option = opts.find((o) => String(o.value) === raw);
			return write(option ? option.value : raw);
		}
		write(raw);
	}

	function display(v: unknown) {
		return v === undefined || v === null ? '' : String(v);
	}
</script>

<div class="field" class:has-override={overrides.length > 0}>
	<label for={id}>{label}</label>
	{#if type === 'bool'}
		<select {id} value={value === undefined ? '' : String(value)} onchange={(e) => onInput(e.currentTarget.value)}>
			{#if !required}<option value="">{placeholder || 'default'}</option>{/if}
			<option value="true">true</option>
			<option value="false">false</option>
		</select>
	{:else if type === 'select'}
		<select {id} value={display(value)} onchange={(e) => onInput(e.currentTarget.value)}>
			{#if !required}<option value="">{placeholder || 'default'}</option>{/if}
			{#each opts as o}<option value={String(o.value)}>{o.label}</option>{/each}
			{#if value !== undefined && !opts.some((o) => String(o.value) === String(value))}
				<option value={display(value)}>{display(value)} (?)</option>
			{/if}
		</select>
	{:else if type === 'color'}
		<span class="color">
			<input {id} type="text" value={display(value)} {placeholder} oninput={(e) => onInput(e.currentTarget.value)} />
			<input type="color" value={typeof value === 'string' && /^#[0-9a-f]{6}$/i.test(value) ? value : '#000000'} oninput={(e) => onInput(e.currentTarget.value)} aria-label="{label} picker" />
		</span>
	{:else}
		<input
			{id}
			type={type === 'number' ? 'number' : type === 'date' ? 'date' : 'text'}
			{step}
			{min}
			{max}
			value={display(value)}
			{placeholder}
			oninput={(e) => onInput(e.currentTarget.value)}
		/>
	{/if}
	{#if hint}<span class="hint">{hint}</span>{/if}
	{#each overrides as l}
		<span class="override">
			{l}: <b>{display(doc.get(l, path)) || '–'}</b> ({m['creator.override']()})
			<button type="button" class="ghost" onclick={() => doc.resetOverride(l, path)}>{m['creator.use_shared']()}</button>
		</span>
	{/each}
</div>

<style>
	.field {
		display: flex;
		flex-direction: column;
		gap: var(--space-025);
		min-width: 0;
	}
	label {
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
		font-weight: var(--font-weight-regular);
		text-transform: uppercase;
		letter-spacing: var(--tracking-label);
		color: var(--ink);
	}
	.color {
		display: flex;
		gap: var(--space-05);
	}
	.color input[type='color'] {
		flex: none;
	}
	/* a per-locale override: ink text, a cyan rule (the "live" difference), no tinted fill */
	.override {
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
		color: var(--ink);
		border-left: 0.5em solid var(--cyan);
		padding: var(--space-025) var(--space-05);
	}
</style>
