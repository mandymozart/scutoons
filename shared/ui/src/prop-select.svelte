<script lang="ts" module>
	export interface PropOption<T = string | number | boolean> {
		value: T;
		label: string;
		/** shown but not pickable (e.g. a generator that isn't set up) */
		disabled?: boolean;
	}
</script>

<script lang="ts" generics="T extends string | number | boolean">
	import Icon from './icon.svelte';
	import FieldReset from './field-reset.svelte';

	/**
	 * A compact select for property rows (agents/research/EDITOR-INPUTS.md): the select itself (a secondary-button frame
	 * with the chevron, DESIGN.md "Controls"), an icon or short tag on its left instead of a caption above it.
	 * Unless `required`, the first option is the default – "(placeholder)" – and choosing it unsets the value
	 * (`undefined`); while a value is set, the reset button (FieldReset) at the right end does the same. A stored value
	 * that isn't an option is kept and shown with "(?)".
	 */
	let {
		value,
		options,
		onchange,
		label,
		icon,
		tag,
		placeholder = '',
		required = false,
		disabled = false,
		id,
		testid
	}: {
		value: T | null | undefined;
		options: PropOption<T>[];
		onchange: (value: T | undefined) => void;
		/** the accessible name and the tooltip */
		label: string;
		icon?: string;
		tag?: string;
		/** the default's name, shown as "(…)" in the empty option */
		placeholder?: string;
		required?: boolean;
		disabled?: boolean;
		id?: string;
		testid?: string;
	} = $props();

	const shown = $derived(value === undefined || value === null ? '' : String(value));
	const resettable = $derived(!required && !disabled && shown !== '');
	const unknown = $derived(shown !== '' && !options.some((o) => String(o.value) === shown));

	function pick(raw: string) {
		if (raw === '') return onchange(undefined);
		const o = options.find((x) => String(x.value) === raw);
		if (o) onchange(o.value);
	}
</script>

<span class="sc-prop-select" class:has-lead={!!(icon || tag)} class:has-reset={resettable} title={label} data-testid={testid}>
	<select {id} aria-label={label} {disabled} onchange={(e) => pick(e.currentTarget.value)}>
		{#if !required}<option value="" selected={shown === ''}>{placeholder ? `(${placeholder})` : '–'}</option>{/if}
		{#each options as o (String(o.value))}<option value={String(o.value)} selected={String(o.value) === shown} disabled={o.disabled}>{o.label}</option>{/each}
		{#if unknown}<option value={shown} selected>{shown} (?)</option>{/if}
	</select>
	{#if icon || tag}
		<span class="lead" aria-hidden="true">{#if icon}<Icon name={icon} size="1.1em" />{:else}{tag}{/if}</span>
	{/if}
	{#if resettable}<FieldReset onreset={() => onchange(undefined)} testid={testid ? `${testid}-reset` : undefined} />{/if}
</span>

<style>
	.sc-prop-select {
		position: relative;
		display: block;
		min-width: 0;
	}
	select {
		box-sizing: border-box;
		width: 100%;
		height: var(--control-height);
		margin: 0;
		padding: 0 calc(var(--space-05) + 1.25em) 0 var(--space-05);
		appearance: none;
		border: var(--border-width-regular) solid var(--border-muted);
		border-radius: var(--radius-none);
		background: var(--paper)
			url("data:image/svg+xml,%3Csvg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 24 24' fill='none' stroke='%23000' stroke-width='2.5' stroke-linecap='square'%3E%3Cpolyline points='6 9 12 15 18 9'/%3E%3C/svg%3E")
			no-repeat right var(--space-05) center / 1em;
		color: var(--ink);
		font: inherit;
		text-overflow: ellipsis;
		cursor: pointer;
		transition: border-color var(--duration-fast);
	}
	.has-lead select {
		padding-left: 1.75rem;
	}
	/* the reset takes the right end (as in every field); the chevron moves in front of it */
	.has-reset select {
		padding-right: calc(var(--space-05) + 1.25em + var(--field-reset-width, 1.5rem));
		background-position: right calc(var(--field-reset-width, 1.5rem) + var(--space-025)) center;
	}
	select:is(:hover, :focus):not(:disabled) {
		border-color: var(--ink);
	}
	select:disabled {
		cursor: not-allowed;
		color: var(--text-muted);
	}
	.lead {
		position: absolute;
		inset: 0 auto 0 0;
		display: flex;
		align-items: center;
		justify-content: center;
		width: 1.75rem;
		color: var(--text-muted);
		pointer-events: none;
	}
	@media (prefers-reduced-motion: reduce) {
		select {
			transition: none;
		}
	}
</style>
