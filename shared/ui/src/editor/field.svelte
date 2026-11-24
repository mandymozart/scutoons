<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLInputAttributes } from 'svelte/elements';
	import { dateText } from './date-text.ts';

	/**
	 * A labelled row with its field (deck-kit controls.rs `scrub` / `row`): the label muted at the left (42 % of the row, at
	 * most 6.86rem), the field in the rest – the control ground, its line, a control's corner, a control high. A control of
	 * its own goes in as `children`; else a text input bound to `value`. `bare`: the field alone, named by `label`, where
	 * the words stand elsewhere. `type="date"`: the day typed (`2026-11-15`, `15.11.2026`), never the browser's picker; a
	 * day that exists goes out through `ondate` as ISO (empty = unset), anything else shows the error line.
	 */
	type Props = {
		label: string;
		value?: string;
		bare?: boolean;
		ondate?: (day: string) => void;
		children?: Snippet;
		'data-kit-state'?: string;
	} & Omit<HTMLInputAttributes, 'value' | 'children'>;
	let { label, value = $bindable(''), type = 'text', bare = false, ondate, children, 'data-kit-state': kitState, ...rest }: Props = $props();

	let wrong = $state(false);
	const typed = (text: string) => {
		const day = dateText(text);
		wrong = day === null;
		if (day === null) return;
		value = day;
		ondate?.(day);
	};
</script>

{#snippet field()}
	{#if type === 'date'}
		<input
			{...rest}
			type="text"
			inputmode="numeric"
			autocomplete="off"
			spellcheck="false"
			placeholder={rest.placeholder ?? 'YYYY-MM-DD'}
			{value}
			aria-label={bare ? label : undefined}
			aria-invalid={wrong || undefined}
			oninput={() => (wrong = false)}
			onchange={(e) => typed(e.currentTarget.value)}
		/>
	{:else}
		<input {type} bind:value aria-label={bare ? label : undefined} {...rest} />
	{/if}
{/snippet}

{#if bare}
	{@render field()}
{:else}
	<label class="kit-field" data-kit-state={kitState}>
		<span class="name">{label}</span>
		{#if children}{@render children()}{:else}{@render field()}{/if}
	</label>
{/if}

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
