<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLInputAttributes } from 'svelte/elements';

	/**
	 * A labelled row with its field (deck-kit controls.rs `scrub` / `row`): the label muted at the left (42 % of the row, at
	 * most 6.86rem), the field in the rest – the control ground, its line, a control's corner, a control high. A control of
	 * its own goes in as `children`; else a text input bound to `value`.
	 */
	type Props = { label: string; value?: string; children?: Snippet; 'data-kit-state'?: string } & Omit<HTMLInputAttributes, 'value' | 'children'>;
	let { label, value = $bindable(''), type = 'text', children, 'data-kit-state': kitState, ...rest }: Props = $props();
</script>

<label class="kit-field" data-kit-state={kitState}>
	<span class="name">{label}</span>
	{#if children}{@render children()}{:else}<input {type} bind:value {...rest} />{/if}
</label>

<style>
	.kit-field {
		display: grid;
		grid-template-columns: min(42%, 6.86rem) minmax(0, 1fr);
		align-items: center;
		min-height: var(--editor-row);
	}
	.name {
		min-width: 0;
		overflow: hidden;
		color: var(--tool-muted);
		text-overflow: ellipsis;
		white-space: nowrap;
	}
	input {
		box-sizing: border-box;
		width: 100%;
		height: var(--control-height);
		margin: 0;
		padding: 0 var(--editor-pad);
		border: var(--editor-edge) solid var(--kit-field-rest-border);
		border-radius: var(--editor-radius);
		background: var(--kit-field-rest-ground);
		color: var(--kit-field-rest-text);
		font: inherit;
		text-shadow: none;
	}
	input::placeholder {
		color: var(--tool-muted);
	}
	input:hover,
	.kit-field[data-kit-state~='hover'] input {
		border-color: var(--kit-field-hover-border);
	}
	input:focus,
	.kit-field[data-kit-state~='on'] input {
		border-color: var(--kit-field-on-border);
		outline: none;
	}
	input:focus-visible,
	.kit-field[data-kit-state~='focus'] input {
		outline: 2px solid var(--kit-focus-rest-ring);
		outline-offset: 1px;
	}
	input:disabled {
		color: var(--kit-field-disabled-text);
		opacity: var(--kit-field-disabled-opacity);
		cursor: not-allowed;
	}
	input[aria-invalid='true'] {
		border-color: var(--kit-field-error-border);
	}
</style>
