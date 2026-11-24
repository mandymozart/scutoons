<script lang="ts">
	import { Select } from '@scutoons/ui/editor';
	import type { ContentDoc } from '$lib/editor/doc.svelte.ts';
	import type { Path } from '@scutoons/content';
	import ListField from '$lib/components/fields/list-field.svelte';
	import UnitNumber from '$lib/components/fields/unit-number.svelte';
	import type { UnitRefs } from '@scutoons/ui';
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
		testid = '',
		onchange,
		source,
		unit
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
		/** the field's tooltip (label and control): an explanation or technical detail, if useful */
		title?: string;
		/** required keys are never removed (empty number → 0, empty text → '', empty list → []) */
		required?: boolean;
		testid?: string;
		/** called after the field wrote its value (a preset that follows a choice) */
		onchange?: (next: unknown) => void;
		/** a field that stands for several keys (the group's Layout): its value, and its write instead of `doc.set(path)` */
		source?: { value: unknown; write: (next: unknown) => void };
		/** a number with a unit switch (core units): the value is stored in `stored`, shown in the unit chosen for `kind` */
		unit?: { kind: string; dimension: string; stored: string; fallback?: string; refs?: UnitRefs };
	} = $props();

	const value = $derived(source ? source.value : doc.get(doc.primary, path));
	const opts = $derived(options.map((o) => (typeof o === 'object' ? o : { value: o, label: String(o) })));
	const id = $derived(`f-${path.join('-')}`);

	function write(next: unknown) {
		if (source) source.write(next);
		else doc.set(path, next);
		onchange?.(next);
	}

	function onInput(raw: string) {
		if (type === 'number') {
			if (raw.trim() === '') return write(required ? 0 : undefined);
			const n = Number(raw);
			if (Number.isFinite(n)) write(n);
			return;
		}
		if (raw === '' && !required) return write(undefined);
		// the select yields "true"/"false" – the schema wants a boolean
		if (type === 'bool') return write(raw === 'true');
		if (type === 'select') {
			const option = opts.find((o) => String(o.value) === raw);
			return write(option ? option.value : raw);
		}
		write(raw);
	}

	/** yes / no in words; a select's empty option shows the default in brackets ("(no)" for a bool placeholder "true" / "false") */
	const yesNo = (v: string) => (v === 'true' ? m['creator.yes']() : v === 'false' ? m['creator.no']() : v);
	const defaultOption = $derived(!placeholder ? m['creator.field_default_plain']() : type === 'bool' ? `(${yesNo(placeholder)})` : type === 'select' && placeholder !== '–' ? `(${placeholder})` : placeholder);

	function display(v: unknown) {
		return v === undefined || v === null ? '' : String(v);
	}
</script>

<div class="field" {title} data-testid={testid || undefined}>
	<label for={id}>{label}</label>
	{#if type === 'list'}
		<ListField {value} onchange={(list) => write(list.length || required ? list : undefined)} testid={testid ? `${testid}-list` : id} />
	{:else if type === 'bool'}
		<Select {id} {label} value={value === undefined ? '' : String(value)} options={[...(required ? [] : [{ value: '', label: defaultOption }]), { value: 'true', label: m['creator.yes']() }, { value: 'false', label: m['creator.no']() }]} onchange={(v) => onInput(v)} />
	{:else if type === 'select'}
		<Select
			{id}
			{label}
			value={display(value)}
			options={[
				...(required ? [] : [{ value: '', label: defaultOption }]),
				...opts.map((o) => ({ value: String(o.value), label: o.label })),
				...(value !== undefined && !opts.some((o) => String(o.value) === String(value)) ? [{ value: display(value), label: `${display(value)} (?)` }] : [])
			]}
			onchange={(v) => onInput(v)}
		/>
	{:else if type === 'number' && unit}
		<UnitNumber {id} {label} {...unit} {value} {min} {max} {placeholder} {required} onchange={(v) => write(v === undefined ? (required ? 0 : undefined) : v)} testid={testid ? `${testid}-input` : undefined} />
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
		color: var(--tool-text);
	}
	.color {
		display: flex;
		gap: var(--space-05);
	}
	.color input[type='color'] {
		flex: none;
	}
</style>
