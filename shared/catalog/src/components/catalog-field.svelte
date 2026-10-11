<script lang="ts">
	import { Field } from '@scutoons/ui';
	import { m } from '../messages.ts';

	/**
	 * One plain catalog field (@scutoons/ui Field: label, hint, the problem of the last save): a text, number, URL,
	 * date, multi-line text or a select. `null` = empty (numbers, selects with `none`).
	 */
	let {
		label,
		value,
		type = 'text',
		options = [],
		none,
		hint,
		error = null,
		required = false,
		placeholder,
		onchange,
		testid
	}: {
		label: string;
		value: unknown;
		type?: 'text' | 'number' | 'url' | 'textarea' | 'select';
		/** the choices of a select */
		options?: { value: string; label?: string }[];
		/** a select's empty choice (its label; value null) – omitted = a choice is required */
		none?: string;
		hint?: string;
		error?: string | null;
		required?: boolean;
		placeholder?: string;
		onchange: (value: any) => void;
		testid?: string;
	} = $props();

	const text = $derived(value === null || value === undefined ? '' : String(value));
	// a value the options don't know (an older entry) stays choosable
	const choices = $derived(text && !options.some((o) => o.value === text) ? [...options, { value: text, label: `${text} (?)` }] : options);

	function number(raw: string): number | null {
		if (raw.trim() === '') return null;
		const n = Number(raw);
		return Number.isNaN(n) ? null : n;
	}
</script>

<Field {label} {hint} {error} {required}>
	{#snippet children({ id, describedby, invalid })}
		{#if type === 'select'}
			<select {id} aria-describedby={describedby} aria-invalid={invalid} value={text} onchange={(e) => onchange(e.currentTarget.value === '' ? null : e.currentTarget.value)} data-testid={testid}>
				{#if none !== undefined || !text}<option value="">{none ?? m['catalog.choose']()}</option>{/if}
				{#each choices as o (o.value)}<option value={o.value}>{o.label ?? o.value}</option>{/each}
			</select>
		{:else if type === 'textarea'}
			<textarea {id} rows="3" aria-describedby={describedby} aria-invalid={invalid} value={text} {placeholder} oninput={(e) => onchange(e.currentTarget.value)} data-testid={testid}></textarea>
		{:else if type === 'number'}
			<input {id} type="number" step="any" aria-describedby={describedby} aria-invalid={invalid} value={text} {placeholder} oninput={(e) => onchange(number(e.currentTarget.value))} data-testid={testid} />
		{:else}
			<input {id} {type} aria-describedby={describedby} aria-invalid={invalid} value={text} {placeholder} oninput={(e) => onchange(e.currentTarget.value)} autocomplete="off" data-testid={testid} />
		{/if}
	{/snippet}
</Field>
