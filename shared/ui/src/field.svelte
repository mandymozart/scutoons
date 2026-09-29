<script lang="ts">
	import type { Snippet } from 'svelte';

	/**
	 * Form field (admin UI pattern `.field` / label / `.field-hint` / `.has-error` / `.field-error`).
	 * The control is the app's own input/select/textarea, rendered by `children` with the ids to wire up:
	 * `{#snippet children({ id, describedby, invalid })}<input {id} aria-describedby={describedby} aria-invalid={invalid} />{/snippet}`
	 */
	const uid = $props.id();

	let {
		label,
		hint,
		error,
		id: idProp,
		required = false,
		children
	}: {
		label: string;
		hint?: string;
		/** message shown instead of nothing; marks the field invalid */
		error?: string | null;
		id?: string;
		required?: boolean;
		children: Snippet<[{ id: string; describedby: string | undefined; invalid: boolean }]>;
	} = $props();

	const id = $derived(idProp ?? `field-${uid}`);
	const hintId = $derived(`${id}-hint`);
	const errorId = $derived(`${id}-error`);
	const describedby = $derived([hint ? hintId : null, error ? errorId : null].filter(Boolean).join(' ') || undefined);
</script>

<div class="field" class:has-error={!!error}>
	<label for={id}>
		{label}{#if required}<span class="required" aria-hidden="true"> *</span>{/if}
	</label>
	{@render children({ id, describedby, invalid: !!error })}
	{#if hint}
		<p class="field-hint" id={hintId}>{hint}</p>
	{/if}
	{#if error}
		<p class="field-error" id={errorId} role="alert">{error}</p>
	{/if}
</div>

<style>
	.field {
		display: flex;
		flex-direction: column;
		gap: var(--space-025);
		margin: 0 0 var(--space-1);
	}
	label {
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
		color: var(--text-muted);
	}
	.required {
		color: var(--ink);
	}
	.field :global(:is(input:not([type='checkbox']):not([type='radio']), select, textarea)) {
		box-sizing: border-box;
		width: 100%;
		margin: 0;
		padding: var(--space-05) var(--space-075);
		border: var(--border-width-hairline) solid var(--border-subtle);
		border-radius: var(--radius-none);
		background: var(--paper);
		color: var(--ink);
		font: inherit;
	}
	.field :global(:is(input, select, textarea):focus-visible) {
		outline: var(--border-width-regular) solid var(--ink);
		outline-offset: -1px;
	}
	.field-hint,
	.field-error {
		margin: 0;
		font-size: var(--type-caption-size);
		line-height: var(--type-caption-line);
	}
	.field-hint {
		color: var(--text-muted);
	}
	/* ink text + state-colored rule: the error red is below 4.5:1 as small text */
	.field-error {
		color: var(--ink);
		padding-left: var(--space-05);
		border-left: var(--border-width-regular) solid var(--state-error);
	}
	.has-error :global(:is(input, select, textarea)) {
		border-color: var(--state-error);
	}
</style>
