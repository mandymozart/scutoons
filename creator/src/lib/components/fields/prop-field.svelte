<script lang="ts">
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import type { Path } from '@scutoons/content';
	import ListField from '$lib/components/fields/list-field.svelte';
	import { m } from '$lib/paraglide/messages.js';

	/**
	 * A field of a language-neutral document (chapter, scene, season – the same in every language). Empty input removes
	 * the key, so the renderer default applies (shown as placeholder). `list`: names (authors), one input per entry.
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
		title,
		required = false,
		testid = ''
	}: {
		doc: ContentDoc;
		path: Path;
		label: string;
		type?: 'text' | 'number' | 'select' | 'bool' | 'color' | 'date' | 'list';
		options?: (string | number | { value: string | number | boolean; label: string })[];
		placeholder?: string;
		step?: number;
		min?: number;
		max?: number;
		hint?: string;
		/** the label's tooltip (technical detail, if useful) */
		title?: string;
		/** required keys are never removed (empty number → 0, empty text → '', empty list → []) */
		required?: boolean;
		testid?: string;
	} = $props();

	const value = $derived(doc.get(doc.primary, path));
	const opts = $derived(options.map((o) => (typeof o === 'object' ? o : { value: o, label: String(o) })));
	const id = $derived(`f-${path.join('-')}`);

	function write(next: unknown) {
		doc.set(path, next);
	}

	function onInput(raw: string) {
		if (type === 'number') {
			if (raw.trim() === '') return write(required ? 0 : undefined);
			const n = Number(raw);
			if (Number.isFinite(n)) write(n);
			return;
		}
		if (raw === '' && !required) return write(undefined);
		// the <select> yields "true"/"false" – the schema wants a boolean
		if (type === 'bool') return write(raw === 'true');
		if (type === 'select') {
			const option = opts.find((o) => String(o.value) === raw);
			return write(option ? option.value : raw);
		}
		write(raw);
	}

	/** a yes / no value in words; the bool select's empty option names the default (placeholder "true" / "false") */
	const yesNo = (v: string) => (v === 'true' ? m['creator.yes']() : v === 'false' ? m['creator.no']() : v);
	const defaultOption = $derived(placeholder ? (type === 'bool' ? m['creator.field_default']({ value: yesNo(placeholder) }) : placeholder) : m['creator.field_default_plain']());

	function display(v: unknown) {
		return v === undefined || v === null ? '' : String(v);
	}
</script>

<div class="field" data-testid={testid || undefined}>
	<label for={id} {title}>{label}</label>
	{#if type === 'list'}
		<ListField {value} onchange={(list) => write(list.length || required ? list : undefined)} testid={testid ? `${testid}-list` : id} />
	{:else if type === 'bool'}
		<select {id} value={value === undefined ? '' : String(value)} onchange={(e) => onInput(e.currentTarget.value)}>
			{#if !required}<option value="">{defaultOption}</option>{/if}
			<option value="true">{m['creator.yes']()}</option>
			<option value="false">{m['creator.no']()}</option>
		</select>
	{:else if type === 'select'}
		<select {id} value={display(value)} onchange={(e) => onInput(e.currentTarget.value)}>
			{#if !required}<option value="">{defaultOption}</option>{/if}
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
</style>
