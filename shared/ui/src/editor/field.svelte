<script lang="ts">
	import type { Snippet } from 'svelte';
	import type { HTMLInputAttributes } from 'svelte/elements';

	/**
	 * A labelled row with its field (deck-kit controls.rs `scrub` / `row`): the label muted at the left (42 % of the row, at
	 * most 6.86rem), the field in the rest – the control ground, its line, a control's corner, a control high. A control of
	 * its own goes in as `children`; else a text input bound to `value`.
	 */
	type Props = { label: string; value?: string; children?: Snippet } & Omit<HTMLInputAttributes, 'value' | 'children'>;
	let { label, value = $bindable(''), type = 'text', children, ...rest }: Props = $props();
</script>

<label class="kit-field">
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
		border: var(--editor-edge) solid var(--tool-line);
		border-radius: var(--editor-radius);
		background: var(--tool-control);
		color: var(--tool-text);
		font: inherit;
		text-shadow: none;
	}
	input:hover {
		border-color: var(--tool-muted);
	}
	input:focus-visible {
		outline: 2px solid var(--tool-accent);
		outline-offset: -1px;
	}
</style>
